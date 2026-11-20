use crate::Material;
use crate::MaterialExt;
use crate::MaterialUiExt as _;
use crate::Rt;
use crate::adaptive::Density;
use crate::msg;
use egui::{Align, Layout, Margin, Rangef, Rect, Sense, Vec2, pos2, vec2};
use egui_mineral::kit::state::TempState;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::Composite;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    focus::RovingFocus,
    kit,
    layout::{Axis, Constraints},
    perf::VirtualList,
    select::ItemKey,
    semantic::{self, Role, SemanticNode, StableId},
    widget::WidgetResponse,
};

use crate::actions::{IconButton, IconButtonVariant};
use crate::containment::Divider;
use crate::containment::{Card, CardVariant, ListItem, ListLines, ListSlot};
use crate::selection::CheckButton;
use crate::widget::{
    parts,
    row_parts::{self, RowColors, RowState},
};
use crate::{
    spec::{defaults, tokens::list},
    tr,
};
use std::collections::HashSet;

// No MD3 counterpart
const DEFAULT_LIST_WIDTH: f32 = 200.0;
const DEFAULT_ROWS: f32 = 4.0;

// Checked rows of each list, by the row's value id.
#[derive(Clone, Default)]
struct TransferState {
    source: HashSet<egui::Id>,
    target: HashSet<egui::Id>,
}

impl TempState for TransferState {}

#[derive(Clone, Copy)]
enum Side {
    Source,
    Target,
}

impl Side {
    const fn key(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Target => "target",
        }
    }
}

#[derive(Clone, Copy)]
enum Move {
    AllToTarget,
    ToTarget,
    ToSource,
    AllToSource,
}

impl Move {
    const ALL: [Self; 4] = [Self::AllToTarget, Self::ToTarget, Self::ToSource, Self::AllToSource];

    const fn key(self) -> &'static str {
        match self {
            Self::AllToTarget => "to-target-all",
            Self::ToTarget => "to-target",
            Self::ToSource => "to-source",
            Self::AllToSource => "to-source-all",
        }
    }

    const fn message(self) -> crate::Msg {
        match self {
            Self::AllToTarget => msg::TRANSFER_ALL_TO_TARGET,
            Self::ToTarget => msg::TRANSFER_TO_TARGET,
            Self::ToSource => msg::TRANSFER_TO_SOURCE,
            Self::AllToSource => msg::TRANSFER_ALL_TO_SOURCE,
        }
    }
}

// MD3 list container padding
fn list_padding() -> Margin {
    Margin::from(vec2(0.0, defaults::list::VERTICAL_PADDING))
}

/// Two lists moving items between them; non-MD3 (MUI `TransferList` pattern).
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct TransferList<'a, K> {
    salt: KeyedSalt,
    label: Option<String>,
    values: Vec<K>,
    labels: Vec<String>,
    chosen: &'a mut Vec<K>,
    source_title: Option<String>,
    target_title: Option<String>,
    disabled: bool,
}

impl<'a, K: ItemKey> TransferList<'a, K> {
    /// Items are split by `chosen`, the bound target values in target order; values no item carries and repeats
    /// are dropped. Agent key prefix `<id_salt>`, rows `<id_salt>-<side>-<value>`.
    pub fn new(id_salt: impl Into<KeyedSalt>, chosen: &'a mut Vec<K>) -> Self {
        Self {
            salt: id_salt.into(),
            label: None,
            values: Vec::new(),
            labels: Vec::new(),
            chosen,
            source_title: None,
            target_title: None,
            disabled: false,
        }
    }

    /// Appends an item carrying `value`.
    pub fn item(mut self, value: K, label: impl Into<String>) -> Self {
        self.values.push(value);
        self.labels.push(label.into());
        self
    }

    /// Appends `(value, label)` pairs.
    pub fn items(mut self, items: impl IntoIterator<Item = (K, impl Into<String>)>) -> Self {
        for (value, label) in items {
            self = self.item(value, label);
        }
        self
    }

    fn row_id(&self, i: usize) -> egui::Id {
        egui::Id::unique(&self.values[i])
    }

    /// Accessible name of the group; default `mineral-transfer`.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Source list header; default the `mineral-transfer-source` message.
    pub fn source_title(mut self, source_title: impl Into<String>) -> Self {
        self.source_title = Some(source_title.into());
        self
    }

    /// Target list header; default the `mineral-transfer-target` message.
    pub fn target_title(mut self, target_title: impl Into<String>) -> Self {
        self.target_title = Some(target_title.into());
        self
    }

    crate::widget::setters! {
        /// Greys out both lists and the move buttons.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    fn header_height(density: Density) -> f32 {
        row_parts::height(density, ListLines::Two)
    }

    fn emit_group(&self, rt: &Rt, id: egui::Id) {
        let ctx = rt.ctx();
        semantic::emit_parent(ctx, id, || {
            let name = crate::tr_or(rt.i18n(), self.label.as_deref(), msg::TRANSFER);
            let mut n = SemanticNode::new(StableId::of(&self.salt, "transfer"), Role::Group, name);
            n.state.disabled = self.disabled;
            n
        });
    }

    fn default_height(density: Density) -> f32 {
        Self::header_height(density)
            + crate::spec::tokens::divider::THICKNESS
            + DEFAULT_ROWS * row_parts::row_height(density)
            + list_padding().sum().y
    }

    fn list(
        &self,
        ui: &mut MineralUi<'_, Material>,
        side: Side,
        title: &str,
        rows: &[usize],
        sel: &mut HashSet<egui::Id>,
        extent: Vec2,
    ) {
        let rt = ui.rt();
        let height = extent.y;
        let key = side.key();
        Card::new(self.salt.child((key, "card")), title)
            .agent_key(StableId::of(&self.salt, (key, "card")))
            .variant(CardVariant::Outlined)
            .width(egui::Rangef::point(extent.x))
            .content_padding(list_padding())
            .show(ui, |ui| {
                ui.set_height((height - list_padding().sum().y).max(0.0));
                ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                self.header(ui, side, title, rows, sel);
                rt.ui(ui).mineral(Divider::new((key, "divider")));
                self.rows(ui, side, rows, sel);
            });
    }

    fn header(
        &self,
        ui: &mut MineralUi<'_, Material>,
        side: Side,
        title: &str,
        rows: &[usize],
        sel: &mut HashSet<egui::Id>,
    ) {
        let side = side.key();
        let rt = ui.rt();
        let t = rt.tokens();
        let all = !rows.is_empty() && rows.iter().all(|&i| sel.contains(&self.row_id(i)));
        let some = rows.iter().any(|&i| sel.contains(&self.row_id(i)));
        let mut on = all;
        let f = rt.i18n().format();
        let count = rt.i18n().tr(msg::TransferCount {
            selected: f.count(sel.len()).into(),
            total: f.count(rows.len()).into(),
        });
        let width = ui.available_width();
        let h = Self::header_height(ui.density());
        let (rect, _) = ui.allocate_exact_size(vec2(width, h), Sense::hover());
        let dir = rt.i18n().direction();
        let slot_w = row_parts::control_slot(ui.density());
        let start = defaults::list::START_PADDING;
        let slot_x = dir.start_x(rect, Rangef::new(start, start + slot_w));
        let slot = Rect::from_min_size(pos2(slot_x, rect.top()), vec2(slot_w, rect.height()));
        let r = egui_mineral::layout::child(
            ui,
            "all",
            slot,
            Layout::centered_and_justified(egui::Direction::TopDown),
            |ui| {
                ui.mineral(
                    CheckButton::new(self.salt.child((side, "all")), title, &mut on)
                        .label_visible(false)
                        .indeterminate(some && !all)
                        .disabled(self.disabled || rows.is_empty())
                        .agent_key(StableId::of(&self.salt, (side, "all-checkbox"))),
                )
            },
        )
        .inner;
        if r.changed() {
            if on {
                sel.extend(rows.iter().map(|&i| self.row_id(i)));
            } else {
                sel.clear();
            }
        }
        let text_w =
            rect.width() - start - slot_w - defaults::list::LEADING_CONTENT_END_PADDING - defaults::list::END_PADDING;
        let p = ui.painter();
        let col = RowColors::of(
            RowState {
                disabled: self.disabled,
                ..RowState::default()
            },
            &t,
        );
        let head = parts::type_wrapped(p, title, list::ITEM_LABEL_TEXT_FONT, col.label, text_w, 1);
        let sub = parts::type_wrapped(p, &count, list::ITEM_SUPPORTING_TEXT_FONT, col.supporting, text_w, 1);
        let mut y = rect.center().y - f32::midpoint(head.size().y, sub.size().y);
        let text_start = start + slot_w + defaults::list::LEADING_CONTENT_END_PADDING;
        for g in [head, sub] {
            let x = dir.start_x(rect, Rangef::new(text_start, text_start + g.size().x));
            let h = g.size().y;
            p.galley(pos2(x, y), g, egui::Color32::PLACEHOLDER);
            y += h;
        }
    }

    fn rows(&self, ui: &mut MineralUi<'_, Material>, side: Side, rows: &[usize], sel: &mut HashSet<egui::Id>) {
        let side = side.key();
        let rt = ui.rt();
        let row_h = row_parts::row_height(ui.density());
        let mut ids = Vec::new();
        VirtualList::uniform(self.salt.child((side, "rows")), row_h).show(ui, rows.len(), |ui, n| {
            let (i, value) = (rows[n], &self.values[rows[n]]);
            let row = self.row_id(i);
            let r = rt.ui(ui).mineral(
                ListItem::new(self.salt.child((side, value)), &self.labels[i])
                    .leading(ListSlot::new().checkbox(sel.contains(&row)))
                    .disabled(self.disabled)
                    .agent_key(StableId::of(&self.salt, (side, value))),
            );
            if r.output().leading_toggled && !sel.remove(&row) {
                sel.insert(row);
            }
            ids.push(r.id);
        });
        RovingFocus::new(Axis::Vertical)
            .wrap(false)
            .update(rt.core(), &ids, |_| true);
    }
}

impl<K: ItemKey> TransferList<'_, K> {
    fn apply(&mut self, m: Move, source: &[usize], st: &mut TransferState) {
        match m {
            Move::AllToTarget => {
                self.chosen.extend(source.iter().map(|&i| self.values[i].clone()));
                st.source.clear();
            }
            Move::ToTarget => {
                let picked: Vec<K> = source
                    .iter()
                    .filter(|&&i| st.source.contains(&self.row_id(i)))
                    .map(|&i| self.values[i].clone())
                    .collect();
                self.chosen.extend(picked);
                st.source.clear();
            }
            Move::ToSource => {
                let target = &st.target;
                self.chosen.retain(|v| !target.contains(&egui::Id::unique(v)));
                st.target.clear();
            }
            Move::AllToSource => {
                self.chosen.clear();
                st.target.clear();
            }
        }
    }
}

impl<K: ItemKey> Composite for TransferList<'_, K> {
    type Kit = Material;
    type Output = ();

    egui_mineral::kit::widget_meta!(salt);

    fn compose(&mut self, ui: &mut MineralUi<'_, Material>, constraints: Constraints) -> WidgetResponse {
        let rt = ui.rt();
        let base = ui.make_persistent_id(self.salt.id_salt());
        let state_id = base.with("transfer");
        self.emit_group(rt, base);
        let mut st = TransferState::load(rt.ctx(), state_id);
        let len = self.values.len();
        // Unknown and repeated values are dropped; repeats would share row ids.
        let mut chosen = vec![false; len];
        let values = &self.values;
        self.chosen.retain(|v| {
            values
                .iter()
                .position(|x| x == v)
                .is_some_and(|i| !std::mem::replace(&mut chosen[i], true))
        });
        let target: Vec<usize> = self
            .chosen
            .iter()
            .filter_map(|v| values.iter().position(|x| x == v))
            .collect();
        let source: Vec<usize> = (0..len).filter(|&i| !chosen[i]).collect();
        let checked = |set: &HashSet<egui::Id>, i: &usize| set.contains(&self.row_id(*i));
        let title = |custom: &Option<String>, id| crate::tr_or(rt.i18n(), custom.as_deref(), id);
        let src_title = title(&self.source_title, msg::TRANSFER_SOURCE);
        let dst_title = title(&self.target_title, msg::TRANSFER_TARGET);
        let dir = rt.i18n().direction();
        let icons = rt.icons();
        let icon = |m: Move| {
            let forward = matches!(m, Move::AllToTarget | Move::ToTarget);
            let right = dir.pick(forward, !forward);
            match (matches!(m, Move::AllToTarget | Move::AllToSource), right) {
                (false, true) => icons.chevron_right,
                (false, false) => icons.chevron_left,
                (true, true) => icons.keyboard_double_arrow_right,
                (true, false) => icons.keyboard_double_arrow_left,
            }
        };
        let src_any = source.iter().any(|i| checked(&st.source, i));
        let dst_any = target.iter().any(|i| checked(&st.target, i));
        let mut moved = None;
        let gap = crate::adaptive::grid(2);
        let button = crate::adaptive::min_interactive(ui.density());
        let total = constraints.constrain(vec2(
            2.0 * DEFAULT_LIST_WIDTH + button + 2.0 * gap,
            Self::default_height(ui.density()),
        ));
        let height = total.y;
        let list = vec2(((total.x - button - 2.0 * gap) / 2.0).max(f32::MIN_POSITIVE), height);
        let layout = dir.row().with_cross_align(Align::Min);
        let inner = ui.allocate_ui_with_layout(total, layout, |ui| {
            ui.spacing_mut().item_spacing.x = gap;
            self.list(&mut rt.ui(ui), Side::Source, &src_title, &source, &mut st.source, list);
            ui.allocate_ui_with_layout(vec2(button, height), Layout::top_down(Align::Center), |ui| {
                let n = Move::ALL.len() as f32;
                ui.spacing_mut().item_spacing.y = crate::adaptive::grid(1);
                ui.add_space(((height - n * button - (n - 1.0) * crate::adaptive::grid(1)) / 2.0).max(0.0));
                for m in Move::ALL {
                    let enabled = match m {
                        Move::AllToTarget => !source.is_empty(),
                        Move::ToTarget => src_any,
                        Move::ToSource => dst_any,
                        Move::AllToSource => !self.chosen.is_empty(),
                    };
                    let k = m.key();
                    let r = rt.ui(ui).mineral(
                        IconButton::new(self.salt.child(k), icon(m), tr(rt.i18n(), m.message()))
                            .variant(IconButtonVariant::Outlined)
                            .disabled(self.disabled || !enabled)
                            .agent_key(StableId::of(&self.salt, (k, "button"))),
                    );
                    if r.clicked() {
                        moved = Some(m);
                    }
                }
            });
            self.list(&mut rt.ui(ui), Side::Target, &dst_title, &target, &mut st.target, list);
        });
        if let Some(m) = moved {
            self.apply(m, &source, &mut st);
        }
        st.store(rt.ctx(), state_id);
        kit::nodes::remember_rect(rt.ctx(), base, inner.response.rect);
        let response = inner.response;
        WidgetResponse::new(response, ()).with_changed(moved.is_some())
    }
}
