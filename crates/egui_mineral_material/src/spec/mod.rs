//! Material 3 component specs as data.
//!
//! - [`tokens`] (feature `unstable-spec`): one module per Jetpack Compose Material3 token object (`androidx.compose.material3.tokens`,
//!   generated from Google's design token database), e.g. [`tokens::outlined_text_field::FOCUS_OUTLINE_WIDTH`] or
//!   [`tokens::button_small::LEADING_SPACE`].
//! - [`defaults`] (feature `unstable-spec`): values Compose hard-codes in its component `*Defaults` instead of reading a token object.
//! - Role enums resolved from the key token objects: [`ColorRole`], [`ShapeRole`] (with [`ShapeScale`] and
//!   [`Corners`]), [`Typeface`], [`TypeScale`], [`ElevationLevel`].
//!
//! Units and types: dp and sp as `f32` points, opacities as `f32`, durations as [`std::time::Duration`],
//! colors as [`ColorRole`], shapes as [`ShapeRole`] or [`ShapeScale`], type as [`TypeScale`], elevation as
//! [`ElevationLevel`], easings as [`CubicBezier`](egui_mineral::motion::CubicBezier).
//!
//! [`tokens`] comes from `tools/md3_tokens` at androidx commit [`tokens::SOURCE_COMMIT`]; do not edit it.
//! Regenerate with `tools/md3_tokens/fetch.sh [sha]` (vendors the sources) and
//! `cargo run -p md3_tokens -- generate` (offline, deterministic). The token sources are Apache-2.0
//! (The Android Open Source Project); see `NOTICE` and `LICENSE-APACHE`.
#![cfg_attr(
    not(feature = "unstable-spec"),
    doc = "",
    doc = "[`tokens`]: https://docs.rs/egui_mineral_material/latest/egui_mineral_material/spec/tokens/index.html",
    doc = "[`defaults`]: https://docs.rs/egui_mineral_material/latest/egui_mineral_material/spec/defaults/index.html",
    doc = "[`tokens::SOURCE_COMMIT`]: https://docs.rs/egui_mineral_material/latest/egui_mineral_material/spec/tokens/constant.SOURCE_COMMIT.html",
    doc = "[`tokens::outlined_text_field::FOCUS_OUTLINE_WIDTH`]: https://docs.rs/egui_mineral_material/latest/egui_mineral_material/spec/tokens/outlined_text_field/constant.FOCUS_OUTLINE_WIDTH.html",
    doc = "[`tokens::button_small::LEADING_SPACE`]: https://docs.rs/egui_mineral_material/latest/egui_mineral_material/spec/tokens/button_small/constant.LEADING_SPACE.html"
)]

#[cfg(feature = "unstable-spec")]
pub mod defaults;
#[cfg(not(feature = "unstable-spec"))]
#[expect(
    dead_code,
    unreachable_pub,
    reason = "whole module tree; public with the `unstable-spec` feature"
)]
pub(crate) mod defaults;
#[cfg(feature = "unstable-spec")]
pub mod tokens;
#[cfg(not(feature = "unstable-spec"))]
#[expect(
    dead_code,
    unreachable_pub,
    reason = "whole module tree; public with the `unstable-spec` feature"
)]
pub(crate) mod tokens;

use egui::CornerRadius;
use egui_mineral::i18n::Direction;
pub use tokens::roles::{ColorRole, ElevationLevel, ShapeRole, ShapeScale, TypeScale, Typeface};

use crate::tokens::Shape;

/// Corners a [`ShapeRole`] rounds; `Start` and `End` follow the layout direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Corners {
    /// Every corner.
    All,
    /// Top-left and top-right.
    Top,
    /// Bottom-left and bottom-right.
    Bottom,
    /// Leading edge corners.
    Start,
    /// Trailing edge corners.
    End,
}

impl ShapeRole {
    /// Radius from `shape` on the corners of [`Self::corners`], mirrored for `direction`.
    pub const fn corner_radius(self, shape: &Shape, direction: Direction) -> CornerRadius {
        let r = self.scale().radius(shape);
        let (left, right) = match (self.corners(), direction) {
            (Corners::All, _) => return CornerRadius::same(r),
            (Corners::Top, _) => {
                return CornerRadius {
                    nw: r,
                    ne: r,
                    sw: 0,
                    se: 0,
                };
            }
            (Corners::Bottom, _) => {
                return CornerRadius {
                    nw: 0,
                    ne: 0,
                    sw: r,
                    se: r,
                };
            }
            (Corners::Start, Direction::Ltr) | (Corners::End, Direction::Rtl) => (r, 0),
            (Corners::Start | Corners::End, _) => (0, r),
        };
        CornerRadius {
            nw: left,
            ne: right,
            sw: left,
            se: right,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_end_follow_direction() {
        let shape = Shape::default();
        let ltr = ShapeRole::CornerLargeStart.corner_radius(&shape, Direction::Ltr);
        let r = shape.large;
        assert_eq!((ltr.nw, ltr.ne, ltr.sw, ltr.se), (r, 0, r, 0));
        assert_eq!(
            ShapeRole::CornerLargeEnd.corner_radius(&shape, Direction::Rtl),
            ltr,
            "End in RTL is Start in LTR"
        );
    }
}
