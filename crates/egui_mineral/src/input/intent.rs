use std::time::Duration;

use egui::Context;

/// Target that applies once requested continuously for a delay (hover cards, submenus); a
/// different target restarts the wait.
#[derive(Clone, Debug, PartialEq)]
pub struct HoverIntent<T> {
    pending: Option<(T, f64)>,
}

impl<T> Default for HoverIntent<T> {
    fn default() -> Self {
        Self { pending: None }
    }
}

impl<T: Clone + PartialEq> HoverIntent<T> {
    /// Keeps the wait running while `target` stays pending.
    pub fn request(&mut self, ctx: &Context, target: T) {
        if self.pending.as_ref().is_none_or(|(p, _)| *p != target) {
            self.pending = Some((target, ctx.input(|i| i.time)));
        }
    }

    /// Drops the pending target.
    pub fn cancel(&mut self) {
        self.pending = None;
    }

    /// Target waiting for its delay.
    pub fn pending(&self) -> Option<&T> {
        self.pending.as_ref().map(|(t, _)| t)
    }

    /// Pending target once it waited `delay`; until then schedules a repaint for it.
    pub fn due(&mut self, ctx: &Context, delay: Duration) -> Option<T> {
        let (_, since) = self.pending.as_ref()?;
        let wait = delay.as_secs_f64() - (ctx.input(|i| i.time) - since);
        if wait <= 0.0 {
            self.pending.take().map(|(t, _)| t)
        } else {
            ctx.request_repaint_after(crate::kit::sanitize::duration(wait, "hover intent wait"));
            None
        }
    }
}

/// Delays before a [`HoverIntent::follow`]ed flag flips.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HoverDelays {
    /// Wait before turning on.
    pub open: Duration,
    /// Wait before turning off.
    pub close: Duration,
}

/// Pointer or focus state of a hover target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Engagement {
    /// Hovered or focused.
    Engaged,
    /// Neither hovered nor focused.
    Idle,
}

impl From<bool> for Engagement {
    fn from(engaged: bool) -> Self {
        if engaged { Self::Engaged } else { Self::Idle }
    }
}

impl HoverIntent<bool> {
    /// Moves `open` toward `engagement`: on after `delays.open`, off after `delays.close`.
    pub fn follow(&mut self, ctx: &Context, open: &mut bool, engagement: Engagement, delays: HoverDelays) {
        let engaged = engagement == Engagement::Engaged;
        if engaged == *open {
            self.cancel();
            return;
        }
        self.request(ctx, engaged);
        if let Some(v) = self.due(ctx, if engaged { delays.open } else { delays.close }) {
            *open = v;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use egui::Context;

    use super::{Engagement, HoverDelays, HoverIntent};
    use crate::test_support::{at, run};

    #[test]
    fn hover_intent_waits_for_its_delay() {
        let ctx = Context::default();
        let delays = HoverDelays {
            open: Duration::from_millis(500),
            close: Duration::from_millis(200),
        };
        let mut intent = HoverIntent::default();
        let mut open = false;
        let mut pass = |time: f64, engaged: bool, open: &mut bool| {
            run(&ctx, at(time, vec![]), |ui| {
                intent.follow(ui.ctx(), open, Engagement::from(engaged), delays);
            });
        };
        pass(0.0, true, &mut open);
        pass(0.4, true, &mut open);
        assert!(!open, "still waiting");
        pass(0.5, true, &mut open);
        assert!(open);
        pass(1.0, false, &mut open);
        pass(1.1, true, &mut open);
        pass(1.2, false, &mut open);
        assert!(open, "re-engaging restarts the close wait");
        pass(1.4, false, &mut open);
        assert!(open);
        pass(1.5, false, &mut open);
        assert!(!open);
    }
}
