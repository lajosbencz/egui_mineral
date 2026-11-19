use std::time::Duration;

use egui::Id;
use egui_mineral::{kit::state::TempState, motion::CubicBezier, runtime::Core};

#[derive(Clone, Copy, Debug, PartialEq)]
struct Tween {
    from: f32,
    to: f32,
    start: f64,
}

impl TempState for Tween {}

// CSS `transition` of one number: eased from the value at the retarget; the first sight snaps.
pub(crate) fn tween(core: &Core, id: Id, target: f32, d: Duration, curve: CubicBezier) -> f32 {
    let ctx = core.ctx();
    let key = id.with("mineral::tween");
    let now = ctx.input(|i| i.time);
    let secs = core.motion().duration(d).as_secs_f32();
    let value = |t: Tween| {
        let k = if secs > 0.0 {
            ((now - t.start) as f32 / secs).clamp(0.0, 1.0)
        } else {
            1.0
        };
        t.from + (t.to - t.from) * curve.ease(k)
    };
    let prev = Tween::try_load(ctx, key);
    let state = match prev {
        Some(t) if (t.to - target).abs() <= f32::EPSILON => t,
        Some(t) => Tween {
            from: value(t),
            to: target,
            start: now,
        },
        None => Tween {
            from: target,
            to: target,
            start: now,
        },
    };
    if prev != Some(state) {
        state.store(ctx, key);
    }
    if (state.from - state.to).abs() > f32::EPSILON && f64::from(secs) > now - state.start {
        egui_mineral::motion::request_frame(ctx);
    }
    value(state)
}
