//! `PullToRefreshDefaults` and the private indicator values (`pulltorefresh/PullToRefresh.kt`).

use crate::spec::{ColorRole, ElevationLevel, ShapeRole};

/// `PullToRefreshDefaults.PositionalThreshold`.
pub const POSITIONAL_THRESHOLD: f32 = 80.0;
/// `PullToRefreshDefaults.IndicatorMaxDistance`.
pub const INDICATOR_MAX_DISTANCE: f32 = POSITIONAL_THRESHOLD;
/// `PullToRefreshDefaults.Elevation` (`ElevationTokens.Level2`), as a level.
pub const ELEVATION: ElevationLevel = ElevationLevel::Level2;
/// `PullToRefreshDefaults.indicatorShape` (`CircleShape`).
pub const INDICATOR_SHAPE: ShapeRole = ShapeRole::CornerFull;
/// `PullToRefreshDefaults.indicatorContainerColor`.
pub const INDICATOR_CONTAINER_COLOR: ColorRole = ColorRole::SurfaceContainerHigh;
/// `PullToRefreshDefaults.indicatorColor`.
pub const INDICATOR_COLOR: ColorRole = ColorRole::OnSurfaceVariant;
/// `SpinnerContainerSize`.
pub const CONTAINER_SIZE: f32 = 40.0;
/// `SpinnerSize`.
pub const SPINNER_SIZE: f32 = 16.0;
/// `StrokeWidth`.
pub const STROKE_WIDTH: f32 = 2.5;
/// `ArcRadius`.
pub const ARC_RADIUS: f32 = 5.5;
/// `ArrowWidth`.
pub const ARROW_WIDTH: f32 = 10.0;
/// `ArrowHeight`.
pub const ARROW_HEIGHT: f32 = 5.0;
/// `MaxProgressArc`, in turns.
pub const MAX_PROGRESS_ARC: f32 = 0.8;
/// `MinAlpha`, of the arrow below the threshold.
pub const MIN_ALPHA: f32 = 0.3;
/// `MaxAlpha`.
pub const MAX_ALPHA: f32 = 1.0;
/// `DragMultiplier`.
pub const DRAG_MULTIPLIER: f32 = 0.5;
/// `PullToRefreshStateImpl` animations: `Animatable.animateTo`'s default `spring()` damping
/// (`Spring.DampingRatioNoBouncy`).
pub const ANIMATION_DAMPING: f32 = 1.0;
/// `spring()` stiffness (`Spring.StiffnessMedium`).
pub const ANIMATION_STIFFNESS: f32 = 1500.0;
