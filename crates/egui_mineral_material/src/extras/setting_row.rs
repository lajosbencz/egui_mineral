use crate::Material;
use crate::MaterialExt;
use crate::MaterialUiExt as _;
use crate::adaptive::Density;
use egui_mineral::i18n::Sides;
use egui_mineral::semantic::AgentKey;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;
use std::sync::Arc;

use egui::{Galley, Rangef, Rect, Sense, Ui, Vec2, pos2, vec2};
use egui_mineral::{
    i18n::Direction,
    layout, perf,
    semantic::{self, Role, SemanticNode},
};

use crate::containment::ListLines;
use crate::spec::{defaults::list as d, tokens::list};
use crate::widget::{
    parts,
    row_parts::{self, RowColors},
};

// No MD3 counterpart
const PAD_START: f32 = d::START_PADDING;
const PAD_END: f32 = d::END_PADDING;
const PAD_Y: f32 = d::VERTICAL_PADDING;
const GAP: f32 = d::TRAILING_CONTENT_START_PADDING;

/// Result of [`SettingRow::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SettingRowResponse<R> {
    /// Row response.
    pub response: egui::Response,
    /// Output of the control closure.
    pub inner: R,
    /// Label above the control: the row is narrower than the first layout breakpoint.
    pub stacked: bool,
}

/// Label and optional supporting text on the leading side, the control (or a read-only value)
/// in the trailing column. Rows narrower than the first [`layout::breakpoint`] stack the label
/// above the control.
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct SettingRow {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    supporting: Option<String>,
}

struct Text {
    label: Arc<Galley>,
    supporting: Option<Arc<Galley>>,
}

impl Text {
    fn height(&self) -> f32 {
        self.label.size().y + self.supporting.as_ref().map_or(0.0, |s| s.size().y)
    }

    fn paint(&self, ui: &Ui, rect: Rect, dir: Direction) {
        let p = ui.painter();
        let mut y = rect.center().y - self.height() / 2.0;
        for g in std::iter::once(&self.label).chain(self.supporting.as_ref()) {
            let x = dir.start_x(rect, Rangef::new(0.0, 0.0 + g.size().x));
            p.galley(pos2(x, y), g.clone(), egui::Color32::PLACEHOLDER);
            y += g.size().y;
        }
    }
}

impl SettingRow {
    /// `label` is the leading text and the control's accessible name. Agent key `<id_salt>-setting`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            key: egui_mineral::agent_key!("setting"),
            salt: id_salt,
            label: label.into(),
            supporting: None,
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-setting` agent key.
        agent_key(key)
    }

    /// Secondary line below the label.
    pub fn supporting(mut self, supporting: impl Into<String>) -> Self {
        self.supporting = Some(supporting.into());
        self
    }

    fn text(&self, ui: &MineralUi<'_, Material>, width: f32) -> Text {
        let col = colors(ui);
        let p = ui.painter();
        Text {
            label: parts::type_wrapped(p, &self.label, list::ITEM_LABEL_TEXT_FONT, col.label, width, 0),
            supporting: self
                .supporting
                .as_ref()
                .map(|s| parts::type_wrapped(p, s, list::ITEM_SUPPORTING_TEXT_FONT, col.supporting, width, 0)),
        }
    }

    fn min_height(&self, density: Density) -> f32 {
        row_parts::height(
            density,
            if self.supporting.is_some() {
                ListLines::Two
            } else {
                ListLines::One
            },
        )
    }

    /// `control` gets the row label: pass it as the control's accessible name and hide the
    /// control's own visible label (`label_visible(false)`), so the name is announced once.
    /// Supporting text is emitted as its own label node.
    pub fn show<R>(
        self,
        ui: &mut MineralUi<'_, Material>,
        control: impl FnOnce(&mut MineralUi<'_, Material>, &str) -> R,
    ) -> SettingRowResponse<R> {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let dir = rt.i18n().direction();
        let id = ui.make_persistent_id(self.salt.id_salt());
        let w = ui.available_width();
        let stacked = layout::breakpoint(rt.core(), w) == 0;
        let inner_w = w - PAD_START - PAD_END;
        let (response, inner) = if stacked {
            let text = self.text(ui, inner_w);
            let (rect, _) = ui.allocate_exact_size(vec2(w, text.height() + 2.0 * PAD_Y), Sense::hover());
            text.paint(ui, edge_inset(rect, dir), dir);
            let row = ui.allocate_ui_with_layout(vec2(w, 0.0), dir.row(), |ui| {
                ui.add_space(PAD_START);
                control(&mut rt.ui(ui), &self.label)
            });
            ui.add_space(PAD_Y);
            (
                ui.interact(rect.union(row.response.rect), id, Sense::hover()),
                row.inner,
            )
        } else {
            let memo = id.with("control");
            let ctrl = perf::size_or_discard(ctx, memo).unwrap_or(Vec2::ZERO);
            let text = self.text(ui, inner_w - ctrl.x - GAP);
            let h = (text.height().max(ctrl.y) + 2.0 * PAD_Y).max(self.min_height(ui.density()));
            let (rect, _) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
            let response = ui.interact(rect, id, Sense::hover());
            let content = edge_inset(rect, dir);
            text.paint(ui, content, dir);
            let end = dir.reversed().row().with_cross_align(egui::Align::Min);
            // Top-aligned so the measured size is the control's own; centered with last pass's size.
            let top = content.center().y - ctrl.y / 2.0;
            let col_rect = Rect::from_x_y_ranges(content.x_range(), top..=content.bottom().max(top));
            let col = egui_mineral::layout::child(ui, self.salt.child("control"), col_rect, end, |ui| {
                control(ui, &self.label)
            });
            perf::store_size(ctx, memo, col.response.rect.size());
            (response, col.inner)
        };
        self.emit_supporting(ui, response.rect);
        SettingRowResponse {
            response,
            inner,
            stacked,
        }
    }

    /// Read-only row: `value` sits in the control column (below the label when stacked). The
    /// row is one label node named by the label, carrying `value`.
    pub fn show_value(self, ui: &mut MineralUi<'_, Material>, value: impl Into<String>) -> SettingRowResponse<()> {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let value = value.into();
        let dir = rt.i18n().direction();
        let col = colors(ui);
        let id = ui.make_persistent_id(self.salt.id_salt());
        let w = ui.available_width();
        let stacked = layout::breakpoint(rt.core(), w) == 0;
        let inner_w = w - PAD_START - PAD_END;
        let value_galley = |ui: &Ui, width: f32| {
            parts::type_wrapped(
                ui.painter(),
                &value,
                list::ITEM_TRAILING_SUPPORTING_TEXT_FONT,
                col.trailing,
                width,
                0,
            )
        };
        let response = if stacked {
            let text = self.text(ui, inner_w);
            let v = value_galley(ui, inner_w);
            let h = text.height() + v.size().y + 2.0 * PAD_Y;
            let min_h = self.min_height(ui.density());
            let (rect, response) = ui.allocate_exact_size(vec2(w, h.max(min_h)), Sense::hover());
            let content = edge_inset(rect, dir);
            let label_rect = Rect::from_min_size(content.min, vec2(content.width(), text.height()));
            text.paint(ui, label_rect, dir);
            let x = dir.start_x(content, Rangef::new(0.0, 0.0 + v.size().x));
            ui.painter()
                .galley(pos2(x, label_rect.bottom()), v, egui::Color32::PLACEHOLDER);
            response
        } else {
            let v = value_galley(ui, f32::INFINITY);
            let text = self.text(ui, inner_w - v.size().x - GAP);
            let h = (text.height().max(v.size().y) + 2.0 * PAD_Y).max(self.min_height(ui.density()));
            let (rect, response) = ui.allocate_exact_size(vec2(w, h), Sense::hover());
            let content = edge_inset(rect, dir);
            text.paint(ui, content, dir);
            let x = dir.reversed().start_x(content, Rangef::new(0.0, 0.0 + v.size().x));
            ui.painter().galley(
                pos2(x, content.center().y - v.size().y / 2.0),
                v,
                egui::Color32::PLACEHOLDER,
            );
            response
        };
        let response = ui.interact(response.rect, id, Sense::hover());
        semantic::emit(ctx, Some(response.id), response.rect, || {
            let mut n = SemanticNode::new(self.key.id(&self.salt), Role::Label, &self.label);
            n.value = Some(value);
            n
        });
        self.emit_supporting(ui, response.rect);
        SettingRowResponse {
            response,
            inner: (),
            stacked,
        }
    }

    fn emit_supporting(&self, ui: &egui::Ui, rect: Rect) {
        if let Some(s) = &self.supporting {
            semantic::emit_in(ui, None, rect, || {
                SemanticNode::new(self.key.id(&self.salt).sub("supporting"), Role::Label, s)
            });
        }
    }
}

fn colors(ui: &MineralUi<'_, Material>) -> RowColors {
    let rt = ui.rt();
    RowColors::of(row_parts::RowState::default(), &rt.tokens())
}

fn edge_inset(rect: Rect, dir: Direction) -> Rect {
    let Sides { left, right } = dir.sides(PAD_START, PAD_END);
    Rect::from_x_y_ranges(rect.left() + left..=rect.right() - right, rect.y_range())
}
