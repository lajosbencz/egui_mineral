use crate::Material;
use crate::MaterialExt;
use crate::MaterialUiExt as _;
use egui_mineral::i18n::I18n;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;
use std::sync::Arc;

use egui::{Color32, Galley, Rangef, Rect, Sense, Shape, Ui, UiBuilder, pos2, scroll_area::ScrollBarVisibility, vec2};
use egui_mineral::widget::Availability;
use egui_mineral::{
    focus::{self, RovingFocus},
    i18n::Direction,
    kit,
    layout::{Axis, Constraints, Measure},
    semantic::{self, Role, SemanticNode, StableId},
    shortcut::Mnemonic,
    widget::{MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::widget::{indication, parts};
use crate::{
    MaterialIcon, motion,
    spec::{
        defaults::tab as tx,
        tokens::{divider, primary_navigation_tab as primary, secondary_navigation_tab as secondary},
    },
    tokens::Tokens,
};
use crate::{
    SelectionResponse,
    widget::common::{self, Interaction},
};

/// Compose `PrimaryTabRow` and `SecondaryTabRow` styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum TabsVariant {
    #[default]
    /// `PrimaryTabRow`: content-width indicator.
    Primary,
    /// `SecondaryTabRow`: tab-width indicator.
    Secondary,
}

/// Entry of [`Tabs`].
#[derive(Clone, Debug)]
#[must_use]
pub struct Tab {
    icon: Option<MaterialIcon>,
    label: String,
    disabled: bool,
    mnemonic: Option<Mnemonic>,
    salt: Option<egui_mineral::semantic::KeyedSalt>,
}

impl Tab {
    crate::widget::setters! {
        /// Default the index.
        item_salt
    }

    /// `label` is the visible text and accessible name.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            icon: None,
            label: label.into(),
            disabled: false,
            salt: None,
            mnemonic: None,
        }
    }

    /// Stacked above the label.
    pub fn icon(mut self, icon: MaterialIcon) -> Self {
        self.icon = Some(icon);
        self
    }
    crate::widget::setters! {
        /// Greys out the tab; it cannot be selected.
        disabled
    }

    /// Alt+letter selects the tab.
    pub fn mnemonic(mut self, mnemonic: char) -> Self {
        self.mnemonic = Some(Mnemonic::new(mnemonic));
        self
    }
}

/// scrollable tabs size to content. Arrow keys move focus and select (automatic activation).
#[derive(Debug)]
#[must_use = "call .show(..)"]
pub struct Tabs<'a> {
    salt: KeyedSalt,
    label: String,
    selected: &'a mut usize,
    specs: Vec<Tab>,
    variant: TabsVariant,
    scrollable: bool,
    disabled: bool,
}

impl<'a> Tabs<'a> {
    /// `label` names the tab list for assistive technology.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, selected: &'a mut usize) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            label: label.into(),
            selected,
            specs: Vec::new(),
            variant: TabsVariant::default(),
            scrollable: false,
            disabled: false,
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    /// Appends one tab.
    pub fn item(mut self, tab: impl Into<Tab>) -> Self {
        self.specs.push(tab.into());
        self
    }

    /// Appends tabs.
    pub fn items(mut self, tabs: impl IntoIterator<Item = impl Into<Tab>>) -> Self {
        self.specs.extend(tabs.into_iter().map(Into::into));
        self
    }

    crate::widget::setters! {
        /// Disables every tab.
        disabled
    }

    /// Default [`TabsVariant::Primary`].
    pub fn variant(mut self, variant: TabsVariant) -> Self {
        self.variant = variant;
        self
    }

    /// `ScrollableTabRow`: tabs size to content and scroll; default fixed equal widths.
    pub fn scrollable(mut self, scrollable: bool) -> Self {
        self.scrollable = scrollable;
        self
    }

    /// Paints the row and updates `selected` on a pick; a `selected` past the last tab moves onto it.
    pub fn show(mut self, ui: &mut MineralUi<'_, Material>) -> SelectionResponse {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let t = rt.tokens();
        for s in &mut self.specs {
            s.disabled |= self.disabled;
        }
        let n = self.specs.len();
        crate::navigation::nav::clamp_selected(self.selected, n, "Tabs selected");
        let id = ui.make_persistent_id(self.salt.id_salt());
        let mut sel = *self.selected;
        let contents: Vec<TabContent> = self.specs.iter().map(|s| TabContent::new(ui, s)).collect();
        let height = contents
            .iter()
            .map(TabContent::height)
            .fold(primary::CONTAINER_HEIGHT, f32::max);
        let height = ui.dense(height, crate::adaptive::Density::MIN);
        let outer_w = ui.available_width();
        let bar = Rect::from_min_size(ui.cursor().min, vec2(outer_w, height));
        let container = ui.interact(bar, id.with("tablist"), Sense::hover());
        let divider_shape = ui.painter().add(Shape::Noop);
        let fixed_w = outer_w / n.max(1) as f32;
        // (tab width, indicator content width) per `TabRowImpl` / `ScrollableTabRowImpl`.
        let widths: Vec<(f32, f32)> = contents
            .iter()
            .map(|c| {
                let intrinsic = c.width();
                let pad = 2.0 * tx::TEXT_PADDING;
                if self.scrollable {
                    (
                        intrinsic.max(tx::SCROLLABLE_MIN_TAB_WIDTH),
                        (intrinsic - pad).max(tx::INDICATOR_MIN_WIDTH),
                    )
                } else {
                    (fixed_w, (intrinsic.min(fixed_w) - pad).max(tx::INDICATOR_MIN_WIDTH))
                }
            })
            .collect();
        let tab_stop = focus::tab_stop(ctx, id, Some(sel), self.specs.len(), |i| {
            self.specs.get(i).is_some_and(|s| !s.disabled)
        });
        let dir = rt.i18n().direction();
        let mut picked = None;
        let row = TabRow {
            id,
            contents: &contents,
            widths: &widths,
            specs: &self.specs,
            tab_stop,
            height,
            dir,
        };
        let mut body = |ui: &mut Ui| self.show_row(&mut rt.ui(ui), &row, &mut sel, &mut picked, &t);
        let layout = rt.i18n().direction().row();
        ui.scope_builder(UiBuilder::new().max_rect(bar).layout(layout), |ui| {
            if self.scrollable {
                egui::ScrollArea::horizontal()
                    .id_salt(id)
                    .scroll_bar_visibility(ScrollBarVisibility::AlwaysHidden)
                    .show(ui, body);
            } else {
                body(ui);
            }
        });
        // `HorizontalDivider`, under the indicator.
        let line = Rect::from_x_y_ranges(bar.x_range(), bar.bottom() - divider::THICKNESS..=bar.bottom());
        ui.painter()
            .set(divider_shape, Shape::rect_filled(line, 0.0, t.role(divider::COLOR)));

        let response = container;
        let changed = picked.is_some() && sel != *self.selected;
        if changed {
            *self.selected = sel;
        }
        semantic::emit(ctx, Some(response.id), bar, || {
            let mut n = SemanticNode::new(StableId::of(&self.salt, "tablist"), Role::TabList, &self.label);
            n.state.disabled = self.disabled;
            n
        });
        SelectionResponse::new(response, changed.then_some(sel), changed)
    }

    fn show_row(
        &self,
        ui: &mut MineralUi<'_, Material>,
        row: &TabRow<'_>,
        sel: &mut usize,
        picked: &mut Option<usize>,
        t: &Tokens,
    ) {
        let rt = ui.rt();
        let mut ids = Vec::with_capacity(self.specs.len());
        let mut responses = Vec::with_capacity(self.specs.len());
        ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
        if self.scrollable {
            // The scroll content starts viewport-wide; tabs past it would shrink to fit.
            let total: f32 = row.widths.iter().map(|w| w.0).sum();
            ui.set_min_width(total + 2.0 * tx::SCROLLABLE_EDGE_PADDING);
            ui.add_space(tx::SCROLLABLE_EDGE_PADDING);
        }
        for (i, (spec, content)) in self.specs.iter().zip(row.contents).enumerate() {
            let own = crate::widget::item_salt(&self.salt, spec.salt.as_ref(), None, i);
            let r = rt.ui(ui).mineral(TabItem {
                salt: own.child("tab"),
                key: StableId::of(&own, "tab"),
                spec,
                content,
                variant: self.variant,
                selected: i == *sel,
                tab_stop: focus::TabStop::at(i, row.tab_stop),
                width: row.widths[i].0,
                height: row.height,
            });
            if r.changed() {
                *picked = Some(i);
            }
            ids.push(r.id);
            responses.push(r.response().clone());
        }
        if self.scrollable {
            ui.add_space(tx::SCROLLABLE_EDGE_PADDING);
        }
        if let Some(i) = *picked {
            *sel = i;
        }
        if let Some(i) = RovingFocus::new(Axis::Horizontal).update_group(ui.core(), row.id, &ids, |i| {
            row.specs.get(i).is_some_and(|s| !s.disabled)
        }) {
            *sel = i;
            *picked = Some(i);
        }
        if picked.is_some()
            && let Some(r) = responses.get(*sel)
        {
            r.scroll_to_me(Some(egui::Align::Center));
        }
        if let (Some(first), Some(r)) = (responses.first(), responses.get(*sel)) {
            let indicator = Indicator {
                origin: row.dir.start_edge(first.rect),
                dir: row.dir,
                tab: r.rect,
                content_width: row.widths[*sel].1,
            };
            indicator.paint(ui, row.id, self.variant, t);
        }
    }
}

struct TabRow<'a> {
    id: egui::Id,
    contents: &'a [TabContent],
    widths: &'a [(f32, f32)],
    specs: &'a [Tab],
    tab_stop: usize,
    height: f32,
    dir: Direction,
}

// Compose TabIndicatorOffsetNode
struct Indicator {
    // First tab start edge; keeps the spring immune to scrolling
    origin: f32,
    dir: Direction,
    tab: Rect,
    content_width: f32,
}

impl Indicator {
    fn paint(
        &self,
        ui: &egui_mineral::widget::MineralUi<'_, crate::Material>,
        id: egui::Id,
        variant: TabsVariant,
        t: &Tokens,
    ) {
        let rt = ui.rt();
        // Logical distance from the first tab's start edge.
        let start = self.dir.sign() * (self.dir.start_edge(self.tab) - self.origin);
        let start =
            egui_mineral::motion::spring(rt.core(), id.with("indicator-offset"), start, motion::DEFAULT_SPATIAL);
        let tab_w = egui_mineral::motion::spring(
            rt.core(),
            id.with("indicator-tab"),
            self.tab.width(),
            motion::DEFAULT_SPATIAL,
        );
        let left = self.dir.offset_x(self.origin, Rangef::new(start, start + tab_w));
        let (width, radius) = match variant {
            TabsVariant::Primary => (
                egui_mineral::motion::spring(
                    rt.core(),
                    id.with("indicator-width"),
                    self.content_width,
                    motion::DEFAULT_SPATIAL,
                ),
                primary::ACTIVE_INDICATOR_SHAPE,
            ),
            TabsVariant::Secondary => (tab_w, egui::CornerRadius::ZERO),
        };
        let center = left + tab_w / 2.0;
        let h = primary::ACTIVE_INDICATOR_HEIGHT;
        let rect = Rect::from_x_y_ranges(
            center - width / 2.0..=center + width / 2.0,
            self.tab.bottom() - h..=self.tab.bottom(),
        );
        ui.painter()
            .rect_filled(rect, radius, t.role(primary::ACTIVE_INDICATOR_COLOR));
    }
}

// Compose TabBaselineLayout
struct TabContent {
    label: Arc<Galley>,
    icon: Option<MaterialIcon>,
}

impl TabContent {
    fn new(ui: &MineralUi<'_, Material>, spec: &Tab) -> Self {
        let rt = ui.rt();
        Self {
            label: parts::label_galley(
                rt.core(),
                ui.painter(),
                &spec.label,
                primary::LABEL_TEXT_FONT.font(ui.ctx()),
                Color32::PLACEHOLDER,
                spec.mnemonic,
            ),
            icon: spec.icon,
        }
    }

    fn width(&self) -> f32 {
        let text = self.label.size().x + 2.0 * tx::TEXT_PADDING;
        if self.icon.is_some() {
            text.max(primary::ICON_SIZE)
        } else {
            text
        }
    }

    fn height(&self) -> f32 {
        let text = self.label.size().y;
        if self.icon.is_some() {
            tx::LARGE_HEIGHT.max(primary::ICON_SIZE + text + tx::ICON_DISTANCE_FROM_BASELINE)
        } else {
            primary::CONTAINER_HEIGHT.max(text)
        }
    }

    fn baseline(&self) -> f32 {
        self.label
            .rows
            .first()
            .and_then(|r| r.row.glyphs.first().map(|g| r.pos.y + g.pos.y))
            .unwrap_or_else(|| self.label.size().y)
    }
}

struct TabItem<'a> {
    salt: KeyedSalt,
    key: StableId,
    spec: &'a Tab,
    content: &'a TabContent,
    variant: TabsVariant,
    selected: bool,
    tab_stop: focus::TabStop,
    width: f32,
    height: f32,
}

impl TabItem<'_> {
    // Tokens give inactive tabs own roles, unlike Compose Tab; MD3 has no disabled tab
    fn colors(&self, i: Interaction, t: &Tokens) -> (Color32, Color32) {
        if i.disabled {
            let c = common::disabled_content(t);
            return (c, c);
        }
        let (label, icon) = match (self.variant, self.selected) {
            (TabsVariant::Primary, true) => (primary::ACTIVE_LABEL_TEXT_COLOR, primary::ACTIVE_ICON_COLOR),
            (TabsVariant::Primary, false) if i.pressed => (
                primary::INACTIVE_PRESSED_LABEL_TEXT_COLOR,
                primary::INACTIVE_PRESSED_ICON_COLOR,
            ),
            (TabsVariant::Primary, false) if i.hovered => (
                primary::INACTIVE_HOVER_LABEL_TEXT_COLOR,
                primary::INACTIVE_HOVER_ICON_COLOR,
            ),
            (TabsVariant::Primary, false) if i.focused => (
                primary::INACTIVE_FOCUS_LABEL_TEXT_COLOR,
                primary::INACTIVE_FOCUS_ICON_COLOR,
            ),
            (TabsVariant::Primary, false) => (primary::INACTIVE_LABEL_TEXT_COLOR, primary::INACTIVE_ICON_COLOR),
            (TabsVariant::Secondary, true) => (secondary::ACTIVE_LABEL_TEXT_COLOR, secondary::ACTIVE_ICON_COLOR),
            (TabsVariant::Secondary, false) if i.pressed => {
                (secondary::PRESSED_LABEL_TEXT_COLOR, secondary::PRESSED_ICON_COLOR)
            }
            (TabsVariant::Secondary, false) if i.hovered => {
                (secondary::HOVER_LABEL_TEXT_COLOR, secondary::HOVER_ICON_COLOR)
            }
            (TabsVariant::Secondary, false) if i.focused => {
                (secondary::FOCUS_LABEL_TEXT_COLOR, secondary::FOCUS_ICON_COLOR)
            }
            (TabsVariant::Secondary, false) => (secondary::INACTIVE_LABEL_TEXT_COLOR, secondary::INACTIVE_ICON_COLOR),
        };
        (t.role(label), t.role(icon))
    }
}

impl MineralWidget for TabItem<'_> {
    type Kit = Material;
    type State = ();
    type Output = ();

    egui_mineral::kit::widget_meta!(salt);

    fn sense(&self) -> Sense {
        focus::roving_sense(Sense::click(), self.tab_stop)
    }

    fn enabled(&self) -> bool {
        !self.spec.disabled
    }

    fn access_key(&self) -> Option<Mnemonic> {
        self.spec.mnemonic
    }

    fn layout(&self, _ui: &MineralUi<'_, Material>, constraints: Constraints, (): &Self::State) -> Measure {
        constraints.constrain(vec2(self.width, self.height)).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Material>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let response = &frame.response;
        kit::focus::follow_press(response);
        let rect = frame.zones.visual;
        let dis = self.spec.disabled;
        let press = parts::press(ui, frame, Availability::disabled_if(dis));
        let i = press.interaction;
        let clicked = press.clicked;
        let label = &self.content.label;
        let (fg, icon_fg) = self.colors(i, &t);
        let p = ui.painter();
        let text_x = rect.center().x - label.size().x / 2.0;
        match self.content.icon {
            Some(icon) => {
                // `placeTextAndIcon`: baselines measured up from the tab bottom.
                let baseline = rect.bottom() - tx::TEXT_BASELINE_WITH_ICON - primary::ACTIVE_INDICATOR_HEIGHT;
                let text_top = baseline - self.content.baseline();
                let icon_bottom = baseline - tx::ICON_DISTANCE_FROM_BASELINE;
                common::icon(
                    p,
                    pos2(rect.center().x, icon_bottom - primary::ICON_SIZE / 2.0),
                    icon,
                    primary::ICON_SIZE,
                    icon_fg,
                );
                p.galley(pos2(text_x, text_top), label.clone(), fg);
            }
            None => {
                p.galley(pos2(text_x, rect.center().y - label.size().y / 2.0), label.clone(), fg);
            }
        }
        // `Tab`: bounded ripple in the content color.
        indication::paint(ui, response, rect, egui::CornerRadius::ZERO, fg, i, &t);
        WidgetResponse::new(response.clone(), ()).with_changed(clicked && !self.selected)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(self.key.clone(), Role::Tab, &self.spec.label)
            .clickable(Availability::disabled_if(self.spec.disabled));
        n.state.selected = Some(self.selected);
        Some(n)
    }
}
