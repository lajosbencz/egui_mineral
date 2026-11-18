//! `LoadingIndicator.kt` private values.

use std::time::Duration;

/// `GlobalRotationDurationMillis`, one full turn.
pub const GLOBAL_ROTATION: Duration = Duration::from_millis(4666);
/// `MorphIntervalMillis`, between shape morphs.
pub const MORPH_INTERVAL: Duration = Duration::from_millis(650);
/// Morph `spring(dampingRatio)`.
pub const MORPH_DAMPING: f32 = 0.6;
/// Morph `spring(stiffness)`.
pub const MORPH_STIFFNESS: f32 = 200.0;
/// Rotation added per morph (`QuarterRotation`), in degrees.
pub const MORPH_ROTATION: f32 = 90.0;
