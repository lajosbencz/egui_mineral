use egui_mineral::kit::choice::Choice;
use egui_mineral::select::ItemKey;
use std::collections::{HashMap, HashSet};

use egui::{Frame, Id, Margin, Panel, Sense, Stroke, UiBuilder, pos2, text::LayoutJob, vec2};
use egui_mineral::{
    i18n::{Direction, I18n},
    kit,
    kit::paint::FocusStroke,
    layout::{Constraints, Measure},
    semantic::{self, AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{Availability, Interaction, MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::{
    Fluent, FluentExt, Icon,
    spec::ColorToken::{
        ColorCompoundBrandForeground1, ColorNeutralBackground4, ColorNeutralBackground4Hover,
        ColorNeutralBackground4Pressed, ColorNeutralForeground1, ColorNeutralForeground2,
        ColorNeutralForeground2BrandSelected, ColorStrokeFocus2,
    },
    spec::{defaults::nav as d, tokens::geometry as g},
    surfaces::Divider,
    theme::Radius,
    typography::TypeRamp,
    widget::paint,
};

/// `density` of a [`Nav`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NavDensity {
    /// `medium`: 40pt rows.
    #[default]
    Medium,
    /// `small`: 28pt rows.
    Small,
}

macro_rules! row_setters {
    () => {
        crate::widget::setters! {
            item_salt
        }
    };
}

// Binding of a nav's selection with its lifetime erased, so `NavUi` needs none for it.
trait Bound<K> {
    fn holds(&self, value: &K) -> bool;
    fn holds_id(&self, id: Id) -> bool;
    fn write(&mut self, value: K) -> bool;
}

impl<K: ItemKey> Bound<K> for Choice<'_, K> {
    fn holds_id(&self, id: Id) -> bool {
        self.selected().any(|v| Id::unique(v) == id)
    }

    fn holds(&self, value: &K) -> bool {
        self.contains(value)
    }

    fn write(&mut self, value: K) -> bool {
        self.set(value)
    }
}

/// Fluent `NavItem`: a link row of a [`Nav`].
#[derive(Clone, Debug)]
#[must_use]
pub struct NavItem<K> {
    value: K,
    label: String,
    icon: Option<Icon>,
    salt: Option<KeyedSalt>,
}

impl<K> NavItem<K> {
    /// `value` is what the nav's `selected` holds while this item is selected; `label` is the visible text and
    /// accessible name.
    pub fn new(value: K, label: impl Into<String>) -> Self {
        Self {
            value,
            label: label.into(),
            icon: None,
            salt: None,
        }
    }

    /// `icon` slot; the filled glyph shows while selected.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    row_setters!();
}

/// Fluent `NavSubItem`: an indented link row inside a [`NavCategory`].
#[derive(Clone, Debug)]
#[must_use]
pub struct NavSubItem<K> {
    value: K,
    label: String,
    salt: Option<KeyedSalt>,
}

impl<K> NavSubItem<K> {
    /// `value` is what the nav's `selected` holds while this item is selected.
    pub fn new(value: K, label: impl Into<String>) -> Self {
        Self {
            value,
            label: label.into(),
            salt: None,
        }
    }

    row_setters!();
}

/// Fluent `NavCategory`: a row that expands its sub items. Closed, it carries the selection indicator while one
/// of its sub items is selected (known once the category has been open).
#[derive(Clone, Debug)]
#[must_use]
pub struct NavCategory<K> {
    value: K,
    label: String,
    icon: Option<Icon>,
    salt: Option<KeyedSalt>,
}

impl<K> NavCategory<K> {
    /// `value` identifies the category among the nav's open categories.
    pub fn new(value: K, label: impl Into<String>) -> Self {
        Self {
            value,
            label: label.into(),
            icon: None,
            salt: None,
        }
    }

    /// `icon` slot; the filled glyph shows while a sub item is selected.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    row_setters!();
}

/// Fluent `NavSectionHeader`: a caption above a group of rows.
#[derive(Clone, Debug)]
#[must_use]
pub struct NavSectionHeader(String);

impl NavSectionHeader {
    /// Visible text.
    pub fn new(text: impl Into<String>) -> Self {
        Self(text.into())
    }
}

impl<T: Into<String>> From<T> for NavSectionHeader {
    fn from(text: T) -> Self {
        Self::new(text)
    }
}

/// Fluent `NavDivider`: a [`Divider`] with 4pt above and below.
#[derive(Clone, Debug)]
#[must_use]
pub struct NavDivider {
    salt: KeyedSalt,
}

impl NavDivider {
    /// Keyed by `id_salt`.
    pub fn new(id_salt: impl Into<KeyedSalt>) -> Self {
        Self { salt: id_salt.into() }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Item,
    Sub,
    Category { open: bool },
}

struct Row {
    salt: KeyedSalt,
    label: String,
    icon: Option<Icon>,
    kind: Kind,
    selected: bool,
    density: NavDensity,
}

impl Row {
    // sharedNavStyles.styles.ts `useRootDefaultClassName` and `useSmallStyles` padding: block, end, start
    fn padding(&self) -> (f32, f32, f32) {
        let block = match self.density {
            NavDensity::Medium => g::SPACING_VERTICAL_M_NUDGE,
            NavDensity::Small => g::SPACING_VERTICAL_XS,
        };
        let start = match (self.kind, self.density) {
            (Kind::Sub, NavDensity::Medium) => d::SUB_ITEM_INDENT,
            (Kind::Sub, NavDensity::Small) => d::SUB_ITEM_INDENT_SMALL,
            _ => g::SPACING_HORIZONTAL_M_NUDGE,
        };
        (block, g::SPACING_HORIZONTAL_S, start)
    }

    const fn text(&self) -> TypeRamp {
        if self.selected {
            TypeRamp::Body1Strong
        } else {
            TypeRamp::Body1
        }
    }
}

impl MineralWidget for Row {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt, no_target);

    fn sense(&self) -> Sense {
        Sense::click()
    }

    fn layout(&self, ui: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        let (block, ..) = self.padding();
        let width = if c.max.x.is_finite() {
            c.max.x
        } else {
            ui.available_width()
        };
        c.constrain(vec2(
            width,
            2.0 * block + TypeRamp::Body1.line_height().max(d::ICON_SIZE),
        ))
        .into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let r = &frame.response;
        let i = Interaction::of(r, Availability::Enabled);
        kit::interact::hover_cursor(ui.core(), r, Availability::Enabled);
        let rect = frame.zones.visual;
        let fill = if i.pressed {
            ColorNeutralBackground4Pressed
        } else if i.hovered {
            ColorNeutralBackground4Hover
        } else {
            ColorNeutralBackground4
        };
        let radius = Radius::Medium.on(rect);
        ui.painter().rect_filled(rect, radius, t.color(fill));
        let dir = ui.i18n().direction();
        let (block, end, start) = self.padding();
        let at = |from: f32, width: f32| dir.start_x(rect, egui::Rangef::new(from, from + width));
        let cy = rect.center().y;
        let carries = self.selected;
        if carries {
            // useIndicatorStyles `::after`: starts `indicatorOffset` before the content box
            let x = at(start - d::INDICATOR_OFFSET, d::INDICATOR_WIDTH);
            let bar = egui::Rect::from_min_size(
                pos2(x, rect.top() + block),
                vec2(d::INDICATOR_WIDTH, d::INDICATOR_HEIGHT),
            );
            ui.painter()
                .rect_filled(bar, Radius::Circular.on(bar), t.color(ColorCompoundBrandForeground1));
        }
        let mut cursor = start;
        if let Some(icon) = self.icon {
            let color = if carries {
                ColorNeutralForeground2BrandSelected
            } else {
                ColorNeutralForeground2
            };
            paint::icon(
                ui.painter(),
                pos2(at(cursor, d::ICON_SIZE) + d::ICON_SIZE / 2.0, cy),
                icon,
                carries,
                d::ICON_SIZE,
                t.color(color),
            );
            cursor += d::ICON_SIZE + g::SPACING_VERTICAL_L;
        }
        let color = t.color(ColorNeutralForeground2);
        let job = LayoutJob::single_section(self.label.clone(), self.text().format(ui.ctx(), color));
        let galley = ui.painter().layout_job(job);
        let tx = at(cursor, galley.size().x);
        ui.painter().galley(pos2(tx, cy - galley.size().y / 2.0), galley, color);
        if let Kind::Category { open } = self.kind {
            let icon = if open {
                ui.icons().chevron_up
            } else {
                ui.icons().chevron_down
            };
            let x = at(rect.width() - end - d::ICON_SIZE, d::ICON_SIZE) + d::ICON_SIZE / 2.0;
            paint::icon(ui.painter(), pos2(x, cy), icon, false, d::ICON_SIZE, color);
        }
        if i.focused {
            ui.painter().add(
                FocusStroke::inside(Stroke::new(g::STROKE_WIDTH_THICK, t.color(ColorStrokeFocus2))).shape(rect, radius),
            );
        }
        WidgetResponse::new(r.clone(), ()).with_changed(r.clicked())
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let (kind, role) = match self.kind {
            Kind::Item => (egui_mineral::agent_key!("navitem"), Role::Link),
            Kind::Sub => (egui_mineral::agent_key!("navsubitem"), Role::Link),
            Kind::Category { .. } => (egui_mineral::agent_key!("navcategory"), Role::Button),
        };
        let mut n = SemanticNode::new(kind.id(&self.salt), role, &self.label).clickable(Availability::Enabled);
        n.state.selected = Some(self.selected);
        if let Kind::Category { open } = self.kind {
            n.state.expanded = Some(open);
        }
        Some(n)
    }
}

/// Result of one [`NavUi`] row.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct NavItemResponse<R = ()> {
    /// Row response.
    pub response: egui::Response,
    /// Activated this pass.
    pub clicked: bool,
    /// What the category's `add` closure returned while open; `None` for other rows and closed categories.
    pub inner: Option<R>,
}

/// Places the rows of a [`Nav`].
pub struct NavUi<'u, 's, K> {
    ui: &'u mut MineralUi<'s, Fluent>,
    salt: &'u KeyedSalt,
    id: Id,
    selected: &'u mut dyn Bound<K>,
    open: &'u mut Vec<Id>,
    multiple: bool,
    density: NavDensity,
    picked: &'u mut Option<K>,
    changed: &'u mut bool,
    values: Vec<Id>,
    children: &'u mut Children,
}

// Entries of categories no longer placed are pruned.
#[derive(Clone, Default)]
struct Children {
    known: HashMap<Id, Vec<Id>>,
    seen: HashSet<Id>,
}

impl<K: ItemKey> NavUi<'_, '_, K> {
    fn row(&mut self, value: &K, salt: Option<&KeyedSalt>, row: Row) -> egui::Response {
        let salt = salt.cloned().unwrap_or_else(|| self.salt.child(value));
        let r = self.ui.mineral(Row { salt, ..row });
        r.response().clone()
    }

    /// `NavItem`: a click selects `value`.
    pub fn item(&mut self, item: NavItem<K>) -> NavItemResponse {
        self.values.push(Id::unique(&item.value));
        let row = Row {
            salt: self.salt.clone(),
            label: item.label,
            icon: item.icon,
            kind: Kind::Item,
            selected: self.selected.holds(&item.value),
            density: self.density,
        };
        let response = self.row(&item.value, item.salt.as_ref(), row);
        self.select(response, item.value)
    }

    /// `NavSubItem`: a click selects `value`.
    pub fn sub_item(&mut self, item: NavSubItem<K>) -> NavItemResponse {
        self.values.push(Id::unique(&item.value));
        let row = Row {
            salt: self.salt.clone(),
            label: item.label,
            icon: None,
            kind: Kind::Sub,
            selected: self.selected.holds(&item.value),
            density: self.density,
        };
        let response = self.row(&item.value, item.salt.as_ref(), row);
        self.select(response, item.value)
    }

    fn select(&mut self, response: egui::Response, value: K) -> NavItemResponse {
        let clicked = response.clicked();
        if clicked {
            *self.changed |= self.selected.write(value.clone());
            *self.picked = Some(value);
        }
        NavItemResponse {
            response,
            clicked,
            inner: None,
        }
    }

    /// `NavCategory`: a click toggles it; `add` places the [`NavSubItem`]s while it is open.
    pub fn category<R>(
        &mut self,
        category: NavCategory<K>,
        add: impl FnOnce(&mut NavUi<'_, '_, K>) -> R,
    ) -> NavItemResponse<R> {
        let value = category.value;
        let vid = Id::unique(&value);
        let open = self.open.contains(&vid);
        self.children.seen.insert(vid);
        let known = self.children.known.get(&vid).map_or(&[][..], Vec::as_slice);
        let selected = !open && known.iter().any(|v| self.selected.holds_id(*v));
        let row = Row {
            salt: self.salt.clone(),
            label: category.label,
            icon: category.icon,
            kind: Kind::Category { open },
            selected,
            density: self.density,
        };
        let response = self.row(&value, category.salt.as_ref(), row);
        let clicked = response.clicked();
        if clicked {
            if open {
                self.open.retain(|v| *v != vid);
            } else {
                if !self.multiple {
                    self.open.clear();
                }
                self.open.push(vid);
            }
        }
        let inner = (open && !clicked).then(|| {
            let mut child = NavUi {
                ui: &mut *self.ui,
                salt: self.salt,
                id: self.id,
                selected: &mut *self.selected,
                open: &mut *self.open,
                multiple: self.multiple,
                density: self.density,
                picked: &mut *self.picked,
                changed: &mut *self.changed,
                values: Vec::new(),
                children: &mut *self.children,
            };
            let out = add(&mut child);
            let values = child.values;
            self.children.known.insert(vid, values);
            out
        });
        NavItemResponse {
            response,
            clicked,
            inner,
        }
    }

    /// `NavSectionHeader`.
    pub fn section_header(&mut self, header: impl Into<NavSectionHeader>) {
        let t = self.ui.tokens();
        let color = t.color(ColorNeutralForeground1);
        let job = LayoutJob::single_section(header.into().0, TypeRamp::Caption1Strong.format(self.ui.ctx(), color));
        let galley = self.ui.painter().layout_job(job);
        let height = galley.size().y + 2.0 * d::SECTION_HEADER_MARGIN;
        let width = self.ui.available_width();
        let (rect, _) = self.ui.allocate_exact_size(vec2(width, height), Sense::hover());
        let dir = self.ui.i18n().direction();
        let x = dir.start_x(
            rect,
            egui::Rangef::new(d::SECTION_HEADER_INDENT, d::SECTION_HEADER_INDENT + galley.size().x),
        );
        self.ui
            .painter()
            .galley(pos2(x, rect.top() + d::SECTION_HEADER_MARGIN), galley, color);
    }

    /// `NavDivider`.
    pub fn divider(&mut self, divider: NavDivider) {
        self.ui.add_space(d::DIVIDER_MARGIN);
        self.ui.mineral(Divider::new(divider.salt));
        self.ui.add_space(d::DIVIDER_MARGIN);
    }
}

/// Result of showing a [`Nav`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct NavResponse<K, R = ()> {
    /// Spans every row; `mark_changed` when `changed`.
    pub response: egui::Response,
    /// Agrees with `response.changed()`: the bound value changed this frame.
    pub changed: bool,
    /// Value of the item picked this frame, also when it was already selected.
    pub selected: Option<K>,
    /// What `add` returned.
    pub inner: R,
}

/// Fluent `Nav`: a column of [`NavItem`]s, [`NavCategory`]s with [`NavSubItem`]s, section headers and dividers
/// bound to one selected value. Every row is a Tab stop.
#[derive(Debug)]
#[must_use = "call .show(..)"]
pub struct Nav<'a, K> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    choice: Choice<'a, K>,
    density: NavDensity,
    multiple: bool,
    default_open: Vec<K>,
    gap: f32,
}

impl<'a, K: ItemKey> Nav<'a, K> {
    /// Selects the item whose value equals `selected`; `label` names the navigation for assistive technology.
    /// Agent key `<id_salt>-nav`; rows without a `salt(..)` are keyed `<id_salt>-<value>`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, selected: &'a mut K) -> Self {
        Self::with(id_salt.into(), label.into(), Choice::new(selected))
    }

    /// [`Self::new`] that selects no item while `selected` is `None`.
    pub fn optional(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, selected: &'a mut Option<K>) -> Self {
        Self::with(id_salt.into(), label.into(), Choice::optional(selected))
    }

    fn with(salt: KeyedSalt, label: String, choice: Choice<'a, K>) -> Self {
        Self {
            salt,
            key: egui_mineral::agent_key!("nav"),
            label,
            choice,
            density: NavDensity::Medium,
            multiple: d::MULTIPLE,
            default_open: Vec::new(),
            gap: 0.0,
        }
    }

    /// Default [`NavDensity::Medium`].
    pub fn density(mut self, density: NavDensity) -> Self {
        self.density = density;
        self
    }

    /// Several categories stay open together; default on.
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.multiple = multiple;
        self
    }

    /// Categories open until the first click (`defaultOpenCategories`).
    pub fn default_open(mut self, values: impl IntoIterator<Item = K>) -> Self {
        self.default_open = values.into_iter().collect();
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-nav` agent key.
        agent_key(key)
    }

    /// Shows the rows `add` places.
    pub fn show<R>(
        self,
        ui: &mut MineralUi<'_, Fluent>,
        add: impl FnOnce(&mut NavUi<'_, '_, K>) -> R,
    ) -> NavResponse<K, R> {
        let rt = ui.rt();
        let Self {
            salt,
            key,
            label,
            mut choice,
            density,
            multiple,
            default_open,
            gap,
        } = self;
        let id = ui.make_persistent_id(salt.id_salt());
        let open_id = id.with("mineral::nav-open");
        let mut open: Vec<Id> = ui.ctx().data(|d| d.get_temp(open_id)).unwrap_or_else(|| {
            let take = if multiple { usize::MAX } else { 1 };
            default_open.iter().take(take).map(Id::unique).collect()
        });
        let mut changed = false;
        let mut picked = None;
        let children_id = id.with("mineral::nav-children");
        let mut children: Children = ui.ctx().data(|d| d.get_temp(children_id)).unwrap_or_default();
        children.seen.clear();
        let group = ui
            .push_id(salt.id_salt(), |ui| {
                ui.scope_builder(
                    UiBuilder::new().layout(egui::Layout::top_down(egui::Align::Min)),
                    |ui| {
                        ui.spacing_mut().item_spacing.y = gap;
                        let mut nav = NavUi {
                            ui: &mut rt.ui(ui),
                            salt: &salt,
                            id,
                            selected: &mut choice,
                            open: &mut open,
                            multiple,
                            density,
                            picked: &mut picked,
                            changed: &mut changed,
                            values: Vec::new(),
                            children: &mut children,
                        };
                        add(&mut nav)
                    },
                )
            })
            .inner;
        let Children { mut known, seen } = children;
        known.retain(|v, _| seen.contains(v));
        ui.ctx().data_mut(|d| {
            d.insert_temp(open_id, open);
            d.insert_temp(
                children_id,
                Children {
                    known,
                    seen: HashSet::new(),
                },
            );
        });
        semantic::emit(ui.ctx(), Some(group.response.id), group.response.rect, || {
            SemanticNode::new(key.id(&salt), Role::Navigation, &label)
        });
        let mut response = group.response;
        if changed {
            response.mark_changed();
        }
        NavResponse {
            response,
            changed,
            selected: picked,
            inner: group.inner,
        }
    }
}

/// Fluent `NavDrawer`: a [`Nav`] in an inline panel on the start edge, on `colorNeutralBackground4`.
#[derive(Debug)]
#[must_use = "call .show(..)"]
pub struct NavDrawer<'a, K> {
    nav: Nav<'a, K>,
    width: f32,
}

impl<'a, K: ItemKey> NavDrawer<'a, K> {
    /// See [`Nav::new`]; 260pt wide.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, selected: &'a mut K) -> Self {
        Self::with(Nav::new(id_salt, label, selected))
    }

    /// See [`Nav::optional`]; 260pt wide.
    pub fn optional(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, selected: &'a mut Option<K>) -> Self {
        Self::with(Nav::optional(id_salt, label, selected))
    }

    fn with(mut nav: Nav<'a, K>) -> Self {
        nav.gap = g::SPACING_VERTICAL_XXS;
        Self {
            nav,
            width: d::DRAWER_WIDTH,
        }
    }

    /// Default 260pt.
    pub fn width(mut self, width: f32) -> Self {
        self.width = kit::sanitize::at_least(width, 0.0.., "NavDrawer width");
        self
    }

    /// Default [`NavDensity::Medium`].
    pub fn density(mut self, density: NavDensity) -> Self {
        self.nav = self.nav.density(density);
        self
    }

    /// Several categories stay open together; default on.
    pub fn multiple(mut self, multiple: bool) -> Self {
        self.nav = self.nav.multiple(multiple);
        self
    }

    /// Categories open until the first click.
    pub fn default_open(mut self, values: impl IntoIterator<Item = K>) -> Self {
        self.nav = self.nav.default_open(values);
        self
    }

    /// Docked in the layout of `ui` on the start edge, beside what is added after it.
    pub fn show<R>(
        self,
        ui: &mut MineralUi<'_, Fluent>,
        add: impl FnOnce(&mut NavUi<'_, '_, K>) -> R,
    ) -> NavResponse<K, R> {
        let rt = ui.rt();
        let t = rt.tokens();
        let dir = rt.i18n().direction();
        let panel_id = ui.make_persistent_id(("mineral::fluent-nav-drawer", self.nav.salt.id_salt()));
        let panel = match dir {
            Direction::Ltr => Panel::left(panel_id),
            Direction::Rtl => Panel::right(panel_id),
        };
        let (end, start) = d::BODY_PADDING;
        let margin = match dir {
            Direction::Ltr => Margin {
                left: start as i8,
                right: end as i8,
                ..Margin::ZERO
            },
            Direction::Rtl => Margin {
                left: end as i8,
                right: start as i8,
                ..Margin::ZERO
            },
        };
        panel
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame::NONE)
            .exact_size(self.width)
            .show(ui, |ui| {
                ui.painter()
                    .rect_filled(ui.max_rect(), 0, t.color(ColorNeutralBackground4));
                Frame::new()
                    .inner_margin(margin)
                    .show(ui, |ui| self.nav.show(&mut rt.ui(ui), add))
                    .inner
            })
            .inner
    }
}
