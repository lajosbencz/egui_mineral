use egui::{Pos2, Response, Ui, Vec2, style::ScrollAnimation};

use super::PanFrame;
use crate::{layout::Axis, motion::Fling, runtime::Core};

const COAST_STOP: f32 = 5.0;

/// Pan arbitration for a single-axis pannable inside a scroll area: claims a pan whose travel
/// at the first frame (past the touch slop) lies mostly on its axis, hands everything else, the
/// overshoot and the release fling to the enclosing [`egui::ScrollArea`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PanClaim {
    // Press origin and axis decision of the pan in progress.
    pan: Option<(Pos2, bool)>,
    chained: bool,
}

impl PanClaim {
    /// Per pass: the frame of the pan that started on `response` to apply when it is claimed on `axis`, otherwise
    /// `None`, the pass then relays the pan and its release velocity to the enclosing scroll area.
    /// Only the start is hit-tested, the pan stays this widget's when the scroll moves it away from the press.
    pub fn update(&mut self, ui: &Ui, core: &Core, axis: Axis, response: &Response, enabled: bool) -> Option<PanFrame> {
        let pan = match self.pan {
            Some((origin, _)) => core.input().pan.filter(|p| p.origin == origin),
            None if enabled => core.input().pan_on(response),
            None => None,
        };
        let Some(p) = pan else {
            self.pan = None;
            return None;
        };
        let first = self.pan.is_none();
        let total = p.pos - p.origin;
        let (_, own) = *self
            .pan
            .get_or_insert((p.origin, axis.main(total).abs() >= axis.cross(total).abs()));
        if p.ended {
            self.pan = None;
        }
        if own {
            return Some(p);
        }
        if first {
            // The pass relays the rest, this widget may scroll out of view mid-pan.
            forward(ui, p.delta);
            core.relay(p.origin);
        }
        None
    }

    /// Hands `over`, the part of a claimed frame's travel the widget could not use, to the enclosing scroll area;
    /// coasts the frame's `axis` release velocity there when anything was chained during the pan.
    pub fn chain(&mut self, core: &Core, ui: &Ui, axis: Axis, frame: &PanFrame, over: f32) {
        if over != 0.0 {
            forward(ui, axis.vec2(over, 0.0));
            self.chained = true;
        }
        if frame.ended && std::mem::take(&mut self.chained) {
            core.coast(axis.vec2(axis.main(frame.velocity), 0.0));
        }
    }
}

/// Pan and release velocity of a declined pan, forwarded to the enclosing scroll area by the pass.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Relay {
    pub(crate) origin: Option<Pos2>,
    pub(crate) coast: Vec2,
}

impl Relay {
    /// Runs before the app's `Ui`; `pan` is this pass's.
    pub(crate) fn step(&mut self, ui: &Ui, fling: &Fling, pan: Option<PanFrame>) {
        if let Some(origin) = self.origin {
            match pan.filter(|p| p.origin == origin) {
                Some(p) => {
                    forward(ui, p.delta);
                    if p.ended {
                        self.origin = None;
                        self.coast = p.velocity;
                    }
                }
                None => self.origin = None,
            }
        }
        if self.coast == Vec2::ZERO {
            return;
        }
        let (dt, down) = ui.input(|i| (i.stable_dt, i.pointer.any_down()));
        if down || self.coast.length() < COAST_STOP {
            self.coast = Vec2::ZERO;
            return;
        }
        forward(ui, self.coast * dt);
        self.coast *= fling.deceleration.clamp(0.0, 0.999).powf(dt * 1000.0);
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(16));
    }
}

fn forward(ui: &Ui, delta: Vec2) {
    if delta != Vec2::ZERO {
        ui.scroll_with_delta_animation(delta, ScrollAnimation::none());
    }
}

#[cfg(test)]
mod tests {
    use egui::{Event, PointerButton, Pos2, Sense, TouchPhase, pos2, vec2};
    use egui_kittest::Harness;

    use super::*;
    use crate::{
        input::synth,
        test_support::{Run, harness},
    };

    const RANGE: f32 = 30.0;

    #[derive(Default)]
    struct Host {
        claim: PanClaim,
        page: f32,
        pos: f32,
    }

    // A pannable inside a vertical page, panning on `axis` over `0..=RANGE`.
    fn page(axis: Axis) -> Harness<'static, Run<Host>> {
        harness(
            move |ui, p: &mut Host| {
                let rt = ui.rt();
                let out = egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(200.0, 100.0), Sense::hover());
                    let r = ui.interact(rect, ui.make_persistent_id("pannable"), Sense::drag());
                    if let Some(f) = p.claim.update(ui, rt.core(), axis, &r, true) {
                        let step = -axis.main(f.delta);
                        let over = p.pos + step - (p.pos + step).clamp(0.0, RANGE);
                        p.claim.chain(rt.core(), ui, axis, &f, -over);
                        p.pos += step - over;
                    }
                    ui.allocate_space(vec2(1.0, 2000.0));
                });
                p.page = out.state.clamped_offset().y;
            },
            Host::default(),
        )
    }

    // eframe web order: pointerdown runs a pass before touchstart arrives.
    fn drag(h: &mut Harness<'_, Run<Host>>, from: Pos2, step: egui::Vec2, n: u32) {
        h.input_mut()
            .events
            .push(synth::pointer_down(from, PointerButton::Primary));
        h.step();
        h.input_mut()
            .events
            .extend([synth::touch(from, TouchPhase::Start), Event::PointerMoved(from)]);
        h.step();
        for i in 1..=n {
            let at = from + step * i as f32;
            h.input_mut()
                .events
                .extend([synth::touch(at, TouchPhase::Move), Event::PointerMoved(at)]);
            h.step();
        }
        let end = from + step * n as f32;
        h.input_mut().events.extend([
            synth::pointer_up(end, PointerButton::Primary),
            Event::PointerGone,
            synth::touch(end, TouchPhase::End),
        ]);
        h.step();
    }

    #[test]
    fn cross_axis_pan_goes_to_the_page_and_coasts() {
        let mut h = page(Axis::Horizontal);
        h.step();
        drag(&mut h, pos2(100.0, 60.0), vec2(0.0, -20.0), 6);
        let released = h.state().value.page;
        h.run_steps(300);
        let s = &h.state().value;
        assert!(s.pos.abs() < f32::EPSILON, "claimed nothing: {}", s.pos);
        assert!(released > 50.0, "page followed the pan: {released}");
        assert!(s.page > released + 20.0, "page coasted: {released} -> {}", s.page);
    }

    #[test]
    fn own_axis_pan_is_claimed_and_the_overshoot_chains() {
        let mut h = page(Axis::Vertical);
        h.step();
        drag(&mut h, pos2(100.0, 80.0), vec2(0.0, -20.0), 6);
        let released = h.state().value.page;
        h.run_steps(300);
        let s = &h.state().value;
        assert!((s.pos - RANGE).abs() < 0.01, "claimed up to its range: {}", s.pos);
        assert!(released > 50.0, "overshoot reached the page: {released}");
        assert!(s.page > released + 20.0, "overshoot coasted: {released} -> {}", s.page);
    }
}
