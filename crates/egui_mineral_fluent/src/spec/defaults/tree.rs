//! `react-tree/library/src/components/{Tree/useTreeStyles.styles.ts,TreeItem/useTreeItemStyles.styles.ts,
//! TreeItemLayout/useTreeItemLayoutStyles.styles.ts,TreeItemPersonaLayout/useTreeItemPersonaLayoutStyles.styles.ts,
//! TreeItemChevron.tsx}` and `react-checkbox/library/src/components/Checkbox/useCheckboxStyles.styles.ts`.

/// `useRootBaseStyles` `rowGap` and the subtree `paddingTop` (`spacingVerticalXXS`).
pub const ROW_GAP: f32 = 2.0;
/// `useRootStyles` `leaf` and `branch` `paddingLeft` per level (`spacingHorizontalXXL`).
pub const LEVEL_INDENT: f32 = 24.0;
/// `useRootBaseStyles` `minHeight` of `medium`.
pub const MIN_HEIGHT: f32 = 32.0;
/// `useRootStyles.small` `minHeight`.
pub const MIN_HEIGHT_SMALL: f32 = 24.0;
/// `useExpandIconBaseStyles` `minWidth`.
pub const EXPAND_ICON_WIDTH: f32 = 24.0;
/// `ChevronRight12Regular` of `TreeItemChevron`.
pub const EXPAND_ICON_SIZE: f32 = 12.0;
/// `useIconBaseStyles` `fontSize` (`fontSizeBase500`).
pub const ICON_SIZE: f32 = 20.0;
/// Checkbox indicator side and `indicator` margin: the `selector` slot is `indicator + 2 * margin` wide.
pub const SELECTOR_INDICATOR: f32 = 16.0;
/// `useIndicatorStyles` `margin` inline (`spacingHorizontalS`).
pub const SELECTOR_MARGIN: f32 = 8.0;
/// `useMediaBaseStyles` `width` and `height`.
pub const MEDIA_SIZE: f32 = 32.0;
/// `useMediaBaseStyles` `padding` inline start (`spacingHorizontalXXS`) and end (`spacingHorizontalXS`).
pub const MEDIA_PADDING: (f32, f32) = (2.0, 4.0);
/// `useMainBaseStyles` inline `padding` of the layout (`spacingHorizontalXXS`).
pub const MAIN_PADDING: f32 = 2.0;
/// `useIconBeforeStyles.medium` and `useIconAfterStyles.medium` (`spacingHorizontalXS`).
pub const ICON_GAP: f32 = 4.0;
/// `useIconBeforeStyles.small` and `useIconAfterStyles.small` (`spacingHorizontalXXS`).
pub const ICON_GAP_SMALL: f32 = 2.0;
/// `useAsideBaseStyles` inline `padding` (`spacingHorizontalM`).
pub const ASIDE_PADDING: f32 = 12.0;
/// Persona `useMainBaseStyles` `padding`: block (`spacingVerticalMNudge`), inline start (`spacingHorizontalS`) and
/// end (`spacingHorizontalXS`).
pub const PERSONA_MAIN_PADDING: (f32, f32, f32) = (10.0, 8.0, 4.0);
