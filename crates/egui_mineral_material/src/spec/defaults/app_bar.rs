//! `TopAppBarDefaults` and the private top app bar layout values (`AppBar.kt`).

use egui_mineral::motion::CubicBezier;

/// `TopAppBarTitleInset` (16dp less `TopAppBarHorizontalPadding`): title start without a navigation icon.
pub const TITLE_INSET: f32 = 12.0;
/// `TopAppBarHorizontalPadding` around the title box.
pub const TITLE_HORIZONTAL_PADDING: f32 = 4.0;
/// `MediumTitleBottomPadding`, from the expanded title's last baseline.
pub const MEDIUM_TITLE_BOTTOM_PADDING: f32 = 24.0;
/// `LargeTitleBottomPadding`, from the expanded title's last baseline.
pub const LARGE_TITLE_BOTTOM_PADDING: f32 = 28.0;
/// `TopTitleAlphaEasing`: collapsed-title alpha over the collapsed fraction.
pub const TOP_TITLE_ALPHA_EASING: CubicBezier = CubicBezier::new(0.8, 0.0, 0.8, 0.15);
