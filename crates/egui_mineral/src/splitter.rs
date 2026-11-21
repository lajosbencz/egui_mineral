//! <https://www.w3.org/WAI/ARIA/apg/patterns/windowsplitter>

use egui::{CursorIcon, Id, Key, Modifiers, Rect, Response, Sense, UiBuilder, pos2};

use crate::{
    kit,
    layout::{Axis, Fraction, MinTarget, Zones},
    range::{RangeModel, Track},
    runtime::Stylekit,
    semantic::{self, AgentKey, KeyedSalt, Role, SemanticNode, StableId},
    widget::{Availability, MineralUi},
};

const MIN_STEP: f64 = 1e-6;

/// One of a [`Splitter`]'s panes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pane {
    /// Before the handle: at the reading start side by side, on top when stacked.
    First,
    /// After the handle.
    Second,
}

/// Two-pane splitter behavior (drag, keyboard, fraction); paints nothing, the stylekit draws the handle.
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct Splitter {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    axis: Axis,
    default_fraction: Fraction,
    min_first: f32,
    min_second: f32,
    handle: f32,
    step: Option<f64>,
    jump: Option<f64>,
    disabled: bool,
    collapse: Option<Pane>,
}

/// Result of [`Splitter::show`]; `handle_rect` and `hit_rect` for painting.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SplitterResponse<A, B> {
    /// Result of the first pane closure.
    pub first: A,
    /// Result of the second pane closure.
    pub second: B,
    /// Handle response; senses hover only while disabled.
    pub response: egui::Response,
    /// Visual handle of `handle_thickness`, centered in `hit_rect`.
    pub handle_rect: Rect,
    /// Gap between the panes; the handle's hit zone, at least the minimum target.
    pub hit_rect: Rect,
    /// Share of the split length before the handle; persisted per id, kept while a pane is collapsed.
    pub fraction: Fraction,
    /// Enter on the focused handle or a double click restored [`Splitter::default_fraction`] this pass.
    pub reset: bool,
}

impl Splitter {
    /// Panes side by side at a 0.5 split; `label` is the handle's accessible name.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        Self {
            salt: id_salt.into(),
            key: crate::agent_key!("splitter"),
            label: label.into(),
            axis: Axis::Horizontal,
            default_fraction: Fraction::HALF,
            min_first: 0.0,
            min_second: 0.0,
            handle: 0.0,
            step: None,
            jump: None,
            disabled: false,
            collapse: None,
        }
    }

    /// Collapses `pane` to nothing, past its minimum, and puts the handle at that edge; `None` shows both.
    /// The collapsed pane's closure still runs, invisible. Drags and arrow keys are ignored while collapsed;
    /// Enter and double click report [`SplitterResponse::reset`], so the app can expand.
    pub fn collapse(mut self, pane: Option<Pane>) -> Self {
        self.collapse = pane;
        self
    }

    /// Keeps the current split; the handle senses hover only.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Overrides the `<salt>-splitter` key derived from the id salt.
    pub fn agent_key(mut self, key: impl Into<StableId>) -> Self {
        self.key = AgentKey::Custom(key.into());
        self
    }

    /// Split direction; `Vertical` stacks the panes. Default `Horizontal`.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    /// Initial fraction before any persisted one.
    pub fn default_fraction(mut self, default_fraction: impl Into<Fraction>) -> Self {
        self.default_fraction = default_fraction.into();
        self
    }

    /// Minimum extent of the first pane in points along the axis; negative or non-finite values count as 0.
    pub fn min_first(mut self, min_first: f32) -> Self {
        self.min_first = crate::kit::sanitize::at_least(min_first, 0.0.., "Splitter min_first");
        self
    }

    /// Minimum extent of the second pane in points along the axis; negative or non-finite values count as 0.
    pub fn min_second(mut self, min_second: f32) -> Self {
        self.min_second = crate::kit::sanitize::at_least(min_second, 0.0.., "Splitter min_second");
        self
    }

    /// Visual handle thickness. The gap between panes is this or the minimum target, whichever is
    /// larger. Negative or non-finite values count as 0.
    pub fn handle_thickness(mut self, handle_thickness: f32) -> Self {
        self.handle = crate::kit::sanitize::at_least(handle_thickness, 0.0.., "Splitter handle_thickness");
        self
    }

    /// Arrow-key and AccessKit increment as a fraction of the split length (default [`RangeModel`] step); also
    /// snaps drags. Non-finite values and those below 1e-6 count as 1e-6.
    pub fn step(mut self, step: f64) -> Self {
        self.step = Some(crate::kit::sanitize::at_least(step, MIN_STEP.., "Splitter step"));
        self
    }

    /// Page Up/Down increment as a fraction of the split length (default [`RangeModel`] jump); sanitized as
    /// [`Self::step`].
    pub fn jump(mut self, jump: f64) -> Self {
        self.jump = Some(crate::kit::sanitize::at_least(jump, MIN_STEP.., "Splitter jump"));
        self
    }

    fn range(&self, total: f32) -> RangeModel {
        // Too short for both minimums: the first pane's minimum wins.
        let min = f64::from(self.min_first / total.max(1.0)).min(1.0);
        let max = f64::from(1.0 - self.min_second / total.max(1.0)).max(min);
        let mut range = RangeModel::new(min, max);
        if let Some(step) = self.step {
            range = range.step(step);
        }
        if let Some(jump) = self.jump {
            range = range.jump(jump);
        }
        range
    }

    // Keys, double click and AccessKit from `v`; whether Enter or double click reset it.
    fn input(&self, core: &crate::runtime::Core, handle: &Response, range: &RangeModel, v: f32) -> (f64, bool) {
        let mut v = f64::from(v);
        if self.disabled {
            return (v, false);
        }
        let ctx = core.ctx();
        let reset = handle.double_clicked()
            || handle.has_focus() && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
        if reset {
            return (f64::from(self.default_fraction.get()), true);
        }
        if self.collapse.is_none() {
            if handle.has_focus() {
                v = keyboard(core, handle.id, splitter_track(core, self.axis), range, v);
            }
            if let Some(n) = range.accesskit(core, handle.id, v) {
                v = n;
            }
        }
        (v, false)
    }

    /// Lays out both panes and the handle in the available rect; drag, keys and AccessKit move the split.
    pub fn show<K: Stylekit, A, B>(
        self,
        ui: &mut MineralUi<'_, K>,
        first: impl FnOnce(&mut MineralUi<'_, K>) -> A,
        second: impl FnOnce(&mut MineralUi<'_, K>) -> B,
    ) -> SplitterResponse<A, B> {
        let rt = ui.rt();
        let id = ui.make_persistent_id(self.salt.id_salt());
        let rect = ui.available_rect_before_wrap();
        let horizontal = self.axis == Axis::Horizontal;
        // Side by side, the first pane sits at the reading start; layout runs in logical x.
        let dir = if horizontal {
            rt.core().i18n().direction()
        } else {
            crate::i18n::Direction::Ltr
        };
        let (lo, hi) = if horizontal {
            (rect.left(), rect.right())
        } else {
            (rect.top(), rect.bottom())
        };
        // The gap between panes is the handle's footprint, so its hit zone never overlaps them.
        let gap = Zones::new(egui::Vec2::splat(self.handle).into(), ui.target(MinTarget::Auto))
            .outer
            .width();
        let total = (hi - lo - gap).max(0.0);
        let range = self.range(total);
        let stored = ui.data_mut(|d| *d.get_persisted_mut_or(id, self.default_fraction.get()));
        let mut fraction = stored;

        // Gap slot at `f`, and the visual handle centered in it.
        let span = |f: f32, len: f32| {
            let c = lo + total * f + gap / 2.0;
            let r = c - len / 2.0..=c + len / 2.0;
            if horizontal {
                dir.mirror(Rect::from_x_y_ranges(r, rect.y_range()), rect)
            } else {
                Rect::from_x_y_ranges(rect.x_range(), r)
            }
        };
        let sense = if self.disabled {
            Sense::hover()
        } else {
            Sense::click_and_drag()
        };
        let handle = ui.interact(span(fraction, gap), id.with("handle"), sense);
        if !self.disabled && (handle.hovered() || handle.dragged()) {
            ui.ctx().set_cursor_icon(if horizontal {
                CursorIcon::ResizeColumn
            } else {
                CursorIcon::ResizeRow
            });
        }
        if let Some(p) = handle.interact_pointer_pos().filter(|_| handle.dragged()) {
            let along = if horizontal { dir.mirror_x(p.x, rect) } else { p.y } - gap / 2.0;
            fraction = range.snap(f64::from((along - lo) / total.max(1.0))) as f32;
        }
        let (v, reset) = self.input(
            rt.core(),
            &handle,
            &range,
            if self.collapse.is_some() { stored } else { fraction },
        );
        fraction = range.snap(v) as f32;
        if fraction.to_bits() != stored.to_bits() {
            ui.data_mut(|d| d.insert_persisted(id, fraction));
        }

        let shown = match self.collapse {
            Some(Pane::First) => 0.0,
            Some(Pane::Second) => 1.0,
            None => fraction,
        };
        let h = span(shown, self.handle);
        let slot = span(shown, gap);
        let (r1, r2) = if horizontal {
            let slot = dir.mirror(slot, rect);
            let (a, b) = (
                Rect::from_min_max(rect.min, pos2(slot.left(), rect.bottom())),
                Rect::from_min_max(pos2(slot.right(), rect.top()), rect.max),
            );
            (dir.mirror(a, rect), dir.mirror(b, rect))
        } else {
            (
                Rect::from_min_max(rect.min, pos2(rect.right(), slot.top())),
                Rect::from_min_max(pos2(rect.left(), slot.bottom()), rect.max),
            )
        };
        let pane = |r: Rect, salt: &'static str, pane: Pane| {
            let b = UiBuilder::new().max_rect(r).id_salt((self.salt.id_salt(), salt));
            if self.collapse == Some(pane) { b.invisible() } else { b }
        };
        let a = ui
            .scope_builder(pane(r1, "first", Pane::First), |ui| first(&mut rt.ui(ui)))
            .inner;
        let b = ui
            .scope_builder(pane(r2, "second", Pane::Second), |ui| second(&mut rt.ui(ui)))
            .inner;
        ui.advance_cursor_after_rect(rect);

        emit_semantics(ui.ctx(), &self, &handle, h, &range, shown);
        SplitterResponse {
            first: a,
            second: b,
            response: handle,
            handle_rect: h,
            hit_rect: slot,
            fraction: Fraction::new(fraction),
            reset,
        }
    }
}

/// Handle over a rect resizing one panel: a drag moves its size (dp) by the pointer distance along the track,
/// arrow and Page keys step it within the range as on a [`Splitter`], AccessKit sets it. Paints nothing; the
/// caller paints the handle and emits its [`Role::Splitter`] node.
#[derive(Clone, Copy, Debug)]
#[must_use = "call .show(..)"]
pub struct SizeHandle {
    id: Id,
    rect: Rect,
    track: Track,
    range: RangeModel,
    centered: bool,
}

/// Result of [`SizeHandle::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SizeHandleResponse {
    /// Handle response; `double_clicked` is the app's reset gesture.
    pub response: Response,
    /// Panel size after this pass, dp, snapped to the range.
    pub size: f64,
}

impl SizeHandle {
    /// Hit zone `rect` with egui id `id`; `track` orients the drag and keys, `range` bounds the size.
    pub const fn new(id: Id, rect: Rect, track: Track, range: RangeModel) -> Self {
        Self {
            id,
            rect,
            track,
            range,
            centered: false,
        }
    }

    /// Panel centered on the track, growing on both sides: a drag moves the size by twice the pointer distance;
    /// default off (edge-anchored).
    pub const fn centered(mut self, centered: bool) -> Self {
        self.centered = centered;
        self
    }

    /// Handles input for a panel of `size` this pass.
    pub fn show<K>(self, ui: &MineralUi<'_, K>, size: f64) -> SizeHandleResponse {
        let Self {
            id,
            rect,
            track,
            range,
            centered,
        } = self;
        let core = ui.rt().core();
        let handle = ui.interact(rect, id, Sense::click_and_drag());
        if handle.hovered() || handle.dragged() {
            ui.ctx().set_cursor_icon(match track.axis() {
                Axis::Horizontal => CursorIcon::ResizeColumn,
                Axis::Vertical => CursorIcon::ResizeRow,
            });
        }
        let gain = if centered { 2.0 } else { 1.0 };
        let mut v = size;
        if handle.dragged() {
            v += f64::from(gain * track.sign() * track.axis().main(handle.drag_delta()));
        }
        if handle.has_focus() {
            v = keyboard(core, handle.id, track, &range, v);
        }
        if let Some(n) = range.accesskit(core, handle.id, v) {
            v = n;
        }
        SizeHandleResponse {
            response: handle,
            size: range.snap(v),
        }
    }
}

fn splitter_track(core: &crate::runtime::Core, axis: Axis) -> Track {
    match axis {
        Axis::Horizontal => Track::horizontal(core.i18n().direction()),
        Axis::Vertical => Track::vertical_down(),
    }
}

fn keyboard(core: &crate::runtime::Core, id: Id, track: Track, range: &RangeModel, mut v: f64) -> f64 {
    let ctx = core.ctx();
    let axis = track.axis();
    let cross = match axis {
        Axis::Horizontal => [Key::ArrowUp, Key::ArrowDown],
        Axis::Vertical => [Key::ArrowLeft, Key::ArrowRight],
    };
    kit::interact::lock_arrows(ctx, id, kit::interact::Arrows::Axis(axis));
    for k in kit::interact::plain_key_presses(ctx) {
        if cross.contains(&k) {
            continue;
        }
        if let Some(n) = range.on_key(v, k, track) {
            v = n;
        }
    }
    v
}

fn emit_semantics(
    ctx: &egui::Context,
    splitter: &Splitter,
    handle: &egui::Response,
    rect: Rect,
    range: &RangeModel,
    fraction: f32,
) {
    semantic::emit(ctx, Some(handle.id), rect, || {
        SemanticNode::new(splitter.key.id(&splitter.salt), Role::Splitter, &splitter.label).ranged(
            range.numeric(f64::from(fraction)),
            Availability::disabled_if(splitter.disabled),
        )
    });
}

#[cfg(test)]
mod tests {
    use egui::{Id, Rect, Vec2};

    use super::Splitter;
    use crate::layout::Fraction;
    use crate::{i18n::Direction, test_support::harness};

    #[test]
    fn sibling_splitters_have_distinct_pane_ids() {
        let h = harness(
            |ui, ids: &mut Vec<Id>| {
                ids.clear();
                for salt in ["a", "b"] {
                    let r = Splitter::new(salt, "Split").show(ui, |ui| ui.scope_id(), |ui| ui.scope_id());
                    ids.extend([r.first, r.second]);
                }
            },
            Vec::new(),
        );
        let mut ids = h.state().value.clone();
        ids.sort_by_key(Id::value);
        ids.dedup();
        assert_eq!(ids.len(), 4);
    }

    #[test]
    fn drag_clamps_to_min_sizes() {
        const MIN: (f32, f32) = (100.0, 150.0);
        let mut h = harness(
            |ui, panes: &mut (Rect, Rect, Rect)| {
                let r = Splitter::new("s", "Split").min_first(MIN.0).min_second(MIN.1).show(
                    ui,
                    |ui| ui.max_rect(),
                    |ui| ui.max_rect(),
                );
                *panes = (r.first, r.second, r.hit_rect);
            },
            (Rect::NOTHING, Rect::NOTHING, Rect::NOTHING),
        );
        let screen = h.ctx.content_rect();
        for (to, check) in [(screen.left_center(), 0), (screen.right_center(), 1)] {
            let handle = h.state().value.2.center();
            h.hover_at(handle);
            h.drag_at(handle);
            h.hover_at(to + Vec2::Y);
            h.hover_at(to);
            h.drop_at(to);
            h.run();
            let (first, second, _) = h.state().value;
            let (pane, min) = if check == 0 { (first, MIN.0) } else { (second, MIN.1) };
            assert_eq!(pane.width(), min, "pane {check} stops at its minimum");
        }
    }

    #[test]
    fn rtl_puts_the_first_pane_at_the_start_and_keys_follow_it() {
        let mut h = harness(
            |ui, panes: &mut (Rect, Rect, Fraction)| {
                let i18n = ui.core().i18n();
                if i18n.direction() == Direction::Ltr {
                    i18n.set_locale(&crate::i18n::locale!("ar")).unwrap();
                }
                let r = Splitter::new("s", "Split").show(ui, |ui| ui.max_rect(), |ui| ui.max_rect());
                *panes = (r.first, r.second, r.fraction);
            },
            (Rect::NOTHING, Rect::NOTHING, Fraction::ZERO),
        );
        h.run();
        let (first, second, before) = h.state().value;
        assert!(
            first.left() > second.right(),
            "first pane on the right: {first:?} {second:?}"
        );
        h.key_press(egui::Key::Tab);
        h.run();
        h.key_press(egui::Key::ArrowLeft);
        h.run();
        let (first2, _, after) = h.state().value;
        assert!(after > before, "ArrowLeft grows the start pane in RTL");
        assert!(first2.left() < first.left(), "the handle moves left");
    }

    #[test]
    fn invalid_builder_values_snap_to_the_nearest_valid_one() {
        let s = Splitter::new("s", "split")
            .default_fraction(2.0)
            .min_first(-1.0)
            .min_second(f32::NAN)
            .handle_thickness(f32::NEG_INFINITY)
            .step(0.0)
            .jump(f64::NAN);
        assert_eq!(
            (s.default_fraction.get(), s.min_first, s.min_second, s.handle),
            (1.0, 0.0, 0.0, 0.0)
        );
        assert_eq!((s.step, s.jump), (Some(super::MIN_STEP), Some(super::MIN_STEP)));
    }

    #[test]
    fn enter_resets_to_the_default_fraction() {
        let mut h = harness(
            |ui, out: &mut (Fraction, u32)| {
                let r = Splitter::new("s", "Split")
                    .default_fraction(0.3)
                    .show(ui, |_| (), |_| ());
                out.0 = r.fraction;
                out.1 += u32::from(r.reset);
            },
            (Fraction::ZERO, 0),
        );
        h.run();
        h.key_press(egui::Key::Tab);
        h.run();
        h.key_press(egui::Key::ArrowRight);
        h.run();
        assert!(h.state().value.0.get() > 0.3);
        h.key_press(egui::Key::Enter);
        h.run();
        assert_eq!(h.state().value, (Fraction::new(0.3), 1));
    }

    #[test]
    fn collapsed_pane_gets_no_space_and_keeps_the_fraction() {
        let mut h = harness(
            |ui, out: &mut (Option<super::Pane>, Rect, Rect, Fraction)| {
                let r = Splitter::new("s", "Split").min_first(100.0).collapse(out.0).show(
                    ui,
                    |ui| ui.max_rect(),
                    |ui| ui.max_rect(),
                );
                (out.1, out.2, out.3) = (r.first, r.hit_rect, r.fraction);
            },
            (Some(super::Pane::First), Rect::NOTHING, Rect::NOTHING, Fraction::ZERO),
        );
        h.run();
        let (_, first, hit, fraction) = h.state().value;
        assert_eq!(first.width(), 0.0, "past min_first");
        assert_eq!(hit.left(), first.left(), "handle at the edge");
        assert_eq!(fraction, Fraction::HALF);
        h.key_press(egui::Key::Tab);
        h.run();
        h.key_press(egui::Key::ArrowRight);
        h.run();
        assert_eq!(h.state().value.3, Fraction::HALF, "keys are ignored");
        h.state_mut().value.0 = None;
        h.run();
        assert!(h.state().value.1.width() > 100.0, "expands to the kept fraction");
    }
}
