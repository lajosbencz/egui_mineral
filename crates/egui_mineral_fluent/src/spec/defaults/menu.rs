//! `react-menu/library/src/components/{MenuPopover,MenuList,MenuItem,MenuDivider,MenuGroupHeader,MenuSplitGroup}/use*Styles.styles.ts`,
//! `Menu/useMenu.tsx`, `MenuList/useMenuList.ts`, `MenuPopover/useMenuPopover.ts` and
//! `MenuItemCheckbox/useMenuItemCheckbox.tsx`.

use std::time::Duration;

/// `MenuPopover` `minWidth`.
pub const POPOVER_MIN_WIDTH: f32 = 138.0;
/// `MenuPopover` `maxWidth`.
pub const POPOVER_MAX_WIDTH: f32 = 300.0;
/// `MenuPopover` `padding`.
pub const POPOVER_PADDING: f32 = 4.0;
/// `MenuList` `gap`.
pub const LIST_GAP: f32 = 2.0;
/// `MenuItem` `maxWidth`.
pub const ITEM_MAX_WIDTH: f32 = 290.0;
/// `MenuItem` `minHeight`.
pub const ITEM_MIN_HEIGHT: f32 = 32.0;
/// `MenuItem` `padding` (`spacingVerticalSNudge`).
pub const ITEM_PADDING: f32 = 6.0;
/// `MenuItem` `gap`.
pub const ITEM_GAP: f32 = 4.0;
/// `useIconBaseStyles` and `useSubmenuIndicatorBaseStyles` width, height and `fontSize`.
pub const ICON_SIZE: f32 = 20.0;
/// `Checkmark16Filled`, the default `checkmark` of `MenuItemCheckbox` and `MenuItemRadio`.
pub const CHECKMARK_SIZE: f32 = 16.0;
/// `useContentBaseStyles` and `useSecondaryContentBaseStyles` inline padding.
pub const CONTENT_PADDING: f32 = 2.0;
/// `MenuDivider` block margin.
pub const DIVIDER_MARGIN: f32 = 4.0;
/// `MenuDivider` negative inline margin: the line spans the popover padding.
pub const DIVIDER_BLEED: f32 = 5.0;
/// `MenuGroupHeader` `height`.
pub const GROUP_HEADER_HEIGHT: f32 = 32.0;
/// `MenuGroupHeader` inline padding.
pub const GROUP_HEADER_PADDING: f32 = 8.0;
/// `useMenu` `hoverDelay` default.
pub const HOVER_DELAY: Duration = Duration::from_millis(500);
/// `useMenuList` `useArrowNavigationGroup` `circular`.
pub const WRAP: bool = true;
/// `MenuSplitGroup` trigger width: `MenuItem` `splitItemTrigger` `::before` separator (`strokeWidthThin`), `gap`,
/// the submenu indicator and the inline end padding.
pub const SPLIT_TRIGGER_WIDTH: f32 = 1.0 + ITEM_GAP + ICON_SIZE + ITEM_PADDING;
/// `useMenuSplitGroupStyles` `::before` `height` of the trigger separator.
pub const SPLIT_SEPARATOR_HEIGHT: f32 = 20.0;
