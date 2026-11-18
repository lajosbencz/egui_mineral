//! `ProgressIndicator.kt` and `WavyProgressIndicator.kt` private values.

use std::time::Duration;

/// `LinearIndicatorWidth` (`WavyProgressIndicatorDefaults.LinearContainerWidth`).
pub const LINEAR_WIDTH: f32 = 240.0;
/// `StopIndicatorTrailingSpace`, the most the stop sits in from the end.
pub const STOP_MAX_TRAILING_SPACE: f32 = 6.0;
/// `LinearAnimationDuration`, one indeterminate cycle.
pub const LINEAR_CYCLE: Duration = Duration::from_millis(1750);
/// `FirstLineHeadDelay`, `FirstLineHeadDuration`.
pub const FIRST_LINE_HEAD: [Duration; 2] = [Duration::ZERO, Duration::from_millis(1000)];
/// `FirstLineTailDelay`, `FirstLineTailDuration`.
pub const FIRST_LINE_TAIL: [Duration; 2] = [Duration::from_millis(250), Duration::from_millis(1000)];
/// `SecondLineHeadDelay`, `SecondLineHeadDuration`.
pub const SECOND_LINE_HEAD: [Duration; 2] = [Duration::from_millis(650), Duration::from_millis(850)];
/// `SecondLineTailDelay`, `SecondLineTailDuration`.
pub const SECOND_LINE_TAIL: [Duration; 2] = [Duration::from_millis(900), Duration::from_millis(850)];
/// `CircularAnimationProgressDuration`, one indeterminate cycle.
pub const CIRCULAR_CYCLE: Duration = Duration::from_millis(6000);
/// `CircularAnimationAdditionalRotationDelay`, between quarter turns.
pub const ROTATION_STEP_DELAY: Duration = Duration::from_millis(1500);
/// `CircularAnimationAdditionalRotationDuration`, of one quarter turn.
pub const ROTATION_STEP_DURATION: Duration = Duration::from_millis(300);
/// `CircularGlobalRotationDegreesTarget`, per cycle.
pub const GLOBAL_ROTATION_DEGREES: f32 = 1080.0;
/// `CircularIndeterminateMinProgress`.
pub const INDETERMINATE_MIN_PROGRESS: f32 = 0.1;
/// `CircularIndeterminateMaxProgress`.
pub const INDETERMINATE_MAX_PROGRESS: f32 = 0.87;
/// `WavyProgressIndicatorDefaults.indicatorAmplitude`: flat at or below this progress.
pub const WAVE_START_PROGRESS: f32 = 0.1;
/// `WavyProgressIndicatorDefaults.indicatorAmplitude`: flat at or above this progress.
pub const WAVE_END_PROGRESS: f32 = 0.95;
