// docs/rules.md rule 10: indices here outlive the frame.
#![deny(clippy::indexing_slicing)]
#![warn(clippy::arithmetic_side_effects)]

use egui::{
    Id, Key, Rangef,
    accesskit::{Action, ActionData},
};

use crate::{
    i18n::{Direction, LocaleFormat},
    layout::Axis,
    semantic::NumericRange,
};

/// Screen orientation of a value track; maps positions and arrow keys to value direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Track {
    axis: Axis,
    reversed: bool,
}

impl Track {
    /// Min at the leading edge: left in LTR, right in RTL.
    pub fn horizontal(dir: Direction) -> Self {
        Self {
            axis: Axis::Horizontal,
            reversed: dir == Direction::Rtl,
        }
    }

    /// Max at the top.
    pub fn vertical() -> Self {
        Self {
            axis: Axis::Vertical,
            reversed: true,
        }
    }

    /// Unmirrored: min at the left, regardless of text direction.
    pub fn horizontal_ltr() -> Self {
        Self::horizontal(Direction::Ltr)
    }

    /// Max at the bottom.
    pub fn vertical_down() -> Self {
        Self::vertical().reversed()
    }

    /// Swaps the ends.
    #[must_use]
    pub fn reversed(mut self) -> Self {
        self.reversed = !self.reversed;
        self
    }

    /// Screen axis the value runs along.
    pub fn axis(self) -> Axis {
        self.axis
    }

    /// Max sits at the left/top end.
    pub fn max_at_start(self) -> bool {
        self.reversed
    }

    /// `1.0` when the value grows along the screen axis (right or down), else `-1.0`.
    pub fn sign(self) -> f32 {
        if self.reversed { -1.0 } else { 1.0 }
    }

    fn key_sign(self, key: Key) -> Option<f64> {
        let (forward, back, up, down) = match self.axis {
            Axis::Horizontal => (Key::ArrowRight, Key::ArrowLeft, Key::ArrowUp, Key::ArrowDown),
            Axis::Vertical => (Key::ArrowDown, Key::ArrowUp, Key::ArrowRight, Key::ArrowLeft),
        };
        let along = f64::from(self.sign());
        match key {
            k if k == forward => Some(along),
            k if k == back => Some(-along),
            k if k == up => Some(1.0),
            k if k == down => Some(-1.0),
            _ => None,
        }
    }
}

/// Mapping between values and track positions ([`RangeModel::fraction`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum Scale {
    /// Equal value differences take equal distances.
    #[default]
    Linear,
    /// Equal ratios take equal distances; needs `min > 0`. Key, Increment/Decrement and button
    /// steps move a fixed share of the track (a constant ratio), see [`RangeModel::step_by`].
    Log,
}

/// Bounded value with an optional value grid (`step`) and a [`Scale`].
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "RawRange")]
pub struct RangeModel {
    min: f64,
    max: f64,
    step: Option<f64>,
    jump: Option<f64>,
    scale: Scale,
    #[serde(default)]
    ratio: Option<f64>,
}

#[derive(serde::Deserialize)]
struct RawRange {
    min: f64,
    max: f64,
    step: Option<f64>,
    jump: Option<f64>,
    #[serde(default)]
    scale: Scale,
    #[serde(default)]
    ratio: Option<f64>,
}

impl TryFrom<RawRange> for RangeModel {
    type Error = String;

    fn try_from(r: RawRange) -> Result<Self, String> {
        check_bounds(r.min, r.max)?;
        for (name, v) in [("step", r.step), ("jump", r.jump)] {
            if let Some(v) = v {
                check_size(name, v)?;
            }
        }
        check_scale(r.scale, r.min, r.jump)?;
        if let Some(v) = r.ratio {
            check_ratio(v)?;
        }
        Ok(Self {
            min: r.min,
            max: r.max,
            step: r.step,
            jump: r.jump,
            scale: r.scale,
            ratio: r.ratio,
        })
    }
}

fn bounds_ok(min: f64, max: f64) -> bool {
    min.is_finite() && max.is_finite() && min <= max
}

fn bounds_msg(min: f64, max: f64) -> String {
    format!("RangeModel: invalid bounds {min}..={max}")
}

fn check_bounds(min: f64, max: f64) -> Result<(), String> {
    bounds_ok(min, max).then_some(()).ok_or_else(|| bounds_msg(min, max))
}

fn size_ok(v: f64) -> bool {
    v.is_finite() && v > 0.0
}

fn size_msg(name: &str, v: f64) -> String {
    format!("RangeModel: {name} {v} must be finite and > 0")
}

fn check_size(name: &str, v: f64) -> Result<(), String> {
    size_ok(v).then_some(()).ok_or_else(|| size_msg(name, v))
}

fn check_ratio(v: f64) -> Result<(), String> {
    (v.is_finite() && v > 1.0)
        .then_some(())
        .ok_or_else(|| format!("RangeModel: ratio {v} must be finite and > 1"))
}

fn check_scale(scale: Scale, min: f64, jump: Option<f64>) -> Result<(), String> {
    if scale == Scale::Log && min <= 0.0 {
        Err(format!("RangeModel: log scale needs min > 0, got {min}"))
    } else if scale == Scale::Log && jump.is_some() {
        Err("RangeModel: log scale pages by track share; drop `jump`".to_owned())
    } else {
        Ok(())
    }
}

impl From<std::ops::RangeInclusive<f64>> for RangeModel {
    fn from(r: std::ops::RangeInclusive<f64>) -> Self {
        Self::new(*r.start(), *r.end())
    }
}

const LOG_ARROW: f64 = 0.01;
const LOG_PAGE: f64 = 0.1;

impl RangeModel {
    /// Bounds and step of the model around `value`.
    pub fn numeric(&self, value: f64) -> NumericRange {
        NumericRange {
            value,
            min: self.min(),
            max: self.max(),
            step: self.step_size(),
        }
    }

    /// Continuous (no step) over `min..=max`. A non-finite `min` counts as 0, a non-finite `max` as
    /// `min`, and `max` below `min` as `min`.
    pub fn new(min: f64, max: f64) -> Self {
        let min = crate::kit::sanitize::finite(min, "RangeModel min").unwrap_or(0.0);
        let max = crate::kit::sanitize::finite(max, "RangeModel max").unwrap_or(min);
        if max < min {
            crate::kit::sanitize::report(format_args!("{}; max set to min", bounds_msg(min, max)));
        }
        Self {
            min,
            max: max.max(min),
            step: None,
            jump: None,
            scale: Scale::Linear,
            ratio: None,
        }
    }

    /// Value grid on both scales: values snap to `min + k * step`. A `step` that is not finite and
    /// > 0 is ignored.
    #[must_use]
    pub fn step(mut self, step: f64) -> Self {
        match check_size("step", step) {
            Ok(()) => self.step = Some(step),
            Err(e) => crate::kit::sanitize::report(format_args!("{e}; ignored")),
        }
        self
    }

    /// PageUp/PageDown distance on [`Scale::Linear`]. Ignored when not finite and > 0, or on
    /// [`Scale::Log`].
    #[must_use]
    pub fn jump(mut self, jump: f64) -> Self {
        match check_size("jump", jump).and_then(|()| check_scale(self.scale, self.min, Some(jump))) {
            Ok(()) => self.jump = Some(jump),
            Err(e) => crate::kit::sanitize::report(format_args!("{e}; ignored")),
        }
        self
    }

    /// [`Scale::Log`] arrow step as a value ratio: arrows multiply or divide by `ratio`, pages by
    /// `ratio`^10. Default 1% and 10% of the track. Unused on [`Scale::Linear`]; ignored unless
    /// finite and > 1.
    #[must_use]
    pub fn ratio(mut self, ratio: f64) -> Self {
        match check_ratio(ratio) {
            Ok(()) => self.ratio = Some(ratio),
            Err(e) => crate::kit::sanitize::report(format_args!("{e}; ignored")),
        }
        self
    }

    /// Linear or logarithmic value scale. [`Scale::Log`] with `min <= 0` is ignored; with a
    /// `jump` set it drops the `jump`.
    #[must_use]
    pub fn scale(mut self, scale: Scale) -> Self {
        if let Err(e) = check_scale(scale, self.min, None) {
            crate::kit::sanitize::report(format_args!("{e}; scale ignored"));
            return self;
        }
        if scale == Scale::Log && self.jump.take().is_some() {
            crate::kit::sanitize::report(format_args!("RangeModel: log scale pages by track share; jump dropped"));
        }
        self.scale = scale;
        self
    }

    /// Lower bound, inclusive; finite.
    pub fn min(&self) -> f64 {
        self.min
    }

    /// Upper bound, inclusive; finite.
    pub fn max(&self) -> f64 {
        self.max
    }

    /// `None` is continuous.
    pub fn step_size(&self) -> Option<f64> {
        self.step
    }

    /// `max - min`; 0 for a degenerate range.
    pub fn span(&self) -> f64 {
        self.max - self.min
    }

    #[track_caller]
    fn limit(&self, v: f64) -> f64 {
        if v.is_nan() {
            crate::kit::sanitize::report(format_args!("RangeModel value must not be NaN"));
            return self.min;
        }
        v.clamp(self.min, self.max)
    }

    #[track_caller]
    fn unit(f: f64) -> f64 {
        if f.is_nan() {
            crate::kit::sanitize::report(format_args!("RangeModel fraction must not be NaN"));
            return 0.0;
        }
        f.clamp(0.0, 1.0)
    }

    fn log_span(&self) -> f64 {
        self.max.ln() - self.min.ln()
    }

    /// Clamps into bounds.
    pub fn clamp(&self, v: f64) -> f64 {
        self.limit(v)
    }

    /// Clamps into bounds, then rounds to the nearest step from `min`.
    pub fn snap(&self, v: f64) -> f64 {
        let v = self.limit(v);
        match self.step {
            Some(s) => self.grid(((v - self.min) / s).round(), s).clamp(self.min, self.max),
            None => v,
        }
    }

    // `min + k * step` rounded to the decimals of `min` and `step`, so 0.01 steps from -180 land
    // on 0.6, not 0.5999999999999943.
    fn grid(&self, k: f64, step: f64) -> f64 {
        let raw = self.min + k * step;
        let Some(places) = decimal_places(step)
            .zip(decimal_places(self.min))
            .map(|(a, b)| a.max(b))
        else {
            return raw;
        };
        let scale = 10f64.powi(places);
        let rounded = (raw * scale).round() / scale;
        if rounded.is_finite() { rounded } else { raw }
    }

    /// Track position of `v` in `0.0..=1.0`, through the [`Scale`].
    pub fn fraction(&self, v: f64) -> f64 {
        if self.span() <= 0.0 {
            return 0.0;
        }
        let v = self.limit(v);
        let f = match self.scale {
            Scale::Linear => (v - self.min) / self.span(),
            Scale::Log => (v.ln() - self.min.ln()) / self.log_span(),
        };
        // A log span that rounds to 0 (adjacent floats) divides to NaN or inf.
        if f.is_finite() { f.clamp(0.0, 1.0) } else { 0.0 }
    }

    fn lerp(&self, f: f64) -> f64 {
        let f = Self::unit(f);
        let v = match self.scale {
            Scale::Linear => self.min + f * self.span(),
            Scale::Log if f <= 0.0 => self.min,
            Scale::Log if f >= 1.0 => self.max,
            Scale::Log => (self.min.ln() + f * self.log_span()).exp(),
        };
        v.clamp(self.min, self.max)
    }

    /// Snapped value at track position `f` (clamped to 0.0..=1.0); inverse of [`Self::fraction`].
    pub fn from_fraction(&self, f: f64) -> f64 {
        self.snap(self.lerp(f))
    }

    /// Value under screen coordinate `pos` of `span` (the track extent along its axis).
    pub fn value_at(&self, span: Rangef, pos: f32, track: Track) -> f64 {
        let f = f64::from(if span.span() <= 0.0 {
            0.0
        } else {
            (pos - span.min) / span.span()
        });
        self.from_fraction(if track.reversed { 1.0 - f } else { f })
    }

    /// Screen coordinate of `v` within `span`; inverse of [`Self::value_at`].
    pub fn position_of(&self, span: Rangef, v: f64, track: Track) -> f32 {
        let f = self.fraction(v) as f32;
        egui::lerp(span, if track.reversed { 1.0 - f } else { f })
    }

    fn arrow_step(&self) -> f64 {
        self.step.unwrap_or(self.span() / 100.0)
    }

    fn shift(&self, v: f64, n: f64, page: bool) -> f64 {
        match self.scale {
            Scale::Linear => {
                let unit = if page {
                    self.jump.unwrap_or_else(|| (self.span() / 10.0).max(self.arrow_step()))
                } else {
                    self.arrow_step()
                };
                self.snap(v + n * unit)
            }
            Scale::Log => {
                let from = self.snap(v);
                let unit = match self.ratio {
                    // A zero log span divides to inf; a whole track is the most a step can move.
                    Some(r) => (r.ln() / self.log_span() * if page { 10.0 } else { 1.0 }).min(1.0),
                    None if page => LOG_PAGE,
                    None => LOG_ARROW,
                };
                let next = self.from_fraction(self.fraction(from) + n * unit);
                match self.step {
                    Some(s) if (next - from).abs() < f64::EPSILON && n.abs() > 0.0 => self.snap(from + n.signum() * s),
                    _ => next,
                }
            }
        }
    }

    /// `v` moved by `n` arrow steps, snapped: `step` (else 1% of the span) on [`Scale::Linear`],
    /// [`Self::ratio`] (else 1% of the track) on [`Scale::Log`].
    pub fn step_by(&self, v: f64, n: i64) -> f64 {
        self.shift(v, n as f64, false)
    }

    /// Value after `key`: arrows step ([`Self::step_by`]); PageUp/PageDown move `jump` (else 10%
    /// of the span) on [`Scale::Linear`], 10% of the track on [`Scale::Log`].
    pub fn on_key(&self, v: f64, key: Key, track: Track) -> Option<f64> {
        Some(match key {
            Key::PageUp => self.shift(v, 1.0, true),
            Key::PageDown => self.shift(v, -1.0, true),
            Key::Home => self.snap(self.min),
            Key::End => self.snap(self.max),
            _ => self.shift(v, track.key_sign(key)?, false),
        })
    }

    /// Step grid values from `min`; empty when continuous or when the grid would exceed `max_ticks`.
    pub fn ticks(&self, max_ticks: usize) -> Vec<f64> {
        let Some(s) = self.step else { return Vec::new() };
        let last = (self.span() / s).floor();
        if last >= max_ticks as f64 {
            return Vec::new();
        }
        (0..max_ticks)
            .map(|i| i as f64)
            .take_while(|i| *i <= last)
            .map(|i| self.grid(i, s))
            .collect()
    }

    /// `v` after this pass's keys (while `id` has focus; arrows stay with it) and AccessKit requests on `id`.
    pub fn drive(&self, core: &crate::runtime::Core, id: Id, v: f64, track: Track) -> f64 {
        let ctx = core.ctx();
        let mut v = v;
        if ctx.memory(|m| m.has_focus(id)) {
            crate::kit::focus::lock_arrows(ctx, id, crate::kit::focus::Arrows::Both);
            for k in crate::kit::input::plain_key_presses(ctx) {
                v = self.on_key(v, k, track).unwrap_or(v);
            }
        }
        self.accesskit(core, id, v).unwrap_or(v)
    }

    /// Value requested by this pass's AccessKit `SetValue`/`Increment`/`Decrement` requests on `id`; `None`
    /// without any.
    pub fn accesskit(&self, core: &crate::runtime::Core, id: Id, v: f64) -> Option<f64> {
        let ctx = core.ctx();
        // Fetched outside `ctx.input`: never take the I18n lock under the ctx lock.
        let locale = core.i18n().format();
        ctx.input(|i| {
            let mut out = None;
            for r in i.accesskit_action_requests(id, Action::SetValue) {
                out = match &r.data {
                    Some(ActionData::NumericValue(n)) if n.is_finite() => Some(self.snap(*n)),
                    Some(ActionData::NumericValue(n)) => {
                        tracing::warn!(value = n, "SetValue: non-finite number");
                        out
                    }
                    Some(ActionData::Value(s)) => {
                        let parsed = self.parse(s, locale.as_ref());
                        if parsed.is_none() {
                            tracing::warn!(value = %s, "SetValue: not a number");
                        }
                        parsed.or(out)
                    }
                    _ => out,
                };
            }
            let inc = i.num_accesskit_action_requests(id, Action::Increment);
            let dec = i.num_accesskit_action_requests(id, Action::Decrement);
            if inc > 0 || dec > 0 {
                let count = |n: usize| i64::try_from(n).unwrap_or(i64::MAX);
                out = Some(self.step_by(out.unwrap_or(v), count(inc).saturating_sub(count(dec))));
            }
            out
        })
    }

    /// Locale number, then plain Rust float syntax; clamped, not snapped. `None` for non-numbers
    /// and non-finite values.
    pub fn parse(&self, text: &str, locale: &dyn LocaleFormat) -> Option<f64> {
        locale
            .parse_number(text)
            .or_else(|| text.trim().parse().ok())
            .filter(|v: &f64| v.is_finite())
            .map(|v| self.clamp(v))
    }
}

/// Index of the value closest to `v`.
pub fn nearest_thumb(values: &[f64], v: f64) -> Option<usize> {
    values
        .iter()
        .enumerate()
        .min_by(|a, b| (a.1 - v).abs().total_cmp(&(b.1 - v).abs()))
        .map(|(i, _)| i)
}

// Fraction digits of the shortest decimal that reads back as `v`; `None` past f64 precision
// (e.g. a step of 1/3).
fn decimal_places(v: f64) -> Option<i32> {
    let text = v.abs().to_string();
    let places = text.split_once('.').map_or(0, |(_, f)| f.len());
    i32::try_from(places).ok().filter(|p| *p <= 15)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snap_keys_and_track_mapping() {
        let r = RangeModel::new(0.0, 10.0).step(2.5);
        let (ltr, rtl) = (Track::horizontal(Direction::Ltr), Track::horizontal(Direction::Rtl));
        assert!((r.snap(3.9) - 5.0).abs() < 1e-9);
        assert_eq!(r.on_key(5.0, Key::ArrowLeft, rtl), Some(7.5));
        assert_eq!(r.on_key(5.0, Key::ArrowUp, rtl), Some(7.5));
        assert_eq!(r.on_key(5.0, Key::End, ltr), Some(10.0));
        let span = Rangef::new(100.0, 200.0);
        assert!((r.value_at(span, 150.0, ltr) - 5.0).abs() < 1e-9);
        assert!((r.position_of(span, 10.0, rtl) - 100.0).abs() < 1e-4);
        assert_eq!(r.ticks(10).len(), 5);
        assert_eq!(r.parse(" 6.1 ", &crate::test_support::en_us()), Some(6.1));
        assert_eq!(r.parse("12", &crate::test_support::en_us()), Some(10.0));
    }

    #[test]
    fn vertical_track_keys_and_positions() {
        let r = RangeModel::new(0.0, 10.0).step(1.0);
        let top = Track::vertical();
        let down = Track::vertical_down();
        assert_eq!(down, top.reversed());
        let span = Rangef::new(0.0, 100.0);
        assert!(top.max_at_start() && !down.max_at_start());
        assert!((r.position_of(span, 10.0, top) - 0.0).abs() < 1e-4);
        assert!((r.value_at(span, 100.0, top)).abs() < 1e-9);
        assert!((r.value_at(span, 100.0, down) - 10.0).abs() < 1e-9);
        assert_eq!(r.on_key(5.0, Key::ArrowUp, top), Some(6.0));
        assert_eq!(r.on_key(5.0, Key::ArrowDown, top), Some(4.0));
        assert_eq!(r.on_key(5.0, Key::ArrowRight, top), Some(6.0));
        assert_eq!(r.on_key(5.0, Key::ArrowLeft, top), Some(4.0));
        assert_eq!(r.on_key(5.0, Key::ArrowDown, down), Some(6.0));
        assert_eq!(r.on_key(5.0, Key::ArrowUp, down), Some(4.0));
        assert_eq!(r.on_key(5.0, Key::Enter, top), None);
    }

    #[test]
    fn accesskit_ignores_non_finite_numbers() {
        let ctx = crate::test_support::ctx();
        let rt = crate::test_support::runtime(&ctx);
        let id = Id::unique("r");
        let set = egui::Event::AccessKitActionRequest(egui::accesskit::ActionRequest {
            action: Action::SetValue,
            target_tree: egui::accesskit::TreeId::ROOT,
            target_node: id.accesskit_id(),
            data: Some(ActionData::NumericValue(f64::NAN)),
        });
        let mut got = Some(0.0);
        crate::test_support::run(&ctx, crate::test_support::input(vec![set]), |_| {
            got = RangeModel::new(0.0, 10.0).accesskit(rt.core(), id, 5.0);
        });
        assert_eq!(got, None);
    }

    #[test]
    fn log_scale_maps_ratios_and_steps_by_track_share() {
        let ltr = Track::horizontal_ltr();
        let r = RangeModel::new(20.0, 20_000.0).scale(Scale::Log);
        assert!((r.fraction(200.0) - 1.0 / 3.0).abs() < 1e-12, "a decade is a third");
        for v in [20.0, 63.0, 1000.0, 20_000.0] {
            assert!((r.from_fraction(r.fraction(v)) - v).abs() < 1e-9 * v, "round trip {v}");
        }
        assert!(
            (r.value_at(Rangef::new(0.0, 300.0), 100.0, ltr) - 200.0).abs() < 1e-3,
            "f32 screen space"
        );
        assert!((r.step_by(1000.0, 1) - 1000.0 * 1000f64.powf(0.01)).abs() < 1e-9);
        let page = r.on_key(1000.0, Key::PageDown, ltr).unwrap();
        assert!((page - 1000.0 / 1000f64.powf(0.1)).abs() < 1e-9);

        let grid = RangeModel::new(1.0, 10.0).step(1.0).scale(Scale::Log);
        assert_eq!(grid.step_by(5.0, 1), 6.0, "a share below the grid moves one step");
        assert_eq!(grid.step_by(5.0, -1), 4.0);
        assert_eq!(grid.step_by(10.0, 1), 10.0);
        assert_eq!(grid.step_by(5.0, 0), 5.0);

        let ratio = r.ratio(2.0);
        assert!(
            (ratio.step_by(1000.0, 1) - 2000.0).abs() < 1e-9,
            "arrows multiply by the ratio"
        );
        let wide = RangeModel::new(1.0, 1e7).scale(Scale::Log).ratio(2.0);
        let page = wide.on_key(4096.0, Key::PageDown, ltr).unwrap();
        assert!((page - 4.0).abs() < 1e-9, "pages by ratio^10");
        assert_eq!(r.ratio(1.0), r, "a ratio of 1 is ignored");
        assert_eq!(
            RangeModel::new(5.0, 5.0).scale(Scale::Log).ratio(2.0).step_by(5.0, 0),
            5.0
        );
    }

    #[test]
    fn ticks_of_huge_span_do_not_overflow() {
        assert_eq!(RangeModel::new(0.0, 1e300).step(1e-300).ticks(10), Vec::<f64>::new());
    }

    #[test]
    fn invalid_arguments_become_the_nearest_valid_model() {
        let r = RangeModel::new(5.0, 1.0);
        assert_eq!((r.min(), r.max()), (5.0, 5.0), "max rises to min");
        let r = RangeModel::new(f64::NAN, f64::INFINITY);
        assert_eq!((r.min(), r.max()), (0.0, 0.0));
        assert_eq!(RangeModel::new(0.0, 10.0).step(-1.0).step_size(), None);
        assert_eq!(RangeModel::new(0.0, 10.0).step(f64::NAN).step_size(), None);
        let log = RangeModel::new(0.0, 10.0).scale(Scale::Log);
        assert_eq!(log, RangeModel::new(0.0, 10.0), "log needs min > 0");
        let log = RangeModel::new(1.0, 10.0).jump(2.0).scale(Scale::Log);
        assert_eq!(log, RangeModel::new(1.0, 10.0).scale(Scale::Log), "log drops the jump");
        let linear = RangeModel::new(1.0, 10.0).scale(Scale::Log).jump(2.0);
        assert_eq!(
            linear,
            RangeModel::new(1.0, 10.0).scale(Scale::Log),
            "log ignores a jump"
        );
    }

    #[test]
    fn nan_and_overflowing_log_ratios_stay_in_bounds() {
        let r = RangeModel::new(0.0, 10.0).step(2.5);
        assert_eq!(
            (r.snap(f64::NAN), r.fraction(f64::NAN), r.from_fraction(f64::NAN)),
            (0.0, 0.0, 0.0)
        );
        let log = RangeModel::new(1e-308, 1e308).scale(Scale::Log);
        let mid = log.from_fraction(0.5);
        assert!(mid.ln().abs() < 1.0, "geometric middle of the decades: {mid}");
        assert!((log.fraction(mid) - 0.5).abs() < 1e-9);
        assert_eq!(log.from_fraction(1.0), 1e308);
    }

    #[test]
    fn snapped_values_are_the_nearest_decimal() {
        let phase = RangeModel::new(-180.0, 180.0).step(0.01);
        assert_eq!(phase.snap(0.6), 0.6);
        assert_eq!(phase.snap(0.604), 0.6);
        let level = RangeModel::new(0.01, 255.0).step(0.01);
        assert_eq!(level.snap(100.0), 100.0);
        assert_eq!(RangeModel::new(0.0, 1.0).step(0.1).ticks(20)[3], 0.3);
        let thirds = RangeModel::new(0.0, 1.0).step(1.0 / 3.0);
        assert_eq!(thirds.snap(0.3), 1.0 / 3.0);
    }
}
