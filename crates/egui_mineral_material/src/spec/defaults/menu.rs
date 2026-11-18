//! `MenuDefaults` (`MenuDefaults.kt`) and the private dropdown menu values (`Menu.kt`).

use crate::spec::TypeScale;

/// `DropdownMenuVerticalPadding`, above and below the baseline menu items.
pub const VERTICAL_PADDING: f32 = 8.0;
/// `MenuListItemContainerHeight`.
pub const ITEM_HEIGHT: f32 = 48.0;
/// `DropdownMenuItemHorizontalPadding`: item content padding and icon-to-text space.
pub const ITEM_HORIZONTAL_PADDING: f32 = 12.0;
/// `DropdownMenuItemContent` text style, also used by group labels.
pub const ITEM_FONT: TypeScale = TypeScale::LabelLarge;
/// `MenuVerticalMargin`: kept free between the menu and the window edge.
pub const VERTICAL_MARGIN: f32 = 48.0;
/// `DropdownMenuItemDefaultMinWidth`.
pub const ITEM_MIN_WIDTH: f32 = 112.0;
/// `DropdownMenuItemDefaultMaxWidth`.
pub const ITEM_MAX_WIDTH: f32 = 280.0;
/// `DropdownMenuSelectableItemPadding` horizontal: inset of a selectable item in its group.
pub const SELECTABLE_ITEM_INSET: f32 = 4.0;
/// `MenuDefaults.itemHorizontalArrangement` spacing between icons and text.
pub const ICON_TEXT_SPACING: f32 = 8.0;
/// `DropdownMenuGroupVerticalPadding`.
pub const GROUP_VERTICAL_PADDING: f32 = 2.0;
/// `DropdownMenuGroupDefaultMinHeight`, the group label height.
pub const GROUP_LABEL_HEIGHT: f32 = 32.0;
/// `MenuDefaults.DropdownMenuGroupLabelHorizontalPadding` start.
pub const GROUP_LABEL_START_PADDING: f32 = 12.0;
/// `MenuDefaults.HorizontalDividerPadding` horizontal.
pub const DIVIDER_HORIZONTAL_PADDING: f32 = 12.0;
/// `MenuDefaults.HorizontalDividerPadding` vertical.
pub const DIVIDER_VERTICAL_PADDING: f32 = 2.0;
