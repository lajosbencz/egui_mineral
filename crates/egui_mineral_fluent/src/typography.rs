use egui::{Color32, FontFamily, FontId, TextFormat};

use crate::spec::tokens::typography as ty;

/// Font metrics of one ramp style; size and line height in points.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TypeSpec {
    /// In points.
    pub size: f32,
    /// In points.
    pub line_height: f32,
    /// CSS weight, 100..=900.
    pub weight: u16,
}

/// Fluent type ramp (`typographyStyles`), smallest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TypeRamp {
    /// `caption2`.
    Caption2,
    /// `caption2Strong`.
    Caption2Strong,
    /// `caption1`.
    Caption1,
    /// `caption1Strong`.
    Caption1Strong,
    /// `caption1Stronger`.
    Caption1Stronger,
    /// `body1`.
    Body1,
    /// `body1Strong`.
    Body1Strong,
    /// `body1Stronger`.
    Body1Stronger,
    /// `body2`.
    Body2,
    /// `subtitle2`.
    Subtitle2,
    /// `subtitle2Stronger`.
    Subtitle2Stronger,
    /// `subtitle1`.
    Subtitle1,
    /// `title3`.
    Title3,
    /// `title2`.
    Title2,
    /// `title1`.
    Title1,
    /// `largeTitle`.
    LargeTitle,
    /// `display`.
    Display,
}

impl TypeRamp {
    /// Every style, smallest first.
    pub const ALL: &'static [Self] = &[
        Self::Caption2,
        Self::Caption2Strong,
        Self::Caption1,
        Self::Caption1Strong,
        Self::Caption1Stronger,
        Self::Body1,
        Self::Body1Strong,
        Self::Body1Stronger,
        Self::Body2,
        Self::Subtitle2,
        Self::Subtitle2Stronger,
        Self::Subtitle1,
        Self::Title3,
        Self::Title2,
        Self::Title1,
        Self::LargeTitle,
        Self::Display,
    ];

    /// Values of this style.
    // tokens global/typographyStyles.ts
    pub const fn spec(self) -> TypeSpec {
        let (size, line_height, weight) = match self {
            Self::Caption2 => (
                ty::FONT_SIZE_BASE_100,
                ty::LINE_HEIGHT_BASE_100,
                ty::FONT_WEIGHT_REGULAR,
            ),
            Self::Caption2Strong => (
                ty::FONT_SIZE_BASE_100,
                ty::LINE_HEIGHT_BASE_100,
                ty::FONT_WEIGHT_SEMIBOLD,
            ),
            Self::Caption1 => (
                ty::FONT_SIZE_BASE_200,
                ty::LINE_HEIGHT_BASE_200,
                ty::FONT_WEIGHT_REGULAR,
            ),
            Self::Caption1Strong => (
                ty::FONT_SIZE_BASE_200,
                ty::LINE_HEIGHT_BASE_200,
                ty::FONT_WEIGHT_SEMIBOLD,
            ),
            Self::Caption1Stronger => (ty::FONT_SIZE_BASE_200, ty::LINE_HEIGHT_BASE_200, ty::FONT_WEIGHT_BOLD),
            Self::Body1 => (
                ty::FONT_SIZE_BASE_300,
                ty::LINE_HEIGHT_BASE_300,
                ty::FONT_WEIGHT_REGULAR,
            ),
            Self::Body1Strong => (
                ty::FONT_SIZE_BASE_300,
                ty::LINE_HEIGHT_BASE_300,
                ty::FONT_WEIGHT_SEMIBOLD,
            ),
            Self::Body1Stronger => (ty::FONT_SIZE_BASE_300, ty::LINE_HEIGHT_BASE_300, ty::FONT_WEIGHT_BOLD),
            Self::Body2 => (
                ty::FONT_SIZE_BASE_400,
                ty::LINE_HEIGHT_BASE_400,
                ty::FONT_WEIGHT_REGULAR,
            ),
            Self::Subtitle2 => (
                ty::FONT_SIZE_BASE_400,
                ty::LINE_HEIGHT_BASE_400,
                ty::FONT_WEIGHT_SEMIBOLD,
            ),
            Self::Subtitle2Stronger => (ty::FONT_SIZE_BASE_400, ty::LINE_HEIGHT_BASE_400, ty::FONT_WEIGHT_BOLD),
            Self::Subtitle1 => (
                ty::FONT_SIZE_BASE_500,
                ty::LINE_HEIGHT_BASE_500,
                ty::FONT_WEIGHT_SEMIBOLD,
            ),
            Self::Title3 => (
                ty::FONT_SIZE_BASE_600,
                ty::LINE_HEIGHT_BASE_600,
                ty::FONT_WEIGHT_SEMIBOLD,
            ),
            Self::Title2 => (
                ty::FONT_SIZE_HERO_700,
                ty::LINE_HEIGHT_HERO_700,
                ty::FONT_WEIGHT_SEMIBOLD,
            ),
            Self::Title1 => (
                ty::FONT_SIZE_HERO_800,
                ty::LINE_HEIGHT_HERO_800,
                ty::FONT_WEIGHT_SEMIBOLD,
            ),
            Self::LargeTitle => (
                ty::FONT_SIZE_HERO_900,
                ty::LINE_HEIGHT_HERO_900,
                ty::FONT_WEIGHT_SEMIBOLD,
            ),
            Self::Display => (
                ty::FONT_SIZE_HERO_1000,
                ty::LINE_HEIGHT_HERO_1000,
                ty::FONT_WEIGHT_SEMIBOLD,
            ),
        };
        TypeSpec {
            size,
            line_height,
            weight,
        }
    }

    /// In points.
    pub const fn size(self) -> f32 {
        self.spec().size
    }

    /// In points.
    pub const fn line_height(self) -> f32 {
        self.spec().line_height
    }

    /// Proportional font at the style size; see [`font`].
    pub fn font(self, ctx: &egui::Context) -> FontId {
        let spec = self.spec();
        font(ctx, spec.size, spec.weight)
    }

    /// Font and line height; glyphs center in the line box.
    pub fn format(self, ctx: &egui::Context, color: Color32) -> TextFormat {
        format(ctx, self.spec(), color)
    }
}

/// Proportional font of `size`; the semibold family for weights >= 600 when installed
/// ([`crate::fonts::install_semibold`]).
pub fn font(ctx: &egui::Context, size: f32, weight: u16) -> FontId {
    let family = if weight >= ty::FONT_WEIGHT_SEMIBOLD && crate::fonts::has_semibold(ctx) {
        FontFamily::Name(crate::fonts::SEMIBOLD.into())
    } else {
        FontFamily::Proportional
    };
    FontId::new(size, family)
}

/// [`TextFormat`] of `spec`: its font with a `line_height` line box.
pub fn format(ctx: &egui::Context, spec: TypeSpec, color: Color32) -> TextFormat {
    TextFormat {
        line_height: Some(spec.line_height),
        ..TextFormat::simple(font(ctx, spec.size, spec.weight), color)
    }
}
