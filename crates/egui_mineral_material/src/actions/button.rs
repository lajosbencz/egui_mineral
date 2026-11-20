use crate::Material;
use crate::MaterialExt;
use crate::MaterialUiExt as _;
use crate::adaptive::Density;
use crate::widget::common::elevation_tokens;
use egui::{Color32, CornerRadius, Id, KeyboardShortcut, Rect, Sense, Stroke, StrokeKind, pos2, vec2};
use egui_mineral::i18n::I18n;
use egui_mineral::i18n::Sides;
use egui_mineral::semantic::AgentKey;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::semantic::StableId;
use egui_mineral::widget::Availability;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    focus,
    i18n::Direction,
    layout::{Constraints, Measure, MinTarget},
    select::Check,
    semantic::{Role, SemanticNode},
    shortcut::Mnemonic,
    widget::{MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::widget::{
    common::{self, Interaction},
    indication, parts,
};
use crate::{
    Icon, motion,
    spec::{
        ColorRole, ElevationLevel, ShapeRole, ShapeScale, TypeScale, defaults,
        tokens::{
            baseline_button, button_large, button_medium, button_small, button_x_large, button_x_small,
            elevated_button, filled_button, filled_tonal_button, outlined_button, text_button, tonal_button,
        },
    },
    tokens::Tokens,
};
use crate::{
    Size,
    actions::icon_button::{self, IconButtonWidth},
};

/// Compose `ElevatedButton`, `Button`, `FilledTonalButton`, `OutlinedButton` and `TextButton` styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum ButtonVariant {
    /// `ElevatedButton`: tonal surface with a shadow.
    Elevated,
    #[default]
    /// `Button`: primary container, highest emphasis.
    Filled,
    /// `FilledTonalButton`: secondary container.
    Tonal,
    /// `OutlinedButton`: outline, no container.
    Outlined,
    /// No toggle or split button counterpart in Compose ([`ContainerVariant`]).
    Text,
}

/// [`ButtonVariant`] without [`ButtonVariant::Text`]: Compose has no text toggle or split button.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum ContainerVariant {
    /// `ElevatedToggleButton`.
    Elevated,
    #[default]
    /// `ToggleButton`.
    Filled,
    /// `TonalToggleButton`.
    Tonal,
    /// `OutlinedToggleButton`.
    Outlined,
}

impl From<ContainerVariant> for ButtonVariant {
    fn from(variant: ContainerVariant) -> Self {
        match variant {
            ContainerVariant::Elevated => Self::Elevated,
            ContainerVariant::Filled => Self::Filled,
            ContainerVariant::Tonal => Self::Tonal,
            ContainerVariant::Outlined => Self::Outlined,
        }
    }
}

/// Placement of a [`PushButton`]'s one icon; a bare [`Icon`] is leading.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum ButtonIcon {
    /// Before the label.
    Leading(Icon),
    /// After the label, e.g. a drop-down arrow on a menu trigger.
    Trailing(Icon),
}

impl ButtonIcon {
    pub(crate) const fn icon(self) -> Icon {
        match self {
            Self::Leading(icon) | Self::Trailing(icon) => icon,
        }
    }

    const fn is_trailing(self) -> bool {
        matches!(self, Self::Trailing(_))
    }
}

impl From<Icon> for ButtonIcon {
    fn from(icon: Icon) -> Self {
        Self::Leading(icon)
    }
}

impl From<crate::MaterialIcon> for ButtonIcon {
    fn from(icon: crate::MaterialIcon) -> Self {
        Self::from(Icon::from(icon))
    }
}

/// Resting shape of a [`PushButton`]; toggle buttons morph between the two.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum ButtonShape {
    #[default]
    /// Fully rounded.
    Round,
    /// Size-dependent rounded rectangle (`ButtonDefaults.squareShape`).
    Square,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Shapes {
    pub round: ShapeRole,
    pub square: ShapeRole,
    pub pressed: ShapeRole,
    pub checked_round: ShapeRole,
    pub checked_square: ShapeRole,
}

impl Shapes {
    pub(crate) fn target(self, shape: ButtonShape, pressed: bool, checked: bool) -> ShapeRole {
        match (pressed, shape, checked) {
            (true, ..) => self.pressed,
            (false, ButtonShape::Round, false) => self.round,
            (false, ButtonShape::Square, false) => self.square,
            (false, ButtonShape::Round, true) => self.checked_round,
            (false, ButtonShape::Square, true) => self.checked_square,
        }
    }
}

// Compose ButtonDefaults.*For(buttonHeight)
#[derive(Clone, Copy, Debug)]
pub(crate) struct Metrics {
    pub height: f32,
    pub icon: f32,
    pub gap: f32,
    pub font: TypeScale,
    pub shapes: Shapes,
    pub outline: f32,
    /// `None` derives it from variant and icon, as the baseline button.
    pub space: Option<Space>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Space {
    pub leading: f32,
    pub trailing: f32,
}

macro_rules! sized {
    ($m:ident, $space:expr, $gap:expr, $font:expr) => {
        Metrics {
            height: $m::CONTAINER_HEIGHT,
            icon: $m::ICON_SIZE,
            gap: $gap,
            font: $font,
            // A round toggle checks to the square token and vice versa (Compose `ToggleButtonDefaults`).
            shapes: Shapes {
                round: $m::CONTAINER_SHAPE_ROUND,
                square: $m::CONTAINER_SHAPE_SQUARE,
                pressed: $m::PRESSED_CONTAINER_SHAPE,
                checked_round: $m::SELECTED_CONTAINER_SHAPE_SQUARE,
                checked_square: $m::SELECTED_CONTAINER_SHAPE_ROUND,
            },
            outline: $m::OUTLINED_OUTLINE_WIDTH,
            space: Some($space),
        }
    };
}

impl Size {
    // Compose ButtonDefaults.textStyleFor
    pub(crate) const fn metrics(self) -> Metrics {
        use defaults::button::{X_SMALL_ICON_LABEL_SPACE, X_SMALL_SPACE};
        match self {
            Self::XSmall => sized!(
                button_x_small,
                Space {
                    leading: X_SMALL_SPACE,
                    trailing: X_SMALL_SPACE,
                },
                X_SMALL_ICON_LABEL_SPACE,
                TypeScale::LabelLarge
            ),
            Self::Small => sized!(
                button_small,
                Space {
                    leading: button_small::LEADING_SPACE,
                    trailing: button_small::TRAILING_SPACE,
                },
                button_small::ICON_LABEL_SPACE,
                TypeScale::LabelLarge
            ),
            Self::Medium => sized!(
                button_medium,
                Space {
                    leading: button_medium::LEADING_SPACE,
                    trailing: button_medium::TRAILING_SPACE,
                },
                button_medium::ICON_LABEL_SPACE,
                TypeScale::TitleMedium
            ),
            Self::Large => sized!(
                button_large,
                Space {
                    leading: button_large::LEADING_SPACE,
                    trailing: button_large::TRAILING_SPACE,
                },
                button_large::ICON_LABEL_SPACE,
                TypeScale::HeadlineSmall
            ),
            Self::XLarge => sized!(
                button_x_large,
                Space {
                    leading: button_x_large::LEADING_SPACE,
                    trailing: button_x_large::TRAILING_SPACE,
                },
                button_x_large::ICON_LABEL_SPACE,
                TypeScale::HeadlineLarge
            ),
        }
    }
}

// Compose ButtonDefaults
const BASELINE: Metrics = Metrics {
    height: baseline_button::CONTAINER_HEIGHT,
    icon: defaults::button::ICON_SIZE,
    gap: button_small::ICON_LABEL_SPACE,
    font: TypeScale::LabelLarge,
    shapes: Shapes {
        round: baseline_button::CONTAINER_SHAPE_ROUND,
        square: baseline_button::CONTAINER_SHAPE_SQUARE,
        pressed: baseline_button::PRESSED_CONTAINER_SHAPE,
        checked_round: baseline_button::SELECTED_CONTAINER_SHAPE_SQUARE,
        checked_square: baseline_button::SELECTED_CONTAINER_SHAPE_ROUND,
    },
    outline: button_small::OUTLINED_OUTLINE_WIDTH,
    space: None,
};

pub(crate) fn radius(t: &Tokens, scale: ShapeScale, height: f32) -> f32 {
    f32::from(scale.radius(&t.shape)).min(height / 2.0)
}

// Compose FastSpatial, DefaultEffects, shapeByInteraction
pub(crate) fn morph(ui: &MineralUi<'_, Material>, id: Id, target: f32, toggle: bool) -> f32 {
    let spring = if toggle {
        motion::FAST_SPATIAL
    } else {
        motion::DEFAULT_EFFECTS
    };
    egui_mineral::motion::spring(ui.core(), id, target, spring).max(0.0)
}

pub(crate) fn sided(start: f32, end: f32, dir: Direction) -> CornerRadius {
    let Sides { left: l, right: r } = dir.sides(start, end);
    let (l, r) = (CornerRadius::from(l), CornerRadius::from(r));
    CornerRadius {
        nw: l.nw,
        sw: l.sw,
        ne: r.ne,
        se: r.se,
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Joined {
    pub start: bool,
    pub end: bool,
    pub inner: ShapeScale,
    pub inner_pressed: ShapeScale,
    /// Checked items turn fully round (Compose `connectedButtonCheckedShape`).
    pub checked_full: bool,
}

// Compose ButtonElevation
#[derive(Clone, Copy, Debug)]
pub(crate) struct Elevation {
    pub rest: ElevationLevel,
    pub hovered: ElevationLevel,
    pub focused: ElevationLevel,
    pub pressed: ElevationLevel,
    pub disabled: ElevationLevel,
}

impl Elevation {
    pub(crate) const NONE: Self = Self::flat(ElevationLevel::Level0);

    pub(crate) const fn flat(level: ElevationLevel) -> Self {
        Self {
            rest: level,
            hovered: level,
            focused: level,
            pressed: level,
            disabled: level,
        }
    }

    pub(crate) fn level(self, i: Interaction) -> ElevationLevel {
        if i.disabled {
            self.disabled
        } else if i.pressed {
            self.pressed
        } else if i.hovered {
            self.hovered
        } else if i.focused {
            self.focused
        } else {
            self.rest
        }
    }
}

// Compose ButtonColors, ToggleButtonColors
#[derive(Clone, Copy, Debug)]
pub(crate) struct Style {
    pub container: Color32,
    pub content: Color32,
    pub outline: Option<Color32>,
    pub elevation: Elevation,
}

pub(crate) fn pick(t: &Tokens, checked: Option<bool>, [on, off, plain]: [ColorRole; 3]) -> Color32 {
    t.role(match checked {
        Some(true) => on,
        Some(false) => off,
        None => plain,
    })
}

pub(crate) fn style(variant: ButtonVariant, checked: Option<bool>, enabled: bool, t: &Tokens) -> Style {
    let pick = |on, off, plain| pick(t, checked, [on, off, plain]);
    let none = Color32::TRANSPARENT;
    let (container, content, outline, elevation) = match variant {
        ButtonVariant::Filled => {
            use filled_button as m;
            (
                pick(
                    m::SELECTED_CONTAINER_COLOR,
                    m::UNSELECTED_CONTAINER_COLOR,
                    m::CONTAINER_COLOR,
                ),
                pick(
                    m::SELECTED_PRESSED_LABEL_TEXT_COLOR,
                    m::UNSELECTED_PRESSED_LABEL_TEXT_COLOR,
                    m::LABEL_TEXT_COLOR,
                ),
                None,
                elevation_tokens!(m; hovered: HOVERED_CONTAINER_ELEVATION, focused: FOCUSED_CONTAINER_ELEVATION),
            )
        }
        ButtonVariant::Elevated => {
            use elevated_button as m;
            (
                pick(
                    m::SELECTED_CONTAINER_COLOR,
                    m::UNSELECTED_CONTAINER_COLOR,
                    m::CONTAINER_COLOR,
                ),
                pick(
                    m::SELECTED_PRESSED_LABEL_TEXT_COLOR,
                    m::UNSELECTED_PRESSED_LABEL_TEXT_COLOR,
                    m::LABEL_TEXT_COLOR,
                ),
                None,
                elevation_tokens!(m; hovered: HOVERED_CONTAINER_ELEVATION, focused: FOCUSED_CONTAINER_ELEVATION),
            )
        }
        // Compose: `FilledTonalButtonTokens` for buttons, `TonalButtonTokens` for toggles.
        ButtonVariant::Tonal => (
            pick(
                tonal_button::SELECTED_CONTAINER_COLOR,
                tonal_button::UNSELECTED_CONTAINER_COLOR,
                filled_tonal_button::CONTAINER_COLOR,
            ),
            pick(
                tonal_button::SELECTED_LABEL_TEXT_COLOR,
                tonal_button::UNSELECTED_LABEL_TEXT_COLOR,
                filled_tonal_button::LABEL_TEXT_COLOR,
            ),
            None,
            elevation_tokens!(filled_tonal_button; hovered: HOVER_CONTAINER_ELEVATION, focused: FOCUS_CONTAINER_ELEVATION),
        ),
        ButtonVariant::Outlined => {
            use outlined_button as m;
            let on = checked == Some(true);
            (
                if on { t.role(m::SELECTED_CONTAINER_COLOR) } else { none },
                pick(
                    m::SELECTED_LABEL_TEXT_COLOR,
                    m::UNSELECTED_LABEL_TEXT_COLOR,
                    m::LABEL_TEXT_COLOR,
                ),
                (!on).then(|| t.role(m::OUTLINE_COLOR)),
                Elevation::NONE,
            )
        }
        ButtonVariant::Text => (none, t.role(defaults::button::TEXT_LABEL_COLOR), None, Elevation::NONE),
    };
    let (container, content, outline) = if enabled {
        (container, content, outline)
    } else {
        disabled(variant, checked, outline.is_some(), t)
    };
    Style {
        container,
        content,
        outline,
        elevation,
    }
}

fn disabled(
    variant: ButtonVariant,
    checked: Option<bool>,
    outline: bool,
    t: &Tokens,
) -> (Color32, Color32, Option<Color32>) {
    let a = |role, alpha| t.role_alpha(role, alpha);
    let none = Color32::TRANSPARENT;
    match (variant, checked) {
        (ButtonVariant::Filled, _) => {
            use filled_button as m;
            (
                a(m::DISABLED_CONTAINER_COLOR, m::DISABLED_CONTAINER_OPACITY),
                a(m::DISABLED_LABEL_TEXT_COLOR, m::DISABLED_LABEL_TEXT_OPACITY),
                None,
            )
        }
        (ButtonVariant::Elevated, _) => {
            use elevated_button as m;
            (
                a(m::DISABLED_CONTAINER_COLOR, m::DISABLED_CONTAINER_OPACITY),
                a(m::DISABLED_LABEL_TEXT_COLOR, m::DISABLED_LABEL_TEXT_OPACITY),
                None,
            )
        }
        (ButtonVariant::Tonal, None) => {
            use filled_tonal_button as m;
            (
                a(m::DISABLED_CONTAINER_COLOR, m::DISABLED_CONTAINER_OPACITY),
                a(m::DISABLED_LABEL_TEXT_COLOR, m::DISABLED_LABEL_TEXT_OPACITY),
                None,
            )
        }
        (ButtonVariant::Tonal, Some(_)) => {
            use tonal_button as m;
            (
                a(m::DISABLED_CONTAINER_COLOR, m::DISABLED_CONTAINER_OPACITY),
                a(m::DISABLED_LABEL_TEXT_COLOR, m::DISABLED_LABEL_TEXT_OPACITY),
                None,
            )
        }
        (ButtonVariant::Outlined, _) => {
            use outlined_button as m;
            // Compose `OutlinedToggleButtonDefaults` fills every disabled toggle.
            let fill = if checked.is_some() {
                a(m::DISABLED_OUTLINE_COLOR, m::DISABLED_CONTAINER_OPACITY)
            } else {
                none
            };
            (
                fill,
                a(m::DISABLED_LABEL_TEXT_COLOR, m::DISABLED_LABEL_TEXT_OPACITY),
                outline.then(|| a(m::OUTLINE_COLOR, m::DISABLED_CONTAINER_OPACITY)),
            )
        }
        (ButtonVariant::Text, _) => (
            none,
            a(text_button::DISABLED_LABEL_COLOR, text_button::DISABLED_LABEL_OPACITY),
            None,
        ),
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Surface {
    pub rect: Rect,
    pub radius: CornerRadius,
    pub fill: Color32,
    pub elevation: Elevation,
    pub outline: Option<Stroke>,
}

impl Surface {
    pub(crate) fn of(rect: Rect, radius: CornerRadius, s: Style, outline_width: f32) -> Self {
        Self {
            rect,
            radius,
            fill: s.container,
            elevation: s.elevation,
            outline: s.outline.map(|c| Stroke::new(outline_width, c)),
        }
    }

    pub(crate) fn paint_fill(&self, p: &egui::Painter, i: Interaction, t: &Tokens) {
        common::elevation(p, self.rect, self.radius, self.elevation.level(i), t);
        p.rect_filled(self.rect, self.radius, self.fill);
    }

    pub(crate) fn paint_outline(&self, p: &egui::Painter) {
        if let Some(o) = self.outline {
            p.rect_stroke(self.rect, self.radius, o, StrokeKind::Inside);
        }
    }

    pub(crate) fn paint(
        &self,
        ui: &egui_mineral::widget::MineralUi<'_, crate::Material>,
        response: &egui::Response,
        content: Color32,
        i: Interaction,
        t: &Tokens,
    ) {
        self.paint_fill(ui.painter(), i, t);
        indication::paint(ui, response, self.rect, self.radius, content, i, t);
        self.paint_outline(ui.painter());
    }
}

/// MD3 common button; [`ToggleButton`] is the checkable counterpart.
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct PushButton<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    icon: Option<ButtonIcon>,
    variant: ButtonVariant,
    size: Option<Size>,
    shape: ButtonShape,
    selected: Option<bool>,
    checked: Option<&'a mut bool>,
    disabled: bool,
    mnemonic: Option<Mnemonic>,
    shortcut: Option<KeyboardShortcut>,
    pub(crate) width: Option<f32>,
    pub(crate) space: Option<Space>,
    pub(crate) min_width: Option<f32>,
    pub(crate) icon_only: bool,
    pub(crate) joined: Option<Joined>,
    pub(crate) role: Role,
    pub(crate) min_target: MinTarget,
    pub(crate) tab_stop: focus::TabStop,
}

impl PushButton<'_> {
    /// `label` is the visible text and accessible name; agent key `<id_salt>-button`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        let label = label.into();
        Self {
            salt: id_salt,
            key: egui_mineral::agent_key!("button"),
            label,
            icon: None,
            variant: ButtonVariant::default(),
            size: None,
            shape: ButtonShape::default(),
            selected: None,
            checked: None,
            disabled: false,
            mnemonic: None,
            shortcut: None,
            width: None,
            space: None,
            min_width: None,
            icon_only: false,
            joined: None,
            role: Role::Button,
            min_target: crate::adaptive::MIN_INTERACTIVE,
            tab_stop: focus::TabStop::Stop,
        }
    }

    /// Default [`ButtonVariant::Filled`].
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Leading unless given as [`ButtonIcon::Trailing`]; replaces an earlier icon.
    pub fn icon(mut self, icon: impl Into<ButtonIcon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// M3 Expressive size family; unset uses the baseline `ButtonDefaults` metrics (toggles [`Size::Small`]).
    pub fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }

    /// Default [`ButtonShape::Round`].
    pub fn shape(mut self, shape: ButtonShape) -> Self {
        self.shape = shape;
        self
    }

    pub(crate) fn selected(mut self, selected: bool) -> Self {
        self.selected = Some(selected);
        self
    }

    crate::widget::setters! {
        /// Greys out the button and ignores input.
        disabled
    }

    /// Alt+letter activates the button; the letter is underlined while mnemonics are visible.
    pub fn mnemonic(mut self, mnemonic: char) -> Self {
        self.mnemonic = Some(Mnemonic::new(mnemonic));
        self
    }

    /// Activates like a click while enabled and visible.
    pub fn shortcut(mut self, shortcut: KeyboardShortcut) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-button` agent key.
        agent_key(key)
    }

    fn sized(&self) -> Option<Size> {
        self.size.or(self.selected.map(|_| Size::Small))
    }

    fn metrics(&self, density: Density) -> Metrics {
        let m = self.sized().map_or(BASELINE, Size::metrics);
        Metrics {
            height: if self.size.is_none() {
                let glyph = if self.icon.is_some() || self.icon_only {
                    m.icon
                } else {
                    0.0
                };
                density.dense_to(m.height, m.font.line_height().max(glyph) + crate::adaptive::CONTENT_PAD)
            } else {
                m.height
            },
            ..m
        }
    }

    fn icon_only_metrics(&self) -> (f32, f32) {
        let (size, icon) = icon_button::metrics(self.size.unwrap_or(Size::Small), IconButtonWidth::Default);
        (size.x, icon)
    }

    // Compose ButtonDefaults.ContentPadding
    fn space(&self, m: &Metrics) -> Space {
        if let Some(s) = self.space.or(m.space) {
            return s;
        }
        let (plain, icon_side, other) = if self.variant == ButtonVariant::Text {
            use defaults::button::{TEXT_ICON_END_SPACE, TEXT_SPACE};
            (TEXT_SPACE, TEXT_SPACE, TEXT_ICON_END_SPACE)
        } else {
            use baseline_button::{LEADING_SPACE, TRAILING_SPACE};
            (LEADING_SPACE, defaults::button::ICON_SIDE_SPACE, TRAILING_SPACE)
        };
        let (leading, trailing) = match self.icon {
            None => (plain, plain),
            Some(icon) if icon.is_trailing() => (other, icon_side),
            Some(_) => (icon_side, other),
        };
        Space { leading, trailing }
    }

    fn min_width(&self) -> f32 {
        match (self.min_width, self.selected) {
            (Some(w), _) => w,
            (None, None) => defaults::button::MIN_WIDTH,
            (None, Some(_)) => 0.0,
        }
    }

    fn natural_width(&self, ui: &MineralUi<'_, Material>, m: &Metrics) -> f32 {
        let rt = ui.rt();
        if self.icon_only {
            return self.icon_only_metrics().0;
        }
        let text = parts::type_galley(
            rt.core(),
            ui.painter(),
            &self.label,
            m.font,
            Color32::PLACEHOLDER,
            self.mnemonic,
        );
        let Space {
            leading: lead,
            trailing: trail,
        } = self.space(m);
        let icon = if self.icon.is_some() { m.icon + m.gap } else { 0.0 };
        (lead + icon + text.size().x + trail).max(self.min_width())
    }

    fn corners(
        &self,
        ui: &MineralUi<'_, Material>,
        id: Id,
        rect: Rect,
        pressed: bool,
        m: &Metrics,
        t: &Tokens,
    ) -> CornerRadius {
        let rt = ui.rt();
        let h = rect.height().min(rect.width());
        let full = h / 2.0;
        let checked = self.selected == Some(true);
        let toggle = self.selected.is_some();
        let Some(j) = self.joined else {
            let target = m.shapes.target(self.shape, pressed, checked);
            return CornerRadius::from(morph(ui, id.with("shape"), radius(t, target.scale(), h), toggle));
        };
        let round = checked && j.checked_full && !pressed;
        let base = m.shapes.target(self.shape, false, false);
        let outer = if round { full } else { radius(t, base.scale(), h) };
        let inner = match (pressed, round) {
            (true, _) => radius(t, j.inner_pressed, h),
            (false, true) => full,
            (false, false) => radius(t, j.inner, h),
        };
        let outer = morph(ui, id.with("outer"), outer, toggle);
        let inner = morph(ui, id.with("inner"), inner, toggle);
        sided(
            if j.start { outer } else { inner },
            if j.end { outer } else { inner },
            rt.i18n().direction(),
        )
    }
}

impl MineralWidget for PushButton<'_> {
    type Kit = Material;
    type State = ();
    type Output = ();

    egui_mineral::kit::widget_meta!(salt, enabled);

    fn access_key(&self) -> Option<Mnemonic> {
        self.mnemonic
    }

    fn accelerator(&self) -> Option<KeyboardShortcut> {
        self.shortcut
    }

    fn sense(&self) -> Sense {
        focus::roving_sense(Sense::click(), self.tab_stop)
    }

    fn min_target(&self, _: &MineralUi<'_, Self::Kit>) -> MinTarget {
        self.min_target
    }

    fn layout(&self, ui: &MineralUi<'_, Material>, constraints: Constraints, (): &Self::State) -> Measure {
        let m = self.metrics(ui.density());
        let w = self.width.unwrap_or_else(|| self.natural_width(ui, &m));
        constraints.constrain(vec2(w, m.height)).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Material>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let rt = ui.rt();
        let t = ui.tokens();
        let m = self.metrics(ui.density());
        let response = &frame.response;
        let rect = frame.zones.visual;
        let press = parts::press(ui, frame, Availability::disabled_if(self.disabled));
        let i = press.interaction;
        let toggled = press.clicked && parts::flip(&mut self.checked, &mut self.selected);

        let s = style(self.variant, self.selected, !self.disabled, &t);
        let r = self.corners(ui, response.id, rect, i.pressed, &m, &t);
        Surface::of(rect, r, s, m.outline).paint(ui, &frame.response, s.content, i, &t);
        let p = ui.painter();
        let fg = s.content;

        if self.icon_only {
            if let Some(icon) = self.icon {
                common::icon(p, rect.center(), icon.icon(), self.icon_only_metrics().1, fg);
            }
            return WidgetResponse::new(response.clone(), ()).with_changed(toggled);
        }
        let text = parts::type_galley(rt.core(), p, &self.label, m.font, fg, self.mnemonic);
        let dir = ui.i18n().direction();
        let space = self.space(&m);
        let Sides { left, right } = dir.sides(space.leading, space.trailing);
        let icon_w = if self.icon.is_some() { m.icon + m.gap } else { 0.0 };
        let content = icon_w + text.size().x;
        // Compose centers the content row inside the padding.
        let start = rect.left() + left + (rect.width() - left - right - content) / 2.0;
        let trailing = self.icon.is_some_and(ButtonIcon::is_trailing);
        let icon_left = match dir {
            Direction::Ltr => !trailing,
            Direction::Rtl => trailing,
        };
        let (icon_x, text_x) = if icon_left {
            (start, start + icon_w)
        } else {
            (start + text.size().x + m.gap, start)
        };
        if let Some(icon) = self.icon {
            common::icon(p, pos2(icon_x + m.icon / 2.0, rect.center().y), icon.icon(), m.icon, fg);
        }
        p.galley(pos2(text_x, rect.center().y - text.size().y / 2.0), text, fg);

        WidgetResponse::new(response.clone(), ()).with_changed(toggled)
    }

    fn semantics(&self, _: &I18n, _response: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let mut node = SemanticNode::new(self.key.id(&self.salt), self.role, &self.label)
            .clickable(Availability::disabled_if(self.disabled));
        match self.role {
            Role::RadioButton => {
                node.state.checked = self.selected.map(Check::from);
                node.state.selected = self.selected;
            }
            _ => node.state.checked = self.selected.map(Check::from),
        }
        Some(node)
    }
}

/// Compose `ToggleButton`: a click flips the bound value and reports `changed`; the shape morphs
/// between round and square. Text buttons do not toggle, see [`ContainerVariant`].
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct ToggleButton<'a>(PushButton<'a>);

impl<'a> ToggleButton<'a> {
    /// `label` is the visible text and accessible name; agent key `<id_salt>-button`; `checked` is flipped on click.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, checked: &'a mut bool) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        let mut b = PushButton::new(id_salt, label);
        b.selected = Some(*checked);
        b.checked = Some(checked);
        Self(b)
    }

    /// Default [`ContainerVariant::Filled`].
    pub fn variant(mut self, variant: ContainerVariant) -> Self {
        self.0 = self.0.variant(variant.into());
        self
    }

    /// Leading unless given as [`ButtonIcon::Trailing`]; replaces an earlier icon.
    pub fn icon(mut self, icon: impl Into<ButtonIcon>) -> Self {
        self.0 = self.0.icon(icon);
        self
    }

    /// M3 Expressive size family; unset is [`Size::Small`].
    pub fn size(mut self, size: Size) -> Self {
        self.0 = self.0.size(size);
        self
    }

    /// Default [`ButtonShape::Round`].
    pub fn shape(mut self, shape: ButtonShape) -> Self {
        self.0 = self.0.shape(shape);
        self
    }

    /// Greys out the button and ignores input.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.0 = self.0.disabled(disabled);
        self
    }

    /// Alt+letter activates the button; the letter is underlined while mnemonics are visible.
    pub fn mnemonic(mut self, mnemonic: char) -> Self {
        self.0 = self.0.mnemonic(mnemonic);
        self
    }

    /// Activates like a click while enabled and visible.
    pub fn shortcut(mut self, shortcut: KeyboardShortcut) -> Self {
        self.0 = self.0.shortcut(shortcut);
        self
    }

    /// Replaces the `<id_salt>-button` agent key.
    pub fn agent_key(mut self, key: impl Into<StableId>) -> Self {
        self.0 = self.0.agent_key(key);
        self
    }
}

impl MineralWidget for ToggleButton<'_> {
    type Kit = Material;
    type State = ();
    type Output = ();

    egui_mineral::kit::delegate_widget_meta!(0);

    fn layout(&self, ui: &MineralUi<'_, Material>, constraints: Constraints, state: &Self::State) -> Measure {
        self.0.layout(ui, constraints, state)
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Material>, frame: &WidgetFrame<'_>, state: &mut ()) -> WidgetResponse {
        self.0.update(ui, frame, state)
    }

    fn semantics(&self, i18n: &I18n, response: &WidgetResponse, state: &()) -> Option<SemanticNode> {
        self.0.semantics(i18n, response, state)
    }
}

/// Spec dialog action row: `actions` as text buttons `BUTTONS_SPACING` apart, in the current layout;
/// the tag of the clicked one.
pub fn text_actions<'a, T>(
    ui: &mut MineralUi<'_, Material>,
    actions: impl IntoIterator<Item = (T, PushButton<'a>)>,
) -> Option<T> {
    let rt = ui.rt();
    ui.spacing_mut().item_spacing.x = defaults::dialog::BUTTONS_SPACING;
    let mut hit = None;
    for (tag, b) in actions {
        if rt.ui(ui).mineral(b.variant(ButtonVariant::Text)).clicked() {
            hit = Some(tag);
        }
    }
    hit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_shapes_swap_round_and_square() {
        let s = Size::Small.metrics().shapes;
        assert_eq!(
            s.target(ButtonShape::Round, false, true),
            button_small::SELECTED_CONTAINER_SHAPE_SQUARE
        );
        assert_eq!(
            s.target(ButtonShape::Square, false, true),
            button_small::SELECTED_CONTAINER_SHAPE_ROUND
        );
        assert_eq!(
            s.target(ButtonShape::Round, false, false),
            button_small::CONTAINER_SHAPE_ROUND
        );
        assert_eq!(
            s.target(ButtonShape::Square, true, true),
            button_small::PRESSED_CONTAINER_SHAPE
        );
    }
}
