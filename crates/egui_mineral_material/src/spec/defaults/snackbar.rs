//! Private snackbar layout values (`Snackbar.kt`, legacy layout: `isSnackbarStylingFixEnabled`
//! defaults to false) and host timing (`SnackbarHost.kt`).

use std::time::Duration;

/// `SnackbarHostState` shows one snackbar at a time.
pub const MAX_VISIBLE: usize = 1;
/// `SnackbarDuration.Short` (`SnackbarDuration.toMillis`).
pub const DURATION_SHORT: Duration = Duration::from_millis(4000);
/// `SnackbarDuration.Long` (`SnackbarDuration.toMillis`).
pub const DURATION_LONG: Duration = Duration::from_millis(10000);

/// `ContainerMaxWidth`, of the content row.
pub const CONTAINER_MAX_WIDTH: f32 = 600.0;
/// `HeightToFirstLine`: first text baseline of a multi-line snackbar.
pub const HEIGHT_TO_FIRST_LINE: f32 = 30.0;
/// `HorizontalSpacing`, start padding.
pub const HORIZONTAL_SPACING: f32 = 16.0;
/// `HorizontalSpacingButtonSide`, end padding without a dismiss action.
pub const HORIZONTAL_SPACING_BUTTON_SIDE: f32 = 8.0;
/// `TextEndExtraSpacing`, after the text without a dismiss action.
pub const TEXT_END_EXTRA_SPACING: f32 = 8.0;
/// `LegacySnackbarVerticalPadding`, around the text.
pub const TEXT_VERTICAL_PADDING: f32 = 6.0;
/// `Snackbar(snackbarData)` outer padding.
pub const OUTER_PADDING: f32 = 12.0;
