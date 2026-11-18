use std::time::Duration;

use egui::{Context, Id};

/// Key-repeat delay and interval.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Repeat {
    /// Hold time before the first repeat.
    pub delay: Duration,
    /// Time between repeats after `delay`.
    pub interval: Duration,
}

/// Pointer state of a press-and-hold control.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Hold {
    /// Pointer pressed on the control this pass.
    Down,
    /// Released; resets the hold.
    Up,
}

#[derive(Clone, Copy, Default)]
struct HoldState {
    start: f64,
    emitted: u32,
    pass: u64,
}

/// Restarts after a pass without a call (the button stopped rendering while held); timing from
/// [`Policies::repeat`](crate::policy::Policies::repeat).
pub fn hold_repeat(core: &crate::runtime::Core, id: Id, pointer: Hold) -> u32 {
    let ctx = core.ctx();
    let Repeat { delay, interval } = core.policies().repeat;
    let timing = (delay.as_secs_f64(), interval.as_secs_f64());
    let key = id.with("mineral::hold_repeat");
    if pointer == Hold::Up {
        ctx.data_mut(|d| d.remove::<HoldState>(key));
        return 0;
    }
    let (now, pass) = (ctx.input(|i| i.time), ctx.cumulative_pass_nr());
    let HoldState { start, emitted, .. } = ctx
        .data_mut(|d| d.remove_temp::<HoldState>(key))
        .filter(|h| pass.saturating_sub(h.pass) <= 1)
        .unwrap_or(HoldState {
            start: now,
            emitted: 0,
            pass,
        });
    let held = now - start;
    let (delay, interval) = timing;
    let due = if held >= delay {
        #[expect(clippy::cast_sign_loss, reason = "held >= delay; `as` saturates large counts")]
        let repeats = ((held - delay) / interval).floor() as u32;
        repeats.saturating_add(2)
    } else {
        1
    };
    ctx.data_mut(|d| {
        d.insert_temp(
            key,
            HoldState {
                start,
                emitted: due,
                pass,
            },
        );
    });
    let wait = if held < delay { delay - held } else { interval };
    ctx.request_repaint_after(crate::kit::sanitize::duration(wait, "hold repeat wait"));
    // Time can step back (app-supplied `RawInput::time`).
    due.saturating_sub(emitted)
}

/// Steps due for a press-and-hold button: repeats while held, else one per click. A click that ends a
/// hold adds none.
pub fn button_steps(core: &crate::runtime::Core, response: &egui::Response, id: Id) -> u32 {
    let ctx = core.ctx();
    let held = id.with("held");
    let down = response.is_pointer_button_down_on();
    let n = hold_repeat(core, id, if down { Hold::Down } else { Hold::Up });
    if n > 0 {
        ctx.data_mut(|d| d.insert_temp(held, true));
    }
    if down {
        return n;
    }
    let was_held = ctx.data_mut(|d| d.remove_temp::<bool>(held)).unwrap_or(false);
    n.saturating_add(u32::from(response.clicked() && !was_held))
}

/// Outcome of an update: value changed or not.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Change {
    /// Bound value changed this pass; restarts the debounce.
    Changed,
    /// No change this pass.
    Unchanged,
}

/// Time-based debounce on the egui input clock.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Debounce {
    delay: f64,
}

impl Debounce {
    /// Fires once `delay` passed without a change.
    pub fn new(delay: Duration) -> Self {
        Self {
            delay: delay.as_secs_f64(),
        }
    }

    /// `true` once `delay` passed since the last [`Change::Changed`].
    pub fn update(&self, ctx: &Context, id: Id, change: Change) -> bool {
        let key = id.with("mineral::debounce");
        let now = ctx.input(|i| i.time);
        if change == Change::Changed {
            ctx.data_mut(|d| d.insert_temp(key, now));
        }
        let Some(last) = ctx.data(|d| d.get_temp::<f64>(key)) else {
            return false;
        };
        let left = last + self.delay - now;
        if left <= 0.0 {
            ctx.data_mut(|d| d.remove::<f64>(key));
            true
        } else {
            ctx.request_repaint_after(crate::kit::sanitize::duration(left, "debounce wait"));
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{at, run};

    fn pass(ctx: &Context, time: f64, mut add: impl FnMut(&Context)) {
        run(ctx, at(time, vec![]), |ui| add(ui.ctx()));
    }

    #[test]
    fn hold_restarts_after_unrendered_pass() {
        let ctx = crate::test_support::ctx();
        let rt = crate::test_support::runtime(&ctx);
        let id = Id::unique("b");
        let mut n = 0;
        pass(&ctx, 0.0, |_| n = hold_repeat(rt.core(), id, Hold::Down));
        assert_eq!(n, 1);
        pass(&ctx, 1.0, |_| {});
        pass(&ctx, 10.0, |_| n = hold_repeat(rt.core(), id, Hold::Down));
        assert_eq!(n, 1);
    }

    #[test]
    fn hold_repeats_after_delay_at_interval() {
        let Repeat { delay, interval } = crate::test_support::policies().repeat;
        let (delay, interval) = (delay.as_secs_f64(), interval.as_secs_f64());
        let ctx = crate::test_support::ctx();
        let rt = crate::test_support::runtime(&ctx);
        let id = Id::unique("b");
        let steps = |time: f64, pointer: Hold| {
            let mut n = 0;
            pass(&ctx, time, |_| n = hold_repeat(rt.core(), id, pointer));
            n
        };
        assert_eq!(steps(0.0, Hold::Down), 1);
        assert_eq!(steps(delay / 2.0, Hold::Down), 0);
        assert_eq!(steps(delay, Hold::Down), 1);
        assert_eq!(steps(delay + interval * 1.5, Hold::Down), 1);
        assert_eq!(steps(delay + interval * 1.6, Hold::Down), 0);
        assert_eq!(steps(delay + interval * 1.7, Hold::Up), 0);
        assert_eq!(steps(delay + interval * 1.8, Hold::Down), 1, "release resets the hold");
    }

    #[test]
    fn button_steps_repeat_while_held_and_release_adds_none() {
        use egui::{Pos2, Rect, Sense, vec2};

        use crate::{input::synth::pointer_up, test_support::press};

        let Repeat { delay, interval } = crate::test_support::policies().repeat;
        let (delay, interval) = (delay.as_secs_f64(), interval.as_secs_f64());
        let ctx = crate::test_support::ctx();
        let rt = crate::test_support::runtime(&ctx);
        let id = Id::unique("b");
        let rect = Rect::from_min_size(Pos2::new(10.0, 10.0), vec2(40.0, 40.0));
        let steps = |time: f64, events: Vec<egui::Event>| {
            let mut n = 0;
            run(&ctx, at(time, events), |ui| {
                let response = ui.interact(rect, id, Sense::click());
                n = button_steps(rt.core(), &response, id);
            });
            n
        };
        steps(0.0, vec![egui::Event::PointerMoved(rect.center())]);
        assert_eq!(steps(1.0, press(rect.center())), 1);
        assert_eq!(steps(1.0 + delay, vec![]), 1);
        assert_eq!(steps(1.0 + delay + interval * 1.5, vec![]), 1);
        assert_eq!(
            steps(
                1.0 + delay + interval * 1.6,
                vec![pointer_up(rect.center(), egui::PointerButton::Primary)]
            ),
            0
        );
    }

    #[test]
    fn debounce_fires_after_delay_and_changed_restarts() {
        const DELAY: Duration = Duration::from_millis(300);
        let d = DELAY.as_secs_f64();
        let ctx = crate::test_support::ctx();
        let id = Id::unique("d");
        let debounce = Debounce::new(DELAY);
        let fired = |time: f64, change: Change| {
            let mut fired = false;
            pass(&ctx, time, |ctx| fired = debounce.update(ctx, id, change));
            fired
        };
        assert!(!fired(0.0, Change::Changed));
        assert!(!fired(d / 2.0, Change::Unchanged));
        assert!(!fired(d * 0.75, Change::Changed));
        assert!(!fired(d * 1.25, Change::Unchanged), "restarted at 0.75 * delay");
        assert!(fired(d * 2.0, Change::Unchanged));
        assert!(!fired(d * 3.0, Change::Unchanged), "fires once");
    }
}
