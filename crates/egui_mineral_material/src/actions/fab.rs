use std::sync::Arc;

use crate::Material;
use crate::MaterialExt;
use crate::Rt;
use crate::actions::button::Surface;
use egui::{Color32, CornerRadius, Galley, IdSalt, KeyboardShortcut, Painter, Rangef, Rect, pos2, vec2};
use egui_mineral::i18n::I18n;
use egui_mineral::semantic::AgentKey;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::Availability;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    layout::{Constraints, Measure, MinTarget},
    semantic::{Role, SemanticNode},
    widget::{MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::widget::{
    common::{self},
    parts,
};
use crate::{
    MaterialIcon, motion,
    spec::{
        ColorRole, ShapeScale, TypeScale, defaults,
        tokens::{
            extended_fab_large, extended_fab_medium, extended_fab_primary, extended_fab_small, fab_baseline, fab_large,
            fab_medium, fab_primary_container, fab_secondary_container, fab_small,
        },
    },
    tokens::Tokens,
};
use crate::{actions::button::Elevation, spec::ElevationLevel};

/// Compose `FloatingActionButton` sizes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum FabSize {
    /// `SmallFloatingActionButton` (`FabSmallTokens`).
    Small,
    #[default]
    /// `FloatingActionButton` (`FabBaselineTokens`).
    Baseline,
    /// `MediumFloatingActionButton` (`FabMediumTokens`).
    Medium,
    /// `LargeFloatingActionButton` (`FabLargeTokens`).
    Large,
}

impl FabSize {
    pub(crate) fn side(self) -> f32 {
        match self {
            Self::Small => fab_small::CONTAINER_HEIGHT,
            Self::Baseline => fab_baseline::CONTAINER_HEIGHT,
            Self::Medium => fab_medium::CONTAINER_HEIGHT,
            Self::Large => fab_large::CONTAINER_HEIGHT,
        }
    }

    fn width(self) -> f32 {
        match self {
            Self::Small => fab_small::CONTAINER_WIDTH,
            Self::Baseline => fab_baseline::CONTAINER_WIDTH,
            Self::Medium => fab_medium::CONTAINER_WIDTH,
            Self::Large => fab_large::CONTAINER_WIDTH,
        }
    }

    pub(crate) fn corner(self, t: &Tokens) -> u8 {
        let scale = match self {
            Self::Small => fab_small::CONTAINER_SHAPE.scale(),
            Self::Baseline => fab_baseline::CONTAINER_SHAPE.scale(),
            Self::Medium => defaults::fab::MEDIUM_SHAPE,
            Self::Large => fab_large::CONTAINER_SHAPE.scale(),
        };
        scale.radius(&t.shape)
    }

    pub(crate) fn icon(self) -> f32 {
        match self {
            Self::Small => fab_small::ICON_SIZE,
            Self::Baseline => fab_baseline::ICON_SIZE,
            Self::Medium => fab_medium::ICON_SIZE,
            Self::Large => defaults::fab::LARGE_ICON_SIZE,
        }
    }
}

/// Compose `ExtendedFloatingActionButton` sizes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum ExtendedFabSize {
    #[default]
    /// `SmallExtendedFloatingActionButton`.
    Small,
    /// `MediumExtendedFloatingActionButton`.
    Medium,
    /// `LargeExtendedFloatingActionButton`.
    Large,
}

#[derive(Clone, Copy, Debug)]
struct ExtMetrics {
    height: f32,
    shape: ShapeScale,
    icon: f32,
    lead: f32,
    trail: f32,
    gap: f32,
    font: TypeScale,
}

macro_rules! ext {
    ($m:ident, $shape:expr, $gap:expr, $font:expr) => {
        ExtMetrics {
            height: $m::CONTAINER_HEIGHT,
            shape: $shape,
            icon: $m::ICON_SIZE,
            lead: $m::LEADING_SPACE,
            trail: $m::TRAILING_SPACE,
            gap: $gap,
            font: $font,
        }
    };
}

impl ExtendedFabSize {
    // Compose *ExtendedFabTextStyle
    fn metrics(self) -> ExtMetrics {
        match self {
            Self::Small => ext!(
                extended_fab_small,
                extended_fab_small::CONTAINER_SHAPE.scale(),
                extended_fab_small::ICON_LABEL_SPACE,
                TypeScale::TitleMedium
            ),
            Self::Medium => ext!(
                extended_fab_medium,
                defaults::fab::MEDIUM_SHAPE,
                defaults::fab::EXTENDED_MEDIUM_ICON_LABEL_SPACE,
                TypeScale::TitleLarge
            ),
            Self::Large => ext!(
                extended_fab_large,
                extended_fab_large::CONTAINER_SHAPE.scale(),
                defaults::fab::EXTENDED_LARGE_ICON_LABEL_SPACE,
                TypeScale::HeadlineSmall
            ),
        }
    }
}

/// FAB color set (Compose `FloatingActionButtonDefaults.containerColor` choices).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum FabColors {
    /// `surfaceContainerHigh` with a `primary` icon.
    Surface,
    #[default]
    /// `primaryContainer` / `onPrimaryContainer`.
    PrimaryContainer,
    /// `secondaryContainer` / `onSecondaryContainer`.
    SecondaryContainer,
    /// `tertiaryContainer` / `onTertiaryContainer`.
    TertiaryContainer,
}

impl FabColors {
    fn roles(self) -> (ColorRole, ColorRole) {
        match self {
            // m3.material.io FAB color set; Compose has no token for it.
            Self::Surface => (ColorRole::SurfaceContainerHigh, ColorRole::Primary),
            Self::PrimaryContainer => (
                fab_primary_container::CONTAINER_COLOR,
                fab_primary_container::ICON_COLOR,
            ),
            Self::SecondaryContainer => (
                fab_secondary_container::CONTAINER_COLOR,
                fab_secondary_container::ICON_COLOR,
            ),
            // Compose `contentColorFor(tertiaryContainer)`; no token at the pinned commit.
            Self::TertiaryContainer => (ColorRole::TertiaryContainer, ColorRole::OnTertiaryContainer),
        }
    }

    pub(crate) fn colors(self, t: &Tokens) -> (Color32, Color32) {
        let (container, content) = self.roles();
        (t.role(container), t.role(content))
    }
}

pub(crate) fn fab_colors(color: FabColors, disabled: bool, t: &Tokens) -> (Color32, Color32) {
    if disabled {
        (common::disabled_container(t), common::disabled_content(t))
    } else {
        color.colors(t)
    }
}

/// Compose `FloatingActionButtonDefaults` elevation presets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FabElevation {
    /// `elevation()`.
    #[default]
    Default,
    /// `loweredElevation()`.
    Lowered,
}

fn elevation(e: FabElevation) -> Elevation {
    use extended_fab_primary as l;
    use fab_primary_container as m;
    match e {
        FabElevation::Lowered => Elevation {
            rest: l::LOWERED_CONTAINER_ELEVATION,
            hovered: l::LOWERED_HOVER_CONTAINER_ELEVATION,
            focused: l::LOWERED_FOCUS_CONTAINER_ELEVATION,
            pressed: l::LOWERED_PRESSED_CONTAINER_ELEVATION,
            disabled: ElevationLevel::Level0,
        },
        FabElevation::Default => Elevation {
            rest: m::CONTAINER_ELEVATION,
            hovered: m::HOVERED_CONTAINER_ELEVATION,
            focused: m::FOCUSED_CONTAINER_ELEVATION,
            pressed: m::PRESSED_CONTAINER_ELEVATION,
            disabled: ElevationLevel::Level0,
        },
    }
}

fn surface(rect: Rect, radius: CornerRadius, fill: Color32, e: FabElevation) -> Surface {
    Surface {
        rect,
        radius,
        fill,
        elevation: elevation(e),
        outline: None,
    }
}

#[derive(Clone, Debug)]
struct FabCore {
    salt: KeyedSalt,
    key: AgentKey,
    icon: MaterialIcon,
    label: String,
    color: FabColors,
    disabled: bool,
    shortcut: Option<KeyboardShortcut>,
    elevation: FabElevation,
}

impl FabCore {
    fn new(salt: KeyedSalt, icon: MaterialIcon, label: String) -> Self {
        Self {
            key: AgentKey::Kind("fab"),
            salt,
            icon,
            label,
            color: FabColors::default(),
            disabled: false,
            shortcut: None,
            elevation: FabElevation::Default,
        }
    }

    fn paint(
        &self,
        ui: &mut MineralUi<'_, Material>,
        frame: &WidgetFrame<'_>,
        radius: impl FnOnce(&Tokens) -> CornerRadius,
    ) -> (Color32, Rect) {
        let t = ui.tokens();
        let (bg, fg) = fab_colors(self.color, self.disabled, &t);
        let rect = frame.zones.visual;
        let i = parts::press(ui, frame, Availability::disabled_if(self.disabled)).interaction;
        surface(rect, radius(&t), bg, self.elevation).paint(ui, &frame.response, fg, i, &t);
        (fg, rect)
    }

    fn semantics(&self) -> SemanticNode {
        SemanticNode::new(self.key.id(&self.salt), Role::Button, &self.label)
            .clickable(Availability::disabled_if(self.disabled))
    }
}

macro_rules! fab_setters {
    () => {
        crate::widget::setters! {
            /// Greys out the FAB and ignores input.
            disabled |s| s.core.disabled
        }

        /// Default [`FabElevation::Default`].
        pub fn elevation(mut self, elevation: FabElevation) -> Self {
            self.core.elevation = elevation;
            self
        }

        /// Default [`FabColors::PrimaryContainer`].
        pub fn colors(mut self, colors: impl Into<FabColors>) -> Self {
            self.core.color = colors.into();
            self
        }

        /// Activates like a click while enabled and visible.
        pub fn shortcut(mut self, shortcut: KeyboardShortcut) -> Self {
            self.core.shortcut = Some(shortcut);
            self
        }

        crate::widget::setters! {
            /// Replaces the `<id_salt>-fab` agent key.
            agent_key(key) |s| s.core.key
        }
    };
}

macro_rules! fab_widget_meta {
    () => {
        fn id_salt(&self) -> IdSalt {
            self.core.salt.id_salt()
        }

        fn enabled(&self) -> bool {
            !self.core.disabled
        }

        fn accelerator(&self) -> Option<KeyboardShortcut> {
            self.core.shortcut
        }

        fn min_target(&self) -> MinTarget {
            crate::adaptive::MIN_INTERACTIVE
        }

        fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
            Some(self.core.semantics())
        }
    };
}

/// MD3 floating action button (Compose `FloatingActionButton`).
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Fab {
    core: FabCore,
    pub(crate) size: FabSize,
}

impl Fab {
    /// `label` is the accessible name (icon only); agent key `<id_salt>-fab`.
    pub fn new(id_salt: impl Into<KeyedSalt>, icon: MaterialIcon, label: impl Into<String>) -> Self {
        Self {
            core: FabCore::new(id_salt.into(), icon, label.into()),
            size: FabSize::default(),
        }
    }

    /// Default [`FabSize::Baseline`].
    pub fn size(mut self, size: impl Into<FabSize>) -> Self {
        self.size = size.into();
        self
    }

    fab_setters!();
}

impl MineralWidget for Fab {
    type Kit = Material;
    type State = ();
    type Output = ();

    fab_widget_meta!();

    fn layout(&self, _ui: &MineralUi<'_, Material>, c: Constraints, (): &Self::State) -> Measure {
        c.constrain(vec2(self.size.width(), self.size.side())).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Material>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let size = self.size;
        let (fg, rect) = self.core.paint(ui, frame, |t| CornerRadius::same(size.corner(t)));
        common::icon(ui.painter(), rect.center(), self.core.icon, size.icon(), fg);
        WidgetResponse::new(frame.response.clone(), ())
    }
}

/// MD3 extended FAB with a label (Compose `ExtendedFloatingActionButton`).
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct ExtendedFab {
    core: FabCore,
    size: ExtendedFabSize,
    expanded: bool,
}

impl ExtendedFab {
    /// `label` is the visible text and accessible name; agent key `<id_salt>-fab`.
    pub fn new(id_salt: impl Into<KeyedSalt>, icon: MaterialIcon, label: impl Into<String>) -> Self {
        Self {
            core: FabCore::new(id_salt.into(), icon, label.into()),
            size: ExtendedFabSize::default(),
            expanded: true,
        }
    }

    /// Default [`ExtendedFabSize::Small`].
    pub fn size(mut self, size: impl Into<ExtendedFabSize>) -> Self {
        self.size = size.into();
        self
    }

    /// `false` animates down to the icon-only FAB; default `true`.
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    fab_setters!();

    // Compose FastSpatial, FastEffects
    fn progress(&self, ui: &egui_mineral::widget::MineralUi<'_, crate::Material>) -> (f32, f32) {
        let rt = ui.rt();
        let id = ui.make_persistent_id(self.core.salt.id_salt());
        (
            egui_mineral::motion::spring(
                rt.core(),
                id.with("width"),
                f32::from(self.expanded),
                motion::FAST_SPATIAL,
            ),
            egui_mineral::motion::spring(
                rt.core(),
                id.with("alpha"),
                f32::from(self.expanded),
                motion::FAST_EFFECTS,
            ),
        )
    }

    fn label(&self, rt: &Rt, p: &Painter, m: &ExtMetrics, color: Color32) -> Arc<Galley> {
        parts::type_galley(rt.core(), p, &self.core.label, m.font, color, None)
    }
}

impl MineralWidget for ExtendedFab {
    type Kit = Material;
    type State = ();
    type Output = ();

    fab_widget_meta!();

    fn layout(&self, ui: &MineralUi<'_, Material>, c: Constraints, (): &Self::State) -> Measure {
        let rt = ui.rt();
        let m = self.size.metrics();
        let text = self.label(rt, ui.painter(), &m, Color32::PLACEHOLDER);
        let full = m.lead + m.icon + m.gap + text.size().x + m.trail;
        // Collapsed to the square minimum width (Compose `ExtendedFloatingActionButton` layout).
        let w = egui::lerp(m.height..=full.max(m.height), self.progress(ui).0);
        c.constrain(vec2(w, m.height)).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Material>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let rt = ui.rt();
        let m = self.size.metrics();
        let (fg, rect) = self
            .core
            .paint(ui, frame, |t| CornerRadius::same(m.shape.radius(&t.shape)));
        let dir = ui.i18n().direction();
        // The icon stays at the leading space, collapsed or not.
        let from_start = |x: f32, w: f32| dir.start_x(rect, Rangef::new(x, x + w));
        let icon_x = from_start(m.lead, m.icon) + m.icon / 2.0;
        common::icon(ui.painter(), pos2(icon_x, rect.center().y), self.core.icon, m.icon, fg);
        let (width, alpha) = self.progress(ui);
        if width > 0.0 {
            let p = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
            let color = common::with_alpha(fg, alpha.clamp(0.0, 1.0));
            let text = self.label(rt, &p, &m, color);
            let x = from_start(m.lead + m.icon + m.gap, text.size().x);
            p.galley(pos2(x, rect.center().y - text.size().y / 2.0), text, color);
        }
        WidgetResponse::new(frame.response.clone(), ())
    }
}
