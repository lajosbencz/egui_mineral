//! `ModalNavigationDrawer` (`NavigationDrawer.kt`) and `ModalWideNavigationRail`
//! (`WideNavigationRail.kt`) show and hide motion.

use std::time::Duration;

/// `AnchoredDraggableDefaultAnimationSpec` (`TweenSpec(256)`): the drawer settle after a drag.
pub const DRAWER_SETTLE_DURATION: Duration = Duration::from_millis(256);
/// `DrawerPositionalThreshold`, of the drawer width.
pub const DRAWER_POSITIONAL_THRESHOLD: f32 = 0.5;
/// `DrawerVelocityThreshold`, points/s.
pub const DRAWER_VELOCITY_THRESHOLD: f32 = 400.0;
/// `ModalWideNavigationRail` `modalExpanded`: position progress from which the rail expands.
pub const RAIL_EXPAND_PROGRESS: f32 = 0.3;
