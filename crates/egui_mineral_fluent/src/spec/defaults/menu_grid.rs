//! `react-menu-grid-preview/library/src/components/{MenuGrid/useMenuGridStyles.styles.ts,
//! MenuGridRow/useMenuGridRowStyles.styles.ts,MenuGridCell/useMenuGridCellStyles.styles.ts,
//! MenuGridItem/useMenuGridItemStyles.styles.ts}` and the `react-menu` `MenuItem` styles it shares.

/// `MenuGrid` `gap`.
pub const GRID_GAP: f32 = 2.0;
/// `MenuGridRow` `padding`: block (`spacingVerticalXS`) and inline (`spacingVerticalSNudge`).
pub const ROW_PADDING: (f32, f32) = (4.0, 6.0);
/// `MenuGridRow` `minHeight`.
pub const ROW_MIN_HEIGHT: f32 = 32.0;
/// `MenuGridRow` `maxWidth`.
pub const ROW_MAX_WIDTH: f32 = 290.0;
/// `MenuGridRow` `gap`.
pub const ROW_GAP: f32 = 4.0;
/// `MenuGridCell` `minHeight`: matches a small button.
pub const CELL_MIN_HEIGHT: f32 = 24.0;
/// `MenuItem` `useIconBaseStyles` width, height and `fontSize`.
pub const ICON_SIZE: f32 = 20.0;
/// Width of an action cell: the icon plus the small button's inline padding.
pub const ACTION_WIDTH: f32 = 24.0;
/// `MenuItem` `useContentBaseStyles` inline padding.
pub const CONTENT_PADDING: f32 = 2.0;
