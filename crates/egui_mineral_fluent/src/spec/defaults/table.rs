//! `react-table/library/src/components/{Table,TableRow,TableCell,TableHeaderCell,TableSelectionCell,TableCellLayout,
//! TableResizeHandle,DataGridRow,DataGridCell}/use*Styles.styles.ts`; the arrays run `medium`, `small`,
//! `extra-small`.

/// `useTableCellStyles` table-layout `height` per size.
pub const ROW_HEIGHTS: [f32; 3] = [44.0, 34.0, 24.0];
/// `useTableCellStyles.root` and `useTableHeaderCellStyles.root` inline `padding` (`spacingHorizontalS`).
pub const CELL_PADDING: f32 = 8.0;
/// `useTableSelectionCellStyles` `CELL_WIDTH`.
pub const SELECTION_WIDTH: f32 = 44.0;
/// `useTableHeaderCellStyles.button` `gap` (`spacingHorizontalXS`).
pub const HEADER_GAP: f32 = 4.0;
/// `useTableHeaderCell` `sortIcons`: `ArrowUpRegular` and `ArrowDownRegular` at `fontSize={12}`.
pub const SORT_ICON: f32 = 12.0;
/// `useTableHeaderCellStyles.sortIcon` `paddingTop` (`spacingVerticalXXS`).
pub const SORT_ICON_PADDING_TOP: f32 = 2.0;
/// `useTableRowStyles` row `borderBottom` width (`strokeWidthThin`).
pub const ROW_BORDER: f32 = 1.0;
/// `useTableRowStyles` focus `outline` width.
pub const FOCUS_OUTLINE: f32 = 2.0;
/// `useTableResizeHandleStyles` `width`: the grab area, centered on the column edge (`margin: 0 -8px`).
pub const RESIZE_HANDLE: f32 = 16.0;
/// `useTableResizeHandleStyles` visible line width.
pub const RESIZE_LINE: f32 = 1.0;
/// `useTableCellLayoutStyles` `gap` (`spacingHorizontalS`).
pub const LAYOUT_GAP: f32 = 8.0;
/// `useTableCellLayoutStyles` media `fontSize`: `medium` and `small` 20, `extra-small` 16, `primary` 24.
pub const MEDIA: [f32; 3] = [20.0, 20.0, 16.0];
/// `useTableCellLayoutStyles.mediaPrimary` `fontSize`.
pub const MEDIA_PRIMARY: f32 = 24.0;
