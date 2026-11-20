// Compose FloatingToolbar.kt, AppBar.kt FlexibleBottomAppBar
use crate::Material;
use crate::MaterialExt;
use crate::MaterialUiExt as _;
use crate::actions::button::Surface;
use crate::{actions::button::Elevation, spec::ElevationLevel};
use egui_mineral::i18n::I18n;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;

use egui::{Id, Layout, Rangef, Rect, Response, Sense, Ui, UiBuilder, pos2, vec2};
use egui_mineral::widget::Availability;
use egui_mineral::{
    action::{Clicked, ItemResponses},
    focus::RovingFocus,
    i18n::Direction,
    layout::{Axis, Constraints, Edge, Measure},
    semantic::{self, Role, SemanticNode, StableId},
    shortcut,
    widget::{MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::actions::icon_button::{IconButton, Palette};
use crate::{
    Icon,
    adaptive::Density,
    motion,
    spec::{
        ColorRole,
        defaults::toolbar as cx,
        tokens::{docked_toolbar as docked, fab_baseline, fab_medium, floating_toolbar as floating},
    },
};
use crate::{
    IconAction,
    widget::item_salt,
    widget::{
        common::{self},
        parts,
    },
};

fn slot(ui: &MineralUi<'_, Material>) -> f32 {
    IconButton::slot(ui.density())
}

// `height` at default density; the padding around the action slot stays.
fn dense_height(height: f32, item: f32) -> f32 {
    height - (crate::adaptive::min_interactive(Density::DEFAULT) - item)
}

/// Result of [`DockedToolbar::show`] and [`FloatingToolbar::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ToolbarResponse {
    /// Toolbar container response.
    pub response: Response,
    /// Clicked action without a [`IconAction::command`], with its [`IconAction::key`]; the index counts icon
    /// actions only.
    pub clicked: Option<Clicked>,
    /// Targets of the shown icon actions by index (custom items excluded) and [`IconAction::key`], e.g. to
    /// anchor a [`Menu::show_at`](crate::selection::Menu::show_at); actions that are not shown (overflowed,
    /// toolbar off screen or collapsed) have none.
    pub actions: ItemResponses<Rect>,
    /// Paired FAB clicked; see [`FloatingToolbar::fab`].
    pub fab_clicked: bool,
}

// Room offered to a custom item so it measures at its natural width.
const NATURAL_WIDTH: f32 = 10_000.0;

enum Item<'a> {
    Action(IconAction),
    Custom(Custom<'a>),
}

type CustomFn<'a> = Box<dyn FnOnce(&mut MineralUi<'_, Material>) -> Response + 'a>;

struct Custom<'a> {
    salt: KeyedSalt,
    add: Option<CustomFn<'a>>,
}

impl Item<'_> {
    fn is_custom(&self) -> bool {
        matches!(self, Self::Custom(_))
    }
}

fn custom_id(toolbar: &KeyedSalt, c: &Custom<'_>) -> Id {
    Id::unique(("mineral::toolbar_custom", toolbar.id_salt(), c.salt.id_salt()))
}

// Main-axis length per item, `None` when hidden. Custom items take the width measured last pass; vertical
// toolbars and, beyond `room`, trailing custom items are hidden.
fn plan(ui: &Ui, salt: &KeyedSalt, list: &[Item<'_>], item: f32, axis: Axis, room: Option<f32>) -> Vec<Option<f32>> {
    let mut widths: Vec<Option<f32>> = list
        .iter()
        .map(|i| match i {
            Item::Action(_) => Some(item),
            Item::Custom(_) if axis == Axis::Vertical => None,
            Item::Custom(c) => {
                Some(egui_mineral::perf::size_or_discard(ui.ctx(), custom_id(salt, c)).map_or(0.0, |s| s.x))
            }
        })
        .collect();
    if let Some(room) = room {
        while widths.iter().flatten().sum::<f32>() > room {
            let tail = list
                .iter()
                .zip(&widths)
                .rposition(|(i, w)| i.is_custom() && w.is_some_and(|w| w > 0.0));
            match tail.and_then(|i| widths.get_mut(i)) {
                Some(w) => *w = None,
                None => break,
            }
        }
    }
    widths
}

fn total(plan: &[Option<f32>]) -> f32 {
    plan.iter().flatten().sum()
}

#[expect(clippy::too_many_arguments, reason = "one laid-out row")]
fn items(
    ui: &mut MineralUi<'_, Material>,
    rect: Rect,
    salt: &KeyedSalt,
    list: &mut [Item<'_>],
    plan: &[Option<f32>],
    palette: Palette,
    axis: Axis,
    gap: f32,
) -> (Option<Clicked>, ItemResponses<Rect>) {
    let rt = ui.rt();
    let layout = match axis {
        Axis::Horizontal => rt.i18n().direction().row(),
        Axis::Vertical => Layout::top_down(egui::Align::Center),
    };
    let item = slot(ui);
    let mut clicked = None;
    let mut rects = ItemResponses::default();
    // (widget id, enabled) in roving order
    let mut stops: Vec<(Id, bool)> = Vec::new();
    ui.scope_builder(UiBuilder::new().max_rect(rect).layout(layout), |ui| {
        ui.spacing_mut().item_spacing = vec2(gap, gap);
        let mut index = 0;
        for (entry, shown) in list.iter_mut().zip(plan) {
            if shown.is_none() {
                continue;
            }
            match entry {
                Item::Action(a) => {
                    let i = index;
                    index += 1;
                    let mut b = IconButton::new(
                        item_salt(salt, a.salt.as_ref(), a.command.as_ref(), i),
                        a.icon,
                        &a.label,
                    )
                    .disabled(a.disabled);
                    if let Some(sel) = a.selected {
                        b = b.selected(sel);
                    }
                    b.palette = palette;
                    let r = rt.ui(ui).mineral(b);
                    if r.clicked() {
                        match &a.command {
                            Some(c) => shortcut::fire(ui.ctx(), c),
                            None => clicked = Some(Clicked::new(i, a.action_key.clone())),
                        }
                    }
                    stops.push((r.response().id, !a.disabled));
                    rects.push(i, a.action_key.clone(), r.response().rect);
                }
                Item::Custom(c) => {
                    let Some(add) = c.add.take() else { continue };
                    let cache = custom_id(salt, c);
                    let out = ui.allocate_ui_with_layout(vec2(NATURAL_WIDTH, item), layout, |ui| add(&mut rt.ui(ui)));
                    egui_mineral::perf::store_size(ui.ctx(), cache, out.response.rect.size());
                    if out.inner.sense.is_focusable() {
                        stops.push((out.inner.id, true));
                    }
                }
            }
        }
    });
    let ids: Vec<Id> = stops.iter().map(|s| s.0).collect();
    RovingFocus::new(axis).update(ui.core(), &ids, |i| stops.get(i).is_some_and(|s| s.1));
    (clicked, rects)
}

fn announce(ctx: &egui::Context, response: &Response, core: &Core<'_>, expanded: Option<bool>) {
    semantic::emit(ctx, Some(response.id), response.rect, || {
        let mut n = SemanticNode::new(StableId::of(&core.salt, "toolbar"), Role::Toolbar, &core.label);
        n.state.expanded = expanded;
        n.state.disabled = core.disabled;
        n
    });
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Docked,
    Floating,
}

struct PairedFab {
    icon: Icon,
    label: String,
    position: Edge,
}

/// Compose `FloatingToolbarDefaults.standardFloatingToolbarColors` / `vibrantFloatingToolbarColors`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ToolbarColors {
    #[default]
    /// Surface container colors (`FloatingToolbarDefaults.standardFloatingToolbarColors`).
    Standard,
    /// Primary container colors (`vibrantFloatingToolbarColors`).
    Vibrant,
}

/// MD3 toolbar (Compose `DockedToolbar`, `HorizontalFloatingToolbar`); the constructors pick the
/// kind: [`Toolbar::docked`] or [`Toolbar::floating`].
#[derive(Clone, Copy, Debug)]
pub enum Toolbar {}

impl Toolbar {
    /// `FlexibleBottomAppBar`; `label` names the toolbar for assistive technology.
    pub fn docked<'a>(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> DockedToolbar<'a> {
        let id_salt: KeyedSalt = id_salt.into();
        DockedToolbar(Core::new(id_salt, label))
    }

    /// `HorizontalFloatingToolbar`; `label` names the toolbar for assistive technology.
    pub fn floating<'a>(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> FloatingToolbar<'a> {
        let id_salt: KeyedSalt = id_salt.into();
        FloatingToolbar {
            core: Core::new(id_salt, label),
            axis: Axis::Horizontal,
            fab: None,
            expanded: true,
        }
    }
}

struct Core<'a> {
    salt: KeyedSalt,
    label: String,
    vibrant: bool,
    disabled: bool,
    items: Vec<Item<'a>>,
}

impl Core<'_> {
    fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            label: label.into(),
            vibrant: false,
            disabled: false,
            items: Vec::new(),
        }
    }

    fn apply_disabled(&mut self) {
        for i in &mut self.items {
            if let Item::Action(a) = i {
                a.disabled |= self.disabled;
            }
        }
    }

    fn container(&self, kind: Kind) -> (ColorRole, Palette) {
        match (kind, self.vibrant) {
            (_, true) => (floating::VIBRANT_CONTAINER_COLOR, Palette::Vibrant),
            (Kind::Docked, false) => (docked::CONTAINER_COLOR, Palette::Surface),
            (Kind::Floating, false) => (floating::STANDARD_CONTAINER_COLOR, Palette::Surface),
        }
    }
}

/// Setters of [`Core`] for a toolbar kind that exposes it as `self.core()`.
macro_rules! core_setters {
    () => {
        /// Replaces the `<id_salt>` key prefix.
        pub fn agent_key(mut self, key: impl Into<StableId>) -> Self {
            let core = self.core();
            core.salt = core.salt.clone().with_key(key.into());
            self
        }

        /// Default [`ToolbarColors::Standard`].
        pub fn colors(mut self, colors: ToolbarColors) -> Self {
            self.core().vibrant = colors == ToolbarColors::Vibrant;
            self
        }

        crate::widget::setters! {
            /// Disables every action.
            disabled |s| s.core().disabled
        }

        /// Appends one icon button.
        pub fn action(mut self, action: impl Into<IconAction>) -> Self {
            self.core().items.push(Item::Action(action.into()));
            self
        }

        /// Appends icon buttons.
        pub fn actions(mut self, actions: impl IntoIterator<Item = impl Into<IconAction>>) -> Self {
            self.core()
                .items
                .extend(actions.into_iter().map(|a| Item::Action(a.into())));
            self
        }

        /// Appends a content slot laid out in the toolbar row at the action height, between the
        /// actions in call order; `add` returns the response of the item's focusable widget, which
        /// joins the arrow-key roving order (a group that roves itself returns its non-focusable
        /// group response and is entered by Tab). Docked toolbars hide trailing custom items that do
        /// not fit; vertical floating toolbars hide them all. The first pass measures the item.
        pub fn custom(
            mut self,
            id_salt: impl Into<KeyedSalt>,
            add: impl FnOnce(&mut MineralUi<'_, Material>) -> Response + 'a,
        ) -> Self {
            self.core().items.push(Item::Custom(Custom {
                salt: id_salt.into(),
                add: Some(Box::new(add)),
            }));
            self
        }
    };
}

/// Docked toolbar (`FlexibleBottomAppBar`), built by [`Toolbar::docked`]; always horizontal.
#[must_use = "call .show(..)"]
pub struct DockedToolbar<'a>(Core<'a>);

egui_mineral::kit::opaque_debug!(DockedToolbar<'_>);

impl<'a> DockedToolbar<'a> {
    fn core(&mut self) -> &mut Core<'a> {
        &mut self.0
    }

    core_setters!();

    /// Spans the available width, `FlexibleHorizontalArrangement` (`SpaceBetween`).
    pub fn show(self, ui: &mut MineralUi<'_, Material>) -> ToolbarResponse {
        let rt = ui.rt();
        let mut core = self.0;
        core.apply_disabled();
        let t = rt.tokens();
        let item = slot(ui);
        let w = ui.available_width();
        let id = ui.make_persistent_id(core.salt.id_salt());
        let (rect, _) = ui.allocate_exact_size(vec2(w, dense_height(docked::CONTAINER_HEIGHT, item)), Sense::hover());
        let response = ui.interact(rect, id, Sense::hover());
        let (mut clicked, mut shown) = (None, ItemResponses::default());
        if ui.is_rect_visible(rect) {
            let (fill, palette) = core.container(Kind::Docked);
            let r = docked::CONTAINER_SHAPE.corner_radius(&t.shape, rt.i18n().direction());
            ui.painter().rect_filled(rect, r, t.role(fill));
            let inner = Rect::from_min_max(
                pos2(
                    rect.left() + docked::CONTAINER_LEADING_SPACE,
                    rect.center().y - item / 2.0,
                ),
                pos2(
                    rect.right() - docked::CONTAINER_TRAILING_SPACE,
                    rect.center().y + item / 2.0,
                ),
            );
            let widths = plan(ui, &core.salt, &core.items, item, Axis::Horizontal, Some(inner.width()));
            let count = widths.iter().flatten().count() as f32;
            let custom = core
                .items
                .iter()
                .zip(&widths)
                .any(|(i, w)| i.is_custom() && w.is_some());
            // Custom items pack at the start; actions alone spread edge to edge.
            let gap = if custom {
                0.0
            } else {
                ((inner.width() - total(&widths)) / (count - 1.0).max(1.0)).max(0.0)
            };
            let mut child = ui.new_child(UiBuilder::new().id_salt(core.salt.id_salt()).max_rect(rect));
            (clicked, shown) = items(
                &mut rt.ui(&mut child),
                inner,
                &core.salt,
                &mut core.items,
                &widths,
                palette,
                Axis::Horizontal,
                gap,
            );
        }
        announce(ui.ctx(), &response, &core, None);
        ToolbarResponse {
            response,
            clicked,
            actions: shown,
            fab_clicked: false,
        }
    }
}

/// Floating toolbar (`HorizontalFloatingToolbar`, `VerticalFloatingToolbar`), built by
/// [`Toolbar::floating`]; can stack vertically and pair with a FAB.
#[must_use = "call .show(..)"]
pub struct FloatingToolbar<'a> {
    core: Core<'a>,
    axis: Axis,
    fab: Option<PairedFab>,
    expanded: bool,
}

egui_mineral::kit::opaque_debug!(FloatingToolbar<'_>);

impl<'a> FloatingToolbar<'a> {
    fn core(&mut self) -> &mut Core<'a> {
        &mut self.core
    }

    core_setters!();

    /// Default [`Axis::Horizontal`].
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    /// Pairs the toolbar with a FAB that grows as the toolbar collapses, at `position`
    /// (Compose `FloatingToolbarHorizontalFabPosition`, `FloatingToolbarVerticalFabPosition`).
    pub fn fab(mut self, icon: impl Into<Icon>, label: impl Into<String>, position: Edge) -> Self {
        self.fab = Some(PairedFab {
            icon: icon.into(),
            label: label.into(),
            position,
        });
        self
    }

    /// Collapsed, a FAB-paired toolbar shows only its FAB.
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    /// Lays out in place.
    pub fn show(mut self, ui: &mut MineralUi<'_, Material>) -> ToolbarResponse {
        let rt = ui.rt();
        self.core.apply_disabled();
        let axis = self.axis;
        let ctx = ui.ctx().clone();
        let id = ui.make_persistent_id(self.core.salt.id_salt());
        let item = slot(ui);
        let widths = plan(ui, &self.core.salt, &self.core.items, item, axis, None);
        // `ContentPadding` around the actions; `heightIn(min = ContainerSize)` across them.
        let length = floating::CONTAINER_LEADING_SPACE + total(&widths) + floating::CONTAINER_TRAILING_SPACE;
        let height = dense_height(floating::CONTAINER_HEIGHT, item);
        let across = height.max(floating::CONTAINER_LEADING_SPACE + item + floating::CONTAINER_TRAILING_SPACE);
        let progress = egui_mineral::motion::spring(
            rt.core(),
            id.with("expanded"),
            f32::from(self.expanded),
            motion::FAST_SPATIAL,
        );
        let fab_range = fab_baseline::CONTAINER_WIDTH..=fab_medium::CONTAINER_WIDTH;
        let size = match &self.fab {
            None => axis.vec2(length, across),
            Some(_) => axis.vec2(
                length + cx::TOOLBAR_TO_FAB_GAP + fab_range.start(),
                across.max(*fab_range.end()),
            ),
        };
        let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
        let response = ui.interact(rect, id, Sense::hover());
        let mut out = ToolbarResponse {
            response,
            clicked: None,
            actions: ItemResponses::default(),
            fab_clicked: false,
        };
        if !ui.is_rect_visible(rect) {
            return out;
        }
        let dir = match axis {
            Axis::Horizontal => rt.i18n().direction(),
            Axis::Vertical => Direction::Ltr,
        };
        // Main-axis span `[from, from + len]` of `rect`, mirrored in RTL.
        let span = |from: f32, len: f32, cross: f32| {
            let start = axis.main_range(rect).min + from;
            let mid = axis.cross(rect.center().to_vec2());
            let r = axis.rect(
                Rangef::new(start, start + len),
                Rangef::new(mid - cross / 2.0, mid + cross / 2.0),
            );
            dir.mirror(r, rect)
        };
        let total = axis.main(rect.size());
        let place = match &self.fab {
            None => {
                let bar = span(0.0, length, across);
                BarPlace {
                    bar,
                    content: bar,
                    elevation: cx::FLOATING_ELEVATION.dp(),
                }
            }
            Some(fab) => {
                let len = length * progress.max(0.0);
                let fab_side = egui::lerp(fab_range.clone(), 1.0 - progress);
                // The bar shrinks toward the FAB; its content stays at the bar start.
                let (bar_from, fab_from) = match fab.position {
                    Edge::End => (length - len, total - fab_side),
                    Edge::Start => (total - length, 0.0),
                };
                let fab_rect = span(fab_from, fab_side, fab_side);
                out.fab_clicked = ui
                    .scope_builder(UiBuilder::new().max_rect(fab_rect), |ui| {
                        rt.ui(ui)
                            .mineral(PairedFabButton {
                                salt: self.core.salt.child("fab"),
                                key: StableId::of(&self.core.salt, "fab"),
                                fab,
                                side: fab_side,
                                vibrant: self.core.vibrant,
                            })
                            .clicked()
                    })
                    .inner;
                let elevation = egui::lerp(
                    cx::FLOATING_COLLAPSED_ELEVATION_WITH_FAB.dp()..=cx::FLOATING_EXPANDED_ELEVATION_WITH_FAB.dp(),
                    progress.min(1.0),
                );
                BarPlace {
                    bar: span(bar_from, len, height),
                    content: span(bar_from, length, height),
                    elevation,
                }
            }
        };
        (out.clicked, out.actions) = self.floating_bar(ui, rect, place, axis, &widths);
        let expanded = self.fab.as_ref().map(|_| self.expanded);
        announce(&ctx, &out.response, &self.core, expanded);
        out
    }
}

#[derive(Clone, Copy)]
struct BarPlace {
    bar: Rect,
    content: Rect,
    elevation: f32,
}

impl FloatingToolbar<'_> {
    fn floating_bar(
        &mut self,
        ui: &mut MineralUi<'_, Material>,
        rect: Rect,
        place: BarPlace,
        axis: Axis,
        widths: &[Option<f32>],
    ) -> (Option<Clicked>, ItemResponses<Rect>) {
        let BarPlace {
            bar,
            content,
            elevation,
        } = place;
        let rt = ui.rt();
        if bar.width() <= 0.0 || bar.height() <= 0.0 {
            return (None, ItemResponses::default());
        }
        let t = rt.tokens();
        let item = slot(ui);
        let (fill, palette) = self.core.container(Kind::Floating);
        let r = floating::CONTAINER_SHAPE.corner_radius(&t.shape, rt.i18n().direction());
        let r = common::radius(bar, r.nw);
        if elevation > 0.0 {
            ui.painter().add(t.shadow_dp(elevation).as_shape(bar, r));
        }
        ui.painter().rect_filled(bar, r, t.role(fill));
        let inner = Rect::from_center_size(content.center(), axis.vec2(total(widths), item));
        let mut child = ui.new_child(UiBuilder::new().id_salt(self.core.salt.id_salt()).max_rect(rect));
        child.set_clip_rect(bar.intersect(ui.clip_rect()));
        items(
            &mut rt.ui(&mut child),
            inner,
            &self.core.salt,
            &mut self.core.items,
            widths,
            palette,
            axis,
            0.0,
        )
    }
}

// Compose FloatingToolbarDefaults.StandardFloatingActionButton, VibrantFloatingActionButton, FabSizeRange
struct PairedFabButton<'a> {
    salt: KeyedSalt,
    key: StableId,
    fab: &'a PairedFab,
    side: f32,
    vibrant: bool,
}

impl MineralWidget for PairedFabButton<'_> {
    type Kit = Material;
    type State = ();
    type Output = ();

    egui_mineral::kit::widget_meta!(salt);

    fn layout(&self, _ui: &MineralUi<'_, Material>, c: Constraints, (): &Self::State) -> Measure {
        c.constrain(vec2(self.side, self.side)).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Material>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let rect = frame.zones.visual;
        // `defaultFloatingToolbarStandardColors` / `VibrantColors` FAB colors.
        let (bg, fg) = if self.vibrant {
            (ColorRole::TertiaryContainer, ColorRole::OnTertiaryContainer)
        } else {
            (ColorRole::PrimaryContainer, ColorRole::OnPrimaryContainer)
        };
        let (bg, fg) = (t.role(bg), t.role(fg));
        let r = fab_baseline::CONTAINER_SHAPE.corner_radius(&t.shape, ui.i18n().direction());
        let i = parts::press(ui, frame, Availability::Enabled).interaction;
        let elevation = Elevation {
            rest: cx::FAB_ELEVATION,
            hovered: cx::FAB_HOVERED_ELEVATION,
            focused: cx::FAB_ELEVATION,
            pressed: cx::FAB_ELEVATION,
            disabled: ElevationLevel::Level0,
        };
        let surface = Surface {
            rect,
            radius: r,
            fill: bg,
            elevation,
            outline: None,
        };
        surface.paint(ui, &frame.response, fg, i, &t);
        common::icon(ui.painter(), rect.center(), self.fab.icon, fab_baseline::ICON_SIZE, fg);
        WidgetResponse::new(frame.response.clone(), ())
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        Some(SemanticNode::new(self.key.clone(), Role::Button, &self.fab.label).clickable(Availability::Enabled))
    }
}
