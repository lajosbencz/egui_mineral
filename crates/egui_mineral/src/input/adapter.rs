use std::{
    sync::{Arc, OnceLock},
    time::Duration,
};

use egui::{Context, Event, Pos2, Sense, TouchPhase, Vec2};

use super::{
    Gesture, GestureConfig, GestureRecognizer, KeyAction, PanFrame, PointerId, PointerKind, PointerState,
    UnifiedInputEvent,
};

#[derive(Default)]
pub(crate) struct InputAdapter {
    recognizer: GestureRecognizer,
    events: Vec<UnifiedInputEvent>,
    kind: Option<PointerKind>,
    force: Option<f32>,
    touch_ended: bool,
    button_down: bool,
    last_kind: Option<PointerKind>,
    synthetic: bool,
    pan_delta: Vec2,
    pan_end: Option<PanFrame>,
}

// Quiet passes share one empty slice instead of allocating.
fn shared(events: &[UnifiedInputEvent]) -> Arc<[UnifiedInputEvent]> {
    static EMPTY: OnceLock<Arc<[UnifiedInputEvent]>> = OnceLock::new();
    if events.is_empty() {
        EMPTY.get_or_init(|| Arc::from([])).clone()
    } else {
        Arc::from(events)
    }
}

impl InputAdapter {
    pub(crate) fn mark_synthetic(&mut self) {
        self.synthetic = true;
    }

    #[tracing::instrument(level = "trace", skip_all)]
    // Second value: delay until a pending long press needs a repaint.
    // Holds the `ctx.input` read: never call it under the runtime's locks.
    pub(crate) fn process(&mut self, ctx: &Context, config: &GestureConfig) -> (InputContext, Option<Duration>) {
        self.events.clear();
        self.pan_delta = Vec2::ZERO;
        self.pan_end = None;
        let (time, pinch) = ctx.input(|i| {
            if let Some(force) = i.events.iter().find_map(|e| match e {
                Event::Touch { force, .. } => Some(*force),
                _ => None,
            }) {
                (self.kind, self.force) = (Some(PointerKind::Touch), force);
                self.last_kind = self.kind;
            }
            self.touch_ended = false;
            for event in &i.events {
                self.translate(config, event, i.time);
            }
            (i.time, i.multi_touch())
        });
        if self.touch_ended {
            (self.kind, self.force) = (None, None);
        }
        if let Some(g) = self.recognizer.tick(config, time) {
            self.events.push(UnifiedInputEvent::Gesture(g));
        }
        if let Some(mt) = pinch
            && (mt.zoom_delta - 1.0).abs() > f32::EPSILON
        {
            self.cancel(time);
            self.events.push(UnifiedInputEvent::Gesture(Gesture::Pinch {
                center: mt.center_pos,
                zoom_delta: mt.zoom_delta,
            }));
        }
        let repaint_after = self.recognizer.long_press_remaining(config, time);
        let modality = self.last_kind.unwrap_or(PointerKind::Mouse);
        self.synthetic = false;
        let pan = self
            .pan_end
            .take()
            .or_else(|| self.recognizer.pan(time))
            .map(|frame| PanFrame {
                delta: self.pan_delta,
                ..frame
            });
        let input = InputContext {
            events: shared(&self.events),
            time,
            modality,
            pan,
        };
        (input, repaint_after)
    }

    fn translate(&mut self, config: &GestureConfig, event: &Event, time: f64) {
        // eframe web delivers `pointerdown` a pass before its `touchstart`: a pointer event without
        // evidence keeps the last kind, only a hover or a wheel is mouse evidence.
        let mouse_evidence = self.kind.is_none()
            && !self.synthetic
            && match event {
                Event::PointerMoved(_) => !self.button_down,
                Event::MouseWheel { .. } => true,
                _ => false,
            };
        if mouse_evidence {
            self.last_kind = Some(PointerKind::Mouse);
        }
        let kind = if self.synthetic {
            PointerKind::Synthetic
        } else {
            self.kind.or(self.last_kind).unwrap_or(PointerKind::Mouse)
        };
        let pressure = self.force.filter(|_| kind == PointerKind::Touch);
        let state = |pos: Pos2| PointerState {
            id: PointerId::PRIMARY,
            kind,
            pos,
            time,
            pressure,
        };
        match event {
            Event::Touch { phase, force, .. } => match phase {
                TouchPhase::Start | TouchPhase::Move => {
                    (self.kind, self.force, self.touch_ended) = (Some(PointerKind::Touch), *force, false);
                    self.last_kind = self.kind;
                }
                TouchPhase::End | TouchPhase::Cancel => self.touch_ended = true,
            },
            Event::PointerMoved(pos) => self.feed(config, UnifiedInputEvent::PointerMoved(state(*pos))),
            Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                ..
            } => {
                self.button_down = *pressed;
                let s = state(*pos);
                self.feed(
                    config,
                    if *pressed {
                        UnifiedInputEvent::PointerDown(s)
                    } else {
                        UnifiedInputEvent::PointerUp(s)
                    },
                );
            }
            Event::PointerGone => {
                self.button_down = false;
                self.cancel(time);
            }
            Event::Key {
                key,
                pressed,
                repeat,
                modifiers,
                ..
            } => {
                self.events.push(UnifiedInputEvent::KeyAction(KeyAction {
                    key: *key,
                    pressed: *pressed,
                    repeat: *repeat,
                    modifiers: *modifiers,
                }));
            }
            _ => {}
        }
    }

    fn cancel(&mut self, time: f64) {
        if let Some(frame) = self.recognizer.pan(time) {
            self.pan_end = Some(PanFrame {
                velocity: Vec2::ZERO,
                ended: true,
                ..frame
            });
        }
        self.recognizer.cancel();
    }

    fn feed(&mut self, config: &GestureConfig, event: UnifiedInputEvent) {
        if let UnifiedInputEvent::PointerDown(p) | UnifiedInputEvent::PointerMoved(p) = &event
            && p.kind != PointerKind::Synthetic
        {
            self.last_kind = Some(p.kind);
        }
        let gesture = match &event {
            UnifiedInputEvent::PointerDown(p) => {
                self.recognizer.down(p);
                None
            }
            UnifiedInputEvent::PointerMoved(p) => self.recognizer.moved(config, p),
            UnifiedInputEvent::PointerUp(p) => {
                let pan = self.recognizer.pan(p.time);
                let gesture = self.recognizer.up(config, p);
                self.pan_end = pan.map(|frame| PanFrame {
                    pos: p.pos,
                    ended: true,
                    ..frame
                });
                gesture
            }
            _ => None,
        };
        if let Some(Gesture::Pan { delta, .. }) = gesture {
            self.pan_delta += delta;
        }
        self.events.push(event);
        if let Some(g) = gesture {
            self.events.push(UnifiedInputEvent::Gesture(g));
        }
    }
}

/// Per-pass unified input: events, time, pointer modality and the active pan.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct InputContext {
    /// Normalized events of this pass, in order.
    pub events: Arc<[UnifiedInputEvent]>,
    /// Seconds on the egui input clock.
    pub time: f64,
    /// Kind of the last non-synthetic pointer, switched to `Mouse` only by a hover or wheel; `Mouse` before any.
    pub modality: PointerKind,
    /// Pan in progress or ended this pass.
    pub pan: Option<PanFrame>,
}

impl Default for InputContext {
    fn default() -> Self {
        Self {
            events: shared(&[]),
            time: 0.0,
            modality: PointerKind::Mouse,
            pan: None,
        }
    }
}

impl InputContext {
    /// Pan that started on `r`, hit-tested like [`Self::tap_on`].
    pub fn pan_on(&self, r: &egui::Response) -> Option<PanFrame> {
        self.pan.filter(|p| hit(r, p.origin, Sense::senses_drag))
    }

    /// Events of this pass aimed inside `r`'s clipped interact rect ([`UnifiedInputEvent::anchor`]) where
    /// [`reaches_layer`] holds for `r`'s layer; `r`'s own child widgets do not block them. Keys have no anchor
    /// and are left out.
    pub fn events_on<'a>(&'a self, r: &'a egui::Response) -> impl Iterator<Item = &'a UnifiedInputEvent> {
        self.events.iter().filter(|e| {
            e.anchor()
                .is_some_and(|pos| r.interact_rect.contains(pos) && reaches_layer(&r.ctx, r.layer_id, pos))
        })
    }

    /// Gesture events of this pass, in order.
    pub fn gestures(&self) -> impl Iterator<Item = &Gesture> {
        self.events.iter().filter_map(|e| match e {
            UnifiedInputEvent::Gesture(g) => Some(g),
            _ => None,
        })
    }

    /// Tap on `r`, hit-tested like egui clicks against the previous pass: inside `r`'s clipped
    /// interact rect on its uncovered layer, under no later click-sensing widget of that layer. A widget
    /// new this pass (a dialog opened by the same tap) gets none. Makes `r.clicked()` true when `r` is enabled and senses clicks.
    pub fn tap_on(&self, r: &mut egui::Response) -> Option<Pos2> {
        let pos = self.gestures().find_map(|g| match g {
            Gesture::Tap { pos } if hit(r, *pos, Sense::senses_click) => Some(*pos),
            _ => None,
        })?;
        if r.enabled() && r.sense.senses_click() {
            // doc(hidden) on egui 0.37; see `Flags::FAKE_PRIMARY_CLICKED`.
            r.flags |= egui::response::Flags::FAKE_PRIMARY_CLICKED;
        }
        Some(pos)
    }
}

/// Input at `pos` reaches `layer`: no window or popup above it covers `pos`, and no modal is open above it.
/// Pair hand-rolled `rect.contains(pos)` checks with it; alone they let input on an overlay reach the
/// widget below.
#[must_use]
pub fn reaches_layer(ctx: &egui::Context, layer: egui::LayerId, pos: Pos2) -> bool {
    ctx.layer_id_at(pos).is_none_or(|l| l == layer) && ctx.memory(|m| m.allows_interaction(layer))
}

fn hit(r: &egui::Response, pos: Pos2, takes: impl Fn(&Sense) -> bool) -> bool {
    let own = r.ctx.viewport(|v| {
        let widgets = &v.prev_pass.widgets;
        let (layer, index) = widgets.order(r.id)?;
        let w = widgets.get(r.id)?;
        let covered = widgets
            .get_layer(layer)
            .skip(index + 1)
            .any(|o| o.enabled && takes(&o.sense) && o.interact_rect.contains(pos));
        Some(layer == r.layer_id && w.interact_rect.contains(pos) && !covered)
    });
    own == Some(true) && reaches_layer(&r.ctx, r.layer_id, pos)
}

#[cfg(test)]
mod tests {
    use egui::{Event, Id, Rect, TouchPhase, pos2};

    use super::*;
    use crate::{
        input::synth,
        test_support::{input, run},
    };

    fn process(a: &mut InputAdapter, ctx: &Context) -> InputContext {
        a.process(ctx, &crate::test_support::GESTURES).0
    }

    fn kinds(ctx: &Context, a: &mut InputAdapter, events: Vec<Event>) -> Vec<PointerKind> {
        let mut got = Vec::new();
        run(ctx, input(events), |ui| {
            got = process(a, ui.ctx())
                .events
                .iter()
                .filter_map(|e| match e {
                    UnifiedInputEvent::PointerDown(p) | UnifiedInputEvent::PointerUp(p) => Some(p.kind),
                    _ => None,
                })
                .collect();
        });
        got
    }

    #[test]
    fn touch_release_is_touch_in_both_backend_orders() {
        let ctx = crate::test_support::ctx();
        let pos = pos2(10.0, 10.0);
        let touch = |phase| synth::touch(pos, phase);
        let down = || synth::pointer_down(pos, egui::PointerButton::Primary);
        let up = || synth::pointer_up(pos, egui::PointerButton::Primary);
        let mut a = InputAdapter::default();
        // egui-winit order
        assert_eq!(
            kinds(&ctx, &mut a, vec![touch(TouchPhase::Start), down()]),
            [PointerKind::Touch]
        );
        assert_eq!(
            kinds(&ctx, &mut a, vec![touch(TouchPhase::End), up(), Event::PointerGone]),
            [PointerKind::Touch]
        );
        // eframe web order
        assert_eq!(
            kinds(&ctx, &mut a, vec![down(), touch(TouchPhase::Start)]),
            [PointerKind::Touch]
        );
        assert_eq!(
            kinds(&ctx, &mut a, vec![up(), Event::PointerGone, touch(TouchPhase::End)]),
            [PointerKind::Touch]
        );
        // no evidence: the kind stays
        assert_eq!(kinds(&ctx, &mut a, vec![down()]), [PointerKind::Touch]);
    }

    fn modality_after(ctx: &Context, a: &mut InputAdapter, events: Vec<Event>) -> PointerKind {
        let mut got = PointerKind::Mouse;
        run(ctx, input(events), |ui| got = process(a, ui.ctx()).modality);
        got
    }

    #[test]
    fn touch_modality_survives_a_touchless_pointerdown_pass() {
        let ctx = crate::test_support::ctx();
        let pos = pos2(10.0, 10.0);
        let touch = |phase| synth::touch(pos, phase);
        let down = || synth::pointer_down(pos, egui::PointerButton::Primary);
        let up = || synth::pointer_up(pos, egui::PointerButton::Primary);
        let wheel = Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            source: egui::MouseWheelSource::Wheel,
            delta: Vec2::Y,
            phase: TouchPhase::Move,
            modifiers: egui::Modifiers::NONE,
        };
        let mut a = InputAdapter::default();
        assert_eq!(modality_after(&ctx, &mut a, vec![]), PointerKind::Mouse);
        assert_eq!(
            modality_after(&ctx, &mut a, vec![touch(TouchPhase::Start)]),
            PointerKind::Touch
        );
        let end = vec![up(), Event::PointerGone, touch(TouchPhase::End)];
        assert_eq!(modality_after(&ctx, &mut a, end), PointerKind::Touch);
        // eframe web: pointerdown pass, then the touchstart pass
        assert_eq!(modality_after(&ctx, &mut a, vec![down()]), PointerKind::Touch);
        let start = vec![touch(TouchPhase::Start), Event::PointerMoved(pos)];
        assert_eq!(modality_after(&ctx, &mut a, start), PointerKind::Touch);
        let end = vec![up(), Event::PointerGone, touch(TouchPhase::End)];
        assert_eq!(modality_after(&ctx, &mut a, end), PointerKind::Touch);
        // hybrid: hover switches to mouse
        assert_eq!(
            modality_after(&ctx, &mut a, vec![Event::PointerMoved(pos)]),
            PointerKind::Mouse
        );
        assert_eq!(
            modality_after(&ctx, &mut a, vec![touch(TouchPhase::Start)]),
            PointerKind::Touch
        );
        assert_eq!(
            modality_after(&ctx, &mut a, vec![touch(TouchPhase::End)]),
            PointerKind::Touch
        );
        assert_eq!(modality_after(&ctx, &mut a, vec![wheel]), PointerKind::Mouse);
    }

    #[test]
    fn tap_skips_widgets_new_this_pass() {
        let ctx = crate::test_support::ctx();
        let pos = pos2(10.0, 10.0);
        let down = || synth::pointer_down(pos, egui::PointerButton::Primary);
        let up = || synth::pointer_up(pos, egui::PointerButton::Primary);
        let mut a = InputAdapter::default();
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(40.0, 40.0));
        let mut hits = (None, None);
        for (events, with_new) in [(vec![], false), (vec![down()], false), (vec![up()], true)] {
            run(&ctx, input(events), |ui| {
                let input = process(&mut a, ui.ctx());
                let mut old = ui.interact(rect, Id::unique("old"), egui::Sense::click());
                hits.0 = input.tap_on(&mut old);
                if with_new {
                    let mut new = ui.interact(rect, Id::unique("new"), egui::Sense::click());
                    hits.1 = input.tap_on(&mut new);
                }
            });
        }
        assert_eq!(hits, (Some(pos), None));
    }

    #[test]
    fn tap_goes_to_the_topmost_click_widget_only() {
        let ctx = crate::test_support::ctx();
        let pos = pos2(10.0, 10.0);
        let mut a = InputAdapter::default();
        let back = Rect::from_min_size(Pos2::ZERO, egui::vec2(80.0, 40.0));
        let front = Rect::from_min_size(Pos2::ZERO, egui::vec2(20.0, 20.0));
        let mut hits = (None, None);
        for events in [
            vec![],
            vec![synth::pointer_down(pos, egui::PointerButton::Primary)],
            vec![synth::pointer_up(pos, egui::PointerButton::Primary)],
        ] {
            run(&ctx, input(events), |ui| {
                let input = process(&mut a, ui.ctx());
                let mut body = ui.interact(back, Id::unique("body"), Sense::click());
                let mut close = ui.interact(front, Id::unique("close"), Sense::click());
                hits = (input.tap_on(&mut body), input.tap_on(&mut close));
            });
        }
        assert_eq!(hits, (None, Some(pos)), "only the widget on top takes the tap");
    }

    // Target at the origin, hit at `pos` on each pass; `above` draws over it after the target.
    fn events_on_target(above: impl Fn(&Context)) -> usize {
        let ctx = crate::test_support::ctx();
        let pos = pos2(10.0, 10.0);
        let mut a = InputAdapter::default();
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(40.0, 40.0));
        let mut got = 0;
        for events in [
            vec![],
            vec![synth::pointer_down(pos, egui::PointerButton::Primary)],
            vec![synth::pointer_up(pos, egui::PointerButton::Primary)],
        ] {
            run(&ctx, input(events), |ui| {
                let input = process(&mut a, ui.ctx());
                let target = ui.interact(rect, Id::unique("target"), Sense::click_and_drag());
                got = input.events_on(&target).count();
                above(ui.ctx());
            });
        }
        got
    }

    #[test]
    fn events_on_reach_an_uncovered_target() {
        assert!(events_on_target(|_| {}) > 0);
    }

    #[test]
    fn events_on_skip_a_target_under_a_window() {
        let got = events_on_target(|ctx| {
            // ast-grep-ignore: raw-egui-area
            egui::Area::new(Id::unique("window"))
                .fixed_pos(Pos2::ZERO)
                .show(ctx, |ui| {
                    ui.allocate_exact_size(egui::vec2(80.0, 80.0), Sense::click());
                });
        });
        assert_eq!(got, 0);
    }

    #[test]
    fn events_on_skip_a_target_under_a_modal() {
        let got = events_on_target(|ctx| {
            // ast-grep-ignore: raw-egui-area
            egui::Modal::new(Id::unique("modal")).show(ctx, |ui| {
                ui.label("dialog");
            });
        });
        assert_eq!(got, 0, "the modal blocks the layer below even outside its frame");
    }
}
