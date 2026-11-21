use egui::{Event, MouseWheelUnit, RawInput};

// Windows and KDE default.
const DEFAULT_LINES: f32 = 3.0;
// Seconds; time constant of the notch-rate estimate.
const RATE_WINDOW: f64 = 0.15;

/// Notched-wheel scrolling: distance per notch and acceleration of fast spins. Applies to egui's line deltas
/// only; trackpads and browsers report point deltas that already carry the OS speed and acceleration.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct WheelScroll {
    /// Lines per notch; `None` reads the OS setting when the runtime starts (Windows, KDE), else 3. macOS and
    /// browsers pre-scale line deltas, there `None` is 1.
    pub lines_per_notch: Option<f32>,
    /// Points per line.
    pub line_height: f32,
    /// Notches/s above which the distance grows proportionally.
    pub accelerate_from: f32,
    /// Distance multiplier cap; 1.0 turns acceleration off.
    pub max_multiplier: f32,
}

impl WheelScroll {
    /// Three 20-point lines per notch, up to 4x on fast spins; no acceleration where the OS applies its own
    /// (macOS, browsers).
    pub const fn baseline() -> Self {
        Self {
            lines_per_notch: None,
            line_height: 20.0,
            accelerate_from: 10.0,
            max_multiplier: if cfg!(any(target_os = "macos", target_os = "ios", target_arch = "wasm32")) {
                1.0
            } else {
                4.0
            },
        }
    }

    /// Points per notch before acceleration.
    #[must_use]
    pub fn base(&self) -> f32 {
        self.lines_per_notch.unwrap_or(DEFAULT_LINES) * self.line_height
    }

    pub(crate) fn resolved(self) -> Self {
        Self {
            lines_per_notch: self.lines_per_notch.or_else(system_lines),
            ..self
        }
    }

    pub(crate) fn sanitized(self) -> Self {
        use crate::kit::sanitize::at_least;
        Self {
            lines_per_notch: self
                .lines_per_notch
                .map(|l| at_least(l, 0.0.., "wheel.lines_per_notch")),
            line_height: at_least(self.line_height, 0.0.., "wheel.line_height"),
            accelerate_from: at_least(self.accelerate_from, f32::MIN_POSITIVE.., "wheel.accelerate_from"),
            max_multiplier: at_least(self.max_multiplier, 1.0.., "wheel.max_multiplier"),
        }
    }
}

impl Default for WheelScroll {
    fn default() -> Self {
        Self::baseline()
    }
}

/// Notch rate across passes; turns [`WheelScroll`] into egui's `line_scroll_speed` per pass.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WheelRate {
    rate: f32,
    last: Option<f64>,
}

impl WheelRate {
    /// Points per line delta for the pass of `raw`, counting its line notches into the rate. Input without
    /// a finite `time` neither counts nor accelerates.
    pub fn line_speed(&mut self, wheel: &WheelScroll, raw: &RawInput) -> f32 {
        let Some(now) = raw.time.filter(|t| t.is_finite()) else {
            return wheel.base();
        };
        let dt = self.last.map_or(f64::INFINITY, |t| (now - t).max(0.0));
        self.last = Some(now);
        let notches: f32 = raw
            .events
            .iter()
            .filter_map(|e| match e {
                Event::MouseWheel {
                    unit: MouseWheelUnit::Line,
                    delta,
                    ..
                } => Some(delta.x.abs().max(delta.y.abs())),
                _ => None,
            })
            .filter(|n| n.is_finite())
            .sum();
        #[expect(clippy::cast_possible_truncation, reason = "decay factor in 0..=1")]
        let decay = (-dt / RATE_WINDOW).exp() as f32;
        #[expect(clippy::cast_possible_truncation, reason = "window is 0.15")]
        let gain = (1.0 / RATE_WINDOW) as f32;
        self.rate = Some(self.rate * decay + notches * gain)
            .filter(|r| r.is_finite())
            .unwrap_or(0.0);
        let multiplier = self.rate / wheel.accelerate_from;
        let multiplier = if multiplier.is_finite() {
            multiplier.max(1.0).min(wheel.max_multiplier.max(1.0))
        } else {
            1.0
        };
        wheel.base() * multiplier
    }
}

#[cfg(windows)]
fn system_lines() -> Option<f32> {
    // SPI_GETWHEELSCROLLLINES persists here; "-1" is page scrolling, left to the default.
    let lines = windows_registry::CURRENT_USER
        .open(r"Control Panel\Desktop")
        .and_then(|k| k.get_string("WheelScrollLines"))
        .ok()?;
    lines.trim().parse::<u16>().ok().filter(|l| *l > 0).map(f32::from)
}

#[cfg(any(target_os = "macos", target_os = "ios", target_arch = "wasm32"))]
#[expect(clippy::unnecessary_wraps, reason = "same signature on every platform")]
const fn system_lines() -> Option<f32> {
    Some(1.0)
}

#[cfg(all(unix, not(any(target_os = "macos", target_os = "ios", target_os = "android"))))]
fn system_lines() -> Option<f32> {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|d| !d.is_empty())
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::Path::new(&h).join(".config")))?;
    kde_lines(&std::fs::read_to_string(config.join("kdeglobals")).ok()?)
}

#[cfg(target_os = "android")]
const fn system_lines() -> Option<f32> {
    None
}

// KConfig INI: `WheelScrollLines` in group `[KDE]`.
#[cfg_attr(
    not(all(unix, not(any(target_os = "macos", target_os = "ios", target_os = "android")))),
    expect(dead_code, reason = "KDE only")
)]
fn kde_lines(kdeglobals: &str) -> Option<f32> {
    let mut group = "";
    for line in kdeglobals.lines().map(str::trim) {
        if let Some(g) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
            group = g;
        } else if group == "KDE"
            && let Some((key, value)) = line.split_once('=')
            && key.trim() == "WheelScrollLines"
        {
            return value.trim().parse::<u16>().ok().filter(|l| *l > 0).map(f32::from);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use egui::{Modifiers, MouseWheelSource, TouchPhase, vec2};

    use super::*;

    fn notch(time: f64, n: usize) -> RawInput {
        let e = Event::MouseWheel {
            unit: MouseWheelUnit::Line,
            delta: vec2(0.0, -1.0),
            phase: TouchPhase::Move,
            source: MouseWheelSource::Wheel,
            modifiers: Modifiers::NONE,
        };
        RawInput {
            time: Some(time),
            events: vec![e; n],
            ..RawInput::default()
        }
    }

    const WHEEL: WheelScroll = WheelScroll {
        lines_per_notch: Some(3.0),
        line_height: 20.0,
        accelerate_from: 10.0,
        max_multiplier: 4.0,
    };

    #[test]
    fn slow_notches_scroll_the_base_distance() {
        let mut r = WheelRate::default();
        for i in 0..5 {
            assert_eq!(r.line_speed(&WHEEL, &notch(f64::from(i) * 0.3, 1)), 60.0);
        }
    }

    #[test]
    fn fast_spins_accelerate_up_to_the_cap_and_decay() {
        let mut r = WheelRate::default();
        let speeds: Vec<f32> = (0..40)
            .map(|i| r.line_speed(&WHEEL, &notch(f64::from(i) * 0.03, 1)))
            .collect();
        assert!(speeds.windows(2).all(|w| w[1] >= w[0]), "{speeds:?}");
        assert!(speeds[39] > 60.0 * 3.0 && speeds[39] <= 60.0 * 4.0, "{speeds:?}");
        assert_eq!(r.line_speed(&WHEEL, &notch(3.0, 1)), 60.0, "a pause resets the run");
    }

    #[test]
    fn point_deltas_and_untimed_input_do_not_accelerate() {
        let mut r = WheelRate::default();
        let mut raw = notch(0.0, 20);
        for e in &mut raw.events {
            if let Event::MouseWheel { unit, .. } = e {
                *unit = MouseWheelUnit::Point;
            }
        }
        assert_eq!(r.line_speed(&WHEEL, &raw), 60.0);
        assert_eq!(
            r.line_speed(
                &WHEEL,
                &RawInput {
                    time: None,
                    ..notch(0.0, 20)
                }
            ),
            60.0
        );
    }

    #[test]
    fn kde_lines_reads_the_kde_group_only() {
        assert_eq!(
            kde_lines("[General]\nWheelScrollLines=9\n[KDE]\nWheelScrollLines = 5\n"),
            Some(5.0)
        );
        assert_eq!(kde_lines("[KDE]\nWheelScrollLines=0\n"), None);
        assert_eq!(kde_lines("[Icons]\nTheme=breeze\n"), None);
    }
}
