use crate::widget::vocab::{ButtonAppearance, ButtonShape};
use egui::{Color32, CornerRadius, KeyboardShortcut, Pos2, Rect, Sense, Stroke, StrokeKind, pos2, vec2};
use egui_mineral::{
    focus as roving,
    i18n::{Direction, I18n},
    kit,
    layout::{Constraints, Measure},
    select::Check,
    semantic::{AgentKey, KeyedSalt, Role, SemanticNode},
    shortcut::Mnemonic,
    widget::{Availability, Interaction, MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::{
    Fluent, FluentExt, Icon, IconPosition, Size,
    spec::ColorToken,
    spec::{
        defaults::{button as d, compound_button as cd, menu_button as md},
        tokens::{geometry as g, typography as ty},
    },
    theme::{Elevation, Radius, Tokens},
    widget::{
        focus, paint,
        state::{Ramp, State},
    },
};

use ColorToken::{
    ColorBrandBackground, ColorBrandBackgroundHover, ColorBrandBackgroundPressed, ColorBrandBackgroundSelected,
    ColorNeutralBackground1, ColorNeutralBackground1Hover, ColorNeutralBackground1Pressed,
    ColorNeutralBackground1Selected, ColorNeutralBackgroundDisabled, ColorNeutralForeground1,
    ColorNeutralForeground1Hover, ColorNeutralForeground1Pressed, ColorNeutralForeground1Selected,
    ColorNeutralForeground2, ColorNeutralForeground2BrandHover, ColorNeutralForeground2BrandPressed,
    ColorNeutralForeground2BrandSelected, ColorNeutralForeground2Hover, ColorNeutralForeground2Pressed,
    ColorNeutralForeground2Selected, ColorNeutralForegroundDisabled, ColorNeutralForegroundOnBrand,
    ColorNeutralStroke1, ColorNeutralStroke1Hover, ColorNeutralStroke1Pressed, ColorNeutralStroke1Selected,
    ColorNeutralStrokeDisabled, ColorNeutralStrokeOnBrand, ColorSubtleBackground, ColorSubtleBackgroundHover,
    ColorSubtleBackgroundPressed, ColorSubtleBackgroundSelected, ColorTransparentBackground,
    ColorTransparentBackgroundHover, ColorTransparentBackgroundPressed, ColorTransparentBackgroundSelected,
};

// Padding, type and icon of one size; `pad_*` are per side, inside the `strokeWidthThin` border.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Metrics {
    pub min_width: f32,
    pub pad_top: f32,
    pub pad_bottom: f32,
    pub pad_x: f32,
    pub font_size: f32,
    pub line_height: f32,
    pub weight: u16,
    pub icon: f32,
    pub icon_gap: f32,
    pub icon_only: f32,
}

impl Metrics {
    // react-button useButtonStyles.styles.ts useRootBaseClassName, useRootStyles, useIconStyles
    pub(crate) const fn button(size: Size, with_icon: bool) -> Self {
        match size {
            Size::Small => {
                let pad = if with_icon {
                    d::SPACING_SMALL_WITH_ICON
                } else {
                    d::SPACING_SMALL
                };
                Self {
                    min_width: d::MIN_WIDTH_SMALL,
                    pad_top: pad,
                    pad_bottom: pad,
                    pad_x: g::SPACING_HORIZONTAL_S,
                    font_size: ty::FONT_SIZE_BASE_200,
                    line_height: ty::LINE_HEIGHT_BASE_200,
                    weight: ty::FONT_WEIGHT_REGULAR,
                    icon: d::ICON_SIZE,
                    icon_gap: g::SPACING_HORIZONTAL_XS,
                    icon_only: d::ICON_ONLY_SMALL,
                }
            }
            Size::Medium => Self {
                min_width: d::MIN_WIDTH,
                pad_top: d::SPACING_MEDIUM,
                pad_bottom: d::SPACING_MEDIUM,
                pad_x: g::SPACING_HORIZONTAL_M,
                font_size: ty::FONT_SIZE_BASE_300,
                line_height: ty::LINE_HEIGHT_BASE_300,
                weight: ty::FONT_WEIGHT_SEMIBOLD,
                icon: d::ICON_SIZE,
                icon_gap: g::SPACING_HORIZONTAL_S_NUDGE,
                icon_only: d::ICON_ONLY_MEDIUM,
            },
            Size::Large => {
                let pad = if with_icon {
                    d::SPACING_LARGE_WITH_ICON
                } else {
                    d::SPACING_LARGE
                };
                Self {
                    min_width: d::MIN_WIDTH,
                    pad_top: pad,
                    pad_bottom: pad,
                    pad_x: g::SPACING_HORIZONTAL_L,
                    font_size: ty::FONT_SIZE_BASE_400,
                    line_height: ty::LINE_HEIGHT_BASE_400,
                    weight: ty::FONT_WEIGHT_SEMIBOLD,
                    icon: d::ICON_SIZE_LARGE,
                    icon_gap: g::SPACING_HORIZONTAL_S_NUDGE,
                    icon_only: d::ICON_ONLY_LARGE,
                }
            }
        }
    }

    // react-button useCompoundButtonStyles.styles.ts useRootStyles sizes, useIconStyles
    pub(crate) const fn compound(size: Size) -> Self {
        let base = Self::button(size, false);
        let (pad_top, pad_bottom, pad_x, font_size, line_height, icon_only) = match size {
            Size::Small => (
                g::SPACING_HORIZONTAL_S,
                g::SPACING_HORIZONTAL_M_NUDGE,
                g::SPACING_HORIZONTAL_S,
                ty::FONT_SIZE_BASE_300,
                ty::LINE_HEIGHT_BASE_300,
                cd::ICON_ONLY_SMALL,
            ),
            Size::Medium => (
                cd::PADDING_TOP_MEDIUM,
                g::SPACING_HORIZONTAL_L,
                g::SPACING_HORIZONTAL_M,
                ty::FONT_SIZE_BASE_300,
                ty::LINE_HEIGHT_BASE_300,
                cd::ICON_ONLY_MEDIUM,
            ),
            Size::Large => (
                cd::PADDING_TOP_LARGE,
                g::SPACING_HORIZONTAL_XL,
                g::SPACING_HORIZONTAL_L,
                ty::FONT_SIZE_BASE_400,
                ty::LINE_HEIGHT_BASE_400,
                cd::ICON_ONLY_LARGE,
            ),
        };
        Self {
            pad_top,
            pad_bottom,
            pad_x,
            font_size,
            line_height,
            icon: cd::ICON_SIZE,
            icon_gap: g::SPACING_HORIZONTAL_M,
            icon_only,
            ..base
        }
    }

    // react-button useCompoundButtonStyles.styles.ts useSecondaryContentStyles: `lineHeight: 100%`
    pub(crate) const fn secondary_size(size: Size) -> f32 {
        match size {
            Size::Small | Size::Medium => ty::FONT_SIZE_BASE_200,
            Size::Large => ty::FONT_SIZE_BASE_300,
        }
    }

    // react-button useMenuButtonStyles.styles.ts useMenuIconStyles
    pub(crate) const fn menu_icon(size: Size) -> f32 {
        match size {
            Size::Small | Size::Medium => md::MENU_ICON_SIZE,
            Size::Large => md::MENU_ICON_SIZE_LARGE,
        }
    }
}

pub(crate) fn corners(shape: ButtonShape, rect: Rect) -> CornerRadius {
    match shape {
        ButtonShape::Rounded => Radius::Medium,
        ButtonShape::Circular => Radius::Circular,
        ButtonShape::Square => Radius::None,
    }
    .on(rect)
}

// react-button useButtonStyles.styles.ts useRootFocusStyles: size radii, overridden by circular and square.
fn focus_corners(shape: ButtonShape, size: Size, rect: Rect) -> CornerRadius {
    match (shape, size) {
        (ButtonShape::Circular, _) => Radius::Circular,
        (ButtonShape::Square, _) => Radius::None,
        (ButtonShape::Rounded, Size::Small) => Radius::Small,
        (ButtonShape::Rounded, Size::Medium) => Radius::Medium,
        (ButtonShape::Rounded, Size::Large) => Radius::Large,
    }
    .on(rect)
}

// Colors of one state: fill, stroke, label, icon; `filled_icon` swaps in the filled glyph.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Look {
    pub fill: Color32,
    pub stroke: Color32,
    pub stroke_width: f32,
    pub content: Color32,
    pub icon: Color32,
    pub secondary: Color32,
    pub filled_icon: bool,
}

#[derive(Clone, Copy, Debug)]
struct Ramps {
    fill: Ramp,
    stroke: Ramp,
    content: Ramp,
    icon: Option<Ramp>,
}

const NEUTRAL_STROKE: Ramp = Ramp::new(
    ColorNeutralStroke1,
    ColorNeutralStroke1Hover,
    ColorNeutralStroke1Pressed,
    ColorNeutralStrokeDisabled,
);
const NEUTRAL_CONTENT: Ramp = Ramp::new(
    ColorNeutralForeground1,
    ColorNeutralForeground1Hover,
    ColorNeutralForeground1Pressed,
    ColorNeutralForegroundDisabled,
);
const TRANSPARENT_FILL: Ramp = Ramp::new(
    ColorTransparentBackground,
    ColorTransparentBackgroundHover,
    ColorTransparentBackgroundPressed,
    ColorTransparentBackground,
);

// react-button useButtonStyles.styles.ts useRootStyles, useRootDisabledStyles
fn rest_ramps(appearance: ButtonAppearance) -> Ramps {
    let neutral_fill = Ramp::new(
        ColorNeutralBackground1,
        ColorNeutralBackground1Hover,
        ColorNeutralBackground1Pressed,
        ColorNeutralBackgroundDisabled,
    );
    let none = Ramp::TRANSPARENT;
    match appearance {
        ButtonAppearance::Secondary => Ramps {
            fill: neutral_fill,
            stroke: NEUTRAL_STROKE,
            content: NEUTRAL_CONTENT,
            icon: None,
        },
        ButtonAppearance::Outline => Ramps {
            fill: TRANSPARENT_FILL,
            stroke: NEUTRAL_STROKE,
            content: NEUTRAL_CONTENT,
            icon: None,
        },
        ButtonAppearance::Primary => Ramps {
            fill: Ramp::new(
                ColorBrandBackground,
                ColorBrandBackgroundHover,
                ColorBrandBackgroundPressed,
                ColorNeutralBackgroundDisabled,
            ),
            stroke: none,
            content: Ramp::new(
                ColorNeutralForegroundOnBrand,
                ColorNeutralForegroundOnBrand,
                ColorNeutralForegroundOnBrand,
                ColorNeutralForegroundDisabled,
            ),
            icon: None,
        },
        ButtonAppearance::Subtle => Ramps {
            fill: Ramp::new(
                ColorSubtleBackground,
                ColorSubtleBackgroundHover,
                ColorSubtleBackgroundPressed,
                ColorTransparentBackground,
            ),
            stroke: none,
            content: Ramp::new(
                ColorNeutralForeground2,
                ColorNeutralForeground2Hover,
                ColorNeutralForeground2Pressed,
                ColorNeutralForegroundDisabled,
            ),
            icon: Some(Ramp::new(
                ColorNeutralForeground2,
                ColorNeutralForeground2BrandHover,
                ColorNeutralForeground2BrandPressed,
                ColorNeutralForegroundDisabled,
            )),
        },
        ButtonAppearance::Transparent => Ramps {
            fill: TRANSPARENT_FILL,
            stroke: none,
            content: Ramp::new(
                ColorNeutralForeground2,
                ColorNeutralForeground2BrandHover,
                ColorNeutralForeground2BrandPressed,
                ColorNeutralForegroundDisabled,
            ),
            icon: None,
        },
    }
}

// react-button useToggleButtonStyles.styles.ts useRootCheckedStyles, useIconCheckedStyles
fn checked_ramps(appearance: ButtonAppearance) -> Ramps {
    let rest = rest_ramps(appearance);
    let selected = |rest: ColorToken, ramp: Ramp| Ramp {
        rest: Some(rest),
        ..ramp
    };
    match appearance {
        ButtonAppearance::Secondary => Ramps {
            fill: selected(ColorNeutralBackground1Selected, rest.fill),
            stroke: selected(ColorNeutralStroke1, rest.stroke),
            content: selected(ColorNeutralForeground1Selected, rest.content),
            icon: None,
        },
        ButtonAppearance::Outline => Ramps {
            fill: selected(ColorTransparentBackgroundSelected, rest.fill),
            stroke: Ramp::new(
                ColorNeutralStroke1,
                ColorNeutralStroke1,
                ColorNeutralStroke1,
                ColorNeutralStrokeDisabled,
            ),
            content: selected(ColorNeutralForeground1Selected, rest.content),
            icon: None,
        },
        ButtonAppearance::Primary => Ramps {
            fill: selected(ColorBrandBackgroundSelected, rest.fill),
            ..rest
        },
        ButtonAppearance::Subtle => Ramps {
            fill: selected(ColorSubtleBackgroundSelected, rest.fill),
            content: selected(ColorNeutralForeground2Selected, rest.content),
            icon: Some(Ramp::new(
                ColorNeutralForeground2BrandSelected,
                ColorNeutralForeground2BrandSelected,
                ColorNeutralForeground2BrandSelected,
                ColorNeutralForegroundDisabled,
            )),
            ..rest
        },
        ButtonAppearance::Transparent => Ramps {
            fill: selected(ColorTransparentBackgroundSelected, rest.fill),
            content: selected(ColorNeutralForeground2BrandSelected, rest.content),
            icon: Some(Ramp::new(
                ColorNeutralForeground2BrandSelected,
                ColorNeutralForeground2BrandSelected,
                ColorNeutralForeground2BrandSelected,
                ColorNeutralForegroundDisabled,
            )),
            ..rest
        },
    }
}

// react-button useMenuButtonStyles.styles.ts useRootExpandedStyles, useIconExpandedStyles
fn expanded_ramps(appearance: ButtonAppearance) -> Ramps {
    let rest = rest_ramps(appearance);
    let all = |c: ColorToken, disabled: Ramp| Ramp::new(c, c, c, c).disabled(disabled.disabled);
    match appearance {
        ButtonAppearance::Secondary => Ramps {
            fill: all(ColorNeutralBackground1Selected, rest.fill),
            stroke: all(ColorNeutralStroke1Selected, rest.stroke),
            content: all(ColorNeutralForeground1Selected, rest.content),
            icon: None,
        },
        ButtonAppearance::Outline => Ramps {
            stroke: all(ColorNeutralStroke1Selected, rest.stroke),
            content: all(ColorNeutralForeground1Selected, rest.content),
            ..rest
        },
        ButtonAppearance::Primary => Ramps {
            fill: all(ColorBrandBackgroundSelected, rest.fill),
            ..rest
        },
        ButtonAppearance::Subtle => Ramps {
            fill: all(ColorSubtleBackgroundSelected, rest.fill),
            content: all(ColorNeutralForeground2Selected, rest.content),
            icon: Some(all(ColorNeutralForeground2BrandSelected, rest.content)),
            ..rest
        },
        ButtonAppearance::Transparent => Ramps {
            fill: all(ColorTransparentBackgroundSelected, rest.fill),
            content: all(ColorNeutralForeground2BrandSelected, rest.content),
            ..rest
        },
    }
}

// react-button useCompoundButtonStyles.styles.ts useRootStyles secondary content colors
fn secondary_ramp(appearance: ButtonAppearance) -> Ramp {
    match appearance {
        ButtonAppearance::Primary => rest_ramps(appearance).content,
        ButtonAppearance::Transparent => Ramp::new(
            ColorNeutralForeground2,
            ColorNeutralForeground2BrandHover,
            ColorNeutralForeground2BrandPressed,
            ColorNeutralForegroundDisabled,
        ),
        ButtonAppearance::Secondary | ButtonAppearance::Outline | ButtonAppearance::Subtle => Ramp::new(
            ColorNeutralForeground2,
            ColorNeutralForeground2Hover,
            ColorNeutralForeground2Pressed,
            ColorNeutralForegroundDisabled,
        ),
    }
}

// Selection of the appearance's state colors: rest, toggled on (`checked`) or menu open (`expanded`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum Mode {
    #[default]
    Rest,
    Checked,
    Expanded,
}

pub(crate) fn look(appearance: ButtonAppearance, mode: Mode, s: State, t: &Tokens) -> Look {
    let r = match mode {
        Mode::Rest => rest_ramps(appearance),
        Mode::Checked => checked_ramps(appearance),
        Mode::Expanded => expanded_ramps(appearance),
    };
    let thicker = match (appearance, mode) {
        (ButtonAppearance::Outline, Mode::Checked | Mode::Expanded) => s != State::Disabled,
        _ => false,
    };
    // `bundleIcon`: subtle and transparent show the filled glyph while hovered or pressed; toggles while checked.
    let filled_icon = s != State::Disabled
        && (mode == Mode::Checked
            || (s.engaged() && matches!(appearance, ButtonAppearance::Subtle | ButtonAppearance::Transparent)));
    Look {
        fill: r.fill.pick(s, t),
        stroke: r.stroke.pick(s, t),
        stroke_width: if thicker {
            g::STROKE_WIDTH_THICKER
        } else {
            g::STROKE_WIDTH_THIN
        },
        content: r.content.pick(s, t),
        icon: r.icon.unwrap_or(r.content).pick(s, t),
        secondary: secondary_ramp(appearance).pick(s, t),
        filled_icon,
    }
}

// Side of a split button part that touches the other part.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Join {
    // Primary action: square end corners, end border recolored.
    Start,
    // Menu part: square start corners, no start border.
    End,
}

fn joined(r: CornerRadius, join: Option<Join>, dir: Direction) -> CornerRadius {
    let square_left = match (join, dir) {
        (None, _) => return r,
        (Some(Join::End), Direction::Ltr) | (Some(Join::Start), Direction::Rtl) => true,
        (Some(Join::Start), Direction::Ltr) | (Some(Join::End), Direction::Rtl) => false,
    };
    if square_left {
        CornerRadius { nw: 0, sw: 0, ..r }
    } else {
        CornerRadius { ne: 0, se: 0, ..r }
    }
}

// react-button useSplitButtonStyles.styles.ts useRootStyles: primary action `borderRightColor`.
fn join_stroke(appearance: ButtonAppearance, s: State, t: &Tokens) -> Color32 {
    match (appearance, s) {
        (_, State::Disabled) => t.color(ColorNeutralStrokeDisabled),
        (ButtonAppearance::Primary, _) => t.color(ColorNeutralStrokeOnBrand),
        (ButtonAppearance::Subtle | ButtonAppearance::Transparent, s) => TRANSPARENT_FILL.pick(s, t),
        (ButtonAppearance::Secondary | ButtonAppearance::Outline, s) => NEUTRAL_STROKE.pick(s, t),
    }
}

/// Fluent `Button`: a label with an optional icon before or after it, in five appearances, three sizes and
/// three shapes.
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct PushButton<'a> {
    salt: KeyedSalt,
    pub(crate) key: AgentKey,
    label: String,
    icon: Option<Icon>,
    icon_position: IconPosition,
    appearance: ButtonAppearance,
    size: Size,
    shape: ButtonShape,
    label_visible: bool,
    disabled: bool,
    mnemonic: Option<Mnemonic>,
    shortcut: Option<KeyboardShortcut>,
    pub(crate) checked: Option<&'a mut bool>,
    pub(crate) mode: Mode,
    pub(crate) expanded: Option<bool>,
    pub(crate) menu_icon: bool,
    pub(crate) secondary: Option<String>,
    pub(crate) compound: bool,
    pub(crate) join: Option<Join>,
    pub(crate) min_width: Option<f32>,
    pub(crate) tab_stop: Option<roving::TabStop>,
    pub(crate) radio: bool,
    pub(crate) metrics: Option<Metrics>,
    pub(crate) inert: bool,
}

impl<'a> PushButton<'a> {
    /// `label` is the visible text and accessible name; agent key `<id_salt>-button`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        Self {
            salt: id_salt.into(),
            key: egui_mineral::agent_key!("button"),
            label: label.into(),
            icon: None,
            icon_position: IconPosition::Before,
            appearance: ButtonAppearance::Secondary,
            size: Size::Medium,
            shape: ButtonShape::Rounded,
            label_visible: true,
            disabled: false,
            mnemonic: None,
            shortcut: None,
            checked: None,
            mode: Mode::Rest,
            expanded: None,
            menu_icon: false,
            secondary: None,
            compound: false,
            join: None,
            min_width: None,
            tab_stop: None,
            radio: false,
            metrics: None,
            inert: false,
        }
    }

    /// Default [`ButtonAppearance::Secondary`].
    pub fn appearance(mut self, appearance: ButtonAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// Default [`Size::Medium`].
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Default [`ButtonShape::Rounded`].
    pub fn shape(mut self, shape: ButtonShape) -> Self {
        self.shape = shape;
        self
    }

    /// Icon at [`Self::icon_position`]; filled while hovered (subtle, transparent) or checked.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Default [`IconPosition::Before`].
    pub fn icon_position(mut self, position: IconPosition) -> Self {
        self.icon_position = position;
        self
    }

    /// `false` with an icon is the square icon-only button; the label stays the accessible name.
    pub fn label_visible(mut self, visible: bool) -> Self {
        self.label_visible = visible;
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

    pub(crate) fn bind(mut self, checked: &'a mut bool) -> Self {
        self.mode = if *checked { Mode::Checked } else { Mode::Rest };
        self.checked = Some(checked);
        self
    }

    fn icon_only(&self) -> bool {
        (self.icon.is_some() || self.menu_icon) && !self.label_visible
    }

    fn metrics(&self) -> Metrics {
        if let Some(m) = self.metrics {
            m
        } else if self.compound {
            Metrics::compound(self.size)
        } else {
            Metrics::button(self.size, self.icon.is_some() || self.icon_only())
        }
    }

    fn text_format(ctx: &egui::Context, m: &Metrics, color: Color32) -> egui::TextFormat {
        crate::typography::format(
            ctx,
            crate::typography::TypeSpec {
                size: m.font_size,
                line_height: m.line_height,
                weight: m.weight,
            },
            color,
        )
    }

    fn secondary_format(&self, ctx: &egui::Context, color: Color32) -> egui::TextFormat {
        let size = Metrics::secondary_size(self.size);
        crate::typography::format(
            ctx,
            crate::typography::TypeSpec {
                size,
                line_height: size,
                weight: ty::FONT_WEIGHT_REGULAR,
            },
            color,
        )
    }

    // Content box: icon, text column (label over secondary), menu chevron.
    fn content(&self, ui: &MineralUi<'_, Fluent>, m: &Metrics) -> (egui::Vec2, f32) {
        let p = ui.painter();
        let label = paint::galley(
            ui.core(),
            p,
            &self.label,
            &Self::text_format(ui.ctx(), m, Color32::PLACEHOLDER),
            self.mnemonic,
        );
        let secondary = self.secondary.as_deref().map(|s| {
            p.layout_job(egui::text::LayoutJob::single_section(
                s.to_owned(),
                self.secondary_format(ui.ctx(), Color32::PLACEHOLDER),
            ))
        });
        let text_w = label.size().x.max(secondary.as_ref().map_or(0.0, |s| s.size().x));
        let text_h = label.size().y + secondary.as_ref().map_or(0.0, |s| s.size().y);
        let icon_w = if self.icon.is_some() { m.icon + m.icon_gap } else { 0.0 };
        let menu_w = if self.menu_icon {
            g::SPACING_HORIZONTAL_XS + Metrics::menu_icon(self.size)
        } else {
            0.0
        };
        let h = text_h.max(if self.icon.is_some() { m.icon } else { 0.0 });
        (vec2(icon_w + text_w + menu_w, h), text_w)
    }

    pub(crate) fn natural(&self, ui: &MineralUi<'_, Fluent>) -> egui::Vec2 {
        let m = self.metrics();
        let border = g::STROKE_WIDTH_THIN * 2.0;
        let min_w = self.min_width.unwrap_or(m.min_width);
        if self.icon_only() && self.icon.is_none() {
            // Chevron only: content width within the icon-only width, down to `min_width`.
            let w = m.pad_top * 2.0 + Metrics::menu_icon(self.size) + border;
            let lo = self.min_width.unwrap_or(m.icon_only).min(m.icon_only);
            return vec2(w.clamp(lo, m.icon_only), m.icon_only);
        }
        if self.icon_only() {
            let menu_w = if self.menu_icon {
                g::SPACING_HORIZONTAL_XS + Metrics::menu_icon(self.size)
            } else {
                0.0
            };
            let h = if self.compound {
                m.icon + m.pad_top + m.pad_bottom + border
            } else {
                m.icon_only
            };
            return vec2((m.icon_only + menu_w).max(self.min_width.unwrap_or(0.0)), h);
        }
        let (content, _) = self.content(ui, &m);
        let h = if self.compound {
            content.y
        } else {
            content.y.max(m.line_height)
        };
        vec2(
            (content.x + m.pad_x * 2.0 + border).max(min_w),
            h + m.pad_top + m.pad_bottom + border,
        )
    }
}

impl MineralWidget for PushButton<'_> {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt, enabled);

    fn access_key(&self) -> Option<Mnemonic> {
        self.mnemonic
    }

    fn accelerator(&self) -> Option<KeyboardShortcut> {
        self.shortcut
    }

    fn sense(&self) -> Sense {
        self.tab_stop
            .map_or(Sense::click(), |t| roving::roving_sense(Sense::click(), t))
    }

    fn layout(&self, ui: &MineralUi<'_, Fluent>, constraints: Constraints, (): &()) -> Measure {
        constraints.constrain(self.natural(ui)).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let response = &frame.response;
        let rect = frame.zones.visual;
        let availability = Availability::disabled_if(self.disabled);
        kit::input::hover_cursor(ui.core(), response, availability);
        if self.tab_stop.is_some() {
            kit::focus::follow_press(response);
        }
        let i = Interaction::of(response, availability);
        let toggled = response.clicked()
            && self.checked.as_deref_mut().is_some_and(|c| {
                *c = !*c;
                true
            });
        if toggled {
            self.mode = if self.checked.as_deref().copied().unwrap_or(false) {
                Mode::Checked
            } else {
                Mode::Rest
            };
        }
        let mode = if self.expanded == Some(true) {
            Mode::Expanded
        } else {
            self.mode
        };
        let s = if self.inert { State::Rest } else { State::of(i) };
        let look = look(self.appearance, mode, s, &t);
        let dir = ui.i18n().direction();
        let r = joined(corners(self.shape, rect), self.join, dir);
        let p = ui.painter();

        if i.focused && self.appearance == ButtonAppearance::Primary {
            // react-button useRootFocusStyles.primary `shadow2`
            paint::shadow(p, rect, r, Elevation::E2, &t);
        }
        p.rect_filled(rect, r, look.fill);
        let stroke_rect = match (self.join, dir) {
            // `borderLeftWidth: 0`: the start border is the primary action's end border.
            (Some(Join::End), Direction::Ltr) => {
                Rect::from_min_max(pos2(rect.left() - g::STROKE_WIDTH_THIN, rect.top()), rect.max)
            }
            (Some(Join::End), Direction::Rtl) => {
                Rect::from_min_max(rect.min, pos2(rect.right() + g::STROKE_WIDTH_THIN, rect.bottom()))
            }
            _ => rect,
        };
        p.with_clip_rect(rect).rect_stroke(
            stroke_rect,
            r,
            Stroke::new(look.stroke_width, look.stroke),
            StrokeKind::Inside,
        );
        if self.join == Some(Join::Start) {
            let x = match dir {
                Direction::Ltr => rect.right() - g::STROKE_WIDTH_THIN / 2.0,
                Direction::Rtl => rect.left() + g::STROKE_WIDTH_THIN / 2.0,
            };
            p.vline(
                x,
                rect.y_range(),
                Stroke::new(g::STROKE_WIDTH_THIN, join_stroke(self.appearance, s, &t)),
            );
        }

        let m = self.metrics();
        let menu = Metrics::menu_icon(self.size);
        if self.icon_only() {
            self.paint_icon_only(ui, rect, &m, menu, look, dir);
        } else {
            self.paint_content(ui, rect, &m, look, dir);
        }
        if i.focused {
            focus::inset(
                ui.painter(),
                rect,
                joined(focus_corners(self.shape, self.size, rect), self.join, dir),
                self.appearance == ButtonAppearance::Primary && !i.hovered,
                &t,
            );
        }
        WidgetResponse::new(response.clone(), ()).with_changed(toggled)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let role = if self.radio { Role::RadioButton } else { Role::Button };
        let mut node = SemanticNode::new(self.key.id(&self.salt), role, &self.label)
            .clickable(Availability::disabled_if(self.disabled));
        if self.checked.is_some() || self.radio {
            node.state.checked = Some(Check::from(self.mode == Mode::Checked));
        }
        node.state.disabled |= self.inert;
        node.state.expanded = self.expanded;
        if let Some(secondary) = &self.secondary {
            node.value = Some(secondary.clone());
        }
        Some(node)
    }
}

impl PushButton<'_> {
    fn paint_icon_only(
        &self,
        ui: &MineralUi<'_, Fluent>,
        rect: Rect,
        m: &Metrics,
        menu: f32,
        look: Look,
        dir: Direction,
    ) {
        let p = ui.painter();
        let icons = ui.icons();
        let icon_w = if self.icon.is_some() { m.icon } else { 0.0 };
        let gap = if self.icon.is_some() && self.menu_icon {
            g::SPACING_HORIZONTAL_XS
        } else {
            0.0
        };
        let menu_w = if self.menu_icon { menu } else { 0.0 };
        let start = rect.center().x - (icon_w + gap + menu_w) / 2.0;
        let mirror = |x: f32| match dir {
            Direction::Ltr => x,
            Direction::Rtl => rect.center().x * 2.0 - x,
        };
        if let Some(icon) = self.icon {
            paint::icon(
                p,
                pos2(mirror(start + icon_w / 2.0), rect.center().y),
                icon,
                look.filled_icon,
                m.icon,
                look.icon,
            );
        }
        if self.menu_icon {
            let x = mirror(start + icon_w + gap + menu_w / 2.0);
            paint::icon(
                p,
                pos2(x, rect.center().y),
                icons.chevron_down,
                false,
                menu,
                look.content,
            );
        }
    }

    fn paint_content(&self, ui: &MineralUi<'_, Fluent>, rect: Rect, m: &Metrics, look: Look, dir: Direction) {
        let p = ui.painter();
        let (content, text_w) = self.content(ui, m);
        let inner = Rect::from_min_max(
            pos2(
                rect.left() + m.pad_x + g::STROKE_WIDTH_THIN,
                rect.top() + m.pad_top + g::STROKE_WIDTH_THIN,
            ),
            pos2(
                rect.right() - m.pad_x - g::STROKE_WIDTH_THIN,
                rect.bottom() - m.pad_bottom - g::STROKE_WIDTH_THIN,
            ),
        );
        // `justifyContent: center`, laid out left to right, then mirrored for right-to-left.
        let left = inner.center().x - content.x / 2.0;
        let mirror = |x: f32| match dir {
            Direction::Ltr => x,
            Direction::Rtl => rect.center().x * 2.0 - x,
        };
        let icon_after = self.icon_position == IconPosition::After && !self.menu_icon;
        let icon_w = if self.icon.is_some() { m.icon + m.icon_gap } else { 0.0 };
        let text_left = if icon_after { left } else { left + icon_w };
        if let Some(icon) = self.icon {
            let x = if icon_after {
                left + text_w + m.icon_gap + m.icon / 2.0
            } else {
                left + m.icon / 2.0
            };
            let y = if self.compound {
                inner.top() + m.icon / 2.0
            } else {
                inner.center().y
            };
            paint::icon(p, pos2(mirror(x), y), icon, look.filled_icon, m.icon, look.icon);
        }
        let label = paint::galley(
            ui.core(),
            p,
            &self.label,
            &Self::text_format(ui.ctx(), m, look.content),
            self.mnemonic,
        );
        let secondary = self.secondary.as_deref().map(|s| {
            p.layout_job(egui::text::LayoutJob::single_section(
                s.to_owned(),
                self.secondary_format(ui.ctx(), look.secondary),
            ))
        });
        let text_h = label.size().y + secondary.as_ref().map_or(0.0, |s| s.size().y);
        let top = if self.compound {
            inner.top()
        } else {
            inner.center().y - text_h / 2.0
        };
        // `textAlign: left` in the compound content column.
        let place = |w: f32, y: f32| -> Pos2 {
            match dir {
                Direction::Ltr => pos2(text_left, y),
                Direction::Rtl => pos2(mirror(text_left) - w, y),
            }
        };
        let label_h = label.size().y;
        p.galley(place(label.size().x, top), label, look.content);
        if let Some(sec) = secondary {
            p.galley(place(sec.size().x, top + label_h), sec, look.secondary);
        }
        if self.menu_icon {
            let size = Metrics::menu_icon(self.size);
            let x = left + icon_w + text_w + g::SPACING_HORIZONTAL_XS + size / 2.0;
            paint::icon(
                p,
                pos2(mirror(x), inner.center().y),
                ui.icons().chevron_down,
                false,
                size,
                look.content,
            );
        }
    }
}
