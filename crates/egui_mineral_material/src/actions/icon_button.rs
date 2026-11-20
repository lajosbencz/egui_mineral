use crate::Material;
use crate::MaterialExt;
use crate::MaterialUiExt as _;
use crate::Rt;
use crate::adaptive::Density;
use egui::{Color32, CornerRadius, KeyboardShortcut, Rect, Vec2, vec2};
use egui_mineral::i18n::I18n;
use egui_mineral::semantic::AgentKey;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::Availability;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    layout::{Constraints, Measure, MinTarget, Zones},
    select::Check,
    semantic::{Role, SemanticNode},
    widget::{MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::communication::Badge;
use crate::widget::{
    common::{self},
    parts,
};
use crate::{
    Icon,
    spec::tokens::{
        filled_icon_button, filled_tonal_icon_button, floating_toolbar, large_icon_button, medium_icon_button,
        outlined_icon_button, small_icon_button, standard_icon_button, x_large_icon_button, x_small_icon_button,
    },
    tokens::Tokens,
};
use crate::{
    Size,
    actions::button::{self, ButtonShape, Shapes, Style},
};

/// Compose `IconButton`, `FilledIconButton`, `FilledTonalIconButton` and `OutlinedIconButton` styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum IconButtonVariant {
    #[default]
    /// `IconButton`: no container.
    Standard,
    /// `FilledIconButton`.
    Filled,
    /// `FilledTonalIconButton`.
    Tonal,
    /// `OutlinedIconButton`.
    Outlined,
}

/// Compose `IconButtonWidthOption`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum IconButtonWidth {
    /// Narrower than tall.
    Narrow,
    #[default]
    /// Square.
    Default,
    /// Wider than tall.
    Wide,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) enum Palette {
    #[default]
    Surface,
    Vibrant,
}

#[derive(Clone, Copy, Debug)]
struct Geometry {
    container: Vec2,
    icon: f32,
    shapes: Shapes,
    outline: f32,
}

macro_rules! geometry {
    ($m:ident, $width:expr, $default:ident) => {
        Geometry {
            container: vec2(
                $m::ICON_SIZE
                    + match $width {
                        IconButtonWidth::Narrow => $m::NARROW_LEADING_SPACE + $m::NARROW_TRAILING_SPACE,
                        IconButtonWidth::Default => 2.0 * $m::$default,
                        IconButtonWidth::Wide => $m::WIDE_LEADING_SPACE + $m::WIDE_TRAILING_SPACE,
                    },
                $m::CONTAINER_HEIGHT,
            ),
            icon: $m::ICON_SIZE,
            shapes: Shapes {
                round: $m::CONTAINER_SHAPE_ROUND,
                square: $m::CONTAINER_SHAPE_SQUARE,
                pressed: $m::PRESSED_CONTAINER_SHAPE,
                checked_round: $m::SELECTED_CONTAINER_SHAPE_ROUND,
                checked_square: $m::SELECTED_CONTAINER_SHAPE_SQUARE,
            },
            outline: $m::OUTLINED_OUTLINE_WIDTH,
        }
    };
}

// Compose IconButtonDefaults.*ContainerSize
fn geometry(size: Size, width: IconButtonWidth) -> Geometry {
    match size {
        Size::XSmall => geometry!(x_small_icon_button, width, DEFAULT_LEADING_SPACE),
        Size::Small => geometry!(small_icon_button, width, DEFAULT_LEADING_SPACE),
        Size::Medium => geometry!(medium_icon_button, width, DEFAULT_LEADING_SPACE),
        Size::Large => geometry!(large_icon_button, width, UNIFORM_LEADING_SPACE),
        Size::XLarge => geometry!(x_large_icon_button, width, DEFAULT_LEADING_SPACE),
    }
}

pub(crate) fn metrics(size: Size, width: IconButtonWidth) -> (Vec2, f32) {
    let g = geometry(size, width);
    (g.container, g.icon)
}

// Compose IconButtonDefaults, *VibrantColors
fn style(variant: IconButtonVariant, checked: Option<bool>, enabled: bool, palette: Palette, t: &Tokens) -> Style {
    let none = Color32::TRANSPARENT;
    let on = checked == Some(true);
    let pick = |on, off, plain| crate::actions::button::pick(t, checked, [on, off, plain]);
    let (container, content, outline, disabled) = match (variant, palette) {
        (IconButtonVariant::Standard, Palette::Vibrant) => {
            use floating_toolbar as m;
            (
                if on {
                    t.role(m::VIBRANT_BUTTON_SELECTED_CONTAINER_COLOR)
                } else {
                    none
                },
                t.role(if on {
                    m::VIBRANT_BUTTON_SELECTED_ICON_COLOR
                } else {
                    m::VIBRANT_BUTTON_UNSELECTED_ICON_COLOR
                }),
                None,
                (
                    none,
                    t.role_alpha(
                        standard_icon_button::DISABLED_COLOR,
                        standard_icon_button::DISABLED_OPACITY,
                    ),
                ),
            )
        }
        (IconButtonVariant::Standard, Palette::Surface) => {
            use standard_icon_button as m;
            (
                none,
                pick(m::SELECTED_COLOR, m::UNSELECTED_COLOR, m::COLOR),
                None,
                (none, t.role_alpha(m::DISABLED_COLOR, m::DISABLED_OPACITY)),
            )
        }
        (IconButtonVariant::Filled, _) => {
            use filled_icon_button as m;
            (
                pick(
                    m::SELECTED_CONTAINER_COLOR,
                    m::UNSELECTED_CONTAINER_COLOR,
                    m::CONTAINER_COLOR,
                ),
                pick(m::SELECTED_COLOR, m::UNSELECTED_COLOR, m::COLOR),
                None,
                (
                    t.role_alpha(m::DISABLED_CONTAINER_COLOR, m::DISABLED_CONTAINER_OPACITY),
                    t.role_alpha(m::DISABLED_COLOR, m::DISABLED_OPACITY),
                ),
            )
        }
        (IconButtonVariant::Tonal, _) => {
            use filled_tonal_icon_button as m;
            (
                pick(
                    m::SELECTED_CONTAINER_COLOR,
                    m::UNSELECTED_CONTAINER_COLOR,
                    m::CONTAINER_COLOR,
                ),
                pick(m::SELECTED_COLOR, m::UNSELECTED_COLOR, m::COLOR),
                None,
                (
                    t.role_alpha(m::DISABLED_CONTAINER_COLOR, m::DISABLED_CONTAINER_OPACITY),
                    t.role_alpha(m::DISABLED_COLOR, m::DISABLED_OPACITY),
                ),
            )
        }
        (IconButtonVariant::Outlined, _) => {
            use outlined_icon_button as m;
            (
                if on { t.role(m::SELECTED_CONTAINER_COLOR) } else { none },
                pick(m::SELECTED_COLOR, m::UNSELECTED_COLOR, m::COLOR),
                (!on).then(|| {
                    if enabled {
                        t.role(m::OUTLINE_COLOR)
                    } else {
                        t.role_alpha(m::OUTLINE_COLOR, m::DISABLED_OPACITY)
                    }
                }),
                (none, t.role_alpha(m::DISABLED_COLOR, m::DISABLED_OPACITY)),
            )
        }
    };
    let (container, content) = if enabled { (container, content) } else { disabled };
    Style {
        container,
        content,
        outline,
        elevation: button::Elevation::NONE,
    }
}

/// MD3 icon button (Compose `IconButton`); toggle forms bind a `&mut bool`.
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct IconButton<'a> {
    salt: KeyedSalt,
    icon: Icon,
    label: String,
    key: AgentKey,
    variant: IconButtonVariant,
    size: Size,
    width: IconButtonWidth,
    shape: ButtonShape,
    selected: Option<bool>,
    checked: Option<&'a mut bool>,
    disabled: bool,
    shortcut: Option<KeyboardShortcut>,
    badge: Option<Badge>,
    target: MinTarget,
    pub(crate) palette: Palette,
}

impl<'a> IconButton<'a> {
    /// `label` is the accessible name (icon only); agent key `<id_salt>-button`.
    pub fn new(id_salt: impl Into<KeyedSalt>, icon: impl Into<Icon>, label: impl Into<String>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            key: AgentKey::Kind("button"),
            salt: id_salt,
            icon: icon.into(),
            label: label.into(),
            variant: IconButtonVariant::default(),
            size: Size::Small,
            width: IconButtonWidth::default(),
            shape: ButtonShape::default(),
            selected: None,
            checked: None,
            disabled: false,
            shortcut: None,
            badge: None,
            target: crate::adaptive::MIN_INTERACTIVE,
            palette: Palette::default(),
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-button` agent key.
        agent_key(key)
    }

    // Slot of a default button in a density-aware row: the 48dp target less density, at least the container.
    pub(crate) fn slot(density: Density) -> f32 {
        let g = geometry(Size::Small, IconButtonWidth::default());
        density.dense_to(
            crate::adaptive::MIN_INTERACTIVE_DP,
            g.icon + crate::adaptive::CONTENT_PAD,
        )
    }

    // Dense composites size the slot to the container instead of the 48dp target.
    pub(crate) fn hit_target(mut self, target: MinTarget) -> Self {
        self.target = target;
        self
    }

    /// Default [`IconButtonVariant::Standard`].
    pub fn variant(mut self, variant: IconButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Default [`Size::Small`].
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Default [`IconButtonWidth::Default`].
    pub fn width(mut self, width: IconButtonWidth) -> Self {
        self.width = width;
        self
    }

    /// Default [`ButtonShape::Round`].
    pub fn shape(mut self, shape: ButtonShape) -> Self {
        self.shape = shape;
        self
    }

    /// Compose `IconToggleButton`: a click flips `checked` and reports `changed`.
    pub fn toggle(
        id_salt: impl Into<KeyedSalt>,
        icon: impl Into<Icon>,
        label: impl Into<String>,
        checked: &'a mut bool,
    ) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        let mut b = Self::new(id_salt, icon, label);
        b.selected = Some(*checked);
        b.checked = Some(checked);
        b
    }

    pub(crate) fn selected(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self
    }

    crate::widget::setters! {
        /// Greys out the button and ignores input.
        disabled
    }

    /// Activates like a click while enabled and visible.
    pub fn shortcut(mut self, shortcut: KeyboardShortcut) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    /// Anchored at the icon's top-end corner and overflows the layout (Compose `BadgedBox`).
    pub fn badge(mut self, badge: Badge) -> Self {
        self.badge = Some(badge);
        self
    }

    pub(crate) fn footprint(&self, rt: &Rt, density: Density) -> Vec2 {
        Zones::new(self.container(density).into(), self.target.points(rt.core()))
            .outer
            .size()
    }

    fn container(&self, density: Density) -> Vec2 {
        let g = geometry(self.size, self.width);
        if self.size != Size::Small {
            return g.container;
        }
        let shrink = g.container.y - density.dense_to(g.container.y, g.icon + crate::adaptive::CONTENT_PAD);
        vec2((g.container.x - shrink).max(g.icon), g.container.y - shrink)
    }
}

impl MineralWidget for IconButton<'_> {
    type Kit = Material;
    type State = ();
    type Output = ();

    egui_mineral::kit::widget_meta!(salt, enabled);

    fn accelerator(&self) -> Option<KeyboardShortcut> {
        self.shortcut
    }

    fn min_target(&self, _: &MineralUi<'_, Self::Kit>) -> MinTarget {
        self.target
    }

    fn layout(&self, ui: &MineralUi<'_, Material>, constraints: Constraints, (): &Self::State) -> Measure {
        constraints.constrain(self.container(ui.density())).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Material>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let rt = ui.rt();
        let t = ui.tokens();
        let response = &frame.response;
        let rect = frame.zones.visual;
        let press = parts::press(ui, frame, Availability::disabled_if(self.disabled));
        let i = press.interaction;
        let toggled = press.clicked && parts::flip(&mut self.checked, &mut self.selected);

        let g = geometry(self.size, self.width);
        let s = style(self.variant, self.selected, !self.disabled, self.palette, &t);
        let h = rect.height().min(rect.width());
        let target = g.shapes.target(self.shape, i.pressed, self.selected == Some(true));
        // Compose icon toggles morph with `DefaultEffects` too.
        let r = button::morph(
            ui,
            response.id.with("shape"),
            button::radius(&t, target.scale(), h),
            false,
        );
        let r = CornerRadius::from(r);
        button::Surface::of(rect, r, s, g.outline).paint(ui, &frame.response, s.content, i, &t);
        let icon = self
            .selected
            .map_or(self.icon, |s| common::selection_icon(self.icon, s));
        let p = ui.painter();
        common::icon(p, rect.center(), icon, g.icon, s.content);
        if let Some(badge) = self.badge {
            badge.paint(rt, p, Rect::from_center_size(rect.center(), Vec2::splat(g.icon)));
        }
        WidgetResponse::new(response.clone(), ()).with_changed(toggled)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let mut node = SemanticNode::new(self.key.id(&self.salt), Role::Button, &self.label)
            .clickable(Availability::disabled_if(self.disabled));
        node.state.checked = self.selected.map(Check::from);
        Some(node)
    }
}
