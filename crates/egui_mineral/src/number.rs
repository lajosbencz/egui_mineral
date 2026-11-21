use egui::{Context, Id, Key, Response, Sense, Vec2};

use crate::{
    edit::{EditBuffer, EditEvent, FieldFocus},
    i18n::LocaleFormat,
    input::{InputContext, PointerKind},
    range::{RangeModel, Track},
};

// Drag distance factor per modifier.
const FINE: f64 = 10.0;
const COARSE: f64 = 0.1;

/// Sense of a drag-to-scrub handle over the unfocused field, so a press-drag scrubs and a click
/// focuses the text. Not focusable: the text field stays the Tab stop and AccessKit node. `None`
/// with touch, where drags scroll and taps go to the text field.
pub fn scrub_sense(input: &InputContext) -> Option<Sense> {
    match input.modality {
        PointerKind::Touch => None,
        PointerKind::Mouse | PointerKind::Synthetic => Some(Sense::CLICK | Sense::DRAG),
    }
}

/// Number a field binds: a plain value, or one the user may leave blank.
#[derive(Debug)]
pub enum NumberRef<'a, N> {
    /// Always a number; blank text keeps the last one.
    Value(&'a mut N),
    /// `None` while the field is blank.
    Optional(&'a mut Option<N>),
}

impl<N: egui::emath::Numeric> NumberRef<'_, N> {
    /// Bound value as `f64`.
    pub fn get(&self) -> Option<f64> {
        match self {
            Self::Value(v) => Some(v.to_f64()),
            Self::Optional(v) => v.map(egui::emath::Numeric::to_f64),
        }
    }

    /// Writes `v` through [`crate::range::to_numeric`]; `None` leaves a [`Self::Value`] as it is.
    pub fn set(&mut self, v: Option<f64>) {
        match (self, v) {
            (Self::Value(b), Some(v)) => **b = crate::range::to_numeric(v),
            (Self::Value(_), None) => {}
            (Self::Optional(b), v) => **b = v.map(crate::range::to_numeric),
        }
    }

    /// [`Self::Optional`].
    pub fn optional(&self) -> bool {
        matches!(self, Self::Optional(_))
    }
}

/// How a scrub drag maps to the value.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub enum ScrubRate {
    /// Dragging across the handle's width spans the whole range, along its [`Scale`](crate::range::Scale).
    #[default]
    Extent,
    /// Points of drag per arrow step ([`RangeModel::step_by`] unit) on a linear range; on a log range
    /// the value doubles every `log_doubling_px` points.
    Pixels {
        /// Points per step, linear ranges.
        px_per_step: f32,
        /// Points per doubling, log ranges.
        log_doubling_px: f32,
    },
}

impl ScrubRate {
    /// [`ScrubRate::Pixels`].
    pub const fn pixels(px_per_step: f32, log_doubling_px: f32) -> Self {
        Self::Pixels {
            px_per_step,
            log_doubling_px,
        }
    }
}

/// Pending-edit model of a numeric text field: an [`EditBuffer`] of the value (`None` is a blank field)
/// plus its text, which is free while editing and the formatted value otherwise. Steps and AccessKit
/// values set the value directly; parsed text and scrubs start an edit.
#[derive(Clone, Debug)]
pub struct NumberEdit {
    range: RangeModel,
    value: EditBuffer<Option<f64>>,
    text: String,
    scrub: Option<f64>,
    scrub_rate: ScrubRate,
    snap: bool,
    blank: bool,
    source: Option<NumberSource>,
}

/// Input that wrote a [`NumberEdit`]'s draft.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum NumberSource {
    /// Parsed text.
    Typed,
    /// Step buttons, arrow and page keys.
    Step,
    /// Mouse wheel or trackpad scroll.
    Wheel,
    /// Drag-to-scrub.
    Scrub,
    /// AccessKit `SetValue`, `Increment` or `Decrement`.
    Accessibility,
}

impl crate::kit::state::TempState for NumberEdit {}

impl NumberEdit {
    /// Keys that step the value instead of moving the caret.
    pub const STEP_KEYS: [Key; 4] = [Key::ArrowUp, Key::ArrowDown, Key::PageUp, Key::PageDown];

    /// Idle edit of `value` snapped into `range`; `fmt` formats the initial text, `None` shows none.
    pub fn new(range: RangeModel, value: Option<f64>, fmt: impl Fn(f64) -> String) -> Self {
        let value = value.map(|v| range.snap(v));
        Self {
            range,
            value: EditBuffer::new(value),
            text: value.map_or_default(fmt),
            scrub: None,
            scrub_rate: ScrubRate::Extent,
            snap: true,
            blank: false,
            source: None,
        }
    }

    /// `false` keeps typed text and the bound value off the step grid, only clamped; steps and
    /// scrubs still land on it. Default `true`.
    pub fn set_snap(&mut self, snap: bool) {
        self.snap = snap;
    }

    /// `true` lets cleared text set the value to `None`; otherwise blank text keeps the previous value.
    /// Default `false`.
    pub fn set_blank(&mut self, blank: bool) {
        self.blank = blank;
    }

    /// Default [`ScrubRate::Extent`].
    pub fn set_scrub_rate(&mut self, rate: ScrubRate) {
        self.scrub_rate = rate;
    }

    fn settle(&self, range: &RangeModel, v: f64) -> f64 {
        if self.snap { range.snap(v) } else { range.clamp(v) }
    }

    /// Start of a pass: adopts the bound `value` and `focus` (see [`EditBuffer::sync`]); idle, or
    /// when the app changed the value during an edit, the buffer shows the formatted value. `fmt` formats the buffer for the rest of the pass.
    pub fn begin<'a>(
        &'a mut self,
        range: RangeModel,
        value: Option<f64>,
        focus: FieldFocus,
        fmt: &'a dyn Fn(f64) -> String,
    ) -> NumberPass<'a> {
        let bound = value.map(|v| self.settle(&range, v));
        self.range = range;
        if !self.value.editing() {
            self.source = None;
        }
        if self.value.sync(&bound, focus) || !self.value.editing() {
            self.text = self.value.draft().map_or_default(fmt);
        }
        NumberPass {
            edit: self,
            bound,
            fmt,
            clamped: false,
        }
    }

    /// Records `value` as written to the bound value (see [`EditBuffer::wrote`]).
    pub fn wrote(&mut self, value: Option<f64>) {
        self.value.wrote(&value);
    }

    /// Draft value, the bound one unless an edit is pending.
    pub fn value(&self) -> Option<f64> {
        *self.value.draft()
    }

    /// Edit pending (see [`EditBuffer::editing`]).
    pub fn editing(&self) -> bool {
        self.value.editing()
    }

    /// Last input that wrote the draft of the open or just committed edit; `None` while idle.
    pub fn source(&self) -> Option<NumberSource> {
        self.source
    }
}

/// One pass of a [`NumberEdit`] with its formatter.
pub struct NumberPass<'a> {
    edit: &'a mut NumberEdit,
    bound: Option<f64>,
    fmt: &'a dyn Fn(f64) -> String,
    clamped: bool,
}

crate::kit::opaque_debug!(NumberPass<'_>);

impl NumberPass<'_> {
    /// Draft value this pass.
    pub fn value(&self) -> Option<f64> {
        *self.edit.value.draft()
    }

    // Steps and scrubs from a blank field start at the in-range value nearest 0.
    fn anchor(&self) -> f64 {
        self.value().unwrap_or_else(|| self.edit.range.snap(0.0))
    }

    /// Edit pending (see [`EditBuffer::editing`]).
    pub fn editing(&self) -> bool {
        self.edit.editing()
    }

    /// See [`NumberEdit::source`].
    pub fn source(&self) -> Option<NumberSource> {
        self.edit.source
    }

    /// Drag-to-scrub in progress.
    pub fn scrubbing(&self) -> bool {
        self.edit.scrub.is_some()
    }

    /// Typed text, a step or a scrub this pass asked for a value outside the range, which was clamped.
    pub fn clamped(&self) -> bool {
        self.clamped
    }

    /// Text buffer the field edits; free text while editing.
    pub fn text_mut(&mut self) -> &mut String {
        &mut self.edit.text
    }

    /// Parses the buffer into the value; keeps the previous value when it is not a number. Blank text
    /// clears the value under [`NumberEdit::set_blank`].
    pub fn apply_text(&mut self, locale: &dyn LocaleFormat) -> bool {
        if self.edit.blank && self.edit.text.trim().is_empty() {
            self.edit.value.start(&self.bound);
            *self.edit.value.draft_mut() = None;
            self.edit.source = Some(NumberSource::Typed);
            return true;
        }
        let range = self.edit.range;
        let Some(raw) = RangeModel::parse_raw(&self.edit.text, locale) else {
            return false;
        };
        self.clamped |= raw < range.min() || raw > range.max();
        self.edit.value.start(&self.bound);
        *self.edit.value.draft_mut() = Some(self.edit.settle(&range, raw));
        self.edit.source = Some(NumberSource::Typed);
        true
    }

    /// Consumes every pending [`NumberEdit::STEP_KEYS`] press from `ctx`'s input and steps once per press; `true` when any stepped.
    pub fn consume_step_keys(&mut self, ctx: &Context) -> bool {
        let mut stepped = false;
        for k in NumberEdit::STEP_KEYS {
            while ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, k)) {
                stepped |= self.step_key(k);
            }
        }
        stepped
    }

    /// Handles a key of [`NumberEdit::STEP_KEYS`]; `false` for any other.
    pub fn step_key(&mut self, key: Key) -> bool {
        if !NumberEdit::STEP_KEYS.contains(&key) {
            return false;
        }
        let from = self.anchor();
        let Some(v) = self.edit.range.on_key(from, key, Track::vertical()) else {
            return false;
        };
        let up = matches!(key, Key::ArrowUp | Key::PageUp);
        self.block_at_bound(from, up);
        self.set(if self.value().is_some() { v } else { from }, NumberSource::Step);
        true
    }

    /// Moves `n` steps via [`RangeModel::step_by`] and reformats the text; `source` is
    /// [`NumberSource::Step`] or [`NumberSource::Wheel`].
    pub fn step(&mut self, n: i64, source: NumberSource) {
        let from = self.anchor();
        if n != 0 {
            self.block_at_bound(from, n > 0);
        }
        let v = if self.value().is_some() {
            self.edit.range.step_by(from, n)
        } else {
            from
        };
        self.set(v, source);
    }

    // A step out of a bound it already sits on.
    fn block_at_bound(&mut self, from: f64, up: bool) {
        let range = self.edit.range;
        self.clamped |= if up { from >= range.max() } else { from <= range.min() };
    }

    /// Applies AccessKit `SetValue`/`Increment`/`Decrement` targeted at `id`.
    pub fn accesskit(&mut self, core: &crate::runtime::Core, id: Id) -> bool {
        let Some(v) = self.edit.range.accesskit(core, id, self.anchor()) else {
            return false;
        };
        self.set(v, NumberSource::Accessibility);
        true
    }

    /// Drag-to-scrub on `handle` (sensing [`scrub_sense`]; `None` while not shown) at the
    /// [`ScrubRate`]: with [`ScrubRate::Extent`] a drag of `extent` points along `track` crosses the
    /// whole range. Shift alone is 10x finer, Ctrl (Cmd on macOS) alone 10x coarser. A scrub is an edit
    /// that ends with the drag: commit once [`Self::scrubbing`] is `false`. `true` when the value changed.
    pub fn scrub(&mut self, handle: Option<&Response>, extent: f32, track: Track) -> bool {
        match handle {
            Some(h) if h.dragged() => {
                let rate = h.ctx.input(|i| {
                    if i.modifiers.shift_only() {
                        FINE
                    } else if i.modifiers.command_only() {
                        COARSE
                    } else {
                        1.0
                    }
                });
                self.scrub_by(h.drag_delta(), extent, track, rate)
            }
            _ => {
                self.edit.scrub = None;
                false
            }
        }
    }

    fn scrub_by(&mut self, delta: Vec2, extent: f32, track: Track, rate: f64) -> bool {
        let range = self.edit.range;
        let px = f64::from(track.sign() * track.axis().main(delta)) / rate;
        // `scrub` holds the track fraction for `Extent`, the unsnapped value for `Pixels`.
        let at = match self.edit.scrub_rate {
            ScrubRate::Extent => {
                if !(extent.is_finite() && extent > 0.0) {
                    crate::kit::sanitize::report(format_args!("scrub extent must be finite and > 0, got {extent}"));
                    return false;
                }
                let from = self.edit.scrub.unwrap_or_else(|| range.fraction(self.anchor()));
                let to = from + px / f64::from(extent);
                self.clamped |= !(0.0..=1.0).contains(&to);
                let to = to.clamp(0.0, 1.0);
                self.edit.scrub = Some(to);
                range.from_fraction(to)
            }
            ScrubRate::Pixels {
                px_per_step,
                log_doubling_px,
            } => {
                let per = match range.scale_kind() {
                    crate::range::Scale::Linear => px_per_step,
                    crate::range::Scale::Log => log_doubling_px,
                };
                if !(per.is_finite() && per > 0.0) {
                    crate::kit::sanitize::report(format_args!("scrub rate must be finite and > 0, got {per}"));
                    return false;
                }
                let from = self.edit.scrub.unwrap_or_else(|| self.anchor());
                let n = px / f64::from(per);
                let to = match range.scale_kind() {
                    crate::range::Scale::Linear => from + n * range.arrow_step(),
                    crate::range::Scale::Log => from * n.exp2(),
                };
                if !to.is_finite() {
                    return false;
                }
                self.clamped |= to < range.min() || to > range.max();
                let to = range.clamp(to);
                self.edit.scrub = Some(to);
                range.snap(to)
            }
        };
        if self.value().is_some_and(|v| (v - at).abs() < f64::EPSILON) {
            return false;
        }
        self.set(at, NumberSource::Scrub);
        true
    }

    /// See [`EditBuffer::commit_requested`].
    pub fn commit_requested(&mut self, ctx: &Context) -> bool {
        self.edit.value.commit_requested(ctx)
    }

    /// Ends the edit keeping the value; reformats the buffer.
    pub fn commit(&mut self) -> Option<EditEvent> {
        let event = self.edit.value.commit();
        if event.is_some() {
            self.reformat();
        }
        event
    }

    /// Ends the edit restoring the value from before it began; reformats the buffer.
    pub fn cancel(&mut self) -> Option<EditEvent> {
        let event = self.edit.value.cancel();
        self.reformat();
        event
    }

    fn reformat(&mut self) {
        self.edit.text = self.value().map_or_default(self.fmt);
    }

    fn set(&mut self, v: f64, source: NumberSource) {
        self.edit.value.start(&self.bound);
        *self.edit.value.draft_mut() = Some(v);
        self.edit.text = (self.fmt)(v);
        self.edit.source = Some(source);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{i18n::Direction, range::Scale};

    fn fmt(v: f64) -> String {
        format!("{v:.1}")
    }

    fn v(p: &NumberPass<'_>) -> f64 {
        p.value().unwrap_or(f64::NAN)
    }

    #[test]
    fn unsnapped_text_is_only_clamped_and_steps_return_to_the_grid() {
        let en = crate::test_support::en_us();
        let range = RangeModel::new(0.0, 10.0).step(0.01);
        let mut e = NumberEdit::new(range, Some(2.0), fmt);
        e.set_snap(false);
        let p = e.begin(range, Some(0.123), FieldFocus::Unfocused, &fmt);
        assert_eq!(p.value(), Some(0.123));
        let mut p = e.begin(range, Some(0.123), FieldFocus::Focused, &fmt);
        *p.text_mut() = "0.456".into();
        assert!(p.apply_text(&en));
        assert_eq!(p.value(), Some(0.456));
        *p.text_mut() = "12".into();
        p.apply_text(&en);
        assert_eq!(p.value(), Some(10.0));
        *p.text_mut() = "0.456".into();
        p.apply_text(&en);
        assert!(p.step_key(Key::ArrowUp));
        assert_eq!(p.value(), Some(0.47));
    }

    #[test]
    fn source_is_the_last_writer_until_the_edit_closes() {
        let en = crate::test_support::en_us();
        let range = RangeModel::new(0.0, 10.0).step(1.0);
        let mut e = NumberEdit::new(range, Some(2.0), fmt);
        let mut p = e.begin(range, Some(2.0), FieldFocus::Focused, &fmt);
        assert_eq!(p.source(), None);
        *p.text_mut() = "5".into();
        p.apply_text(&en);
        assert_eq!(p.source(), Some(NumberSource::Typed));
        p.step(1, NumberSource::Wheel);
        assert_eq!(p.source(), Some(NumberSource::Wheel), "the last writer wins");
        assert!(p.commit().is_some());
        assert_eq!(p.source(), Some(NumberSource::Wheel), "the committed edit keeps it");
        let p = e.begin(range, Some(6.0), FieldFocus::Focused, &fmt);
        assert_eq!(p.source(), None, "an idle field has none");
    }

    #[test]
    fn parse_commit_cancel() {
        let en = crate::test_support::en_us();
        let range = RangeModel::new(0.0, 10.0).step(0.5);
        let mut e = NumberEdit::new(range, Some(2.0), fmt);
        let mut p = e.begin(range, Some(2.0), FieldFocus::Unfocused, &fmt);
        assert_eq!(p.text_mut(), "2.0");

        let mut p = e.begin(range, Some(2.0), FieldFocus::Focused, &fmt);
        assert!(p.editing());
        *p.text_mut() = "7.3".into();
        assert!(p.apply_text(&en));
        assert_eq!(p.value(), Some(7.5));
        assert_eq!(p.commit(), Some(EditEvent::Committed));
        assert_eq!((p.editing(), p.text_mut().as_str()), (false, "7.5"));

        let mut p = e.begin(range, Some(7.5), FieldFocus::Focused, &fmt);
        *p.text_mut() = "abc".into();
        assert!(!p.apply_text(&en));
        assert_eq!(p.value(), Some(7.5));
        *p.text_mut() = "99".into();
        p.apply_text(&en);
        assert_eq!(p.value(), Some(10.0));
        assert_eq!(p.cancel(), Some(EditEvent::Cancelled));
        assert_eq!((p.value(), p.text_mut().as_str()), (Some(7.5), "7.5"));

        let mut p = e.begin(range, Some(7.5), FieldFocus::Focused, &fmt);
        assert!(p.step_key(Key::PageDown));
        assert_eq!(p.value(), Some(6.5));
        assert!(!p.step_key(Key::Home));
        assert_eq!(p.commit(), Some(EditEvent::Committed));
        let p = e.begin(range, Some(6.5), FieldFocus::Unfocused, &fmt);
        assert!(!p.editing());
    }

    #[test]
    fn scrub_maps_drag_through_scale() {
        let range = RangeModel::new(1.0, 1000.0).scale(Scale::Log);
        let mut e = NumberEdit::new(range, Some(1.0), fmt);
        let mut p = e.begin(range, Some(1.0), FieldFocus::Unfocused, &fmt);
        let ltr = Track::horizontal_ltr();
        assert!(p.scrub_by(egui::vec2(100.0, 40.0), 300.0, ltr, 1.0));
        assert!((v(&p) - 10.0).abs() < 1e-9, "a third of the extent is a decade");
        assert!(p.editing() && p.scrubbing());
        p.scrub_by(egui::vec2(100.0, 0.0), 300.0, ltr, FINE);
        assert!(
            (v(&p) - 10.0 * 1000f64.powf(1.0 / 30.0)).abs() < 1e-9,
            "shift is 10x finer"
        );
        let before = v(&p);
        p.scrub_by(egui::vec2(10.0, 0.0), 300.0, ltr, COARSE);
        assert!((v(&p) - before * 10.0).abs() < 1e-9, "ctrl is 10x coarser");
        assert!(p.scrub_by(egui::vec2(-1e6, 0.0), 300.0, Track::horizontal(Direction::Rtl), 1.0));
        assert_eq!(p.value(), Some(1000.0), "leftward grows in RTL; clamped");
        assert!(!p.scrub(None, 300.0, ltr));
        assert!(!p.scrubbing());
        assert_eq!(p.commit(), Some(EditEvent::Committed));
    }

    #[test]
    fn scrub_with_a_degenerate_extent_changes_nothing() {
        let range = RangeModel::new(1.0, 1000.0);
        let mut e = NumberEdit::new(range, Some(1.0), fmt);
        let mut p = e.begin(range, Some(1.0), FieldFocus::Unfocused, &fmt);
        for extent in [0.0, -1.0, f32::NAN] {
            assert!(!p.scrub_by(egui::vec2(100.0, 40.0), extent, Track::horizontal_ltr(), 1.0));
        }
    }

    #[test]
    fn blank_text_clears_an_optional_value_and_steps_restart_near_zero() {
        let en = crate::test_support::en_us();
        let range = RangeModel::new(-5.0, 10.0).step(1.0);
        let mut e = NumberEdit::new(range, None, fmt);
        e.set_blank(true);
        let mut p = e.begin(range, None, FieldFocus::Focused, &fmt);
        assert_eq!((p.value(), p.text_mut().as_str()), (None, ""));
        p.step(1, NumberSource::Step);
        assert_eq!(p.value(), Some(0.0), "the first step fills in the value nearest 0");
        p.step(1, NumberSource::Step);
        assert_eq!(p.value(), Some(1.0));
        p.text_mut().clear();
        assert!(p.apply_text(&en));
        assert_eq!(p.value(), None);
        assert_eq!(p.commit(), Some(EditEvent::Committed));
        e.set_blank(false);
        let mut p = e.begin(range, Some(3.0), FieldFocus::Focused, &fmt);
        p.text_mut().clear();
        assert!(!p.apply_text(&en), "without blank, empty text is not a number");
        assert_eq!(p.value(), Some(3.0));
    }

    #[test]
    fn pixel_rate_scrubs_steps_on_linear_and_doublings_on_log() {
        let ltr = Track::horizontal_ltr();
        let linear = RangeModel::new(0.0, 100.0).step(1.0);
        let mut e = NumberEdit::new(linear, Some(10.0), fmt);
        e.set_scrub_rate(ScrubRate::pixels(4.0, 50.0));
        let mut p = e.begin(linear, Some(10.0), FieldFocus::Unfocused, &fmt);
        p.scrub_by(egui::vec2(2.0, 0.0), 1.0, ltr, 1.0);
        assert_eq!(p.value(), Some(11.0), "half a step rounds up on the grid");
        p.scrub_by(egui::vec2(2.0, 0.0), 1.0, ltr, 1.0);
        assert_eq!(p.value(), Some(11.0), "sub-step motion accumulates");
        p.scrub_by(egui::vec2(36.0, 0.0), 1.0, ltr, 1.0);
        assert_eq!(p.value(), Some(20.0));
        let log = RangeModel::new(1.0, 1e6).scale(Scale::Log);
        let mut e = NumberEdit::new(log, Some(100.0), fmt);
        e.set_scrub_rate(ScrubRate::pixels(4.0, 50.0));
        let mut p = e.begin(log, Some(100.0), FieldFocus::Unfocused, &fmt);
        p.scrub_by(egui::vec2(100.0, 0.0), 1.0, ltr, 1.0);
        assert!((v(&p) - 400.0).abs() < 1e-9, "two doublings, got {}", v(&p));
        assert!(!p.clamped());
    }

    #[test]
    fn clamped_reports_typed_steps_and_scrubs_past_the_bounds() {
        let en = crate::test_support::en_us();
        let range = RangeModel::new(0.0, 10.0).step(1.0);
        let mut e = NumberEdit::new(range, Some(10.0), fmt);
        let mut p = e.begin(range, Some(10.0), FieldFocus::Focused, &fmt);
        p.step(-1, NumberSource::Step);
        assert!(!p.clamped());
        p.step(2, NumberSource::Step);
        assert!(!p.clamped(), "capped at the bound but not blocked");
        p.step(1, NumberSource::Step);
        assert!(p.clamped(), "a step out of the bound it sits on");
        let mut p = e.begin(range, Some(10.0), FieldFocus::Focused, &fmt);
        *p.text_mut() = "42".into();
        p.apply_text(&en);
        assert!(p.clamped() && p.value() == Some(10.0));
        let mut e = NumberEdit::new(range, Some(5.0), fmt);
        let mut p = e.begin(range, Some(5.0), FieldFocus::Unfocused, &fmt);
        p.scrub_by(egui::vec2(-1e4, 0.0), 100.0, Track::horizontal_ltr(), 1.0);
        assert!(p.clamped() && p.value() == Some(0.0));
    }
}
