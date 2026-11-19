//! `react-toolbar/library/src/components/{Toolbar/useToolbarStyles.styles.ts,
//! ToolbarDivider/useToolbarDividerStyles.styles.ts}` and `Toolbar/useToolbar.ts`.

/// `useStyles.small` inline `padding`: block, then inline.
pub const PADDING_SMALL: (f32, f32) = (0.0, 4.0);
/// `useStyles.root` and `.medium` `padding` (also the vertical toolbar's): block, then inline.
pub const PADDING_MEDIUM: (f32, f32) = (4.0, 8.0);
/// `useStyles.large` `padding`: block, then inline.
pub const PADDING_LARGE: (f32, f32) = (4.0, 20.0);
/// `useToolbarDividerStyles` `root` inline `padding`.
pub const DIVIDER_PADDING: f32 = 12.0;
/// `useToolbar` tabster `circular`: arrow keys wrap.
pub const WRAP: bool = true;
