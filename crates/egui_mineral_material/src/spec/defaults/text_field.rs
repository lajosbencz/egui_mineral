//! `TextFieldDefaults` and `OutlinedTextFieldDefaults` (`TextFieldDefaults.kt`), and the layout
//! constants of `internal/TextFieldImpl.kt`, `TextField.kt` and `OutlinedTextField.kt`.

use crate::spec::ColorRole;

/// `TextFieldDefaults.MinHeight`.
pub const MIN_HEIGHT: f32 = 56.0;
/// `TextFieldDefaults.MinWidth`.
pub const MIN_WIDTH: f32 = 280.0;
/// `TextFieldPadding`: horizontal content padding, and vertical without an inside label.
pub const PADDING: f32 = 16.0;
/// `TextFieldWithLabelVerticalPadding` (`TextField.kt`): above the minimized label, below the input.
pub const LABEL_VERTICAL_PADDING: f32 = 8.0;
/// `SupportingTopPadding`.
pub const SUPPORTING_TOP_PADDING: f32 = 4.0;
/// `PrefixSuffixTextPadding`, between an affix and the input.
pub const AFFIX_PADDING: f32 = 2.0;
/// `MinTextLineHeight`: input, placeholder, affix and expanded label line box.
pub const MIN_TEXT_LINE_HEIGHT: f32 = 24.0;
/// `MinFocusedLabelLineHeight`.
pub const MIN_FOCUSED_LABEL_LINE_HEIGHT: f32 = 16.0;
/// `MinSupportingTextLineHeight`.
pub const MIN_SUPPORTING_LINE_HEIGHT: f32 = 16.0;
/// `OutlinedTextFieldInnerPadding` (`OutlinedTextField.kt`): outline gap around the label.
pub const CUTOUT_PADDING: f32 = 4.0;
/// `TextFieldDefaults.colors` `disabledContainerColor`; the token's disabled container is unused.
pub const FILLED_DISABLED_CONTAINER_COLOR: ColorRole = ColorRole::SurfaceContainerHighest;
/// `TextSelectionBackgroundOpacity` (`MaterialTheme.kt`), over the selection handle color.
pub const SELECTION_OPACITY: f32 = 0.4;
/// `rememberTextSelectionColors` (`MaterialTheme.kt`) handle color.
pub const SELECTION_COLOR: ColorRole = ColorRole::Primary;
