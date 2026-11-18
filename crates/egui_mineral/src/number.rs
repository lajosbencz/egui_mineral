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

/// Pending-edit model of a numeric text field: an [`EditBuffer`] of the value plus its text, which
/// is free while editing and the formatted value otherwise. Steps and AccessKit values set the
/// value directly; parsed text and scrubs start an edit.
#[derive(Clone, Debug)]
pub struct NumberEdit {
    range: RangeModel,
    value: EditBuffer<f64>,
    text: String,
    scrub: Option<f64>,
    snap: bool,
}

impl crate::kit::state::TempState for NumberEdit {}

impl NumberEdit {
    /// Keys that step the value instead of moving the caret.
    pub const STEP_KEYS: [Key; 4] = [Key::ArrowUp, Key::ArrowDown, Key::PageUp, Key::PageDown];

    /// Idle edit of `value` snapped into `range`; `fmt` formats the initial text.
    pub fn new(range: RangeModel, value: f64, fmt: impl Fn(f64) -> String) -> Self {
        let value = range.snap(value);
        Self {
            range,
            value: EditBuffer::new(value),
            text: fmt(value),
            scrub: None,
            snap: true,
        }
    }

    /// `false` keeps typed text and the bound value off the step grid, only clamped; steps and
    /// scrubs still land on it. Default `true`.
    pub fn set_snap(&mut self, snap: bool) {
        self.snap = snap;
    }

    fn settle(&self, range: &RangeModel, v: f64) -> f64 {
        if self.snap { range.snap(v) } else { range.clamp(v) }
    }

    /// Start of a pass: adopts the bound `value` and `focus` (see [`EditBuffer::sync`]); idle,
    /// the buffer shows the formatted value. `fmt` formats the buffer for the rest of the pass.
    pub fn begin<'a>(
        &'a mut self,
        range: RangeModel,
        value: f64,
        focus: FieldFocus,
        fmt: &'a dyn Fn(f64) -> String,
    ) -> NumberPass<'a> {
        let bound = self.settle(&range, value);
        self.range = range;
        self.value.sync(&bound, focus);
        if !self.value.editing() {
            self.text = fmt(*self.value.draft());
        }
        NumberPass { edit: self, bound, fmt }
    }

    /// Draft value, the bound one unless an edit is pending.
    pub fn value(&self) -> f64 {
        *self.value.draft()
    }

    /// Edit pending (see [`EditBuffer::editing`]).
    pub fn editing(&self) -> bool {
        self.value.editing()
    }
}

/// One pass of a [`NumberEdit`] with its formatter.
pub struct NumberPass<'a> {
    edit: &'a mut NumberEdit,
    bound: f64,
    fmt: &'a dyn Fn(f64) -> String,
}

crate::kit::opaque_debug!(NumberPass<'_>);

impl NumberPass<'_> {
    /// Draft value this pass.
    pub fn value(&self) -> f64 {
        *self.edit.value.draft()
    }

    /// Edit pending (see [`EditBuffer::editing`]).
    pub fn editing(&self) -> bool {
        self.edit.editing()
    }

    /// Drag-to-scrub in progress.
    pub fn scrubbing(&self) -> bool {
        self.edit.scrub.is_some()
    }

    /// Text buffer the field edits; free text while editing.
    pub fn text_mut(&mut self) -> &mut String {
        &mut self.edit.text
    }

    /// Parses the buffer into the value; keeps the previous value when it is not a number.
    pub fn apply_text(&mut self, locale: &dyn LocaleFormat) -> bool {
        let parsed = (self.edit.range.parse(&self.edit.text, locale)).map(|v| self.edit.settle(&self.edit.range, v));
        if let Some(v) = parsed {
            self.edit.value.start(&self.bound);
            *self.edit.value.draft_mut() = v;
        }
        parsed.is_some()
    }

    /// Handles a key of [`NumberEdit::STEP_KEYS`]; `false` for any other.
    pub fn step_key(&mut self, key: Key) -> bool {
        let Some(v) = NumberEdit::STEP_KEYS
            .contains(&key)
            .then(|| self.edit.range.on_key(self.value(), key, Track::vertical()))
            .flatten()
        else {
            return false;
        };
        self.set(v);
        true
    }

    /// Moves `n` steps via [`RangeModel::step_by`] and reformats the text.
    pub fn step(&mut self, n: i64) {
        self.set(self.edit.range.step_by(self.value(), n));
    }

    /// Applies AccessKit `SetValue`/`Increment`/`Decrement` targeted at `id`.
    pub fn accesskit(&mut self, core: &crate::runtime::Core, id: Id) -> bool {
        let Some(v) = self.edit.range.accesskit(core, id, self.value()) else {
            return false;
        };
        self.set(v);
        true
    }

    /// Drag-to-scrub on `handle` (sensing [`scrub_sense`]; `None` while not shown): a
    /// drag of `extent` points along `track` crosses the whole range through its scale, Shift alone
    /// 10x finer, Ctrl (Cmd on macOS) alone 10x coarser. A scrub is an edit that ends with the drag: commit once [`Self::scrubbing`] is
    /// `false`. `true` when the value changed.
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
        if !(extent.is_finite() && extent > 0.0) {
            crate::kit::sanitize::report(format_args!("scrub extent must be finite and > 0, got {extent}"));
            return false;
        }
        let range = self.edit.range;
        let from = self.edit.scrub.unwrap_or_else(|| range.fraction(self.value()));
        let along = f64::from(track.sign() * track.axis().main(delta)) / f64::from(extent) / rate;
        let at = (from + along).clamp(0.0, 1.0);
        self.edit.scrub = Some(at);
        let v = range.from_fraction(at);
        if (v - self.value()).abs() < f64::EPSILON {
            return false;
        }
        self.edit.value.start(&self.bound);
        self.set(v);
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
            self.edit.text = (self.fmt)(self.value());
        }
        event
    }

    /// Ends the edit restoring the value from before it began; reformats the buffer.
    pub fn cancel(&mut self) -> Option<EditEvent> {
        let event = self.edit.value.cancel();
        self.edit.text = (self.fmt)(self.value());
        event
    }

    fn set(&mut self, v: f64) {
        self.edit.value.start(&self.bound);
        *self.edit.value.draft_mut() = v;
        self.edit.text = (self.fmt)(v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{i18n::Direction, range::Scale};

    fn fmt(v: f64) -> String {
        format!("{v:.1}")
    }

    #[test]
    fn unsnapped_text_is_only_clamped_and_steps_return_to_the_grid() {
        let en = crate::test_support::en_us();
        let range = RangeModel::new(0.0, 10.0).step(0.01);
        let mut e = NumberEdit::new(range, 2.0, fmt);
        e.set_snap(false);
        let p = e.begin(range, 0.123, FieldFocus::Unfocused, &fmt);
        assert_eq!(p.value(), 0.123);
        let mut p = e.begin(range, 0.123, FieldFocus::Focused, &fmt);
        *p.text_mut() = "0.456".into();
        assert!(p.apply_text(&en));
        assert_eq!(p.value(), 0.456);
        *p.text_mut() = "12".into();
        p.apply_text(&en);
        assert_eq!(p.value(), 10.0);
        *p.text_mut() = "0.456".into();
        p.apply_text(&en);
        assert!(p.step_key(Key::ArrowUp));
        assert_eq!(p.value(), 0.47);
    }

    #[test]
    fn parse_commit_cancel() {
        let en = crate::test_support::en_us();
        let range = RangeModel::new(0.0, 10.0).step(0.5);
        let mut e = NumberEdit::new(range, 2.0, fmt);
        let mut p = e.begin(range, 2.0, FieldFocus::Unfocused, &fmt);
        assert_eq!(p.text_mut(), "2.0");

        let mut p = e.begin(range, 2.0, FieldFocus::Focused, &fmt);
        assert!(p.editing());
        *p.text_mut() = "7.3".into();
        assert!(p.apply_text(&en));
        assert_eq!(p.value(), 7.5);
        assert_eq!(p.commit(), Some(EditEvent::Committed));
        assert_eq!((p.editing(), p.text_mut().as_str()), (false, "7.5"));

        let mut p = e.begin(range, 7.5, FieldFocus::Focused, &fmt);
        *p.text_mut() = "abc".into();
        assert!(!p.apply_text(&en));
        assert_eq!(p.value(), 7.5);
        *p.text_mut() = "99".into();
        p.apply_text(&en);
        assert_eq!(p.value(), 10.0);
        assert_eq!(p.cancel(), Some(EditEvent::Cancelled));
        assert_eq!((p.value(), p.text_mut().as_str()), (7.5, "7.5"));

        let mut p = e.begin(range, 7.5, FieldFocus::Focused, &fmt);
        assert!(p.step_key(Key::PageDown));
        assert_eq!(p.value(), 6.5);
        assert!(!p.step_key(Key::Home));
        assert_eq!(p.commit(), Some(EditEvent::Committed));
        let p = e.begin(range, 6.5, FieldFocus::Unfocused, &fmt);
        assert!(!p.editing());
    }

    #[test]
    fn scrub_maps_drag_through_scale() {
        let range = RangeModel::new(1.0, 1000.0).scale(Scale::Log);
        let mut e = NumberEdit::new(range, 1.0, fmt);
        let mut p = e.begin(range, 1.0, FieldFocus::Unfocused, &fmt);
        let ltr = Track::horizontal_ltr();
        assert!(p.scrub_by(egui::vec2(100.0, 40.0), 300.0, ltr, 1.0));
        assert!((p.value() - 10.0).abs() < 1e-9, "a third of the extent is a decade");
        assert!(p.editing() && p.scrubbing());
        p.scrub_by(egui::vec2(100.0, 0.0), 300.0, ltr, FINE);
        assert!(
            (p.value() - 10.0 * 1000f64.powf(1.0 / 30.0)).abs() < 1e-9,
            "shift is 10x finer"
        );
        let before = p.value();
        p.scrub_by(egui::vec2(10.0, 0.0), 300.0, ltr, COARSE);
        assert!((p.value() - before * 10.0).abs() < 1e-9, "ctrl is 10x coarser");
        assert!(p.scrub_by(egui::vec2(-1e6, 0.0), 300.0, Track::horizontal(Direction::Rtl), 1.0));
        assert_eq!(p.value(), 1000.0, "leftward grows in RTL; clamped");
        assert!(!p.scrub(None, 300.0, ltr));
        assert!(!p.scrubbing());
        assert_eq!(p.commit(), Some(EditEvent::Committed));
    }

    #[test]
    fn scrub_with_a_degenerate_extent_changes_nothing() {
        let range = RangeModel::new(1.0, 1000.0);
        let mut e = NumberEdit::new(range, 1.0, fmt);
        let mut p = e.begin(range, 1.0, FieldFocus::Unfocused, &fmt);
        for extent in [0.0, -1.0, f32::NAN] {
            assert!(!p.scrub_by(egui::vec2(100.0, 40.0), extent, Track::horizontal_ltr(), 1.0));
        }
    }
}
