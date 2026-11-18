//! `FloatingActionButtonMenu`, `FloatingActionButtonMenuItem` and `ToggleFloatingActionButton`
//! private values (`FloatingActionButtonMenu.kt`).

use crate::spec::TypeScale;

/// `FabMenuPaddingHorizontal`, beside the menu.
pub const PADDING_HORIZONTAL: f32 = 16.0;
/// `FabMenuButtonPaddingBottom`, below the toggle button.
pub const BUTTON_PADDING_BOTTOM: f32 = 16.0;
/// `FloatingActionButtonMenuItem` label style (`MaterialTheme.typography.titleMedium`).
pub const ITEM_LABEL_FONT: TypeScale = TypeScale::TitleMedium;
/// `FloatingActionButtonMenuItemColumn` stagger: an item count springs by at least one before resting.
pub const STAGGER_VISIBILITY_THRESHOLD: f32 = 1.0;
