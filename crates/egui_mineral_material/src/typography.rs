use egui::{Color32, FontFamily, FontId, TextFormat};

use crate::spec::TypeScale;
use crate::spec::tokens::type_scale;

/// Font metrics of one type scale role; size and line height in points, tracking in em.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct TypeSpec {
    /// In points.
    pub size: f32,
    /// In points.
    pub line_height: f32,
    /// CSS weight, 100..=900.
    pub weight: u16,
    /// Letter spacing in em.
    pub tracking: f32,
}

impl TypeScale {
    /// `TypeScaleTokens` values of this role.
    pub const fn spec(self) -> TypeSpec {
        macro_rules! roles {
            ($($variant:ident => $size:ident $line_height:ident $weight:ident $tracking:ident;)*) => {
                match self {
                    $(Self::$variant => TypeSpec {
                        size: type_scale::$size,
                        line_height: type_scale::$line_height,
                        weight: type_scale::$weight,
                        tracking: type_scale::$tracking,
                    },)*
                }
            };
        }
        roles! {
            BodyLarge => BODY_LARGE_SIZE BODY_LARGE_LINE_HEIGHT BODY_LARGE_WEIGHT BODY_LARGE_TRACKING;
            BodyMedium => BODY_MEDIUM_SIZE BODY_MEDIUM_LINE_HEIGHT BODY_MEDIUM_WEIGHT BODY_MEDIUM_TRACKING;
            BodySmall => BODY_SMALL_SIZE BODY_SMALL_LINE_HEIGHT BODY_SMALL_WEIGHT BODY_SMALL_TRACKING;
            DisplayLarge => DISPLAY_LARGE_SIZE DISPLAY_LARGE_LINE_HEIGHT DISPLAY_LARGE_WEIGHT DISPLAY_LARGE_TRACKING;
            DisplayMedium => DISPLAY_MEDIUM_SIZE DISPLAY_MEDIUM_LINE_HEIGHT DISPLAY_MEDIUM_WEIGHT DISPLAY_MEDIUM_TRACKING;
            DisplaySmall => DISPLAY_SMALL_SIZE DISPLAY_SMALL_LINE_HEIGHT DISPLAY_SMALL_WEIGHT DISPLAY_SMALL_TRACKING;
            HeadlineLarge => HEADLINE_LARGE_SIZE HEADLINE_LARGE_LINE_HEIGHT HEADLINE_LARGE_WEIGHT HEADLINE_LARGE_TRACKING;
            HeadlineMedium => HEADLINE_MEDIUM_SIZE HEADLINE_MEDIUM_LINE_HEIGHT HEADLINE_MEDIUM_WEIGHT HEADLINE_MEDIUM_TRACKING;
            HeadlineSmall => HEADLINE_SMALL_SIZE HEADLINE_SMALL_LINE_HEIGHT HEADLINE_SMALL_WEIGHT HEADLINE_SMALL_TRACKING;
            LabelLarge => LABEL_LARGE_SIZE LABEL_LARGE_LINE_HEIGHT LABEL_LARGE_WEIGHT LABEL_LARGE_TRACKING;
            LabelMedium => LABEL_MEDIUM_SIZE LABEL_MEDIUM_LINE_HEIGHT LABEL_MEDIUM_WEIGHT LABEL_MEDIUM_TRACKING;
            LabelSmall => LABEL_SMALL_SIZE LABEL_SMALL_LINE_HEIGHT LABEL_SMALL_WEIGHT LABEL_SMALL_TRACKING;
            TitleLarge => TITLE_LARGE_SIZE TITLE_LARGE_LINE_HEIGHT TITLE_LARGE_WEIGHT TITLE_LARGE_TRACKING;
            TitleMedium => TITLE_MEDIUM_SIZE TITLE_MEDIUM_LINE_HEIGHT TITLE_MEDIUM_WEIGHT TITLE_MEDIUM_TRACKING;
            TitleSmall => TITLE_SMALL_SIZE TITLE_SMALL_LINE_HEIGHT TITLE_SMALL_WEIGHT TITLE_SMALL_TRACKING;
            BodyLargeEmphasized => BODY_LARGE_EMPHASIZED_SIZE BODY_LARGE_EMPHASIZED_LINE_HEIGHT BODY_LARGE_EMPHASIZED_WEIGHT BODY_LARGE_EMPHASIZED_TRACKING;
            BodyMediumEmphasized => BODY_MEDIUM_EMPHASIZED_SIZE BODY_MEDIUM_EMPHASIZED_LINE_HEIGHT BODY_MEDIUM_EMPHASIZED_WEIGHT BODY_MEDIUM_EMPHASIZED_TRACKING;
            BodySmallEmphasized => BODY_SMALL_EMPHASIZED_SIZE BODY_SMALL_EMPHASIZED_LINE_HEIGHT BODY_SMALL_EMPHASIZED_WEIGHT BODY_SMALL_EMPHASIZED_TRACKING;
            DisplayLargeEmphasized => DISPLAY_LARGE_EMPHASIZED_SIZE DISPLAY_LARGE_EMPHASIZED_LINE_HEIGHT DISPLAY_LARGE_EMPHASIZED_WEIGHT DISPLAY_LARGE_EMPHASIZED_TRACKING;
            DisplayMediumEmphasized => DISPLAY_MEDIUM_EMPHASIZED_SIZE DISPLAY_MEDIUM_EMPHASIZED_LINE_HEIGHT DISPLAY_MEDIUM_EMPHASIZED_WEIGHT DISPLAY_MEDIUM_EMPHASIZED_TRACKING;
            DisplaySmallEmphasized => DISPLAY_SMALL_EMPHASIZED_SIZE DISPLAY_SMALL_EMPHASIZED_LINE_HEIGHT DISPLAY_SMALL_EMPHASIZED_WEIGHT DISPLAY_SMALL_EMPHASIZED_TRACKING;
            HeadlineLargeEmphasized => HEADLINE_LARGE_EMPHASIZED_SIZE HEADLINE_LARGE_EMPHASIZED_LINE_HEIGHT HEADLINE_LARGE_EMPHASIZED_WEIGHT HEADLINE_LARGE_EMPHASIZED_TRACKING;
            HeadlineMediumEmphasized => HEADLINE_MEDIUM_EMPHASIZED_SIZE HEADLINE_MEDIUM_EMPHASIZED_LINE_HEIGHT HEADLINE_MEDIUM_EMPHASIZED_WEIGHT HEADLINE_MEDIUM_EMPHASIZED_TRACKING;
            HeadlineSmallEmphasized => HEADLINE_SMALL_EMPHASIZED_SIZE HEADLINE_SMALL_EMPHASIZED_LINE_HEIGHT HEADLINE_SMALL_EMPHASIZED_WEIGHT HEADLINE_SMALL_EMPHASIZED_TRACKING;
            LabelLargeEmphasized => LABEL_LARGE_EMPHASIZED_SIZE LABEL_LARGE_EMPHASIZED_LINE_HEIGHT LABEL_LARGE_EMPHASIZED_WEIGHT LABEL_LARGE_EMPHASIZED_TRACKING;
            LabelMediumEmphasized => LABEL_MEDIUM_EMPHASIZED_SIZE LABEL_MEDIUM_EMPHASIZED_LINE_HEIGHT LABEL_MEDIUM_EMPHASIZED_WEIGHT LABEL_MEDIUM_EMPHASIZED_TRACKING;
            LabelSmallEmphasized => LABEL_SMALL_EMPHASIZED_SIZE LABEL_SMALL_EMPHASIZED_LINE_HEIGHT LABEL_SMALL_EMPHASIZED_WEIGHT LABEL_SMALL_EMPHASIZED_TRACKING;
            TitleLargeEmphasized => TITLE_LARGE_EMPHASIZED_SIZE TITLE_LARGE_EMPHASIZED_LINE_HEIGHT TITLE_LARGE_EMPHASIZED_WEIGHT TITLE_LARGE_EMPHASIZED_TRACKING;
            TitleMediumEmphasized => TITLE_MEDIUM_EMPHASIZED_SIZE TITLE_MEDIUM_EMPHASIZED_LINE_HEIGHT TITLE_MEDIUM_EMPHASIZED_WEIGHT TITLE_MEDIUM_EMPHASIZED_TRACKING;
            TitleSmallEmphasized => TITLE_SMALL_EMPHASIZED_SIZE TITLE_SMALL_EMPHASIZED_LINE_HEIGHT TITLE_SMALL_EMPHASIZED_WEIGHT TITLE_SMALL_EMPHASIZED_TRACKING;
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

    /// Proportional font at the role size; medium family for weights >= 500 when installed.
    pub fn font(self, ctx: &egui::Context) -> FontId {
        let family = if self.spec().weight >= 500 && crate::fonts::has_medium(ctx) {
            FontFamily::Name(crate::fonts::MEDIUM.into())
        } else {
            FontFamily::Proportional
        };
        FontId::new(self.size(), family)
    }

    /// Font, tracking and line height; glyphs center in the line box like Compose
    /// `LineHeightStyle.Alignment.Center`.
    pub fn format(self, ctx: &egui::Context, color: Color32) -> TextFormat {
        let spec = self.spec();
        TextFormat {
            extra_letter_spacing: spec.tracking,
            line_height: Some(spec.line_height),
            ..TextFormat::simple(self.font(ctx), color)
        }
    }
}
