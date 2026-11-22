use egui::{Pos2, Vec2};

use super::{Gesture, PanFrame, PointerState, SwipeDirection};

const STALE_VELOCITY_SECS: f64 = 0.1;

/// Gesture thresholds supplied by the stylekit through [`Policies`](crate::policy::Policies).
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct GestureConfig {
    /// Max travel for a press to still count as a tap/long-press.
    pub slop: f32,
    /// Hold time that turns a press into [`Gesture::LongPress`].
    pub long_press: std::time::Duration,
    /// Min release velocity (points/s) for a pan to end as a swipe.
    pub swipe_min_velocity: f32,
    /// Rest before leaving the slop for a [`PanMode::Free`](super::PanMode::Free) pannable to claim a touch pan;
    /// a quicker pan scrolls.
    pub pan_hold: std::time::Duration,
}

impl GestureConfig {
    /// Neutral desktop values, as in [`Policies::baseline`](crate::policy::Policies::baseline).
    pub const fn baseline() -> Self {
        Self {
            slop: 8.0,
            long_press: std::time::Duration::from_millis(500),
            swipe_min_velocity: 600.0,
            // UIScrollView delaysContentTouches
            pan_hold: std::time::Duration::from_millis(150),
        }
    }

    /// Max travel for a press to still count as a tap/long-press.
    #[must_use]
    pub const fn slop(self, slop: f32) -> Self {
        Self { slop, ..self }
    }

    /// Hold time that turns a press into [`Gesture::LongPress`].
    #[must_use]
    pub const fn long_press(self, long_press: std::time::Duration) -> Self {
        Self { long_press, ..self }
    }

    /// Rest before leaving the slop for a [`PanMode::Free`](super::PanMode::Free) pannable to claim a touch pan.
    #[must_use]
    pub const fn pan_hold(self, pan_hold: std::time::Duration) -> Self {
        Self { pan_hold, ..self }
    }

    /// Min release velocity (points/s) for a pan to end as a swipe.
    #[must_use]
    pub const fn swipe_min_velocity(self, swipe_min_velocity: f32) -> Self {
        Self {
            swipe_min_velocity,
            ..self
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
enum Phase {
    #[default]
    Idle,
    Pressed {
        start: Pos2,
        start_time: f64,
    },
    LongPressed,
    /// Events of one pass share a timestamp; `velocity` updates only when time advances past `sample_time`.
    Panning {
        start: Pos2,
        held: f64,
        last: Pos2,
        sample: Pos2,
        sample_time: f64,
        velocity: Vec2,
    },
}

/// Pure state machine turning pointer events into [`Gesture`]s.
#[derive(Clone, Debug, Default)]
pub struct GestureRecognizer {
    phase: Phase,
}

impl GestureRecognizer {
    /// `now` is seconds on the egui input clock; `None` unless pressed.
    pub fn long_press_remaining(&self, config: &GestureConfig, now: f64) -> Option<std::time::Duration> {
        let Phase::Pressed { start_time, .. } = self.phase else {
            return None;
        };
        Some(crate::kit::sanitize::duration(
            (start_time + config.long_press.as_secs_f64() - now).max(0.0),
            "long press clock",
        ))
    }

    /// `delta` is left zero; `velocity` is zero once stale at `now`.
    pub fn pan(&self, now: f64) -> Option<PanFrame> {
        match self.phase {
            Phase::Panning {
                start,
                held,
                last,
                sample_time,
                velocity,
                ..
            } => Some(PanFrame {
                origin: start,
                pos: last,
                delta: Vec2::ZERO,
                velocity: fresh(velocity, sample_time, now),
                ended: false,
                held: crate::kit::sanitize::duration(held, "pan hold"),
            }),
            _ => None,
        }
    }

    /// Starts tracking a press; replaces any gesture in progress.
    pub fn down(&mut self, p: &PointerState) {
        self.phase = Phase::Pressed {
            start: p.pos,
            start_time: p.time,
        };
    }

    /// `Some(Gesture::Pan)` once the press left `slop`, and on every later move.
    pub fn moved(&mut self, config: &GestureConfig, p: &PointerState) -> Option<Gesture> {
        match self.phase {
            Phase::Pressed { start, start_time } => {
                if p.pos.distance(start) <= config.slop {
                    return None;
                }
                let (sample, sample_time, velocity) = sample(start, start_time, Vec2::ZERO, p);
                self.phase = Phase::Panning {
                    start,
                    held: (p.time - start_time).max(0.0),
                    last: p.pos,
                    sample,
                    sample_time,
                    velocity,
                };
                Some(Gesture::Pan {
                    pos: p.pos,
                    delta: p.pos - start,
                })
            }
            Phase::Panning {
                start,
                held,
                last,
                sample,
                sample_time,
                velocity,
            } => {
                let delta = p.pos - last;
                let (sample, sample_time, velocity) = self::sample(sample, sample_time, velocity, p);
                self.phase = Phase::Panning {
                    start,
                    held,
                    last: p.pos,
                    sample,
                    sample_time,
                    velocity,
                };
                Some(Gesture::Pan { pos: p.pos, delta })
            }
            Phase::Idle | Phase::LongPressed => None,
        }
    }

    /// Tap, long press or swipe ending the press; `None` for a plain pan or after a fired long press.
    pub fn up(&mut self, config: &GestureConfig, p: &PointerState) -> Option<Gesture> {
        let phase = std::mem::replace(&mut self.phase, Phase::Idle);
        match phase {
            Phase::Pressed { start, start_time } => {
                if p.time - start_time >= config.long_press.as_secs_f64() {
                    Some(Gesture::LongPress { pos: start })
                } else {
                    Some(Gesture::Tap { pos: start })
                }
            }
            Phase::Panning {
                start,
                velocity,
                sample_time,
                ..
            } => {
                let velocity = fresh(velocity, sample_time, p.time);
                (velocity.length() >= config.swipe_min_velocity).then(|| Gesture::Swipe {
                    start,
                    end: p.pos,
                    velocity,
                    direction: direction(velocity),
                })
            }
            Phase::Idle | Phase::LongPressed => None,
        }
    }

    /// Fires [`Gesture::LongPress`] once the hold reaches `long_press` at `now` seconds.
    pub fn tick(&mut self, config: &GestureConfig, now: f64) -> Option<Gesture> {
        if let Phase::Pressed { start, start_time } = self.phase
            && now - start_time >= config.long_press.as_secs_f64()
        {
            self.phase = Phase::LongPressed;
            return Some(Gesture::LongPress { pos: start });
        }
        None
    }

    /// Drops the press without a gesture.
    pub fn cancel(&mut self) {
        self.phase = Phase::Idle;
    }
}

fn sample(from: Pos2, from_time: f64, velocity: Vec2, p: &PointerState) -> (Pos2, f64, Vec2) {
    if p.time > from_time {
        (p.pos, p.time, (p.pos - from) / (p.time - from_time) as f32)
    } else {
        (from, from_time, velocity)
    }
}

fn fresh(velocity: Vec2, sample_time: f64, now: f64) -> Vec2 {
    if now - sample_time > STALE_VELOCITY_SECS {
        Vec2::ZERO
    } else {
        velocity
    }
}

fn direction(v: Vec2) -> SwipeDirection {
    if v.x.abs() >= v.y.abs() {
        if v.x >= 0.0 {
            SwipeDirection::Right
        } else {
            SwipeDirection::Left
        }
    } else if v.y >= 0.0 {
        SwipeDirection::Down
    } else {
        SwipeDirection::Up
    }
}

#[cfg(test)]
mod tests {
    use egui::pos2;

    use super::*;
    use crate::input::{PointerId, PointerKind};

    fn p(x: f32, y: f32, time: f64) -> PointerState {
        PointerState {
            id: PointerId::PRIMARY,
            kind: PointerKind::Synthetic,
            pos: pos2(x, y),
            time,
            pressure: None,
        }
    }

    const C: &GestureConfig = &crate::test_support::GESTURES;

    #[test]
    fn tap_within_slop() {
        let mut r = GestureRecognizer::default();
        r.down(&p(10.0, 10.0, 0.0));
        assert_eq!(r.moved(C, &p(13.0, 12.0, 0.05)), None);
        assert_eq!(
            r.up(C, &p(13.0, 12.0, 0.1)),
            Some(Gesture::Tap { pos: pos2(10.0, 10.0) })
        );
    }

    #[test]
    fn long_press_fires_on_tick_once() {
        let mut r = GestureRecognizer::default();
        r.down(&p(0.0, 0.0, 0.0));
        assert_eq!(r.tick(C, 0.3), None);
        assert_eq!(r.tick(C, 0.6), Some(Gesture::LongPress { pos: pos2(0.0, 0.0) }));
        assert_eq!(r.up(C, &p(0.0, 0.0, 0.7)), None);
    }

    #[test]
    fn slow_drag_pans_without_swipe() {
        let mut r = GestureRecognizer::default();
        r.down(&p(0.0, 0.0, 0.0));
        assert!(matches!(r.moved(C, &p(20.0, 0.0, 0.5)), Some(Gesture::Pan { .. })));
        assert!(matches!(r.moved(C, &p(30.0, 0.0, 1.0)), Some(Gesture::Pan { .. })));
        assert_eq!(r.up(C, &p(30.0, 0.0, 1.1)), None);
    }

    #[test]
    fn pan_velocity_goes_stale_when_held_still() {
        let mut r = GestureRecognizer::default();
        r.down(&p(0.0, 0.0, 0.0));
        r.moved(C, &p(40.0, 0.0, 0.05));
        r.moved(C, &p(80.0, 0.0, 0.1));
        let moving = r.pan(0.12).unwrap();
        assert_eq!(moving.origin, pos2(0.0, 0.0));
        assert!(moving.velocity.x > 0.0);
        assert_eq!(r.pan(0.5).unwrap().velocity, egui::Vec2::ZERO);
        assert!(
            (moving.held.as_secs_f64() - 0.05).abs() < 1e-6,
            "held {:?}",
            moving.held
        );
        r.up(C, &p(80.0, 0.0, 0.5));
        assert!(r.pan(0.5).is_none());
    }

    #[test]
    fn same_pass_moves_do_not_spike_velocity() {
        let mut r = GestureRecognizer::default();
        r.down(&p(0.0, 0.0, 0.0));
        r.moved(C, &p(20.0, 0.0, 0.5));
        r.moved(C, &p(21.0, 0.0, 0.5));
        r.moved(C, &p(30.0, 0.0, 1.0));
        r.moved(C, &p(31.0, 0.0, 1.0));
        assert_eq!(r.up(C, &p(31.0, 0.0, 1.0)), None);
    }

    #[test]
    fn hold_before_release_is_not_swipe() {
        let mut r = GestureRecognizer::default();
        r.down(&p(100.0, 0.0, 0.0));
        r.moved(C, &p(80.0, 0.0, 0.01));
        r.moved(C, &p(20.0, 0.0, 0.05));
        assert_eq!(r.up(C, &p(20.0, 0.0, 2.05)), None);
    }

    #[test]
    fn fast_drag_swipes() {
        let mut r = GestureRecognizer::default();
        r.down(&p(100.0, 0.0, 0.0));
        r.moved(C, &p(80.0, 0.0, 0.01));
        r.moved(C, &p(20.0, 0.0, 0.05));
        let Some(Gesture::Swipe { direction, .. }) = r.up(C, &p(20.0, 0.0, 0.06)) else {
            panic!("expected swipe")
        };
        assert_eq!(direction, SwipeDirection::Left);
    }
}
