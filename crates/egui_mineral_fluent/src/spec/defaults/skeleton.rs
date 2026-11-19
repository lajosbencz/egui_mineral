//! `react-skeleton/library/src/components/SkeletonItem/useSkeletonItemStyles.styles.ts`.

use std::time::Duration;

use egui_mineral::motion::CubicBezier;

/// `animationDuration` of the `wave` sweep.
pub const WAVE_CYCLE: Duration = Duration::from_millis(3000);
/// `animationTimingFunction: ease-in-out` of the `wave` sweep.
pub const WAVE_EASING: CubicBezier = CubicBezier::new(0.42, 0.0, 0.58, 1.0);
/// `animationDuration` of the `pulse`.
pub const PULSE_CYCLE: Duration = Duration::from_millis(1000);
/// `skeletonPulseAnimation` opacity at 50%.
pub const PULSE_MIN_OPACITY: f32 = 0.4;
/// `useRectangleStyles.root` `borderRadius`.
pub const RECTANGLE_RADIUS: f32 = 4.0;
/// `size` default of `SkeletonItem` (`useSkeletonItem`).
pub const DEFAULT_SIZE: u16 = 16;
/// `useRectangleStyles` and `useSizeStyles` keys, the only `size` values.
pub const SIZES: [u16; 20] = [
    8, 12, 14, 16, 20, 22, 24, 28, 32, 36, 40, 48, 52, 56, 64, 72, 92, 96, 120, 128,
];
