use std::{collections::VecDeque, time::Duration};

use egui::Context;

use crate::semantic::Live;

/// Identifier of a queued [`Toast`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ToastId(u64);

/// Queued message with payload, duration and politeness.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct Toast<T> {
    /// Stable handle for [`ToastQueue::dismiss`] and [`ToastQueue::pause`].
    pub id: ToastId,
    /// App data the stylekit renders.
    pub payload: T,
    /// `None` stays until dismissed.
    pub duration: Option<Duration>,
    /// Assertive interrupts screen readers (errors); polite waits.
    pub live: Live,
}

#[derive(Clone, Debug)]
struct Active<T> {
    toast: Toast<T>,
    remaining: Option<f64>,
    paused: bool,
}

/// Queue showing up to `max_visible` toasts with timed expiry.
#[derive(Clone, Debug)]
pub struct ToastQueue<T> {
    max_visible: usize,
    default_duration: Duration,
    queue: VecDeque<Toast<T>>,
    active: Vec<Active<T>>,
    next_id: u64,
    last_tick: Option<f64>,
}

impl<T> ToastQueue<T> {
    /// `max_visible` toasts at once; [`Self::push`] uses `default_duration`. 0 counts as 1 toast and
    /// a zero duration as 1 ms.
    pub fn new(max_visible: usize, default_duration: Duration) -> Self {
        Self {
            max_visible: crate::kit::sanitize::at_least(max_visible, 1.., "ToastQueue max_visible"),
            default_duration: crate::kit::sanitize::nonzero_duration(default_duration, "ToastQueue duration"),
            queue: VecDeque::new(),
            active: Vec::new(),
            next_id: 0,
            last_tick: None,
        }
    }

    /// Upper bound on [`Self::visible`].
    pub fn max_visible(&self) -> usize {
        self.max_visible
    }

    /// Polite toast with the default duration.
    pub fn push(&mut self, payload: T) -> ToastId {
        let d = self.default_duration;
        self.push_with(payload, Some(d), Live::Polite)
    }

    /// `None` stays until dismissed; a zero duration counts as 1 ms.
    pub fn push_with(&mut self, payload: T, duration: Option<Duration>, live: Live) -> ToastId {
        let duration = duration.map(|d| crate::kit::sanitize::nonzero_duration(d, "Toast duration"));
        let id = ToastId(self.next_id);
        self.next_id += 1;
        let toast = Toast {
            id,
            payload,
            duration,
            live,
        };
        if live == Live::Assertive {
            let at = self.queue.iter().take_while(|t| t.live == Live::Assertive).count();
            self.queue.insert(at, toast);
        } else {
            self.queue.push_back(toast);
        }
        id
    }

    /// Removes the toast whether visible or queued.
    pub fn dismiss(&mut self, id: ToastId) {
        self.active.retain(|a| a.toast.id != id);
        self.queue.retain(|t| t.id != id);
    }

    /// Drops visible and queued toasts.
    pub fn clear(&mut self) {
        self.active.clear();
        self.queue.clear();
    }

    /// Freezes the countdown of a visible toast (hover, focus) until [`Self::resume`]; no-op for queued ones.
    pub fn pause(&mut self, id: ToastId) {
        self.set_paused(id, true);
    }

    /// Restarts the countdown frozen by [`Self::pause`].
    pub fn resume(&mut self, id: ToastId) {
        self.set_paused(id, false);
    }

    fn set_paused(&mut self, id: ToastId, paused: bool) {
        if let Some(a) = self.active.iter_mut().find(|a| a.toast.id == id) {
            a.paused = paused;
        }
    }

    /// `now` is seconds on the egui input clock; returns the time to the next expiry.
    pub fn tick(&mut self, now: f64) -> Option<std::time::Duration> {
        let dt = self.last_tick.map_or(0.0, |t| (now - t).max(0.0));
        self.last_tick = Some(now);
        for a in &mut self.active {
            if !a.paused
                && let Some(r) = &mut a.remaining
            {
                *r -= dt;
            }
        }
        self.active.retain(|a| a.remaining.is_none_or(|r| r > 0.0));
        while self.active.len() < self.max_visible
            && let Some(t) = self.queue.pop_front()
        {
            self.active.push(Active {
                remaining: t.duration.map(|d| d.as_secs_f64()),
                toast: t,
                paused: false,
            });
        }
        self.active
            .iter()
            .filter(|a| !a.paused)
            .filter_map(|a| a.remaining)
            .min_by(f64::total_cmp)
            .map(|s| std::time::Duration::try_from_secs_f64(s).unwrap_or(std::time::Duration::MAX))
    }

    /// [`Self::tick`] at the input time, then a repaint for the next expiry.
    pub fn update(&mut self, ctx: &Context) {
        if let Some(left) = self.tick(ctx.input(|i| i.time)) {
            ctx.request_repaint_after(left);
        }
    }

    /// Oldest first; at most [`Self::max_visible`].
    pub fn visible(&self) -> impl Iterator<Item = &Toast<T>> {
        self.active.iter().map(|a| &a.toast)
    }

    /// Queued toasts not yet visible.
    pub fn pending(&self) -> usize {
        self.queue.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timers_pause_and_promotion() {
        let mut q = ToastQueue::new(1, Duration::from_secs(4));
        let a = q.push("a");
        let b = q.push_with("b", Some(Duration::from_secs(1)), Live::Polite);
        assert_eq!(q.tick(0.0), Some(Duration::from_secs(4)));
        q.pause(a);
        assert_eq!(q.tick(10.0), None, "paused toast never expires");
        q.resume(a);
        q.tick(13.9);
        assert_eq!(q.visible().next().map(|t| t.payload), Some("a"));
        q.tick(14.1);
        assert_eq!(q.visible().next().map(|t| t.payload), Some("b"));
        let err = q.push_with("err", None, Live::Assertive);
        q.dismiss(b);
        q.tick(14.2);
        assert_eq!(q.visible().next().map(|t| t.id), Some(err));
    }

    #[test]
    fn assertive_toasts_jump_polite_ones_in_arrival_order() {
        let mut q = ToastQueue::new(3, Duration::from_secs(4));
        q.push("polite");
        q.push_with("first", None, Live::Assertive);
        q.push_with("second", None, Live::Assertive);
        q.tick(0.0);
        let order: Vec<_> = q.visible().map(|t| t.payload).collect();
        assert_eq!(order, ["first", "second", "polite"]);
    }

    #[test]
    fn zero_counts_and_durations_are_raised() {
        let mut q = ToastQueue::new(0, Duration::ZERO);
        assert_eq!(q.max_visible(), 1);
        q.push_with("a", Some(Duration::ZERO), Live::Polite);
        assert_eq!(q.tick(0.0), Some(Duration::from_millis(1)));
    }
}
