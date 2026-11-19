//! `react-nav/library/src/components/{sharedNavStyles.styles.ts,NavSubItem/useNavSubItemStyles.styles.ts,
//! NavSectionHeader/useNavSectionHeaderStyles.styles.ts,NavDivider/useNavDividerStyles.styles.ts,
//! NavDrawer/useNavDrawerStyles.styles.ts,NavDrawerBody/useNavDrawerBodyStyles.styles.ts,
//! NavCategoryItem/useNavCategoryItem.styles.ts}` and `Nav/useNav.ts`.

/// `navItemTokens.defaultDrawerWidth`.
pub const DRAWER_WIDTH: f32 = 260.0;
/// `navItemTokens.indicatorOffset`: the indicator sits this far before the content box.
pub const INDICATOR_OFFSET: f32 = 16.0;
/// `navItemTokens.indicatorWidth`.
pub const INDICATOR_WIDTH: f32 = 4.0;
/// `navItemTokens.indicatorHeight`.
pub const INDICATOR_HEIGHT: f32 = 20.0;
/// `useIconStyles.base` `minWidth` and `minHeight`, and `useExpandIconStyles.base` `height`.
pub const ICON_SIZE: f32 = 20.0;
/// `useNavSubItemSpecificStyles.base` `paddingInlineStart`.
pub const SUB_ITEM_INDENT: f32 = 46.0;
/// `useNavSubItemSpecificStyles.smallBase` `paddingInlineStart`.
pub const SUB_ITEM_INDENT_SMALL: f32 = 40.0;
/// `useNavSectionHeaderStyles` `root` `marginInlineStart`.
pub const SECTION_HEADER_INDENT: f32 = 10.0;
/// `useNavSectionHeaderStyles` `root` `marginBlock`.
pub const SECTION_HEADER_MARGIN: f32 = 8.0;
/// `useNavDividerStyles` `root` `marginTop` and `marginBottom`.
pub const DIVIDER_MARGIN: f32 = 4.0;
/// `useNavDrawerBodyStyles` `root` `padding` inline end (`spacingHorizontalXS`) and start
/// (`spacingHorizontalMNudge`).
pub const BODY_PADDING: (f32, f32) = (4.0, 10.0);
/// `useNav` `multiple` default: several categories stay open together.
pub const MULTIPLE: bool = true;
