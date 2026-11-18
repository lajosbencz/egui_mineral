//! `FloatingToolbarDefaults` (`FloatingToolbar.kt`).

use crate::spec::ElevationLevel;

/// `FloatingToolbarDefaults.ContainerExpandedElevation` and `ContainerCollapsedElevation`.
pub const FLOATING_ELEVATION: ElevationLevel = ElevationLevel::Level0;
/// `FloatingToolbarDefaults.ContainerExpandedElevationWithFab`.
pub const FLOATING_EXPANDED_ELEVATION_WITH_FAB: ElevationLevel = ElevationLevel::Level1;
/// `FloatingToolbarDefaults.ContainerCollapsedElevationWithFab`.
pub const FLOATING_COLLAPSED_ELEVATION_WITH_FAB: ElevationLevel = ElevationLevel::Level0;
/// `FloatingToolbarDefaults.ToolbarToFabGap`.
pub const TOOLBAR_TO_FAB_GAP: f32 = 8.0;
/// `FloatingToolbarDefaults.StandardFloatingActionButton` elevation `defaultElevation`,
/// `pressedElevation` and `focusedElevation`.
pub const FAB_ELEVATION: ElevationLevel = ElevationLevel::Level2;
/// `FloatingToolbarDefaults.StandardFloatingActionButton` elevation `hoveredElevation`.
pub const FAB_HOVERED_ELEVATION: ElevationLevel = ElevationLevel::Level3;
