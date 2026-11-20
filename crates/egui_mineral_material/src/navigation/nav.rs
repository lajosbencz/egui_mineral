use crate::Material;
use crate::MaterialExt;
use crate::MaterialUiExt as _;
use crate::Rt;
use crate::msg;
use egui::{Align, Align2, Color32, Frame, Id, Layout, Pos2, Rangef, Rect, Response, Sense, Vec2, pos2, vec2};
use egui_mineral::i18n::I18n;
use egui_mineral::kit::state::TempState;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::Availability;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    focus::{self, RovingFocus},
    i18n::Direction,
    kit,
    layout::{Axis, Constraints, Measure},
    overlay::Dismiss,
    semantic::{self, Role, SemanticNode, StableId},
    shortcut::Mnemonic,
    widget::{MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::widget::{
    common::{self, with_alpha},
    edge_surface::{self, EdgeSurface},
    indication,
    modal_focus::ModalFocus,
    parts,
};
use crate::{
    Icon,
    adaptive::{Density, WindowClass},
    motion,
    spec::{
        TypeScale,
        defaults::{modal_navigation as mx, navigation as nx},
        tokens::{
            badge as badge_spec, divider, motion as motion_tokens, navigation_bar as bar,
            navigation_bar_horizontal_item as bar_h, navigation_bar_vertical_item as bar_v,
            navigation_drawer as drawer, navigation_rail_baseline_item as rail_item, navigation_rail_collapsed as rail,
            navigation_rail_expanded as rail_expanded, navigation_rail_horizontal_item as rail_h,
            navigation_rail_vertical_item as rail_v,
        },
    },
};
use crate::{SelectionResponse, communication::Badge};
use crate::{actions::IconButton, spec::ElevationLevel};

type FabFn<'a> = Box<dyn FnOnce(&mut MineralUi<'_, Material>) + 'a>;

/// Destination of a [`NavigationBar`], [`NavigationRail`] or [`NavigationDrawer`].
#[derive(Clone, Debug)]
#[must_use]
pub struct NavDestination {
    icon: Icon,
    label: String,
    badge: Option<Badge>,
    disabled: bool,
    mnemonic: Option<Mnemonic>,
    salt: Option<egui_mineral::semantic::KeyedSalt>,
}

impl NavDestination {
    crate::widget::setters! {
        /// Default the index.
        item_salt
    }

    /// `label` is the visible text and accessible name.
    pub fn new(icon: impl Into<Icon>, label: impl Into<String>) -> Self {
        Self {
            icon: icon.into(),
            label: label.into(),
            badge: None,
            disabled: false,
            salt: None,
            mnemonic: None,
        }
    }
    /// Badge on the icon.
    pub fn badge(mut self, badge: Badge) -> Self {
        self.badge = Some(badge);
        self
    }

    /// Alt+letter selects the destination.
    pub fn mnemonic(mut self, mnemonic: char) -> Self {
        self.mnemonic = Some(Mnemonic::new(mnemonic));
        self
    }
    crate::widget::setters! {
        /// Greys out the destination; it cannot be selected.
        disabled
    }

    fn node(&self, key: StableId, selected: bool, i18n: &I18n) -> SemanticNode {
        let mut n = SemanticNode::new(key, Role::Link, &self.label).clickable(Availability::disabled_if(self.disabled));
        n.state.selected = Some(selected);
        n.value = self.badge.and_then(|b| b.text(i18n));
        n
    }
}

/// When a navigation bar or rail shows destination labels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NavLabels {
    #[default]
    /// Every destination shows its label.
    Always,
    /// Only the selected destination shows its label.
    SelectedOnly,
}

#[derive(Clone, Debug)]
struct Group {
    salt: KeyedSalt,
    label: Option<String>,
    disabled: bool,
}

impl Group {
    fn new(salt: KeyedSalt) -> Self {
        Self {
            salt,
            label: None,
            disabled: false,
        }
    }

    fn name(&self, rt: &Rt) -> String {
        crate::tr_or(rt.i18n(), self.label.as_deref(), msg::NAVIGATION)
    }

    fn apply<'d>(&self, items: impl IntoIterator<Item = &'d mut NavDestination>) {
        for d in items {
            d.disabled |= self.disabled;
        }
    }

    fn emit(&self, rt: &Rt, id: Id, rect: Rect, suffix: &'static str, expanded: Option<bool>) {
        let ctx = rt.ctx();
        semantic::emit(ctx, Some(id), rect, || {
            let mut n = SemanticNode::new(StableId::of(&self.salt, suffix), Role::Navigation, self.name(rt));
            n.state.expanded = expanded;
            n.state.disabled = self.disabled;
            n
        });
    }

    fn items(
        &self,
        ui: &mut MineralUi<'_, Material>,
        group: Id,
        items: &[NavDestination],
        selected: usize,
        row: ItemRow,
    ) -> (Option<usize>, Option<Id>) {
        let rt = ui.rt();
        let enabled = |i: usize| items.get(i).is_some_and(|d| !d.disabled);
        let tab_stop = focus::tab_stop(rt.ctx(), group, Some(selected), items.len(), enabled);
        let mut ids = Vec::with_capacity(items.len());
        let mut picked = None;
        let shape = row.shape.dense(ui.density(), row.font);
        for (i, dest) in items.iter().enumerate() {
            let is_selected = i == selected;
            let own = crate::widget::item_salt(&self.salt, dest.salt.as_ref(), None, i);
            let r = ui.mineral(NavItem {
                salt: own.child("nav-item"),
                key: StableId::of(&own, "nav-item"),
                dest,
                selected: is_selected,
                tab_stop: focus::TabStop::at(i, tab_stop),
                shape,
                font: row.font,
                label: is_selected || row.all_labels,
            });
            if r.changed() {
                picked = Some(i);
            }
            ids.push(r.id);
        }
        RovingFocus::new(row.axis).update_group(rt.core(), group, &ids, enabled);
        (picked, ids.get(tab_stop).copied())
    }
}

#[derive(Clone, Copy)]
struct ItemRow {
    shape: ItemShape,
    font: TypeScale,
    all_labels: bool,
    axis: Axis,
}

/// Builder setters of [`Group`], for a type with `group_mut()`.
macro_rules! group_setters {
    () => {
        /// Replaces the `<id_salt>` key prefix.
        pub fn agent_key(mut self, key: impl Into<StableId>) -> Self {
            let group = self.group_mut();
            group.salt = group.salt.clone().with_key(key.into());
            self
        }

        /// Accessible name of the landmark; default `mineral-navigation`.
        pub fn label(mut self, label: impl Into<String>) -> Self {
            self.group_mut().label = Some(label.into());
            self
        }

        crate::widget::setters! {
            /// Disables every destination.
            disabled |s| s.group_mut().disabled
        }
    };
}

/// `item` and `items` over the `items` field.
macro_rules! destination_setters {
    () => {
        /// Appends one destination.
        pub fn item(mut self, destination: impl Into<NavDestination>) -> Self {
            self.items.push(destination.into());
            self
        }

        /// Appends destinations.
        pub fn items(mut self, destinations: impl IntoIterator<Item = impl Into<NavDestination>>) -> Self {
            self.items.extend(destinations.into_iter().map(Into::into));
            self
        }
    };
}

struct ModalPanel<'k> {
    id: Id,
    group: &'k Group,
    open: bool,
    trigger: Option<Id>,
    rect: Rect,
    round: egui::CornerRadius,
    fill: Color32,
    elevation: ElevationLevel,
    scrim: f32,
}

impl ModalPanel<'_> {
    fn show(
        self,
        rt: &Rt,
        mounted: bool,
        add: impl FnOnce(&mut MineralUi<'_, Material>, Id) -> Option<Id>,
    ) -> (Option<Response>, Option<Dismiss>) {
        let ctx = rt.ctx();
        let (fid, trigger, mut open) = (self.id.with("focus"), self.trigger, self.open);
        ModalFocus::scope(ctx, fid, &mut open, trigger, |st, _| {
            let mut close = None;
            let mut response = None;
            if mounted {
                let t = rt.tokens();
                let round = self.round;
                let name = self.group.name(rt);
                let r = EdgeSurface {
                    id: self.id,
                    name: &name,
                    pos: self.rect.min,
                    scrim: Some(t.scrim(self.scrim)),
                }
                .show(rt, |ui| {
                    let surface = ui.allocate_rect(self.rect, Sense::hover());
                    common::elevation(ui.painter(), self.rect, round, self.elevation, &t);
                    ui.painter().rect_filled(self.rect, round, self.fill);
                    let target = add(ui, surface.id);
                    semantic::emit(ctx, None, self.rect, || {
                        SemanticNode::new(StableId::of(&self.group.salt, "dialog"), Role::Dialog, &name)
                    });
                    target
                });
                close = r.close;
                response = Some(r.response);
                if let Some(target) = r.inner {
                    st.enter(ctx, target);
                }
            }
            (response, close)
        })
    }
}

// Compose ModalWideNavigationRail positionProgress
fn rail_modal_progress(rt: &crate::Rt, id: Id, open: bool) -> Option<f32> {
    let p = egui_mineral::motion::spring(rt.core(), id.with("position"), f32::from(open), motion::DEFAULT_EFFECTS);
    (p > 0.0).then_some(p)
}

// Compose ModalNavigationDrawer DrawerState, AnchoredDraggableState; offset runs from -width (closed) to 0 (open)
#[derive(Clone, Copy, Debug)]
struct DrawerMotion {
    open: bool,
    offset: f32,
    grab: Option<Pos2>,
    settle: Option<(f32, f32, f64)>,
}

impl TempState for DrawerMotion {}

#[derive(Clone, Copy)]
struct TriggerId(Id);

impl TempState for TriggerId {}

impl DrawerMotion {
    fn load_for(ctx: &egui::Context, id: Id, open: bool, width: f32) -> Self {
        Self::load_or(ctx, drawer_motion_id(id), || Self {
            open,
            offset: if open { 0.0 } else { -width },
            grab: None,
            settle: None,
        })
    }

    fn store_for(self, ctx: &egui::Context, id: Id) {
        TempState::store(self, ctx, drawer_motion_id(id));
    }

    fn step(rt: &crate::Rt, id: Id, open: bool, width: f32) -> f32 {
        let ctx = rt.ctx();
        let mut m = Self::load_for(ctx, id, open, width);
        let anim = drawer_offset_id(id);
        let target = if open { 0.0 } else { -width };
        if m.open != open {
            m.settle = None;
        }
        let held = m.grab.is_some();
        if let Some((from, to, start)) = m.settle {
            let secs = rt.core().motion().duration(mx::DRAWER_SETTLE_DURATION).as_secs_f32();
            let elapsed = (ctx.input(|i| i.time) - start) as f32;
            let k = if secs > 0.0 { (elapsed / secs).min(1.0) } else { 1.0 };
            m.offset = egui::lerp(from..=to, motion_tokens::EASING_LEGACY_CUBIC_BEZIER.ease(k));
            if k < 1.0 {
                egui_mineral::motion::request_frame(ctx);
            } else {
                m.settle = None;
                egui_mineral::motion::snap(ctx, anim, to);
            }
        } else if !held {
            let spring = if open {
                motion::DEFAULT_SPATIAL
            } else {
                motion::FAST_EFFECTS
            };
            m.offset = egui_mineral::motion::spring(rt.core(), anim, target, spring);
        }
        m.open = open;
        m.store_for(ctx, id);
        m.offset
    }

    // points/s
    fn drag(ctx: &egui::Context, id: Id, open: &mut bool, width: f32, grab: Pos2, dx: f32, release: Option<f32>) {
        let mut m = Self::load_for(ctx, id, *open, width);
        m.offset = (m.offset + dx).clamp(-width, 0.0);
        m.settle = None;
        m.grab = release.is_none().then_some(grab);
        egui_mineral::motion::snap(ctx, drawer_offset_id(id), m.offset);
        if let Some(v) = release {
            let opened = if v.abs() > mx::DRAWER_VELOCITY_THRESHOLD {
                v > 0.0
            } else {
                m.offset > -width * mx::DRAWER_POSITIONAL_THRESHOLD
            };
            *open = opened;
            m.open = opened;
            m.settle = Some((m.offset, if opened { 0.0 } else { -width }, ctx.input(|i| i.time)));
        }
        m.store_for(ctx, id);
        egui_mineral::motion::request_frame(ctx);
    }
}

#[derive(Clone, Copy)]
enum IndicatorTop {
    /// `NavigationBar` / `NavigationRail` `placeLabelAndIcon`: icon, indicator padding, gap and
    /// label centered, the indicator padding as the minimum.
    Baseline,
    /// `NavigationItem`: indicator, gap and label centered.
    Centered,
}

#[derive(Clone, Copy)]
enum ItemShape {
    /// Icon above the label.
    Vertical { size: Vec2, top: IndicatorTop },
    /// Icon before the label; `full` stretches the indicator over the item.
    Horizontal {
        size: Vec2,
        pill_h: f32,
        gap: f32,
        full: bool,
    },
}

// Dense down to the indicator and its label (icon above) or the icon with padding (icon before).
fn dense_height(density: Density, height: f32, horizontal: bool, font: TypeScale) -> f32 {
    let content = if horizontal {
        bar_v::ICON_SIZE + 8.0
    } else {
        bar_v::ACTIVE_INDICATOR_HEIGHT + nx::INDICATOR_TO_LABEL + font.line_height()
    };
    density.dense_to(height, content)
}

fn drawer_item_height(density: Density) -> f32 {
    density.dense_to(
        drawer::ACTIVE_INDICATOR_HEIGHT,
        drawer::LABEL_TEXT_FONT.line_height().max(drawer::ICON_SIZE) + 8.0,
    )
}

impl ItemShape {
    fn dense(self, density: Density, font: TypeScale) -> Self {
        match self {
            Self::Vertical { size, top } => Self::Vertical {
                size: vec2(size.x, dense_height(density, size.y, false, font)),
                top,
            },
            Self::Horizontal {
                size,
                pill_h,
                gap,
                full,
            } => Self::Horizontal {
                size: vec2(size.x, dense_height(density, size.y, true, font)),
                pill_h: dense_height(density, pill_h, true, font),
                gap,
                full,
            },
        }
    }
}

// Compose NavigationBarItemDefaults, NavigationItemColors
struct ItemColors {
    icon: Color32,
    label: Color32,
}

impl ItemColors {
    fn of(c: &crate::color::ColorScheme, selected: bool, enabled: bool, top: bool) -> Self {
        let unselected = Self {
            icon: c.role(bar::ITEM_INACTIVE_ICON_COLOR),
            label: c.role(bar::ITEM_INACTIVE_LABEL_TEXT_COLOR),
        };
        if !enabled {
            return Self {
                icon: with_alpha(unselected.icon, nx::DISABLED_ALPHA),
                label: with_alpha(unselected.label, nx::DISABLED_ALPHA),
            };
        }
        let icon = c.role(bar::ITEM_ACTIVE_ICON_COLOR);
        // Start-icon items use the icon color for the label (`selectedTextColorStartIconPosition`).
        let label = if top {
            c.role(bar::ITEM_ACTIVE_LABEL_TEXT_COLOR)
        } else {
            icon
        };
        if selected { Self { icon, label } } else { unselected }
    }
}

struct NavItem<'a> {
    salt: KeyedSalt,
    key: StableId,
    dest: &'a NavDestination,
    selected: bool,
    tab_stop: focus::TabStop,
    shape: ItemShape,
    font: TypeScale,
    label: bool,
}

#[derive(Clone, Copy)]
struct ModalRail {
    open: bool,
    progress: Option<f32>,
}

#[derive(Clone, Copy)]
struct IndicatorSlot {
    pill: Rect,
    icon: Pos2,
}

impl NavItem<'_> {
    fn layout_indicator(&self, rect: Rect, galley: Option<&egui::Galley>, dir: Direction) -> IndicatorSlot {
        let icon_size = bar_v::ICON_SIZE;
        match self.shape {
            ItemShape::Vertical { top, .. } => {
                let pill = vec2(bar_v::ACTIVE_INDICATOR_WIDTH, bar_v::ACTIVE_INDICATOR_HEIGHT);
                let pad = (pill.y - icon_size) / 2.0;
                let y = match (galley, top) {
                    (None, _) => rect.center().y - pill.y / 2.0,
                    (Some(g), IndicatorTop::Baseline) => {
                        let content = icon_size + pad + nx::INDICATOR_TO_LABEL + g.size().y;
                        rect.top() + ((rect.height() - content) / 2.0).max(pad) - pad
                    }
                    (Some(g), IndicatorTop::Centered) => {
                        let content = pill.y + nx::INDICATOR_TO_LABEL + g.size().y;
                        rect.top() + (rect.height() - content) / 2.0
                    }
                };
                let r = Rect::from_min_size(pos2(rect.center().x - pill.x / 2.0, y), pill);
                IndicatorSlot {
                    pill: r,
                    icon: r.center(),
                }
            }
            ItemShape::Horizontal { pill_h, gap, full, .. } => {
                let lead = bar_h::ACTIVE_INDICATOR_LEADING_SPACE;
                let text_w = galley.as_ref().map_or(0.0, |g| g.size().x + gap);
                let w = if full {
                    rect.width()
                } else {
                    lead + icon_size + text_w + bar_h::ACTIVE_INDICATOR_TRAILING_SPACE
                };
                let r = Rect::from_center_size(rect.center(), vec2(w.min(rect.width()), pill_h));
                let x = dir.start_x(r, Rangef::point(lead + icon_size / 2.0));
                IndicatorSlot {
                    pill: r,
                    icon: pos2(x, r.center().y),
                }
            }
        }
    }

    fn label_pos(&self, rect: Rect, slot: IndicatorSlot, size: Vec2, dir: Direction) -> Pos2 {
        let IndicatorSlot {
            pill: pill_full,
            icon: icon_at,
        } = slot;
        let icon_size = bar_v::ICON_SIZE;
        match self.shape {
            ItemShape::Vertical { .. } => pos2(
                rect.center().x - size.x / 2.0,
                pill_full.bottom() + nx::INDICATOR_TO_LABEL,
            ),
            ItemShape::Horizontal { gap, .. } => {
                let from = icon_size / 2.0 + gap;
                let x = dir.offset_x(icon_at.x, Rangef::new(from, from + size.x));
                pos2(x, icon_at.y - size.y / 2.0)
            }
        }
    }
}

impl MineralWidget for NavItem<'_> {
    type Kit = Material;
    type State = ();
    type Output = ();

    egui_mineral::kit::widget_meta!(salt);

    fn sense(&self) -> Sense {
        focus::roving_sense(Sense::click(), self.tab_stop)
    }

    fn enabled(&self) -> bool {
        !self.dest.disabled
    }

    fn access_key(&self) -> Option<Mnemonic> {
        self.dest.mnemonic
    }

    fn layout(&self, _ui: &MineralUi<'_, Material>, constraints: Constraints, (): &Self::State) -> Measure {
        constraints
            .constrain(match self.shape {
                ItemShape::Vertical { size, .. } | ItemShape::Horizontal { size, .. } => size,
            })
            .into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Material>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let t = rt.tokens();
        let c = &t.color;
        let response = &frame.response;
        kit::focus::follow_press(response);
        let rect = frame.zones.visual;
        let dis = self.dest.disabled;
        let press = parts::press(ui, frame, Availability::disabled_if(dis));
        let i = press.interaction;
        let clicked = press.clicked;
        let on = if self.selected { 1.0 } else { 0.0 };
        let alpha = egui_mineral::motion::spring(
            rt.core(),
            response.id.with("indicator-alpha"),
            on,
            motion::DEFAULT_EFFECTS,
        )
        .clamp(0.0, 1.0);
        let grow =
            egui_mineral::motion::spring(rt.core(), response.id.with("indicator-size"), on, motion::FAST_SPATIAL)
                .max(0.0);
        let top = matches!(self.shape, ItemShape::Vertical { .. });
        let to = ItemColors::of(c, true, !dis, top);
        let from = ItemColors::of(c, false, !dis, top);
        let (icon_col, label_col) = if dis {
            (from.icon, from.label)
        } else {
            (
                from.icon.lerp_to_gamma(to.icon, alpha),
                from.label.lerp_to_gamma(to.label, alpha),
            )
        };
        let icon = common::selection_icon(self.dest.icon, self.selected);
        let icon_size = bar_v::ICON_SIZE;
        let dir = rt.i18n().direction();
        let p = ui.painter();
        let galley = self.label.then(|| {
            parts::label_galley(
                rt.core(),
                p,
                &self.dest.label,
                self.font.font(ctx),
                label_col,
                self.dest.mnemonic,
            )
        });
        let slot = self.layout_indicator(rect, galley.as_deref(), dir);
        let IndicatorSlot {
            pill: pill_full,
            icon: icon_at,
        } = slot;
        let round = common::radius(pill_full, bar::ITEM_ACTIVE_INDICATOR_SHAPE.scale().radius(&t.shape));
        // `NavigationBarItemLayout`: the indicator width grows from the center.
        let pill = match self.shape {
            ItemShape::Vertical { .. } => {
                Rect::from_center_size(pill_full.center(), vec2(pill_full.width() * grow, pill_full.height()))
            }
            ItemShape::Horizontal { .. } => pill_full,
        };
        if alpha > 0.0 && pill.width() > 0.0 {
            let fill = with_alpha(t.role(bar::ITEM_ACTIVE_INDICATOR_COLOR), alpha);
            p.rect_filled(pill, common::radius(pill, round.nw), fill);
        }
        common::icon(p, icon_at, icon, icon_size, icon_col);
        if let Some(b) = self.dest.badge {
            b.paint(rt, p, Rect::from_center_size(icon_at, Vec2::splat(icon_size)));
        }
        if let Some(g) = galley {
            let pos = self.label_pos(rect, slot, g.size(), dir);
            p.with_clip_rect(rect).galley(pos, g, label_col);
        }
        // `IndicatorRipple`: bounded to the indicator, in the container content color.
        indication::paint(ui, response, pill_full, round, c.on_surface, i, &t);
        WidgetResponse::new(response.clone(), ()).with_changed(clicked && !self.selected)
    }

    fn semantics(&self, i18n: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        Some(self.dest.node(self.key.clone(), self.selected, i18n))
    }
}

/// MD3 bottom navigation bar (Compose `NavigationBar`, `ShortNavigationBar`).
#[derive(Debug)]
#[must_use = "call .show(..)"]
pub struct NavigationBar<'a> {
    group: Group,
    selected: &'a mut usize,
    items: Vec<NavDestination>,
    flexible: bool,
    item_axis: Option<Axis>,
    labels: NavLabels,
}

impl<'a> NavigationBar<'a> {
    /// `selected` is the bound destination index, moved onto the last one when past it. Agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>, selected: &'a mut usize) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            group: Group::new(id_salt),
            selected,
            items: Vec::new(),
            flexible: false,
            item_axis: None,
            labels: NavLabels::default(),
        }
    }

    fn group_mut(&mut self) -> &mut Group {
        &mut self.group
    }

    group_setters!();
    destination_setters!();

    /// `ShortNavigationBar` metrics instead of `NavigationBar`.
    pub fn flexible(mut self, flexible: bool) -> Self {
        self.flexible = flexible;
        self
    }

    /// Flexible bar only: [`Axis::Horizontal`] puts the icon beside the label; unset is horizontal from
    /// [`WindowClass::Medium`](crate::adaptive::WindowClass::Medium) up.
    pub fn item_axis(mut self, item_axis: Axis) -> Self {
        self.item_axis = Some(item_axis);
        self
    }

    /// Default [`NavLabels::Always`].
    pub fn labels(mut self, labels: NavLabels) -> Self {
        self.labels = labels;
        self
    }

    /// Paints the bar and updates `selected` on a pick.
    pub fn show(mut self, ui: &mut MineralUi<'_, Material>) -> SelectionResponse {
        let rt = ui.rt();
        let n = self.items.len();
        clamp_selected(self.selected, n, "NavigationBar selected");
        self.group.apply(&mut self.items);
        let t = rt.tokens();
        let id = ui.make_persistent_id(self.group.salt.id_salt());
        let width = ui.available_width();
        // `NavigationBar` (baseline) or `ShortNavigationBar` (flexible).
        let horizontal = self.flexible
            && self.item_axis.map_or_else(
                || WindowClass::of(rt.core()) >= WindowClass::Medium,
                |a| a == Axis::Horizontal,
            );
        let height = if self.flexible {
            bar::CONTAINER_HEIGHT
        } else {
            bar::TALL_CONTAINER_HEIGHT
        };
        let height = dense_height(ui.density(), height, horizontal, bar::LABEL_TEXT_FONT);
        let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        ui.painter().rect_filled(rect, 0, t.role(bar::CONTAINER_COLOR));
        let space = if self.flexible {
            bar::ITEM_BETWEEN_SPACE
        } else {
            nx::BAR_ITEM_SPACE
        };
        let count = n.max(1) as f32;
        let item_w = (width - space * (count - 1.0)) / count;
        let size = vec2(item_w, height);
        let shape = match (self.flexible, horizontal) {
            (false, _) => ItemShape::Vertical {
                size,
                top: IndicatorTop::Baseline,
            },
            (true, false) => ItemShape::Vertical {
                size,
                top: IndicatorTop::Centered,
            },
            (true, true) => ItemShape::Horizontal {
                size,
                pill_h: bar_h::ACTIVE_INDICATOR_HEIGHT,
                gap: bar::ITEM_ACTIVE_INDICATOR_ICON_LABEL_SPACE,
                full: false,
            },
        };
        let row = ItemRow {
            shape,
            font: bar::LABEL_TEXT_FONT,
            all_labels: self.labels == NavLabels::Always,
            axis: Axis::Horizontal,
        };
        let mut picked = None;
        let scope = self.group.salt.child("items");
        egui_mineral::layout::child(ui, scope, rect, rt.i18n().direction().row(), |ui| {
            ui.spacing_mut().item_spacing = vec2(space, 0.0);
            picked = self.group.items(ui, id, &self.items, *self.selected, row).0;
        });
        self.group.emit(rt, response.id, rect, "navigation", None);
        SelectionResponse::pick(response, self.selected, picked)
    }
}

/// Result of [`NavigationRail::show`] and [`NavigationRail::show_modal`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct NavigationRailResponse {
    /// Destination outcome.
    pub selection: SelectionResponse,
    /// Menu button clicked this frame; the app toggles [`NavigationRail::expanded`].
    pub leading_clicked: bool,
    /// Expanded modal rail surface; `None` while collapsed and always from [`NavigationRail::show`].
    pub popup: Option<Response>,
    /// Why the modal rail closed without a pick; the menu button counts as [`Dismiss::Close`]. Always `None` from
    /// [`NavigationRail::show`].
    pub dismissed: Option<Dismiss>,
}

enum RailMode {
    Baseline,
    Expressive(bool),
    Modal(bool),
}

#[derive(Default)]
struct RailBody {
    picked: Option<usize>,
    menu: Option<Id>,
    menu_clicked: bool,
    focus: Option<Id>,
}

/// MD3 navigation rail, optionally expanding modally (Compose `NavigationRail`, `ModalWideNavigationRail`).
#[must_use = "call .show(..)"]
pub struct NavigationRail<'a> {
    group: Group,
    selected: &'a mut usize,
    items: Vec<NavDestination>,
    menu: bool,
    fab: Option<FabFn<'a>>,
    mode: RailMode,
    labels: NavLabels,
}

egui_mineral::kit::opaque_debug!(NavigationRail<'_>);

impl<'a> NavigationRail<'a> {
    /// `selected` is the bound destination index, moved onto the last one when past it. Agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>, selected: &'a mut usize) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            group: Group::new(id_salt),
            selected,
            items: Vec::new(),
            menu: false,
            fab: None,
            mode: RailMode::Baseline,
            labels: NavLabels::default(),
        }
    }

    fn group_mut(&mut self) -> &mut Group {
        &mut self.group
    }

    group_setters!();
    destination_setters!();

    /// Menu icon button at the top; reports [`NavigationRailResponse::leading_clicked`].
    pub fn menu_button(mut self, menu_button: bool) -> Self {
        self.menu = menu_button;
        self
    }

    /// Content below the menu button, usually a FAB.
    pub fn fab(mut self, add: impl FnOnce(&mut MineralUi<'_, Material>) + 'a) -> Self {
        self.fab = Some(Box::new(add));
        self
    }

    /// M3 Expressive rail, collapsed or `expanded`; default baseline `NavigationRail`.
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.mode = RailMode::Expressive(expanded);
        self
    }

    /// Default [`NavLabels::Always`].
    pub fn labels(mut self, labels: NavLabels) -> Self {
        self.labels = labels;
        self
    }

    fn expanded_width(&self, ctx: &egui::Context) -> f32 {
        let font = rail_h::LABEL_TEXT_FONT.font(ctx);
        let text = ctx.fonts_mut(|f| {
            self.items
                .iter()
                .map(|d| {
                    f.layout_no_wrap(d.label.clone(), font.clone(), egui::Color32::PLACEHOLDER)
                        .size()
                        .x
                })
                .fold(0.0, f32::max)
        });
        let item = rail_h::FULL_WIDTH_LEADING_SPACE
            + rail_item::ICON_SIZE
            + rail_h::ICON_LABEL_SPACE
            + text
            + rail_h::FULL_WIDTH_TRAILING_SPACE;
        (2.0 * nx::WIDE_RAIL_ITEM_PADDING + item).clamp(
            rail_expanded::CONTAINER_WIDTH_MINIMUM,
            rail_expanded::CONTAINER_WIDTH_MAXIMUM,
        )
    }

    fn expanded_state(&self) -> Option<bool> {
        match &self.mode {
            RailMode::Baseline => None,
            RailMode::Expressive(e) | RailMode::Modal(e) => Some(*e),
        }
    }

    /// Paints the rail in place and updates `selected` on a pick.
    pub fn show(self, ui: &mut MineralUi<'_, Material>) -> NavigationRailResponse {
        self.run(ui, None)
    }

    /// `ModalWideNavigationRail`: the collapsed rail stays in the layout; while `open` the expanded rail
    /// floats over a scrim. A pick, the menu button or a dismissal closes it; the menu button opens it.
    pub fn show_modal(mut self, ui: &mut MineralUi<'_, Material>, open: &mut bool) -> NavigationRailResponse {
        self.mode = RailMode::Modal(*open);
        self.run(ui, Some(open))
    }

    fn run(mut self, ui: &mut MineralUi<'_, Material>, modal_open: Option<&mut bool>) -> NavigationRailResponse {
        let rt = ui.rt();
        clamp_selected(self.selected, self.items.len(), "NavigationRail selected");
        self.group.apply(&mut self.items);
        let ctx = ui.ctx().clone();
        let t = rt.tokens();
        let id = ui.make_persistent_id(self.group.salt.id_salt());
        let expanded = self.expanded_state();
        let modal_id = modal_id(id);
        let modal = match &self.mode {
            RailMode::Modal(open) => Some(ModalRail {
                open: *open,
                progress: rail_modal_progress(rt, modal_id, *open),
            }),
            _ => None,
        };
        // `NavigationRail` (baseline) or `WideNavigationRail`, whose width springs.
        let width = match self.mode {
            RailMode::Baseline => rail::NARROW_CONTAINER_WIDTH,
            RailMode::Modal(_) => rail::CONTAINER_WIDTH,
            RailMode::Expressive(e) => {
                let target = if e {
                    self.expanded_width(&ctx)
                } else {
                    rail::CONTAINER_WIDTH
                };
                egui_mineral::motion::spring(rt.core(), id.with("width"), target, motion::DEFAULT_SPATIAL)
            }
        };
        let height = ui.available_height();
        let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        ui.painter().rect_filled(rect, 0, t.role(rail::CONTAINER_COLOR));
        let trigger_id = trigger_id(id);
        let mut body = RailBody::default();
        if modal.is_none_or(|m| m.progress.is_none()) {
            body = self.body(ui, id, rect, width);
            if let Some(menu) = body.menu {
                TriggerId(menu).store(&ctx, trigger_id);
                if let Some(e) = expanded {
                    ctx.accesskit_node_builder(menu, |b| {
                        b.set_expanded(e);
                        if modal.is_some() {
                            b.set_has_popup(egui::accesskit::HasPopup::Dialog);
                        }
                    });
                }
            }
        }
        let (popup, dismissed) = modal.map_or((None, None), |state| {
            self.modal_panel(rt, id, rect, state, modal_open, &mut body)
        });
        self.group.emit(rt, response.id, rect, "navigation", expanded);
        NavigationRailResponse {
            selection: SelectionResponse::pick(response, self.selected, body.picked),
            leading_clicked: body.menu_clicked,
            popup,
            dismissed,
        }
    }

    // Compose ModalWideNavigationRail
    fn modal_panel(
        &mut self,
        rt: &Rt,
        id: Id,
        rect: Rect,
        modal: ModalRail,
        modal_open: Option<&mut bool>,
        body: &mut RailBody,
    ) -> (Option<Response>, Option<Dismiss>) {
        let ModalRail { open, progress } = modal;
        let ctx = rt.ctx();
        let t = rt.tokens();
        let modal_id = modal_id(id);
        let trigger_id = trigger_id(id);
        // `ModalWideNavigationRail`: `modalExpanded` springs the width and fades the scrim.
        let expanded = progress.is_some_and(|p| p >= mx::RAIL_EXPAND_PROGRESS);
        let width_id = modal_id.with("width");
        let scrim_id = modal_id.with("scrim");
        if progress.is_none() {
            egui_mineral::motion::snap(ctx, width_id, rail::CONTAINER_WIDTH);
            egui_mineral::motion::snap(ctx, scrim_id, 0.0);
        }
        let target = if expanded {
            self.expanded_width(ctx)
        } else {
            rail::CONTAINER_WIDTH
        };
        let w = egui_mineral::motion::spring(rt.core(), width_id, target, motion::FAST_SPATIAL);
        let scrim = egui_mineral::motion::spring(rt.core(), scrim_id, f32::from(expanded), motion::DEFAULT_EFFECTS);
        let left = rt.i18n().direction().start_x(rect, Rangef::new(0.0, 0.0 + w));
        let panel = Rect::from_min_size(pos2(left, rect.top()), vec2(w, rect.height()));
        let group = self.group.clone();
        let (popup, close) = ModalPanel {
            id: modal_id,
            group: &group,
            open,
            trigger: TriggerId::try_load(ctx, trigger_id).map(|t| t.0),
            rect: panel,
            round: rail_expanded::MODAL_CONTAINER_SHAPE.scale().radius(&t.shape).into(),
            fill: t.role(rail_expanded::MODAL_CONTAINER_COLOR),
            elevation: rail_expanded::MODAL_CONTAINER_ELEVATION,
            scrim,
        }
        .show(rt, progress.is_some(), |ui, panel_id| {
            *body = self.body(ui, modal_id, panel, w);
            group.emit(rt, panel_id, panel, "modal-navigation", Some(true));
            body.focus
        });
        let shown = progress.is_some();
        let dismissed = if !shown || body.picked.is_some() {
            None
        } else {
            close.or(body.menu_clicked.then_some(Dismiss::Close))
        };
        if let Some(open) = modal_open {
            if shown {
                if dismissed.is_some() || body.picked.is_some() {
                    *open = false;
                }
            } else if body.menu_clicked {
                *open = !*open;
            }
        }
        (popup, dismissed)
    }

    fn body(&mut self, ui: &mut MineralUi<'_, Material>, group: Id, rect: Rect, width: f32) -> RailBody {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let expanded = self.expanded_state();
        let expressive = expanded.is_some();
        let wide = expressive && width >= f32::midpoint(rail::CONTAINER_WIDTH, self.expanded_width(ctx));
        let shape = rail_item_shape(expressive, wide, width);
        // (top padding, space below the header, item spacing)
        let (top, header_space, gap) = if expressive {
            (
                rail::TOP_SPACE,
                rail_item::HEADER_SPACE_MINIMUM,
                if wide { 0.0 } else { rail::ITEM_VERTICAL_SPACE },
            )
        } else {
            (
                nx::RAIL_VERTICAL_PADDING,
                nx::RAIL_HEADER_PADDING,
                nx::RAIL_VERTICAL_PADDING,
            )
        };
        let align = if wide {
            rt.i18n().direction().column()
        } else {
            Layout::top_down(Align::Center)
        };
        let inner = if wide {
            rect.shrink2(vec2(nx::WIDE_RAIL_ITEM_PADDING, 0.0))
        } else {
            rect
        };
        let row = ItemRow {
            shape,
            font: if wide {
                rail_h::LABEL_TEXT_FONT
            } else {
                rail_v::LABEL_TEXT_FONT
            },
            all_labels: wide || self.labels == NavLabels::Always,
            axis: Axis::Vertical,
        };
        let mut out = RailBody::default();
        egui_mineral::layout::child(ui, self.group.salt.child("items"), inner, align, |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, gap);
            ui.add_space(top);
            let header = self.menu || self.fab.is_some();
            if self.menu {
                let (icon, label) = if expanded == Some(true) {
                    (ui.icons().menu_open, msg::MENU_CLOSE)
                } else {
                    (ui.icons().menu, msg::MENU)
                };
                let r = ui.mineral(IconButton::new(
                    self.group.salt.child("menu"),
                    icon,
                    crate::tr(ui.i18n(), label),
                ));
                out.menu_clicked = r.clicked();
                out.menu = Some(r.id);
            }
            if let Some(fab) = self.fab.take() {
                fab(ui);
            }
            if header {
                ui.add_space(header_space - gap);
            }
            // Short windows scroll the destinations; the header stays.
            (out.picked, out.focus) = egui::ScrollArea::vertical()
                .id_salt("items")
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    self.group
                        .items(&mut rt.ui(ui), group, &self.items, *self.selected, row)
                })
                .inner;
        });
        out
    }
}

fn rail_item_shape(expressive: bool, wide: bool, width: f32) -> ItemShape {
    if !expressive {
        ItemShape::Vertical {
            size: vec2(rail::NARROW_CONTAINER_WIDTH, rail_v::ACTIVE_INDICATOR_WIDTH),
            top: IndicatorTop::Baseline,
        }
    } else if wide {
        ItemShape::Horizontal {
            size: vec2(
                width - 2.0 * nx::WIDE_RAIL_ITEM_PADDING,
                rail_h::ACTIVE_INDICATOR_HEIGHT,
            ),
            pill_h: rail_h::ACTIVE_INDICATOR_HEIGHT,
            gap: rail_h::ICON_LABEL_SPACE,
            full: true,
        }
    } else {
        ItemShape::Vertical {
            size: vec2(rail::CONTAINER_WIDTH, rail_item::CONTAINER_HEIGHT),
            top: IndicatorTop::Centered,
        }
    }
}

enum DrawerEntry {
    Section(String),
    Item(NavDestination),
    Divider,
}

/// Result of [`NavigationDrawer::show_modal`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct NavigationDrawerModalResponse {
    /// Destination outcome on the drawer surface; `None` while closed.
    pub selection: Option<SelectionResponse>,
    /// `None` when a pick closed the drawer.
    pub dismissed: Option<Dismiss>,
}

/// MD3 navigation drawer, standard or modal (Compose `PermanentNavigationDrawer`, `ModalNavigationDrawer`).
#[must_use = "call .show(..)"]
pub struct NavigationDrawer<'a> {
    drawer: Drawer,
    selected: &'a mut usize,
}

egui_mineral::kit::opaque_debug!(NavigationDrawer<'_>);

impl<'a> NavigationDrawer<'a> {
    /// `selected` is the bound destination index, moved onto the last one when past it. Agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>, selected: &'a mut usize) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            drawer: Drawer {
                group: Group::new(id_salt),
                entries: Vec::new(),
            },
            selected,
        }
    }

    fn group_mut(&mut self) -> &mut Group {
        &mut self.drawer.group
    }

    group_setters!();

    /// Section heading before the following items.
    pub fn section(mut self, title: impl Into<String>) -> Self {
        self.drawer.entries.push(DrawerEntry::Section(title.into()));
        self
    }

    /// Appends one destination.
    pub fn item(mut self, destination: impl Into<NavDestination>) -> Self {
        self.drawer.entries.push(DrawerEntry::Item(destination.into()));
        self
    }

    /// Appends destinations.
    pub fn items(mut self, destinations: impl IntoIterator<Item = impl Into<NavDestination>>) -> Self {
        self.drawer
            .entries
            .extend(destinations.into_iter().map(|d| DrawerEntry::Item(d.into())));
        self
    }

    /// Divider between groups.
    pub fn divider(mut self) -> Self {
        self.drawer.entries.push(DrawerEntry::Divider);
        self
    }

    /// Standard drawer in the layout.
    pub fn show(self, ui: &mut MineralUi<'_, Material>) -> SelectionResponse {
        self.drawer.show(ui, self.selected)
    }

    /// `ModalNavigationDrawer` over a scrim while `open`; a pick or a dismissal clears `open`.
    pub fn show_modal(self, rt: &Rt, open: &mut bool) -> NavigationDrawerModalResponse {
        self.drawer.show_modal(rt, open, self.selected)
    }
}

struct Drawer {
    group: Group,
    entries: Vec<DrawerEntry>,
}

impl Drawer {
    fn prepare(&mut self, selected: &mut usize) {
        let items = self.entries.iter_mut().filter_map(|e| match e {
            DrawerEntry::Item(d) => Some(d),
            DrawerEntry::Section(_) | DrawerEntry::Divider => None,
        });
        self.group.apply(items);
        let n = self
            .entries
            .iter()
            .filter(|e| matches!(e, DrawerEntry::Item(_)))
            .count();
        clamp_selected(selected, n, "NavigationDrawer selected");
    }

    fn show(mut self, ui: &mut MineralUi<'_, Material>, selected: &mut usize) -> SelectionResponse {
        let rt = ui.rt();
        self.prepare(selected);
        let t = rt.tokens();
        let id = ui.make_persistent_id(self.group.salt.id_salt());
        let width = ui.available_width().min(drawer::CONTAINER_WIDTH);
        let height = ui.available_height();
        let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        ui.painter()
            .rect_filled(rect, 0, t.role(drawer::STANDARD_CONTAINER_COLOR));
        let mut picked = None;
        let scope = self.group.salt.child("content");
        egui_mineral::layout::child(ui, scope, rect, rt.i18n().direction().column(), |ui| {
            picked = self.content(ui, id, width, *selected).0;
        });
        self.group.emit(rt, response.id, rect, "navigation", None);
        SelectionResponse::pick(response, selected, picked)
    }

    fn show_modal(mut self, rt: &Rt, open: &mut bool, selected: &mut usize) -> NavigationDrawerModalResponse {
        let ctx = rt.ctx();
        self.prepare(selected);
        let t = rt.tokens();
        let id = egui_mineral::kit::scoped_id(rt.core(), Id::unique(("mineral::drawer", self.group.salt.id_salt())));
        let screen = ctx.content_rect();
        let width = drawer::CONTAINER_WIDTH.min(screen.width());
        let dir = rt.i18n().direction();
        let offset = DrawerMotion::step(rt, id, *open, width);
        let fraction = ((offset + width) / width).clamp(0.0, 1.0);
        let mounted = fraction > 0.0;
        // `horizontalScaleUp`: an overshoot widens the container from the edge.
        let mirror = |r: Rect| dir.mirror(r, screen);
        let panel = mirror(Rect::from_min_size(
            pos2(screen.left() + offset, screen.top()),
            vec2(width, screen.height()),
        ));
        let container = mirror(Rect::from_min_max(
            pos2(screen.left() + offset.min(0.0), screen.top()),
            pos2(screen.left() + offset + width, screen.bottom()),
        ));
        let current = *selected;
        let mut picked = None;
        let (response, close) = ModalPanel {
            id,
            group: &self.group,
            open: *open,
            trigger: None,
            rect: container,
            // `DrawerDefaults.shape`: `CornerLargeEnd`.
            round: crate::actions::button::sided(0.0, f32::from(t.shape.large), dir),
            fill: t.role(drawer::MODAL_CONTAINER_COLOR),
            elevation: drawer::MODAL_CONTAINER_ELEVATION,
            scrim: fraction,
        }
        .show(rt, mounted, |ui, panel_id| {
            let mut target = None;
            let scope = self.group.salt.child("content");
            egui_mineral::layout::child(ui, scope, panel, rt.i18n().direction().column(), |ui| {
                (picked, target) = self.content(ui, id, width, current);
            });
            self.group.emit(rt, panel_id, panel, "navigation", None);
            target
        });
        // A pan that starts on the drawer keeps dragging it after the pointer leaves the container.
        // No MineralUi here.
        let input = rt.core().input();
        let grab = DrawerMotion::load_for(ctx, id, *open, width).grab;
        let pan = input.pan.filter(|p| {
            grab == Some(p.origin) || grab.is_none() && response.as_ref().is_some_and(|r| input.pan_on(r).is_some())
        });
        let sign = dir.sign();
        match (pan, grab) {
            (Some(p), _) => DrawerMotion::drag(
                ctx,
                id,
                open,
                width,
                p.origin,
                sign * p.delta.x,
                p.ended.then_some(sign * p.velocity.x),
            ),
            (None, Some(origin)) => DrawerMotion::drag(ctx, id, open, width, origin, 0.0, Some(0.0)),
            (None, None) => {}
        }
        let dismissed = edge_surface::dismiss(open, close.filter(|_| picked.is_none()));
        if picked.is_some() {
            *open = false;
        }
        let selection = response.map(|response| SelectionResponse::pick(response, selected, picked));
        NavigationDrawerModalResponse { selection, dismissed }
    }

    fn content(
        &self,
        ui: &mut MineralUi<'_, Material>,
        id: Id,
        width: f32,
        selected: usize,
    ) -> (Option<usize>, Option<Id>) {
        let rt = ui.rt();
        let mut ids = Vec::new();
        let mut item_enabled = Vec::new();
        let mut picked = None;
        egui::ScrollArea::vertical()
            .id_salt(id.with("scroll"))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let pad = nx::DRAWER_ITEM_PADDING;
                Frame::new().inner_margin(pad).show(ui, |ui| {
                    ui.set_width((width - 2.0 * pad).max(0.0));
                    ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                    let mut index = 0;
                    let item_h = drawer_item_height(rt.ui(ui).density());
                    for entry in &self.entries {
                        match entry {
                            DrawerEntry::Section(title) => {
                                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), item_h), Sense::hover());
                                let dir = rt.i18n().direction();
                                ui.painter().text(
                                    pos2(dir.start_x(r, Rangef::point(nx::DRAWER_ITEM_START)), r.center().y),
                                    Align2([dir.start_align(), Align::Center]),
                                    title,
                                    drawer::HEADLINE_FONT.font(ui.ctx()),
                                    rt.tokens().role(drawer::HEADLINE_COLOR),
                                );
                            }
                            DrawerEntry::Divider => {
                                let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 17.0), Sense::hover());
                                let col = rt.tokens().role(divider::COLOR);
                                ui.painter().hline(
                                    r.x_range(),
                                    r.center().y,
                                    egui::Stroke::new(divider::THICKNESS, col),
                                );
                            }
                            DrawerEntry::Item(dest) => {
                                let own = crate::widget::item_salt(&self.group.salt, dest.salt.as_ref(), None, index);
                                let item = DrawerItem {
                                    salt: own.child("nav-item"),
                                    key: StableId::of(&own, "nav-item"),
                                    dest,
                                    selected: selected == index,
                                };
                                let r = rt.ui(ui).mineral(item);
                                if r.changed() {
                                    picked = Some(index);
                                }
                                ids.push(r.id);
                                item_enabled.push(!dest.disabled);
                                index += 1;
                            }
                        }
                    }
                });
            });
        RovingFocus::new(Axis::Vertical).update(ui.core(), &ids, |i| item_enabled[i]);
        (picked, ids.get(picked.unwrap_or(selected)).or(ids.first()).copied())
    }
}

struct DrawerItem<'a> {
    salt: KeyedSalt,
    key: StableId,
    dest: &'a NavDestination,
    selected: bool,
}

impl MineralWidget for DrawerItem<'_> {
    type Kit = Material;
    type State = ();
    type Output = ();

    egui_mineral::kit::widget_meta!(salt);

    fn enabled(&self) -> bool {
        !self.dest.disabled
    }

    fn access_key(&self) -> Option<Mnemonic> {
        self.dest.mnemonic
    }

    fn layout(&self, ui: &MineralUi<'_, Material>, constraints: Constraints, (): &Self::State) -> Measure {
        constraints
            .constrain(vec2(constraints.max.x, drawer_item_height(ui.density())))
            .into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Material>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let rt = ui.rt();
        let t = ui.tokens();
        let c = &t.color;
        let response = &frame.response;
        let rect = frame.zones.visual;
        let dis = self.dest.disabled;
        let press = parts::press(ui, frame, Availability::disabled_if(dis));
        let i = press.interaction;
        let clicked = press.clicked;
        let round = common::radius(rect, drawer::ACTIVE_INDICATOR_SHAPE.scale().radius(&t.shape));
        // `NavigationDrawerItemDefaults.colors()`; the container switches without animation.
        let (icon_col, text_col, ripple) = if self.selected {
            (
                t.role(drawer::ACTIVE_ICON_COLOR),
                t.role(drawer::ACTIVE_LABEL_TEXT_COLOR),
                t.role(drawer::ACTIVE_LABEL_TEXT_COLOR),
            )
        } else {
            (
                t.role(drawer::INACTIVE_ICON_COLOR),
                t.role(drawer::INACTIVE_LABEL_TEXT_COLOR),
                c.on_surface,
            )
        };
        let (icon_col, text_col) = if dis {
            (
                with_alpha(icon_col, nx::DISABLED_ALPHA),
                with_alpha(text_col, nx::DISABLED_ALPHA),
            )
        } else {
            (icon_col, text_col)
        };
        let p = ui.painter();
        if self.selected {
            p.rect_filled(rect, round, t.role(drawer::ACTIVE_INDICATOR_COLOR));
        }
        let icon = common::selection_icon(self.dest.icon, self.selected);
        let dir = ui.i18n().direction();
        let x = |along: f32| dir.start_x(rect, Rangef::point(along));
        let icon_size = drawer::ICON_SIZE;
        common::icon(
            p,
            pos2(x(nx::DRAWER_ITEM_START + icon_size / 2.0), rect.center().y),
            icon,
            icon_size,
            icon_col,
        );
        let label = parts::label_galley(
            rt.core(),
            p,
            &self.dest.label,
            drawer::LABEL_TEXT_FONT.font(ui.ctx()),
            text_col,
            self.dest.mnemonic,
        );
        let from = nx::DRAWER_ITEM_START + icon_size + nx::DRAWER_ITEM_SPACE;
        let left = dir.start_x(rect, Rangef::new(from, from + label.size().x));
        p.galley(pos2(left, rect.center().y - label.size().y / 2.0), label, text_col);
        if let Some(b) = self.dest.badge {
            let end = pos2(x(rect.width() - nx::DRAWER_ITEM_END), rect.center().y);
            let align = Align2([dir.end_align(), Align::Center]);
            match b.text(ui.i18n()) {
                Some(text) => {
                    let badge_col = if self.selected {
                        t.role(drawer::ACTIVE_LABEL_TEXT_COLOR)
                    } else {
                        t.role(drawer::LARGE_BADGE_LABEL_COLOR)
                    };
                    p.text(
                        end,
                        align,
                        text,
                        drawer::LARGE_BADGE_LABEL_FONT.font(ui.ctx()),
                        badge_col,
                    );
                }
                None => {
                    p.circle_filled(end, badge_spec::SIZE / 2.0, t.role(badge_spec::COLOR));
                }
            }
        }
        // `Surface(selected, onClick)`: bounded ripple in its content color.
        indication::paint(ui, response, rect, round, ripple, i, &t);
        WidgetResponse::new(response.clone(), ()).with_changed(clicked && !self.selected)
    }

    fn semantics(&self, i18n: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        Some(self.dest.node(self.key.clone(), self.selected, i18n))
    }
}

#[track_caller]
pub(crate) fn clamp_selected(selected: &mut usize, len: usize, what: &str) {
    if let Some(i) = egui_mineral::kit::sanitize::index(*selected, len, what) {
        *selected = i;
    }
}

fn drawer_motion_id(base: egui::Id) -> egui::Id {
    base.with("drawer-motion")
}

fn drawer_offset_id(base: egui::Id) -> egui::Id {
    base.with("drawer-offset")
}

fn modal_id(base: egui::Id) -> egui::Id {
    base.with("modal")
}

fn trigger_id(base: egui::Id) -> egui::Id {
    base.with("trigger")
}
