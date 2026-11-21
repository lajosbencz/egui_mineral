// docs/rules.md rule 10: indices here outlive the frame.
#![deny(clippy::indexing_slicing)]
#![warn(clippy::arithmetic_side_effects)]

use egui::{
    Id, Key, Rangef,
    accesskit::{Action, ActionData},
    emath::Numeric,
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

/// Bounded value with an optional value grid (`step`), keyboard unit (`increment`), decimal
/// precision (`fraction_digits`) and a [`Scale`].
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
    #[serde(default)]
    significant: Option<u8>,
    #[serde(default)]
    increment: Option<f64>,
    #[serde(default)]
    fraction_digits: Option<u8>,
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
    #[serde(default)]
    significant: Option<u8>,
    #[serde(default)]
    increment: Option<f64>,
    #[serde(default)]
    fraction_digits: Option<u8>,
}

impl TryFrom<RawRange> for RangeModel {
    type Error = String;

    fn try_from(r: RawRange) -> Result<Self, String> {
        check_bounds(r.min, r.max)?;
        for (name, v) in [("step", r.step), ("jump", r.jump), ("increment", r.increment)] {
            if let Some(v) = v {
                check_size(name, v)?;
            }
        }
        check_scale(r.scale, r.min)?;
        check_linear(r.scale, "increment", r.increment)?;
        if let Some(v) = r.ratio {
            check_ratio(v)?;
        }
        if let Some(d) = r.significant {
            check_significant(d)?;
        }
        if let Some(d) = r.fraction_digits {
            check_fraction_digits(d)?;
        }
        Ok(Self {
            min: r.min,
            max: r.max,
            step: r.step,
            jump: r.jump,
            scale: r.scale,
            ratio: r.ratio,
            significant: r.significant,
            increment: r.increment,
            fraction_digits: r.fraction_digits,
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

// f64 holds 15 to 17 significant decimal digits.
const MAX_SIGNIFICANT: u8 = 17;

fn check_significant(digits: u8) -> Result<(), String> {
    (1..=MAX_SIGNIFICANT)
        .contains(&digits)
        .then_some(())
        .ok_or_else(|| format!("RangeModel: significant digits {digits} must be in 1..={MAX_SIGNIFICANT}"))
}

// Decimals `decimal_places` and f64 resolve exactly.
const MAX_FRACTION_DIGITS: u8 = 15;

fn check_fraction_digits(digits: u8) -> Result<(), String> {
    (digits <= MAX_FRACTION_DIGITS)
        .then_some(())
        .ok_or_else(|| format!("RangeModel: fraction digits {digits} must be in 0..={MAX_FRACTION_DIGITS}"))
}

fn check_scale(scale: Scale, min: f64) -> Result<(), String> {
    if scale == Scale::Log && min <= 0.0 {
        Err(format!("RangeModel: log scale needs min > 0, got {min}"))
    } else {
        Ok(())
    }
}

fn check_linear(scale: Scale, name: &str, v: Option<f64>) -> Result<(), String> {
    if scale == Scale::Log && v.is_some() {
        Err(format!("RangeModel: log scale moves by track share; drop `{name}`"))
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
            step: self.increment.or(self.step),
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
            significant: None,
            increment: None,
            fraction_digits: None,
        }
    }

    /// Value grid on both scales, the ticks of a discrete slider: values snap to `min + k * step`.
    /// Arrows, wheel and buttons move by it unless [`Self::increment`] is set. A `step` that is not
    /// finite and > 0 is ignored.
    #[must_use]
    pub fn step(mut self, step: f64) -> Self {
        match check_size("step", step) {
            Ok(()) => self.step = Some(step),
            Err(e) => crate::kit::sanitize::report(format_args!("{e}; ignored")),
        }
        self
    }

    /// Grid of a [`Numeric`] value type: integer types get a whole step (default 1) and a whole increment; float
    /// types keep the model.
    #[must_use]
    pub fn for_numeric<N: Numeric>(mut self) -> Self {
        if N::INTEGRAL {
            let step = self.step.unwrap_or(1.0);
            self.step = Some(if step >= 1.0 { step.round() } else { 1.0 });
            self.increment = self.increment.map(|i| i.round().max(1.0));
        }
        self
    }

    /// PageUp/PageDown distance on [`Scale::Linear`]; on [`Scale::Log`] the least a page moves
    /// (the ratio page applies when it moves further). Ignored when not finite and > 0.
    #[must_use]
    pub fn jump(mut self, jump: f64) -> Self {
        match check_size("jump", jump) {
            Ok(()) => self.jump = Some(jump),
            Err(e) => crate::kit::sanitize::report(format_args!("{e}; ignored")),
        }
        self
    }

    /// Arrow, wheel and button distance on [`Scale::Linear`], added to the value as is (no grid
    /// anchored at `min`); a `step` grid still snaps the result. Default `step`, else 1% of the
    /// span. Ignored when not finite and > 0, or on [`Scale::Log`].
    #[must_use]
    pub fn increment(mut self, increment: f64) -> Self {
        match check_size("increment", increment).and_then(|()| check_linear(self.scale, "increment", Some(increment))) {
            Ok(()) => self.increment = Some(increment),
            Err(e) => crate::kit::sanitize::report(format_args!("{e}; ignored")),
        }
        self
    }

    /// Rounds snapped values to `digits` decimals (multiples of 10^-digits from 0), after
    /// [`Self::significant`] and before the `step` grid. Exact `min` and `max` stay reachable.
    /// Ignored above 15.
    #[must_use]
    pub fn fraction_digits(mut self, digits: u8) -> Self {
        match check_fraction_digits(digits) {
            Ok(()) => self.fraction_digits = Some(digits),
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

    /// Rounds snapped values to `digits` significant digits before the `step` grid: a relative grid
    /// for [`Scale::Log`], where `step` is absolute. Exact `min` and `max` stay reachable. Ignored
    /// outside `1..=17`.
    #[must_use]
    pub fn significant(mut self, digits: u8) -> Self {
        match check_significant(digits) {
            Ok(()) => self.significant = Some(digits),
            Err(e) => crate::kit::sanitize::report(format_args!("{e}; ignored")),
        }
        self
    }

    /// Linear or logarithmic value scale. [`Scale::Log`] with `min <= 0` is ignored; it drops an
    /// `increment`.
    #[must_use]
    pub fn scale(mut self, scale: Scale) -> Self {
        if let Err(e) = check_scale(scale, self.min) {
            crate::kit::sanitize::report(format_args!("{e}; scale ignored"));
            return self;
        }
        if let Err(e) = check_linear(scale, "increment", self.increment) {
            crate::kit::sanitize::report(format_args!("{e}; dropped"));
            self.increment = None;
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

    /// Value-to-track mapping chosen with [`Self::scale`].
    pub fn scale_kind(&self) -> Scale {
        self.scale
    }

    /// `None` is continuous.
    pub fn step_size(&self) -> Option<f64> {
        self.step
    }

    /// See [`Self::significant`].
    pub fn significant_digits(&self) -> Option<u8> {
        self.significant
    }

    /// See [`Self::increment`].
    pub fn increment_size(&self) -> Option<f64> {
        self.increment
    }

    /// See [`Self::fraction_digits`].
    pub fn fraction_digit_count(&self) -> Option<u8> {
        self.fraction_digits
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

    /// Clamps into bounds, rounds to [`Self::significant`] digits and [`Self::fraction_digits`]
    /// decimals, then to the nearest step from `min`.
    pub fn snap(&self, v: f64) -> f64 {
        let v = self.round_fraction(self.round_significant(self.limit(v)));
        match self.step {
            Some(s) => self.grid(((v - self.min) / s).round(), s),
            None => v,
        }
        .clamp(self.min, self.max)
    }

    #[expect(clippy::float_cmp, reason = "exact bounds are exempt from rounding")]
    fn round_significant(&self, v: f64) -> f64 {
        match self.significant {
            Some(d) if v.is_finite() && v != 0.0 && v != self.min && v != self.max => {
                round_decimal_digits(v, usize::from(d)).unwrap_or(v)
            }
            _ => v,
        }
    }

    #[expect(clippy::float_cmp, reason = "exact bounds are exempt from rounding")]
    fn round_fraction(&self, v: f64) -> f64 {
        match self.fraction_digits {
            Some(d) if v != self.min && v != self.max => round_places(v, i32::from(d)),
            _ => v,
        }
    }

    // One unit of the last significant digit away from `from`, in the decade it moves into.
    fn significant_step(&self, from: f64, up: bool) -> Option<f64> {
        let prec = i32::from(self.significant?.saturating_sub(1));
        let text = format!("{:.*e}", usize::try_from(prec).ok()?, from.abs());
        let (mantissa, exp) = text.split_once('e')?;
        let exp: i32 = exp.parse().ok()?;
        let decade_start = mantissa.trim_end_matches('0').trim_end_matches('.') == "1";
        let inward = up != (from > 0.0);
        let unit_exp = exp.checked_sub(prec)?.checked_sub(i32::from(decade_start && inward))?;
        let unit: f64 = format!("1e{unit_exp}").parse().ok()?;
        Some(self.snap(if up { from + unit } else { from - unit }))
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
        round_places(raw, places)
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

    pub(crate) fn arrow_step(&self) -> f64 {
        self.increment.or(self.step).unwrap_or(self.span() / 100.0)
    }

    fn shift(&self, v: f64, n: f64, page: bool) -> f64 {
        match self.scale {
            Scale::Linear => {
                let unit = if page {
                    self.jump.unwrap_or_else(|| (self.span() / 10.0).max(self.arrow_step()))
                } else {
                    self.arrow_step()
                };
                let from = self.snap(v);
                let raw = from + n * unit;
                // Decimal sum, so 0.1 + 0.2 lands on 0.3.
                let raw = decimal_places(from)
                    .zip(decimal_places(unit))
                    .map_or(raw, |(a, b)| round_places(raw, a.max(b)));
                let next = self.snap(raw);
                if self.step.is_none() && (next - from).abs() < f64::EPSILON && n.abs() > 0.0 {
                    self.significant_step(from, n > 0.0).unwrap_or(next)
                } else {
                    next
                }
            }
            Scale::Log => {
                let from = self.limit(v);
                let base = self.snap(from);
                let ln_ratio = match self.ratio {
                    // A zero log span divides to inf; a whole track is the most a step can move.
                    Some(r) => r.ln().min(self.log_span()) * if page { 10.0 } else { 1.0 },
                    None if page => LOG_PAGE * self.log_span(),
                    None => LOG_ARROW * self.log_span(),
                };
                let ln_ratio = if ln_ratio.is_finite() {
                    ln_ratio.min(self.log_span())
                } else {
                    0.0
                };
                let one = self.snap(from * (n.signum() * ln_ratio).exp());
                let mut next = self.snap(from * (n * ln_ratio).exp());
                if page && let Some(j) = self.jump {
                    let floor = self.snap(base + n.signum() * j);
                    next = if n > 0.0 { next.max(floor) } else { next.min(floor) };
                }
                if (one - base).abs() >= f64::EPSILON || n == 0.0 || !n.is_finite() {
                    return next;
                }
                match self.step {
                    Some(s) => self.snap(base + n * s),
                    None => stall_steps(n).fold(base, |acc, _| self.significant_step(acc, n > 0.0).unwrap_or(acc)),
                }
            }
        }
    }

    /// `v` moved by `n` arrow steps, snapped: `increment` (else `step`, else 1% of the span) on [`Scale::Linear`],
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
            crate::kit::interact::lock_arrows(ctx, id, crate::kit::interact::Arrows::Both);
            for k in crate::kit::interact::plain_key_presses(ctx) {
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
                    Some(ActionData::NumericValue(_)) => {
                        crate::kit::sanitize::report(format_args!("SetValue on {id:?}: non-finite number"));
                        out
                    }
                    Some(ActionData::Value(s)) => {
                        let parsed = self.parse(s, locale.as_ref());
                        if parsed.is_none() {
                            crate::kit::sanitize::report(format_args!(
                                "SetValue on {id:?}: not a number ({} chars)",
                                s.chars().count()
                            ));
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
        Self::parse_raw(text, locale).map(|v| self.clamp(v))
    }

    pub(crate) fn parse_raw(text: &str, locale: &dyn LocaleFormat) -> Option<f64> {
        locale
            .parse_number(text)
            .or_else(|| text.trim().parse().ok())
            .filter(|v: &f64| v.is_finite())
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

// Beyond 15 digits the scaled integer is no longer exact in f64 and rounding stops being idempotent.
fn round_places(v: f64, places: i32) -> f64 {
    let scale = 10f64.powi(places);
    let scaled = v * scale;
    if scaled.is_nan() || scaled.abs() >= 1e15 {
        return v;
    }
    scaled.round() / scale
}

const MAX_STALLED_STEPS: u32 = 1000;

fn stall_steps(n: f64) -> impl Iterator<Item = u32> {
    (0..MAX_STALLED_STEPS).take_while(move |i| f64::from(*i) + 0.5 < n.abs())
}

// `v` rounded to `digits` significant digits, ties away from zero on the decimal value: 15 digits
// first, so a binary product like 2804.9999999999995 reads as the tie it stands for.
fn round_decimal_digits(v: f64, digits: usize) -> Option<f64> {
    let precision = digits.saturating_sub(1).max(14);
    let text = format!("{:.precision$e}", v.abs());
    let (mantissa, exp) = text.split_once('e')?;
    let mut exp: i32 = exp.parse().ok()?;
    let mut ds: Vec<u8> = mantissa
        .bytes()
        .filter(u8::is_ascii_digit)
        .filter_map(|b| char::from(b).to_digit(10))
        .map(|d| d as u8)
        .collect();
    if digits < ds.len() {
        let up = ds.get(digits).is_some_and(|d| *d >= 5);
        ds.truncate(digits);
        if up {
            let carried = ds.iter_mut().rev().try_for_each(|d| {
                if *d == 9 {
                    *d = 0;
                    Some(())
                } else {
                    *d = d.saturating_add(1);
                    None
                }
            });
            if carried.is_some() {
                ds.insert(0, 1);
                ds.pop();
                exp = exp.checked_add(1)?;
            }
        }
    }
    let digit_text: String = ds.iter().filter_map(|d| char::from_digit(u32::from(*d), 10)).collect();
    let out: f64 = format!("0.{digit_text}e{}", exp.checked_add(1)?).parse().ok()?;
    Some(out.copysign(v))
}

// Fraction digits of the shortest decimal that reads back as `v`; `None` past f64 precision
// (e.g. a step of 1/3).
fn decimal_places(v: f64) -> Option<i32> {
    let text = v.abs().to_string();
    let places = text.split_once('.').map_or(0, |(_, f)| f.len());
    i32::try_from(places).ok().filter(|p| *p <= 15)
}

/// `value` as `N`, rounded for integer types; widgets keep `f64` internally and write back through it.
pub fn to_numeric<N: Numeric>(value: f64) -> N {
    N::from_f64(if N::INTEGRAL { value.round() } else { value })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_types_get_a_whole_grid_and_round() {
        let r = RangeModel::from(0.0..=10.0)
            .step(0.25)
            .increment(2.6)
            .for_numeric::<u8>();
        assert_eq!(r.step_size(), Some(1.0));
        assert_eq!(r.increment_size(), Some(3.0));
        assert_eq!(RangeModel::from(0.0..=10.0).for_numeric::<f32>().step_size(), None);
        assert_eq!(to_numeric::<i32>(2.6), 3);
        assert!((to_numeric::<f32>(2.5) - 2.5).abs() < f32::EPSILON);
    }

    #[test]
    fn significant_digits_grid_log_steps_and_keep_bounds_exact() {
        let r = RangeModel::new(0.1, 100.0).scale(Scale::Log).significant(3);
        assert_eq!(r.step_by(1.2, 1), 1.29, "1% of the track lands on 3 digits");
        assert_eq!(r.step_by(1.29, 1), 1.38, "the next step starts from the rounded value");
        for v in [0.123_456, 1.2, 2.749_041_183_321_324, 99.95, 12.345] {
            assert_eq!(r.snap(r.snap(v)), r.snap(v), "snap is idempotent at {v}");
        }
        assert_eq!(r.snap(2.749_041_183_321_324), 2.75);
        let wide = RangeModel::new(1.0, 20_001.0).significant(3);
        assert_eq!(
            wide.on_key(5.0, Key::End, Track::vertical()),
            Some(20_001.0),
            "max stays reachable"
        );
        assert_eq!(wide.snap(19_999.0), 20_000.0);
    }

    #[test]
    fn significant_steps_never_stall_and_cross_decades() {
        let r = RangeModel::new(0.1, 100.0)
            .scale(Scale::Log)
            .ratio(1.001)
            .significant(2);
        assert_eq!(r.step_by(1.0, 1), 1.1, "a step smaller than a digit moves one digit");
        assert_eq!(
            r.step_by(1.0, -1),
            0.99,
            "down from a decade start uses the decade below"
        );
        assert_eq!(r.step_by(0.99, 1), 1.0);
        let lin = RangeModel::new(0.0, 1e6).significant(1);
        assert_eq!(lin.step_by(1e5, 1), 2e5, "linear steps move too");
        assert_eq!(lin.step_by(1e5, -1), 9e4);
    }

    #[test]
    fn significant_deserializes_validated() {
        let raw = |significant| RawRange {
            min: 1.0,
            max: 10.0,
            step: None,
            jump: None,
            scale: Scale::Linear,
            ratio: None,
            significant,
            increment: None,
            fraction_digits: None,
        };
        let ok = RangeModel::try_from(raw(Some(3))).expect("3 digits");
        assert_eq!(ok.significant_digits(), Some(3));
        assert!(RangeModel::try_from(raw(Some(0))).is_err(), "0 digits is rejected");
        assert!(
            RangeModel::try_from(raw(Some(18))).is_err(),
            "past f64 precision is rejected"
        );
    }

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
        let log = RangeModel::new(1.0, 10.0).increment(2.0).scale(Scale::Log);
        assert_eq!(
            log,
            RangeModel::new(1.0, 10.0).scale(Scale::Log),
            "log drops the increment"
        );
        let log = RangeModel::new(1.0, 10.0).scale(Scale::Log).increment(2.0);
        assert_eq!(
            log,
            RangeModel::new(1.0, 10.0).scale(Scale::Log),
            "log ignores an increment"
        );
    }

    #[test]
    fn log_steps_start_from_the_current_value_and_round_once() {
        let r = RangeModel::new(0.1, 100_000.0)
            .scale(Scale::Log)
            .ratio(1.02)
            .significant(3);
        assert_eq!(r.step_by(1.234, 1), 1.26, "off-grid value is not snapped first");
        assert_eq!(r.step_by(2750.0, 1), 2810.0, "decimal tie rounds away from zero");
        assert_eq!(r.step_by(-0.0 + 2750.0, -1), 2700.0);
        assert_eq!(round_decimal_digits(9.995, 3), Some(10.0), "carry crosses a decade");
        assert_eq!(round_decimal_digits(-2805.0, 3), Some(-2810.0));
    }

    #[test]
    fn log_pages_honor_jump_and_stalled_steps_add_up() {
        let r = RangeModel::new(1.0, 1000.0)
            .scale(Scale::Log)
            .ratio(1.02)
            .step(1.0)
            .jump(10.0);
        assert_eq!(r.on_key(10.0, Key::PageUp, Track::vertical()), Some(20.0));
        assert_eq!(r.on_key(20.0, Key::PageDown, Track::vertical()), Some(10.0));
        let no_jump = RangeModel::new(1.0, 1000.0).scale(Scale::Log).ratio(1.02).step(1.0);
        assert_eq!(
            no_jump.on_key(10.0, Key::PageUp, Track::vertical()),
            Some(12.0),
            "ratio page only"
        );
        for n in 1..=5 {
            assert_eq!(no_jump.step_by(10.0, n), 10.0 + n as f64, "{n} stalled steps");
            assert_eq!(no_jump.step_by(10.0, -n), 10.0 - n as f64);
        }
        let sig = RangeModel::new(0.1, 100.0)
            .scale(Scale::Log)
            .ratio(1.001)
            .significant(2);
        assert_eq!(sig.step_by(1.0, 3), 1.3, "stalled significant steps repeat");
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

    #[test]
    fn fraction_digits_beyond_f64_precision_leave_the_value() {
        let r = RangeModel::new(0.0, f64::MAX).fraction_digits(12);
        let once = r.snap(4_144.642_518_158_023);
        assert_eq!(r.snap(once).to_bits(), once.to_bits());
    }

    #[test]
    fn increments_add_to_the_value_and_fraction_digits_round_from_zero() {
        let angle = RangeModel::new(0.0, 360.0).increment(5.0).fraction_digits(1);
        assert_eq!(angle.step_by(12.3, 1), 17.3);
        assert_eq!(angle.snap(12.34), 12.3);
        let tiny_min = RangeModel::new(1e-6, 10.0).increment(0.01);
        assert_eq!(tiny_min.step_by(1.0, 1), 1.01);
        let offset_min = RangeModel::new(0.01, 10.0).increment(1.0);
        assert_eq!(offset_min.step_by(1.5, 1), 2.5);
        assert_eq!(
            offset_min.step_by(0.01, 1),
            1.01,
            "min stays reachable and steps from it"
        );
        assert_eq!(RangeModel::new(0.0, 1.0).increment(0.1).step_by(0.2, 1), 0.3);
        let log = RangeModel::new(1.0, 10.0).increment(2.0).scale(Scale::Log);
        assert_eq!(log.increment_size(), None, "log drops the increment");
        assert_eq!(log.increment(2.0).increment_size(), None, "log ignores an increment");
        assert_eq!(
            RangeModel::new(0.0, 1.0).fraction_digits(16).fraction_digit_count(),
            None
        );
    }
}
