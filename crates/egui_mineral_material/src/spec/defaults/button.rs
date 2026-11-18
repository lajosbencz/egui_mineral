//! `ButtonDefaults` (`Button.kt`) and `ToggleButtonDefaults` (`ToggleButton.kt`).

use crate::spec::ColorRole;

/// `ButtonDefaults.MinWidth`; toggle buttons have none.
pub const MIN_WIDTH: f32 = 58.0;
/// `ButtonDefaults.IconSize`, for the unsized button.
pub const ICON_SIZE: f32 = 18.0;
/// `ButtonDefaults.ButtonWithIconContentPadding` start.
pub const ICON_SIDE_SPACE: f32 = 16.0;
/// `ButtonDefaults.TextButtonContentPadding` start and end.
pub const TEXT_SPACE: f32 = 12.0;
/// `ButtonDefaults.TextButtonWithIconContentPadding` end.
pub const TEXT_ICON_END_SPACE: f32 = 16.0;
/// `ButtonDefaults.ExtraSmallContentPadding` start and end.
pub const X_SMALL_SPACE: f32 = 12.0;
/// `ButtonDefaults.ExtraSmallIconSpacing`.
pub const X_SMALL_ICON_LABEL_SPACE: f32 = 4.0;
/// `ButtonDefaults.textButtonColors` content.
pub const TEXT_LABEL_COLOR: ColorRole = ColorRole::Primary;
