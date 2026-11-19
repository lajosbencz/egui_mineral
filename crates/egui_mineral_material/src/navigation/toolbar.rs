// Compose FloatingToolbar.kt, AppBar.kt FlexibleBottomAppBar
use crate::Material;
use crate::MaterialExt;
use crate::Rt;
use crate::actions::button::Surface;
use crate::{actions::button::Elevation, spec::ElevationLevel};
use egui_mineral::i18n::I18n;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;

use egui::{Id, Layout, Rangef, Rect, Response, Sense, UiBuilder, pos2, vec2};
use egui_mineral::widget::Availability;
use egui_mineral::{
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
    adaptive::MIN_INTERACTIVE,
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

// Compose IconButton minimumInteractiveComponentSize
fn slot(rt: &Rt) -> f32 {
    MIN_INTERACTIVE.points(rt.core())
}

/// Result of [`DockedToolbar::show`] and [`FloatingToolbar::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ToolbarResponse {
    /// Toolbar container response.
    pub response: Response,
    /// Index of the clicked action.
    pub clicked: Option<usize>,
    /// Shown action targets, in action order; empty while the toolbar is off screen or collapsed. Anchor a menu
    /// to one with [`Menu::show_at`](crate::selection::Menu::show_at).
    pub actions: Vec<Rect>,
    /// Paired FAB clicked; see [`FloatingToolbar::fab`].
    pub fab_clicked: bool,
}

fn actions(
    ui: &mut MineralUi<'_, Material>,
    rect: Rect,
    salt: &KeyedSalt,
    list: &[IconAction],
    palette: Palette,
    axis: Axis,
    gap: f32,
) -> (Option<usize>, Vec<Rect>) {
    let rt = ui.rt();
    let layout = match axis {
        Axis::Horizontal => rt.i18n().direction().row(),
        Axis::Vertical => Layout::top_down(egui::Align::Center),
    };
    let mut clicked = None;
    let shown: Vec<(Id, Rect)> = ui
        .scope_builder(UiBuilder::new().max_rect(rect).layout(layout), |ui| {
            ui.spacing_mut().item_spacing = vec2(gap, gap);
            list.iter()
                .enumerate()
                .map(|(i, a)| {
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
                            None => clicked = Some(i),
                        }
                    }
                    (r.response().id, r.response().rect)
                })
                .collect()
        })
        .inner;
    let (ids, rects): (Vec<Id>, Vec<Rect>) = shown.into_iter().unzip();
    RovingFocus::new(axis).update(ui.core(), &ids, |i| !list[i].disabled);
    (clicked, rects)
}

fn announce(ctx: &egui::Context, response: &Response, core: &Core, expanded: Option<bool>) {
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
    pub fn docked(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> DockedToolbar {
        let id_salt: KeyedSalt = id_salt.into();
        DockedToolbar(Core::new(id_salt, label))
    }

    /// `HorizontalFloatingToolbar`; `label` names the toolbar for assistive technology.
    pub fn floating(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> FloatingToolbar {
        let id_salt: KeyedSalt = id_salt.into();
        FloatingToolbar {
            core: Core::new(id_salt, label),
            axis: Axis::Horizontal,
            fab: None,
            expanded: true,
        }
    }
}

struct Core {
    salt: KeyedSalt,
    label: String,
    vibrant: bool,
    disabled: bool,
    actions: Vec<IconAction>,
}

impl Core {
    fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            label: label.into(),
            vibrant: false,
            disabled: false,
            actions: Vec::new(),
        }
    }

    fn apply_disabled(&mut self) {
        for a in &mut self.actions {
            a.disabled |= self.disabled;
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
            self.core().actions.push(action.into());
            self
        }

        /// Appends icon buttons.
        pub fn actions(mut self, actions: impl IntoIterator<Item = impl Into<IconAction>>) -> Self {
            self.core().actions.extend(actions.into_iter().map(Into::into));
            self
        }
    };
}

/// Docked toolbar (`FlexibleBottomAppBar`), built by [`Toolbar::docked`]; always horizontal.
#[must_use = "call .show(..)"]
pub struct DockedToolbar(Core);

egui_mineral::kit::opaque_debug!(DockedToolbar);

impl DockedToolbar {
    fn core(&mut self) -> &mut Core {
        &mut self.0
    }

    core_setters!();

    /// Spans the available width, `FlexibleHorizontalArrangement` (`SpaceBetween`).
    pub fn show(self, ui: &mut MineralUi<'_, Material>) -> ToolbarResponse {
        let rt = ui.rt();
        let mut core = self.0;
        core.apply_disabled();
        let t = rt.tokens();
        let item = slot(ui.rt());
        let w = ui.available_width();
        let id = ui.make_persistent_id(core.salt.id_salt());
        let (rect, _) = ui.allocate_exact_size(vec2(w, docked::CONTAINER_HEIGHT), Sense::hover());
        let response = ui.interact(rect, id, Sense::hover());
        let (mut clicked, mut shown) = (None, Vec::new());
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
            let n = core.actions.len() as f32;
            let gap = ((inner.width() - n * item) / (n - 1.0).max(1.0)).max(0.0);
            let mut child = ui.new_child(UiBuilder::new().id_salt(core.salt.id_salt()).max_rect(rect));
            (clicked, shown) = actions(
                &mut rt.ui(&mut child),
                inner,
                &core.salt,
                &core.actions,
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
pub struct FloatingToolbar {
    core: Core,
    axis: Axis,
    fab: Option<PairedFab>,
    expanded: bool,
}

egui_mineral::kit::opaque_debug!(FloatingToolbar);

impl FloatingToolbar {
    fn core(&mut self) -> &mut Core {
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
        let item = slot(rt);
        let n = self.core.actions.len() as f32;
        // `ContentPadding` around the actions; `heightIn(min = ContainerSize)` across them.
        let length = floating::CONTAINER_LEADING_SPACE + n * item + floating::CONTAINER_TRAILING_SPACE;
        let across = floating::CONTAINER_HEIGHT
            .max(floating::CONTAINER_LEADING_SPACE + item + floating::CONTAINER_TRAILING_SPACE);
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
            actions: Vec::new(),
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
                    bar: span(bar_from, len, floating::CONTAINER_HEIGHT),
                    content: span(bar_from, length, floating::CONTAINER_HEIGHT),
                    elevation,
                }
            }
        };
        (out.clicked, out.actions) = self.floating_bar(ui, rect, place, axis);
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

impl FloatingToolbar {
    fn floating_bar(
        &self,
        ui: &mut MineralUi<'_, Material>,
        rect: Rect,
        place: BarPlace,
        axis: Axis,
    ) -> (Option<usize>, Vec<Rect>) {
        let BarPlace {
            bar,
            content,
            elevation,
        } = place;
        let rt = ui.rt();
        if bar.width() <= 0.0 || bar.height() <= 0.0 {
            return (None, Vec::new());
        }
        let t = rt.tokens();
        let item = slot(ui.rt());
        let n = self.core.actions.len() as f32;
        let (fill, palette) = self.core.container(Kind::Floating);
        let r = floating::CONTAINER_SHAPE.corner_radius(&t.shape, rt.i18n().direction());
        let r = common::radius(bar, r.nw);
        if elevation > 0.0 {
            ui.painter().add(t.shadow_dp(elevation).as_shape(bar, r));
        }
        ui.painter().rect_filled(bar, r, t.role(fill));
        let inner = Rect::from_center_size(content.center(), axis.vec2(n * item, item));
        let mut child = ui.new_child(UiBuilder::new().id_salt(self.core.salt.id_salt()).max_rect(rect));
        child.set_clip_rect(bar.intersect(ui.clip_rect()));
        actions(
            &mut rt.ui(&mut child),
            inner,
            &self.core.salt,
            &self.core.actions,
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
