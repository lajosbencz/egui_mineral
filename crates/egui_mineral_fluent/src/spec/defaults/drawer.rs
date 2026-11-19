//! `react-drawer/library/src/components/{Drawer,InlineDrawer,OverlayDrawer,DrawerHeader,DrawerHeaderTitle,DrawerBody,DrawerFooter}/use*Styles.styles.ts`
//! `shared/drawerDefaultStyles` and `shared/drawerMotions.ts`.

use std::time::Duration;

/// `drawerSizes.small`.
pub const SIZE_SMALL: f32 = 320.0;
/// `drawerSizes.medium`.
pub const SIZE_MEDIUM: f32 = 592.0;
/// `drawerSizes.large`.
pub const SIZE_LARGE: f32 = 940.0;
/// `DrawerHeader`, `DrawerBody` and `DrawerFooter` `padding` start and end (`spacingHorizontalXXL`).
pub const PADDING_X: f32 = 24.0;
/// `DrawerHeader` `padding` top (`spacingVerticalXXL`).
pub const HEADER_PADDING_TOP: f32 = 24.0;
/// `DrawerHeader` `padding` bottom (`spacingVerticalS`).
pub const HEADER_PADDING_BOTTOM: f32 = 8.0;
/// `DrawerFooter` `padding` top (`spacingVerticalL`).
pub const FOOTER_PADDING_TOP: f32 = 16.0;
/// `DrawerFooter` `padding` bottom (`spacingVerticalXXL`).
pub const FOOTER_PADDING_BOTTOM: f32 = 24.0;
/// `DrawerFooter` `columnGap` (`spacingHorizontalS`).
pub const FOOTER_GAP: f32 = 8.0;
/// `DrawerBody` `:first-child` `paddingTop` and `:last-child` `paddingBottom`.
pub const BODY_EDGE_PADDING: f32 = 25.0;
/// `InlineDrawer` separator border width.
pub const SEPARATOR_WIDTH: f32 = 1.0;
/// `drawerMotions.ts` `durations.small` (`durationGentle`).
pub const MOTION_SMALL: Duration = Duration::from_millis(250);
/// `drawerMotions.ts` `durations.medium` (`durationSlow`).
pub const MOTION_MEDIUM: Duration = Duration::from_millis(300);
/// `drawerMotions.ts` `durations.large` (`durationSlower`).
pub const MOTION_LARGE: Duration = Duration::from_millis(400);
/// `drawerMotions.ts` `durations.full` (`durationUltraSlow`).
pub const MOTION_FULL: Duration = Duration::from_millis(500);
