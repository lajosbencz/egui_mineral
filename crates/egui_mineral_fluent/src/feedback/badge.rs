use std::sync::Arc;

use egui::{Color32, CornerRadius, Galley, Rangef, Rect, Stroke, StrokeKind, pos2, text::LayoutJob, vec2};
use egui_mineral::{
    i18n::{Direction, I18n},
    kit,
    layout::{Constraints, Measure},
    semantic::{AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::ColorToken::{
    ColorBrandBackground, ColorBrandBackground2, ColorBrandForeground1, ColorBrandForeground2, ColorBrandStroke2,
    ColorNeutralBackground1, ColorNeutralBackground4, ColorNeutralBackground5, ColorNeutralForeground1,
    ColorNeutralForeground1Static, ColorNeutralForeground3, ColorNeutralForegroundOnBrand,
    ColorNeutralForegroundStaticInverted, ColorNeutralStroke2, ColorNeutralStrokeAccessible,
    ColorPaletteDarkOrangeBackground1, ColorPaletteDarkOrangeBackground3, ColorPaletteDarkOrangeBorder1,
    ColorPaletteDarkOrangeForeground1, ColorPaletteDarkOrangeForeground3, ColorPaletteGreenBackground1,
    ColorPaletteGreenBackground3, ColorPaletteGreenBorder1, ColorPaletteGreenBorder2, ColorPaletteGreenForeground1,
    ColorPaletteGreenForeground3, ColorPaletteRedBackground1, ColorPaletteRedBackground3, ColorPaletteRedBorder1,
    ColorPaletteRedBorder2, ColorPaletteRedForeground1, ColorPaletteRedForeground3, ColorPaletteYellowBackground1,
    ColorPaletteYellowBackground3, ColorPaletteYellowBorder1, ColorPaletteYellowForeground1,
    ColorPaletteYellowForeground2,
};
use crate::{
    ColorToken, Fluent, FluentExt, Icon, IconPosition,
    spec::{
        defaults::{badge as d, counter_badge as cd},
        tokens::geometry,
    },
    theme::Tokens,
    typography::TypeRamp,
    widget::paint,
};

/// Badge `size`, shared by [`Badge`], [`CounterBadge`] and [`PresenceBadge`](super::PresenceBadge).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BadgeSize {
    /// `tiny`: a 6 px dot, no content.
    Tiny,
    /// `extra-small`: a 10 px dot, no content.
    ExtraSmall,
    /// `small`: 16 high.
    Small,
    /// `medium`: 20 high.
    #[default]
    Medium,
    /// `large`: 24 high.
    Large,
    /// `extra-large`: 32 high.
    ExtraLarge,
}

impl BadgeSize {
    /// Every variant, smallest first.
    pub const ALL: &'static [Self] = &[
        Self::Tiny,
        Self::ExtraSmall,
        Self::Small,
        Self::Medium,
        Self::Large,
        Self::ExtraLarge,
    ];

    const fn height(self) -> f32 {
        match self {
            Self::Tiny => d::TINY,
            Self::ExtraSmall => d::EXTRA_SMALL,
            Self::Small => d::SMALL,
            Self::Medium => d::MEDIUM,
            Self::Large => d::LARGE,
            Self::ExtraLarge => d::EXTRA_LARGE,
        }
    }

    const fn dot(self) -> bool {
        matches!(self, Self::Tiny | Self::ExtraSmall)
    }

    const fn small_to_tiny(self) -> bool {
        matches!(self, Self::Tiny | Self::ExtraSmall | Self::Small)
    }

    const fn padding(self) -> f32 {
        d::TEXT_PADDING
            + match self {
                Self::Tiny | Self::ExtraSmall => 0.0,
                Self::Small => geometry::SPACING_HORIZONTAL_XXS,
                Self::Medium | Self::Large => geometry::SPACING_HORIZONTAL_XS,
                Self::ExtraLarge => geometry::SPACING_HORIZONTAL_S_NUDGE,
            }
    }

    const fn icon(self) -> f32 {
        match self {
            Self::Tiny => d::ICON_TINY,
            Self::ExtraSmall => d::ICON_EXTRA_SMALL,
            Self::Small | Self::Medium => d::ICON_SMALL,
            Self::Large => d::ICON_LARGE,
            Self::ExtraLarge => d::ICON_EXTRA_LARGE,
        }
    }

    const fn icon_gap(self) -> f32 {
        d::TEXT_PADDING
            + match self {
                Self::ExtraLarge => geometry::SPACING_HORIZONTAL_XS,
                _ => geometry::SPACING_HORIZONTAL_XXS,
            }
    }

    const fn text(self) -> TypeRamp {
        if self.small_to_tiny() {
            TypeRamp::Caption2Strong
        } else {
            TypeRamp::Caption1Strong
        }
    }
}

/// Badge `shape`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BadgeShape {
    /// `circular`: pill.
    #[default]
    Circular,
    /// `rounded`: `borderRadiusMedium`, `borderRadiusSmall` from `small` down.
    Rounded,
    /// `square`: no rounding.
    Square,
}

impl BadgeShape {
    /// Every variant.
    pub const ALL: &'static [Self] = &[Self::Circular, Self::Rounded, Self::Square];
}

/// Badge `appearance`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BadgeAppearance {
    /// `filled`: solid fill.
    #[default]
    Filled,
    /// `ghost`: colored content, no fill or border.
    Ghost,
    /// `outline`: colored content with a border.
    Outline,
    /// `tint`: light fill with a border.
    Tint,
}

impl BadgeAppearance {
    /// Every variant.
    pub const ALL: &'static [Self] = &[Self::Filled, Self::Ghost, Self::Outline, Self::Tint];
}

/// Badge `color`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum BadgeColor {
    /// `brand`.
    #[default]
    Brand,
    /// `danger`.
    Danger,
    /// `important`.
    Important,
    /// `informative`.
    Informative,
    /// `severe`.
    Severe,
    /// `subtle`.
    Subtle,
    /// `success`.
    Success,
    /// `warning`.
    Warning,
}

impl BadgeColor {
    /// Every variant.
    pub const ALL: &'static [Self] = &[
        Self::Brand,
        Self::Danger,
        Self::Important,
        Self::Informative,
        Self::Severe,
        Self::Subtle,
        Self::Success,
        Self::Warning,
    ];
}

struct Look {
    fill: Option<ColorToken>,
    text: ColorToken,
    border: Option<ColorToken>,
}

// react-badge useBadgeStyles.styles.ts `${appearance}-${color}` styles
const fn look(appearance: BadgeAppearance, color: BadgeColor) -> Look {
    use BadgeAppearance::{Filled, Ghost, Outline, Tint};
    use BadgeColor::{Brand, Danger, Important, Informative, Severe, Subtle, Success, Warning};
    let (fill, text, border) = match (appearance, color) {
        (Filled, Brand) => (Some(ColorBrandBackground), ColorNeutralForegroundOnBrand, None),
        (Filled, Danger) => (Some(ColorPaletteRedBackground3), ColorNeutralForegroundOnBrand, None),
        (Filled, Important) => (Some(ColorNeutralForeground1), ColorNeutralBackground1, None),
        (Filled, Informative) => (Some(ColorNeutralBackground5), ColorNeutralForeground3, None),
        (Filled, Severe) => (
            Some(ColorPaletteDarkOrangeBackground3),
            ColorNeutralForegroundOnBrand,
            None,
        ),
        (Filled, Subtle) => (Some(ColorNeutralBackground1), ColorNeutralForeground1, None),
        (Filled, Success) => (Some(ColorPaletteGreenBackground3), ColorNeutralForegroundOnBrand, None),
        (Filled, Warning) => (Some(ColorPaletteYellowBackground3), ColorNeutralForeground1Static, None),
        (Ghost, Brand) => (None, ColorBrandForeground1, None),
        (Ghost, Danger) => (None, ColorPaletteRedForeground3, None),
        (Ghost, Important) => (None, ColorNeutralForeground1, None),
        (Ghost, Informative) => (None, ColorNeutralForeground3, None),
        (Ghost, Severe) => (None, ColorPaletteDarkOrangeForeground3, None),
        (Ghost, Subtle) => (None, ColorNeutralForegroundStaticInverted, None),
        (Ghost, Success) => (None, ColorPaletteGreenForeground3, None),
        (Ghost, Warning) => (None, ColorPaletteYellowForeground2, None),
        (Outline, Brand) => (None, ColorBrandForeground1, Some(ColorBrandForeground1)),
        (Outline, Danger) => (None, ColorPaletteRedForeground3, Some(ColorPaletteRedBorder2)),
        (Outline, Important) => (None, ColorNeutralForeground3, Some(ColorNeutralStrokeAccessible)),
        (Outline, Informative) => (None, ColorNeutralForeground3, Some(ColorNeutralStroke2)),
        (Outline, Severe) => (
            None,
            ColorPaletteDarkOrangeForeground3,
            Some(ColorPaletteDarkOrangeForeground3),
        ),
        (Outline, Subtle) => (
            None,
            ColorNeutralForegroundStaticInverted,
            Some(ColorNeutralForegroundStaticInverted),
        ),
        (Outline, Success) => (None, ColorPaletteGreenForeground3, Some(ColorPaletteGreenBorder2)),
        (Outline, Warning) => (None, ColorPaletteYellowForeground2, Some(ColorPaletteYellowForeground2)),
        (Tint, Brand) => (
            Some(ColorBrandBackground2),
            ColorBrandForeground2,
            Some(ColorBrandStroke2),
        ),
        (Tint, Danger) => (
            Some(ColorPaletteRedBackground1),
            ColorPaletteRedForeground1,
            Some(ColorPaletteRedBorder1),
        ),
        (Tint, Important) => (Some(ColorNeutralForeground3), ColorNeutralBackground1, None),
        (Tint, Informative) => (
            Some(ColorNeutralBackground4),
            ColorNeutralForeground3,
            Some(ColorNeutralStroke2),
        ),
        (Tint, Severe) => (
            Some(ColorPaletteDarkOrangeBackground1),
            ColorPaletteDarkOrangeForeground1,
            Some(ColorPaletteDarkOrangeBorder1),
        ),
        (Tint, Subtle) => (
            Some(ColorNeutralBackground1),
            ColorNeutralForeground3,
            Some(ColorNeutralStroke2),
        ),
        (Tint, Success) => (
            Some(ColorPaletteGreenBackground1),
            ColorPaletteGreenForeground1,
            Some(ColorPaletteGreenBorder1),
        ),
        (Tint, Warning) => (
            Some(ColorPaletteYellowBackground1),
            ColorPaletteYellowForeground1,
            Some(ColorPaletteYellowBorder1),
        ),
    };
    Look { fill, text, border }
}

/// Fluent `Badge`: a small status label with optional icon.
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Badge {
    salt: KeyedSalt,
    key: AgentKey,
    text: String,
    icon: Option<Icon>,
    icon_position: IconPosition,
    size: BadgeSize,
    shape: BadgeShape,
    appearance: BadgeAppearance,
    color: BadgeColor,
    pub(super) dot: bool,
    pub(super) hidden: bool,
}

impl Badge {
    /// Medium, filled, brand, circular; `text` may be empty next to an icon. Agent key `<id_salt>-badge`.
    pub fn new(id_salt: impl Into<KeyedSalt>, text: impl Into<String>) -> Self {
        Self {
            salt: id_salt.into(),
            key: AgentKey::Kind("badge"),
            text: text.into(),
            icon: None,
            icon_position: IconPosition::Before,
            size: BadgeSize::Medium,
            shape: BadgeShape::Circular,
            appearance: BadgeAppearance::Filled,
            color: BadgeColor::Brand,
            dot: false,
            hidden: false,
        }
    }

    /// Glyph beside the text.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Default [`IconPosition::Before`].
    pub fn icon_position(mut self, position: IconPosition) -> Self {
        self.icon_position = position;
        self
    }

    /// Default [`BadgeSize::Medium`]; tiny and extra-small show no content.
    pub fn size(mut self, size: BadgeSize) -> Self {
        self.size = size;
        self
    }

    /// Default [`BadgeShape::Circular`].
    pub fn shape(mut self, shape: BadgeShape) -> Self {
        self.shape = shape;
        self
    }

    /// Default [`BadgeAppearance::Filled`].
    pub fn appearance(mut self, appearance: BadgeAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// Default [`BadgeColor::Brand`].
    pub fn color(mut self, color: BadgeColor) -> Self {
        self.color = color;
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-badge` agent key.
        agent_key(key)
    }

    pub(super) fn with_key(mut self, key: AgentKey) -> Self {
        self.key = key;
        self
    }

    fn galley(&self, ui: &MineralUi<'_, Fluent>, color: Color32) -> Option<Arc<Galley>> {
        if self.text.is_empty() || self.size.dot() || self.dot {
            return None;
        }
        let format = self.size.text().format(ui.ctx(), color);
        Some(
            ui.painter()
                .layout_job(LayoutJob::single_section(self.text.clone(), format)),
        )
    }

    fn icon_size(&self) -> Option<f32> {
        self.icon
            .filter(|_| !self.size.dot() && !self.dot)
            .map(|_| self.size.icon())
    }

    // Width of the content including the icon margins, and its start-relative offset from the box start.
    fn content_width(&self, g: Option<&Arc<Galley>>) -> f32 {
        match (self.icon_size(), g) {
            (Some(i), Some(g)) => i + 2.0 + g.size().x,
            (Some(i), None) => i - 2.0 * d::TEXT_PADDING,
            (None, Some(g)) => g.size().x,
            (None, None) => 0.0,
        }
    }

    fn extent(&self, g: Option<&Arc<Galley>>) -> egui::Vec2 {
        if self.hidden {
            return egui::Vec2::ZERO;
        }
        if self.dot {
            return vec2(cd::DOT, cd::DOT);
        }
        let h = self.size.height();
        if self.size.dot() {
            return vec2(h, h);
        }
        vec2(h.max(2.0 * self.size.padding() + self.content_width(g)), h)
    }

    fn radius(&self, rect: Rect) -> CornerRadius {
        CornerRadius::from(match self.shape {
            BadgeShape::Circular => rect.height() / 2.0,
            BadgeShape::Rounded if self.size.small_to_tiny() => geometry::BORDER_RADIUS_SMALL,
            BadgeShape::Rounded => geometry::BORDER_RADIUS_MEDIUM,
            BadgeShape::Square => geometry::BORDER_RADIUS_NONE,
        })
    }

    fn paint(&self, ui: &MineralUi<'_, Fluent>, rect: Rect, t: &Tokens, dir: Direction) {
        let look = look(self.appearance, self.color);
        let text_color = t.color(look.text);
        let p = ui.painter();
        let r = self.radius(rect);
        if let Some(fill) = look.fill {
            p.rect_filled(rect, r, t.color(fill));
        }
        if let Some(border) = look.border {
            p.rect_stroke(
                rect,
                r,
                Stroke::new(geometry::STROKE_WIDTH_THIN, t.color(border)),
                StrokeKind::Inside,
            );
        }
        let g = self.galley(ui, text_color);
        let content = self.content_width(g.as_ref());
        let start = ((rect.width() - content) / 2.0).max(0.0);
        let before = self.icon_position == IconPosition::Before;
        let x = |from: f32, w: f32| dir.start_x(rect, Rangef::new(from, from + w));
        let mut text_from = start;
        if let Some(i) = self.icon_size() {
            let left = match (&g, before) {
                (Some(_), false) => start + g.as_ref().map_or(0.0, |g| g.size().x) + self.size.icon_gap(),
                _ => start - d::TEXT_PADDING,
            };
            if g.is_some() && before {
                text_from = start + i + 2.0;
            }
            if let Some(icon) = self.icon {
                let center = pos2(x(left, i) + i / 2.0, rect.center().y);
                paint::icon(p, center, icon, true, i, text_color);
            }
        }
        if let Some(g) = g {
            let at = pos2(x(text_from, g.size().x), rect.center().y - g.size().y / 2.0);
            p.galley(at, g, text_color);
        }
    }
}

impl MineralWidget for Badge {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt, hover, no_target);

    fn layout(&self, ui: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        let g = self.galley(ui, Color32::PLACEHOLDER);
        c.constrain(self.extent(g.as_ref())).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        if !self.hidden {
            let t = ui.tokens();
            let dir = ui.i18n().direction();
            self.paint(ui, frame.zones.visual, &t, dir);
        }
        WidgetResponse::new(frame.response.clone(), ())
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        (!self.text.is_empty() && !self.hidden)
            .then(|| SemanticNode::new(self.key.id(&self.salt), Role::Label, &self.text))
    }
}

/// `appearance` of a [`CounterBadge`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CounterBadgeAppearance {
    /// `filled`.
    #[default]
    Filled,
    /// `ghost`.
    Ghost,
}

impl CounterBadgeAppearance {
    /// Every variant.
    pub const ALL: &'static [Self] = &[Self::Filled, Self::Ghost];
}

/// `color` of a [`CounterBadge`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CounterBadgeColor {
    /// `brand`.
    #[default]
    Brand,
    /// `danger`.
    Danger,
    /// `important`.
    Important,
    /// `informative`.
    Informative,
}

impl CounterBadgeColor {
    /// Every variant.
    pub const ALL: &'static [Self] = &[Self::Brand, Self::Danger, Self::Important, Self::Informative];
}

/// Fluent `CounterBadge`: a count, capped at an overflow value, hidden at 0 unless it shows zero.
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct CounterBadge {
    salt: KeyedSalt,
    key: AgentKey,
    count: u32,
    overflow: u32,
    show_zero: bool,
    dot: bool,
    size: BadgeSize,
    shape: BadgeShape,
    appearance: CounterBadgeAppearance,
    color: CounterBadgeColor,
}

impl CounterBadge {
    /// Medium, filled, brand, overflow at 99; agent key `<id_salt>-counter-badge`.
    pub fn new(id_salt: impl Into<KeyedSalt>, count: u32) -> Self {
        Self {
            salt: id_salt.into(),
            key: AgentKey::Kind("counter-badge"),
            count,
            overflow: cd::OVERFLOW_COUNT,
            show_zero: false,
            dot: false,
            size: BadgeSize::Medium,
            shape: BadgeShape::Circular,
            appearance: CounterBadgeAppearance::Filled,
            color: CounterBadgeColor::Brand,
        }
    }

    /// `overflowCount`: counts above it show as `<overflow>+`; default 99.
    pub fn overflow_count(mut self, overflow: u32) -> Self {
        self.overflow = overflow;
        self
    }

    /// `showZero`: show a count of 0; default hidden.
    pub fn show_zero(mut self, show_zero: bool) -> Self {
        self.show_zero = show_zero;
        self
    }

    /// `dot`: a 6 px dot without the count.
    pub fn dot(mut self, dot: bool) -> Self {
        self.dot = dot;
        self
    }

    /// Default [`BadgeSize::Medium`].
    pub fn size(mut self, size: BadgeSize) -> Self {
        self.size = size;
        self
    }

    /// Default [`BadgeShape::Circular`].
    pub fn shape(mut self, shape: BadgeShape) -> Self {
        self.shape = shape;
        self
    }

    /// Default [`CounterBadgeAppearance::Filled`].
    pub fn appearance(mut self, appearance: CounterBadgeAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// Default [`CounterBadgeColor::Brand`].
    pub fn color(mut self, color: CounterBadgeColor) -> Self {
        self.color = color;
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-counter-badge` agent key.
        agent_key(key)
    }

    fn text(&self, i18n: &I18n) -> String {
        let f = i18n.format();
        if self.count > self.overflow {
            format!("{}+", f.integer(i64::from(self.overflow)))
        } else {
            f.integer(i64::from(self.count)).clone()
        }
    }

    fn badge(&self, i18n: &I18n) -> Badge {
        let mut badge = Badge::new(self.salt.clone(), self.text(i18n))
            .with_key(self.key.clone())
            .size(self.size)
            .shape(self.shape)
            .appearance(match self.appearance {
                CounterBadgeAppearance::Filled => BadgeAppearance::Filled,
                CounterBadgeAppearance::Ghost => BadgeAppearance::Ghost,
            })
            .color(match self.color {
                CounterBadgeColor::Brand => BadgeColor::Brand,
                CounterBadgeColor::Danger => BadgeColor::Danger,
                CounterBadgeColor::Important => BadgeColor::Important,
                CounterBadgeColor::Informative => BadgeColor::Informative,
            });
        badge.dot = self.dot;
        badge.hidden = self.count == 0 && !self.show_zero && !self.dot;
        badge
    }
}

impl MineralWidget for CounterBadge {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt, hover, no_target);

    fn layout(&self, ui: &MineralUi<'_, Fluent>, c: Constraints, s: &()) -> Measure {
        self.badge(ui.i18n()).layout(ui, c, s)
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, s: &mut ()) -> WidgetResponse {
        self.badge(ui.i18n()).update(ui, frame, s)
    }

    fn semantics(&self, i18n: &I18n, r: &WidgetResponse, s: &()) -> Option<SemanticNode> {
        self.badge(i18n).semantics(i18n, r, s)
    }
}
