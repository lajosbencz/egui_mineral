//! `react-progress/library/src/components/ProgressBar/useProgressBarStyles.styles.ts`.

use std::time::Duration;

use egui_mineral::motion::CubicBezier;

/// `barThicknessValues.medium`.
pub const THICKNESS_MEDIUM: f32 = 2.0;
/// `barThicknessValues.large`.
pub const THICKNESS_LARGE: f32 = 4.0;
/// `ZERO_THRESHOLD`: determinate values up to it snap instead of animating.
pub const ZERO_THRESHOLD: f32 = 0.01;
/// `nonZeroDeterminate` `transitionDuration`.
pub const FILL_DURATION: Duration = Duration::from_millis(300);
/// `transitionTimingFunction: ease`.
pub const FILL_EASING: CubicBezier = CubicBezier::new(0.25, 0.1, 0.25, 1.0);
/// `indeterminate` `maxWidth`, as a fraction of the track.
pub const INDETERMINATE_WIDTH: f32 = 0.33;
/// `ProgressBarIndeterminateMotion` sweep, `left: -33%` to `100%`; not in the vendored styles file.
pub const INDETERMINATE_CYCLE: Duration = Duration::from_millis(3000);
