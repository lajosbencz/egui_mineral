//! Design values and behavior policies a [`Stylekit`](crate::runtime::Stylekit) supplies; owned by
//! [`Core`](crate::runtime::Core).

use std::{sync::Arc, time::Duration};

use egui::CursorIcon;

use crate::{
    input::{GestureConfig, Repeat},
    kit::sanitize::{at_least, nonzero_duration, within},
    layout::TargetPolicy,
    motion::Fling,
};

/// Design values (targets, breakpoints, fling, repeat, gestures, typeahead) and behavior policies.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct Policies {
    /// Minimum hit target per input modality.
    pub target: TargetPolicy,
    /// Strictly ascending width thresholds in points; see [`crate::layout::breakpoint`].
    pub breakpoints: Arc<[f32]>,
    /// Momentum and swipe-to-dismiss thresholds of sheets and carousels.
    pub fling: Fling,
    /// Press-and-hold timing; see [`crate::input::hold_repeat`].
    pub repeat: Repeat,
    /// Tap, long-press and swipe thresholds of the IAL.
    pub gestures: GestureConfig,
    /// Gap after which a keystroke starts a new [`Typeahead`](crate::focus::Typeahead) query.
    pub typeahead: Duration,
    /// egui's Ctrl+H/K/U/W in text edits; off strips them so bindings on them fire. Cocoa has them (macOS, iOS).
    pub readline: bool,
    /// Alt+letter access keys, and F10 or a lone Alt to focus a menu bar.
    pub mnemonics: bool,
    /// Hover cursors.
    pub cursors: Cursors,
}

/// Cursor over a hovered interactive element (see [`crate::kit::input::hover_cursor`]); `None` keeps
/// egui's choice.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct Cursors {
    /// Over enabled interactive elements.
    pub enabled: Option<CursorIcon>,
    /// Over disabled interactive elements.
    pub disabled: Option<CursorIcon>,
}

impl Cursors {
    /// Cursor over enabled interactive elements.
    #[must_use]
    pub const fn enabled(self, cursor: CursorIcon) -> Self {
        Self {
            enabled: Some(cursor),
            ..self
        }
    }

    /// Cursor over disabled interactive elements.
    #[must_use]
    pub const fn disabled(self, cursor: CursorIcon) -> Self {
        Self {
            disabled: Some(cursor),
            ..self
        }
    }
}

impl Policies {
    /// Neutral desktop values; override fields for a design system.
    pub fn baseline() -> Self {
        Self {
            target: TargetPolicy::baseline(),
            breakpoints: [600.0, 840.0].into(),
            fling: Fling {
                deceleration: 0.998,
                dismiss_fraction: crate::layout::Fraction::HALF,
                dismiss_velocity: 1000.0,
            },
            repeat: Repeat {
                delay: Duration::from_millis(500),
                interval: Duration::from_millis(100),
            },
            gestures: GestureConfig::baseline(),
            typeahead: Duration::from_secs(1),
            readline: true,
            mnemonics: false,
            cursors: Cursors::default(),
        }
    }

    pub(crate) fn sanitized(self) -> Self {
        let mut p = self;
        p.target.mouse = at_least(p.target.mouse, 0.0.., "target.mouse");
        p.target.touch = at_least(p.target.touch, 0.0.., "target.touch");
        if !p.breakpoints.windows(2).all(|w| w[0] < w[1]) || !p.breakpoints.iter().all(|b| b.is_finite()) {
            crate::kit::sanitize::report(format_args!(
                "breakpoints must be finite and strictly ascending: {:?}",
                p.breakpoints
            ));
            let mut b: Vec<f32> = p.breakpoints.iter().copied().filter(|b| b.is_finite()).collect();
            b.sort_by(f32::total_cmp);
            b.dedup();
            p.breakpoints = b.into();
        }
        p.fling.deceleration = within(
            p.fling.deceleration,
            0.0..=crate::motion::MAX_DECELERATION,
            "fling.deceleration",
        );
        p.fling.dismiss_velocity = at_least(p.fling.dismiss_velocity, 0.0.., "fling.dismiss_velocity");
        p.repeat.interval = nonzero_duration(p.repeat.interval, "repeat.interval");
        p.gestures.slop = at_least(p.gestures.slop, 0.0.., "gestures.slop");
        p.gestures.swipe_min_velocity = at_least(p.gestures.swipe_min_velocity, 0.0.., "gestures.swipe_min_velocity");
        p
    }
}

#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use crate::test_support::policies;

    #[test]
    fn invalid_values_snap_to_the_nearest_valid_one() {
        let mut p = policies();
        p.target.mouse = f32::NAN;
        p.target.touch = -4.0;
        p.breakpoints = Arc::from([840.0, f32::NAN, 600.0, 600.0]);
        p.fling.deceleration = 1.5;
        p.repeat.interval = Duration::ZERO;
        p.gestures.slop = f32::INFINITY;
        let got = p.sanitized();
        assert_eq!((got.target.mouse, got.target.touch), (0.0, 0.0));
        assert_eq!(&*got.breakpoints, &[600.0, 840.0]);
        assert_eq!(got.fling.deceleration, crate::motion::MAX_DECELERATION);
        assert_eq!(got.repeat.interval, Duration::from_millis(1));
        assert_eq!(got.gestures.slop, 0.0);
    }
}
