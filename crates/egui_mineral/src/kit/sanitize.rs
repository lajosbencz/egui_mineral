//! Replaces invalid arguments and runtime values with the nearest valid one and reports them, instead of
//! panicking (docs/rules.md rule 10).

use std::{
    collections::HashSet,
    fmt,
    ops::{RangeFrom, RangeInclusive, RangeToInclusive},
    panic::Location,
    sync::OnceLock,
    time::Duration,
};

use parking_lot::Mutex;

/// Logs `what` through `tracing::warn` once per call site; `#[track_caller]` callers report their own caller.
#[track_caller]
pub fn report(what: fmt::Arguments<'_>) {
    static SEEN: OnceLock<Mutex<HashSet<&'static Location<'static>>>> = OnceLock::new();
    let at = Location::caller();
    let first = SEEN.get_or_init(Mutex::default).lock().insert(at);
    if first {
        tracing::warn!(target: "egui_mineral", %at, "{what}");
    }
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for f32 {}
    impl Sealed for f64 {}
    impl Sealed for u8 {}
    impl Sealed for usize {}
}

/// Numbers the range sanitizers accept: `f32`, `f64`, `u8`, `usize`.
pub trait Number: Copy + PartialOrd + fmt::Display + sealed::Sealed {
    /// `false` for NaN and infinities.
    fn is_valid(self) -> bool;
}

impl Number for f32 {
    fn is_valid(self) -> bool {
        self.is_finite()
    }
}

impl Number for f64 {
    fn is_valid(self) -> bool {
        self.is_finite()
    }
}

impl Number for u8 {
    fn is_valid(self) -> bool {
        true
    }
}

impl Number for usize {
    fn is_valid(self) -> bool {
        true
    }
}

/// `f32` and `f64`.
pub trait Float: Number {}

impl Float for f32 {}
impl Float for f64 {}

/// `v` when valid and `>= min.start`, else `min.start`.
#[track_caller]
pub fn at_least<T: Number>(v: T, min: RangeFrom<T>, what: &str) -> T {
    if v.is_valid() && v >= min.start {
        v
    } else {
        report(format_args!("{what} must be finite and >= {}, got {v}", min.start));
        min.start
    }
}

/// `v` when valid and `<= max.end`, else `max.end`.
#[track_caller]
pub fn at_most<T: Number>(v: T, max: RangeToInclusive<T>, what: &str) -> T {
    if v.is_valid() && v <= max.end {
        v
    } else {
        report(format_args!("{what} must be finite and <= {}, got {v}", max.end));
        max.end
    }
}

/// `v` limited to `range`; NaN becomes the start, crossed bounds resolve to the start.
#[track_caller]
pub fn within<T: Number>(v: T, range: RangeInclusive<T>, what: &str) -> T {
    let (lo, hi) = (*range.start(), *range.end());
    let hi = if hi < lo { lo } else { hi };
    if v.is_valid() && v >= lo && v <= hi {
        return v;
    }
    report(format_args!("{what} must be finite and in {lo}..={hi}, got {v}"));
    if v > hi { hi } else { lo }
}

/// `Some(v)` when valid; otherwise reports and returns `None`.
#[track_caller]
pub fn finite<T: Float>(v: T, what: &str) -> Option<T> {
    if v.is_valid() {
        return Some(v);
    }
    report(format_args!("{what} must be finite, got {v}"));
    None
}

/// `d` when positive, else 1 ms.
#[track_caller]
pub fn nonzero_duration(d: Duration, what: &str) -> Duration {
    if d.is_zero() {
        report(format_args!("{what} must be > 0"));
        return Duration::from_millis(1);
    }
    d
}

/// `Duration` of `secs`; negative or NaN become zero, infinity and overflow saturate.
#[track_caller]
pub fn duration(secs: f64, what: &str) -> Duration {
    Duration::try_from_secs_f64(secs).unwrap_or_else(|_| {
        report(format_args!(
            "{what} must be a finite, non-negative number of seconds, got {secs}"
        ));
        if secs > 0.0 { Duration::MAX } else { Duration::ZERO }
    })
}

/// `i` when `< len`, else the last index; `None` when `len == 0`.
#[track_caller]
pub fn index(i: usize, len: usize, what: &str) -> Option<usize> {
    if i < len {
        return Some(i);
    }
    report(format_args!("{what} {i} is out of range for {len} items"));
    len.checked_sub(1)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{at_least, at_most, duration, finite, index, nonzero_duration, within};

    #[test]
    fn invalid_values_snap_to_the_nearest_valid_one() {
        assert_eq!(at_least(-1.0, 0.0.., "w"), 0.0);
        assert_eq!(at_least(f32::NAN, 2.0.., "w"), 2.0);
        assert_eq!(at_least(3.0, 0.0.., "w"), 3.0);
        assert_eq!(at_least(f64::NAN, 0.5.., "w"), 0.5);
        assert_eq!(at_most(12_u8, ..=9, "d"), 9);
        assert_eq!(at_most(4_u8, ..=9, "d"), 4);
        assert_eq!(at_most(f32::NAN, ..=1.0, "d"), 1.0);
        assert_eq!(within(5.0, 0.0..=1.0, "f"), 1.0);
        assert_eq!(within(f32::INFINITY, 0.0..=1.0, "f"), 1.0);
        assert_eq!(within(f32::NAN, 0.0..=1.0, "f"), 0.0);
        assert_eq!(within(0.5, 1.0..=0.0, "f"), 1.0, "crossed bounds resolve to the start");
        assert_eq!(index(7, 3, "i"), Some(2));
        assert_eq!(index(0, 0, "i"), None);
        assert_eq!(nonzero_duration(Duration::ZERO, "d"), Duration::from_millis(1));
        assert_eq!((finite(f32::NAN, "f"), finite(2.0, "f")), (None, Some(2.0)));
        assert_eq!(finite(f64::INFINITY, "f").unwrap_or(3.0), 3.0);
        assert_eq!((at_least(0_usize, 1.., "n"), at_least(4_usize, 1.., "n")), (1, 4));
        assert_eq!(duration(1.5, "t"), Duration::from_millis(1500));
        assert_eq!(duration(-1.0, "t"), Duration::ZERO);
        assert_eq!(duration(f64::NAN, "t"), Duration::ZERO);
        assert_eq!(duration(f64::INFINITY, "t"), Duration::MAX);
    }
}
