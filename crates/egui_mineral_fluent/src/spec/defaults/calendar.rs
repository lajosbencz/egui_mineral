//! `react-calendar-compat/library/src/components/{Calendar,CalendarDay,CalendarDayGrid,CalendarPicker}/use*Styles.styles.ts`.

/// `useRootStyles.base` `width` of `Calendar`.
pub const WIDTH: f32 = 220.0;
/// `useRootStyles.dayPickerVisibleAndWeekNumbersShown` `width`.
pub const WIDTH_WEEK_NUMBERS: f32 = 250.0;
/// `CalendarDay` and `CalendarPicker` root `padding`.
pub const PADDING: f32 = 12.0;
/// `useHeaderStyles` `height`.
pub const HEADER_HEIGHT: f32 = 28.0;
/// `useMonthAndYearStyles` `padding` start.
pub const HEADER_TEXT_PAD_START: f32 = 10.0;
/// `useHeaderIconButtonStyles` `height` and `width`.
pub const NAV_BUTTON: f32 = 28.0;
/// `useTableStyles` `marginTop`.
pub const GRID_MARGIN_TOP: f32 = 4.0;
/// `useWrapperStyles` `paddingBottom`.
pub const GRID_PAD_BOTTOM: f32 = 10.0;
/// `useDayButtonStyles` `height` and `width`.
pub const DAY_BUTTON: f32 = 24.0;
/// Day cell: `useDayButtonStyles` size plus `useDayCellStyles` `padding: 2px`.
pub const DAY_CELL: f32 = 28.0;
/// `useDayTodayMarkerStyles` `width` and `height`.
pub const TODAY_MARKER: f32 = 20.0;
/// `useWeekNumberCellStyles` `width`.
pub const WEEK_NUMBER_WIDTH: f32 = 28.0;
/// `useCalendarDayGridStyles` weeks shown.
pub const WEEKS: usize = 6;
/// `useItemButtonStyles` `height` and `width` of a month or year.
pub const ITEM_BUTTON: f32 = 40.0;
/// `useItemButtonStyles` `margin-right`.
pub const ITEM_GAP: f32 = 12.0;
/// `useButtonRowStyles` `marginBottom`.
pub const ITEM_ROW_GAP: f32 = 16.0;
/// `useGridContainerStyles` `marginTop`.
pub const ITEM_GRID_MARGIN_TOP: f32 = 4.0;
/// `CalendarPicker` items per row.
pub const ITEM_COLUMNS: usize = 4;
/// `CalendarMonth` rows of months.
pub const MONTH_ROWS: usize = 3;
/// `CalendarYear` years shown at once (`ITEM_COLUMNS` times `MONTH_ROWS`).
pub const YEARS: i16 = 12;
/// `useGoTodayButtonStyles` `height`.
pub const GO_TODAY_HEIGHT: f32 = 30.0;
/// `useGoTodayButtonStyles` `marginTop`.
pub const GO_TODAY_MARGIN_TOP: f32 = 3.0;
/// `useGoTodayButtonStyles` `marginRight`.
pub const GO_TODAY_MARGIN_END: f32 = 16.0;
/// `useGoTodayButtonStyles` `padding` horizontal.
pub const GO_TODAY_PAD: f32 = 4.0;
/// `useDayCellStyles.focusIndicator` `outlineWidth`.
pub const FOCUS_RING: f32 = 2.0;
