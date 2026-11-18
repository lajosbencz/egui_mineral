//! `TimePickerDefaults`, `TimeInputDefaults` and the private layout values of `TimePicker.kt`
//! (with `ComposeMaterial3Flags.isUpdatedTimepickerToggleEnabled`, its default) and
//! `TimePickerDialog.kt`.

use crate::spec::{ColorRole, TypeScale};

/// `OuterCircleToSizeRatio` times `ClockDialContainerSize`.
pub const OUTER_RADIUS: f32 = 101.0;
/// `InnerCircleToSizeRatio` times `ClockDialContainerSize`.
pub const INNER_RADIUS: f32 = 69.0;
/// `MaxDistance`: 24-hour taps closer to the center pick the inner ring.
pub const INNER_RING_MAX_DISTANCE: f32 = 74.0;
/// `ClockDisplayBottomMargin`: clock display to dial, vertical and horizontal layouts.
pub const CLOCK_DISPLAY_MARGIN: f32 = 36.0;
/// `ClockFaceBottomMargin`.
pub const CLOCK_FACE_BOTTOM_MARGIN: f32 = 24.0;
/// `DisplaySeparatorWidth`.
pub const SEPARATOR_WIDTH: f32 = 24.0;
/// `DisplaySeparator` text `offset(y = -4.dp)`.
pub const SEPARATOR_OFFSET_Y: f32 = -4.0;
/// `PeriodTogglePaddingSmall`: period toggle start padding and the gap between its items.
pub const PERIOD_TOGGLE_GAP: f32 = 4.0;
/// `VibrantPeriodToggleLargePadding`, above the horizontal period toggle.
pub const PERIOD_TOGGLE_TOP_PADDING: f32 = 16.0;
/// `ToggleItem` `contentPadding` horizontal.
pub const PERIOD_ITEM_HORIZONTAL_PADDING: f32 = 4.0;
/// `ToggleItem` `pressedShape` and `checkedShape` corner radius.
pub const PERIOD_ITEM_CHECKED_RADIUS: f32 = 12.0;
/// `defaultTimePickerColors` `periodSelectorContainerColor`.
pub const PERIOD_CONTAINER_COLOR: ColorRole = ColorRole::SurfaceContainerLowest;
/// `defaultTimePickerColors` `periodSelectorSelectedContainerColor`.
pub const PERIOD_SELECTED_CONTAINER_COLOR: ColorRole = ColorRole::PrimaryContainer;
/// `defaultTimePickerColors` `periodSelectorSelectedContentColor`.
pub const PERIOD_SELECTED_CONTENT_COLOR: ColorRole = ColorRole::OnPrimaryContainer;
/// `TimePickerTextField` `unfocusedBorderThickness`.
pub const FIELD_OUTLINE_WIDTH: f32 = 1.0;
/// `TimePickerTextField` `focusedBorderThickness`.
pub const FIELD_FOCUS_OUTLINE_WIDTH: f32 = 2.0;
/// `defaultTimeInputColors` `timeTextFieldColors` focused and unfocused border.
pub const FIELD_OUTLINE_COLOR: ColorRole = ColorRole::Outline;
/// `defaultTimeInputColors` `timeTextFieldColors` error container.
pub const FIELD_ERROR_CONTAINER_COLOR: ColorRole = ColorRole::ErrorContainer;
/// `defaultTimeInputColors` `timeTextFieldColors` error border and text.
pub const FIELD_ERROR_COLOR: ColorRole = ColorRole::Error;
/// `TimePickerTextField` `cursorBrush` color.
pub const FIELD_CURSOR_COLOR: ColorRole = ColorRole::Primary;
/// `SupportLabelTop`.
pub const SUPPORT_TOP_PADDING: f32 = 7.0;
/// `SupportingText` `minLines`.
pub const SUPPORT_MIN_LINES: usize = 2;
/// `TimePickerCustomLayout` `contentPadding` and portrait `portTitleTopPadding`.
pub const DIALOG_PADDING: f32 = 24.0;
/// `TimePickerDialogDefaults.Title` style: `typography.labelMedium`.
pub const DIALOG_TITLE_FONT: TypeScale = TypeScale::LabelMedium;
/// `TimePickerDialogDefaults.Title` color: the dialog `Surface` content color,
/// `contentColorFor(DialogTokens.ContainerColor)`.
pub const DIALOG_TITLE_COLOR: ColorRole = ColorRole::OnSurface;
/// `TimePickerDialogDefaults.Title` bottom padding.
pub const DIALOG_TITLE_BOTTOM_PADDING: f32 = 20.0;
/// `TimePickerDialogLayout` actions `Arrangement.spacedBy`.
pub const DIALOG_ACTIONS_SPACING: f32 = 8.0;
/// `TimePickerCustomLayout` `landContentTopPadding`.
pub const DIALOG_LANDSCAPE_CONTENT_TOP_PADDING: f32 = 16.0;
/// `TimePickerCustomLayout` `landContentActionsPadding`.
pub const DIALOG_LANDSCAPE_CONTENT_ACTIONS_PADDING: f32 = 4.0;
/// `TimePickerCustomLayout` `landActionsBottomPadding`.
pub const DIALOG_LANDSCAPE_ACTIONS_BOTTOM_PADDING: f32 = 8.0;
