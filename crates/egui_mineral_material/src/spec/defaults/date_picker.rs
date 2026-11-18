//! `DatePickerDefaults` and the private layout values of `DatePicker.kt`, `DateRangePicker.kt`,
//! `DateInput.kt`, `DateRangeInput.kt` and `DatePickerDialog.android.kt`.

use crate::spec::ColorRole;

/// `RecommendedSizeForAccessibility`: weekday and day cell size, calendar row height.
pub const CELL_SIZE: f32 = 48.0;
/// `MaxCalendarRows`.
pub const MAX_CALENDAR_ROWS: usize = 6;
/// `DaysInWeek`.
pub const DAYS_IN_WEEK: usize = 7;
/// `MonthYearHeight`.
pub const MONTH_YEAR_HEIGHT: f32 = 56.0;
/// `DatePickerHorizontalPadding`.
pub const HORIZONTAL_PADDING: f32 = 12.0;
/// `DatePickerTitlePadding` and `DatePickerHeadlinePadding` start.
pub const HEADER_START_PADDING: f32 = 24.0;
/// `DatePickerTitlePadding` and `DatePickerHeadlinePadding` end.
pub const HEADER_END_PADDING: f32 = 12.0;
/// `DatePickerTitlePadding` top.
pub const TITLE_TOP_PADDING: f32 = 16.0;
/// `DatePickerHeadlinePadding` bottom.
pub const HEADLINE_BOTTOM_PADDING: f32 = 12.0;
/// `DatePickerModeTogglePadding` end and bottom.
pub const MODE_TOGGLE_PADDING: f32 = 12.0;
/// `DateRangePickerTitlePadding` and `DateRangePickerHeadlinePadding` start.
pub const RANGE_HEADER_START_PADDING: f32 = 64.0;
/// `DateRangePickerHeadline` `Arrangement.spacedBy`, between the dates and the delimiter.
pub const RANGE_HEADLINE_SPACING: f32 = 4.0;
/// `DateRangePickerDefaults.DateRangePickerHeadline` `datesDelimiter` text.
pub const RANGE_HEADLINE_DELIMITER: &str = "-";
/// `HeaderHeightOffset`, taken off `RangeSelectionHeaderContainerHeight`.
pub const RANGE_HEADER_HEIGHT_OFFSET: f32 = 60.0;
/// `YearsVerticalPadding`, between year rows.
pub const YEARS_VERTICAL_SPACING: f32 = 16.0;
/// `YearsInRow`.
pub const YEARS_IN_ROW: usize = 3;
/// `DatePickerDefaults.YearRange`.
pub const YEAR_RANGE: (i16, i16) = (1900, 2100);
/// `DatePickerDefaults.colors` `navigationContentColor`.
pub const NAVIGATION_CONTENT_COLOR: ColorRole = ColorRole::OnSurfaceVariant;
/// `InputTextFieldPadding` start and end.
pub const INPUT_HORIZONTAL_PADDING: f32 = 24.0;
/// `InputTextFieldPadding` top.
pub const INPUT_TOP_PADDING: f32 = 10.0;
/// `InputTextNonErroneousBottomPadding`.
pub const INPUT_BOTTOM_PADDING: f32 = 16.0;
/// `TextFieldSpacing` of `DateRangeInput`.
pub const RANGE_INPUT_SPACING: f32 = 8.0;
/// `DialogButtonsPadding` bottom.
pub const DIALOG_BUTTONS_BOTTOM_PADDING: f32 = 8.0;
/// `DialogButtonsPadding` end.
pub const DIALOG_BUTTONS_END_PADDING: f32 = 6.0;
/// `Icon` default size (`Icon.kt` `DefaultIconSizeModifier`): the year menu arrow.
pub const YEAR_MENU_ICON_SIZE: f32 = 24.0;
/// `CalendarMonthSubheadPadding` start.
pub const MONTH_SUBHEAD_START_PADDING: f32 = 24.0;
/// `CalendarMonthSubheadPadding` top.
pub const MONTH_SUBHEAD_TOP_PADDING: f32 = 20.0;
/// `CalendarMonthSubheadPadding` bottom.
pub const MONTH_SUBHEAD_BOTTOM_PADDING: f32 = 8.0;
