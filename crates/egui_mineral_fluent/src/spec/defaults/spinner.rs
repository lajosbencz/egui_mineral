//! `react-spinner/library/src/components/Spinner/useSpinnerStyles.styles.ts`.

use std::time::Duration;

/// Root `gap` between the ring and the label.
pub const GAP: f32 = 8.0;
/// `extra-tiny` ring size.
pub const EXTRA_TINY: f32 = 16.0;
/// `tiny` ring size.
pub const TINY: f32 = 20.0;
/// `extra-small` ring size.
pub const EXTRA_SMALL: f32 = 24.0;
/// `small` ring size.
pub const SMALL: f32 = 28.0;
/// `medium` ring size.
pub const MEDIUM: f32 = 32.0;
/// `large` ring size.
pub const LARGE: f32 = 36.0;
/// `extra-large` ring size.
pub const EXTRA_LARGE: f32 = 40.0;
/// `huge` ring size.
pub const HUGE: f32 = 44.0;
/// `animationDuration` of the spin and the tail.
pub const CYCLE: Duration = Duration::from_millis(1500);
/// Tail `maskImage` of a left-to-right spinner: start and length in degrees.
pub const MASK_LTR: (f32, f32) = (105.0, 255.0);
/// `rtlTail` `maskImage`: start and length in degrees.
pub const MASK_RTL: (f32, f32) = (0.0, 255.0);
/// Length of the `::before` and `::after` arc segments in degrees.
pub const SEGMENT: f32 = 135.0;
/// Start of the `::before` and `::after` segments of an `rtlTail`, in degrees.
pub const SEGMENT_START_RTL: f32 = 225.0;
/// `rotate` keyframes of the tail: start, middle and end.
pub const TAIL_ROTATION: [f32; 3] = [-135.0, 0.0, 225.0];
/// `rotate` keyframes of `::before`: start, middle, end.
pub const BEFORE_ROTATION: [f32; 3] = [0.0, 105.0, 0.0];
/// `rotate` keyframes of `::after`: start, middle, end.
pub const AFTER_ROTATION: [f32; 3] = [0.0, 225.0, 0.0];
/// `prefers-reduced-motion` tail `conic-gradient(transparent 120deg, currentcolor 360deg)`: start and length.
pub const REDUCED_ARC_LTR: (f32, f32) = (120.0, 240.0);
/// `rtlTail` reduced tail `conic-gradient(currentcolor 0deg, transparent 240deg)`: start and length.
pub const REDUCED_ARC_RTL: (f32, f32) = (0.0, 240.0);
