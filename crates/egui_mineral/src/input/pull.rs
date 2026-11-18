use std::time::Duration;

use crate::kit::sanitize::{at_least, nonzero_duration};

const MIN_CONFIG: f32 = 1e-3;

/// Pull-to-refresh tuning supplied by the stylekit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PullConfig {
    /// Fraction of drag/wheel travel that becomes pull distance.
    pub resistance: f32,
    /// Distance that triggers.
    pub threshold: f32,
    /// Time without wheel input before an unfinished wheel pull resets.
    pub wheel_idle: Duration,
}

/// Pan of the pass along the pull axis (positive pulls past the edge).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PullDrag {
    /// Travel along the pull axis this pass, in points.
    pub delta: f32,
    /// Pan released (or cancelled) this pass.
    pub ended: bool,
}

/// Per-pass input of the pull-to-refresh gesture.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PullInput {
    /// Content is scrolled to the pulled edge.
    pub at_edge: bool,
    /// Pan that began inside the pull region, if any.
    pub drag: Option<PullDrag>,
    /// Wheel delta over the region (positive pulls).
    pub wheel: f32,
    /// Seconds on the egui input clock.
    pub now: f64,
}

/// Result of one pull-to-refresh pass.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct PullStep {
    /// Pull crossed `threshold`: on drag release, or while wheeling.
    pub triggered: bool,
    /// Wheel delta went to the pull; the scroll area should not also scroll.
    pub wheel_consumed: bool,
    /// Time until a pending wheel reset needs a pass.
    pub repaint_after: Option<Duration>,
}

/// Pull-past-the-edge tracker (pull to refresh). A drag pull triggers on release past the
/// threshold; a wheel pull triggers on crossing it.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pull {
    distance: f32,
    held: bool,
    wheel_at: Option<f64>,
}

impl Pull {
    /// Pulled distance in points after resistance; 0 when idle.
    pub fn distance(&self) -> f32 {
        self.distance
    }

    /// Drag held or wheel pull pending reset.
    pub fn live(&self) -> bool {
        self.held || self.wheel_at.is_some()
    }

    /// Drops any pull in progress.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Advances one pass. `resistance` and `threshold` below 0.001 or non-finite count as 0.001; a zero
    /// `wheel_idle` as 1 ms.
    pub fn update(&mut self, cfg: &PullConfig, input: PullInput) -> PullStep {
        let cfg = &PullConfig {
            resistance: at_least(cfg.resistance, MIN_CONFIG.., "PullConfig resistance"),
            threshold: at_least(cfg.threshold, MIN_CONFIG.., "PullConfig threshold"),
            wheel_idle: nonzero_duration(cfg.wheel_idle, "PullConfig wheel_idle"),
        };
        let mut step = PullStep::default();
        match input.drag {
            Some(d) if self.held || (input.at_edge && d.delta > 0.0) => {
                self.held = true;
                self.distance = (self.distance + d.delta * cfg.resistance).max(0.0);
                if d.ended {
                    step.triggered |= self.release(cfg);
                }
            }
            None if self.held => step.triggered |= self.release(cfg),
            _ => {}
        }
        if input.at_edge && input.wheel > 0.0 {
            self.distance += input.wheel * cfg.resistance;
            self.wheel_at = Some(input.now);
            step.wheel_consumed = true;
            if self.distance >= cfg.threshold {
                step.triggered = true;
                self.distance = 0.0;
                self.wheel_at = None;
            }
        } else if let Some(at) = self.wheel_at {
            let idle = Duration::try_from_secs_f64(input.now - at).unwrap_or(Duration::MAX);
            step.repaint_after = cfg.wheel_idle.checked_sub(idle).filter(|w| !w.is_zero());
            if step.repaint_after.is_none() {
                self.distance = 0.0;
                self.wheel_at = None;
            }
        }
        step
    }

    fn release(&mut self, cfg: &PullConfig) -> bool {
        self.held = false;
        let triggered = self.distance >= cfg.threshold;
        self.distance = 0.0;
        triggered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pull_triggers_on_release_past_threshold_and_resets_idle_wheel() {
        let cfg = PullConfig {
            resistance: 0.5,
            threshold: 80.0,
            wheel_idle: Duration::from_millis(200),
        };
        let drag = |delta, ended| PullInput {
            at_edge: true,
            drag: Some(PullDrag { delta, ended }),
            ..PullInput::default()
        };
        let mut p = Pull::default();
        assert!(!p.update(&cfg, drag(100.0, false)).triggered);
        assert!(p.live() && (p.distance() - 50.0).abs() < 1e-4);
        assert!(!p.update(&cfg, drag(0.0, true)).triggered, "50 < 80");
        assert!(!p.live() && p.distance() == 0.0);

        p.update(&cfg, drag(200.0, false));
        assert!(
            p.update(&cfg, PullInput::default()).triggered,
            "pan lost counts as release"
        );

        let wheel = |now| PullInput {
            at_edge: true,
            wheel: 100.0,
            now,
            ..PullInput::default()
        };
        assert!(p.update(&cfg, wheel(0.0)).wheel_consumed);
        let idle = PullInput {
            now: 0.1,
            ..PullInput::default()
        };
        assert!(p.update(&cfg, idle).repaint_after.is_some());
        p.update(&cfg, PullInput { now: 0.3, ..idle });
        assert!(!p.live() && p.distance() == 0.0);

        p.update(&cfg, wheel(1.0));
        assert!(p.update(&cfg, wheel(1.05)).triggered && p.distance() == 0.0);

        let off_edge = PullInput {
            at_edge: false,
            ..drag(100.0, false)
        };
        assert!(!p.update(&cfg, off_edge).triggered && !p.live());
    }
}
