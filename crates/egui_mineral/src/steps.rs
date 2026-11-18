use std::collections::BTreeSet;

/// State of one step in a [`Steps`] flow.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum StepStatus {
    /// Not reached, completed or skipped yet.
    Pending,
    /// Current step while the flow is not finished.
    Active,
    /// Left by [`Steps::forward`].
    Complete,
    /// Optional step left by [`Steps::skip`].
    Skipped,
    /// Flagged by [`Steps::set_error`]; outranks every status but `Disabled`.
    Error,
    /// Passed over by navigation; outranks every other status.
    Disabled,
}

/// Step progression model: current step, completion, skips, errors, optional and disabled steps.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "RawSteps")]
pub struct Steps {
    count: usize,
    current: usize,
    linear: bool,
    complete: BTreeSet<usize>,
    skipped: BTreeSet<usize>,
    errors: BTreeSet<usize>,
    optional: BTreeSet<usize>,
    disabled: BTreeSet<usize>,
}

#[derive(serde::Deserialize)]
struct RawSteps {
    count: usize,
    current: usize,
    linear: bool,
    complete: BTreeSet<usize>,
    skipped: BTreeSet<usize>,
    errors: BTreeSet<usize>,
    optional: BTreeSet<usize>,
    disabled: BTreeSet<usize>,
}

impl TryFrom<RawSteps> for Steps {
    type Error = String;

    fn try_from(r: RawSteps) -> Result<Self, String> {
        if r.count == 0 {
            return Err("Steps: count must be > 0".into());
        }
        let sets = [
            ("complete", &r.complete),
            ("skipped", &r.skipped),
            ("errors", &r.errors),
            ("optional", &r.optional),
            ("disabled", &r.disabled),
        ];
        for (name, set) in sets {
            if let Some(i) = set.iter().find(|i| **i >= r.count) {
                return Err(format!("Steps: {name} index {i} out of range 0..{}", r.count));
            }
        }
        if r.current >= r.count {
            return Err(format!("Steps: current {} out of range 0..{}", r.current, r.count));
        }
        Ok(Self {
            count: r.count,
            current: r.current,
            linear: r.linear,
            complete: r.complete,
            skipped: r.skipped,
            errors: r.errors,
            optional: r.optional,
            disabled: r.disabled,
        })
    }
}

impl Steps {
    /// Linear flow of `count` steps starting at 0; 0 counts as 1.
    ///
    /// Step indices past the end count as the last step in every method taking one.
    pub fn new(count: usize) -> Self {
        Self {
            count: crate::kit::sanitize::at_least(count, 1.., "Steps count"),
            current: 0,
            linear: true,
            complete: BTreeSet::new(),
            skipped: BTreeSet::new(),
            errors: BTreeSet::new(),
            optional: BTreeSet::new(),
            disabled: BTreeSet::new(),
        }
    }

    /// `true` lets [`Self::select`] jump to any enabled step.
    #[must_use]
    pub fn non_linear(mut self, non_linear: bool) -> Self {
        self.linear = !non_linear;
        self
    }

    /// Marks `step` skippable by [`Self::skip`].
    #[must_use]
    pub fn optional_step(mut self, step: usize) -> Self {
        let step = self.clamp(step);
        self.optional.insert(step);
        self
    }

    /// Disabled steps are passed over by navigation. Step 0 stays current only if enabled.
    #[must_use]
    pub fn disable_step(mut self, step: usize) -> Self {
        let step = self.clamp(step);
        self.disabled.insert(step);
        if self.current == step {
            self.current = self.enabled_from(step, true).unwrap_or(step);
        }
        self
    }

    #[track_caller]
    fn clamp(&self, step: usize) -> usize {
        crate::kit::sanitize::index(step, self.count, "Steps step").unwrap_or_default()
    }

    /// Fixed at construction; at least 1.
    pub fn count(&self) -> usize {
        self.count
    }

    /// Index of the current step; never a disabled one unless all are disabled.
    pub fn current(&self) -> usize {
        self.current
    }

    /// `true` when `step` is marked by [`Self::optional_step`].
    pub fn is_optional(&self, step: usize) -> bool {
        let step = self.clamp(step);
        self.optional.contains(&step)
    }

    /// Error flag of `step`; outranks every status but `Disabled` until cleared or [`Self::forward`].
    pub fn set_error(&mut self, step: usize, error: bool) {
        let step = self.clamp(step);
        if error {
            self.errors.insert(step);
        } else {
            self.errors.remove(&step);
        }
    }

    /// Error flag of `step`, regardless of the [`Self::status`] it shows.
    pub fn error(&self, step: usize) -> bool {
        self.errors.contains(&self.clamp(step))
    }

    /// Priority: `Disabled`, `Error`, `Active`, `Complete`, `Skipped`, `Pending`.
    pub fn status(&self, step: usize) -> StepStatus {
        let step = self.clamp(step);
        if self.disabled.contains(&step) {
            StepStatus::Disabled
        } else if self.errors.contains(&step) {
            StepStatus::Error
        } else if step == self.current && !self.is_finished() {
            StepStatus::Active
        } else if self.complete.contains(&step) {
            StepStatus::Complete
        } else if self.skipped.contains(&step) {
            StepStatus::Skipped
        } else {
            StepStatus::Pending
        }
    }

    fn enabled_from(&self, step: usize, forward: bool) -> Option<usize> {
        if forward {
            (step..self.count).find(|s| !self.disabled.contains(s))
        } else {
            (0..=step.min(self.count - 1))
                .rev()
                .find(|s| !self.disabled.contains(s))
        }
    }

    fn passed(&self, step: usize) -> bool {
        self.disabled.contains(&step) || self.complete.contains(&step) || self.skipped.contains(&step)
    }

    /// Enabled, in range and, in a linear flow, every earlier step complete, skipped or disabled.
    pub fn can_select(&self, step: usize) -> bool {
        step < self.count && !self.disabled.contains(&step) && (!self.linear || (0..step).all(|s| self.passed(s)))
    }

    /// Moves to `step` when [`Self::can_select`]; `false` otherwise.
    pub fn select(&mut self, step: usize) -> bool {
        let ok = self.can_select(step);
        if ok {
            self.current = step;
        }
        ok
    }

    /// Completes the current step and moves to the next enabled one; `false` at the last.
    pub fn forward(&mut self) -> bool {
        self.complete.insert(self.current);
        self.skipped.remove(&self.current);
        self.errors.remove(&self.current);
        self.advance()
    }

    /// Skips an optional current step; `false` and no change for a required one.
    pub fn skip(&mut self) -> bool {
        if !self.is_optional(self.current) {
            return false;
        }
        self.skipped.insert(self.current);
        self.complete.remove(&self.current);
        self.advance();
        true
    }

    fn advance(&mut self) -> bool {
        match self.enabled_from(self.current + 1, true) {
            Some(n) => {
                self.current = n;
                true
            }
            None => false,
        }
    }

    /// Moves to the previous enabled step without changing statuses; `false` at the first.
    pub fn back(&mut self) -> bool {
        match self.current.checked_sub(1).and_then(|s| self.enabled_from(s, false)) {
            Some(p) => {
                self.current = p;
                true
            }
            None => false,
        }
    }

    /// Every step complete, skipped or disabled.
    pub fn is_finished(&self) -> bool {
        (0..self.count).all(|s| self.passed(s))
    }

    /// Clears statuses and returns to the first enabled step; keeps optional and disabled marks.
    pub fn reset(&mut self) {
        self.complete.clear();
        self.skipped.clear();
        self.errors.clear();
        self.current = self.enabled_from(0, true).unwrap_or(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_flow_with_optional_and_disabled() {
        let mut s = Steps::new(4).optional_step(1).disable_step(2);
        assert_eq!(s.status(0), StepStatus::Active);
        assert!(!s.can_select(1), "linear: step 0 not done");
        assert!(!s.skip(), "step 0 is not optional");
        assert!(s.forward());
        assert!(s.skip());
        assert_eq!(s.current(), 3, "disabled step passed over");
        assert_eq!(s.status(1), StepStatus::Skipped);
        assert_eq!(s.status(2), StepStatus::Disabled);
        s.set_error(3, true);
        assert!(s.error(3));
        assert_eq!(s.status(3), StepStatus::Error);
        assert!(!s.forward(), "last step");
        assert!(s.is_finished());
        assert_eq!(s.status(3), StepStatus::Complete);
        assert!(s.back());
        assert_eq!(s.current(), 1);
        s.reset();
        assert_eq!(s.current(), 0);
        assert!(!s.is_finished());
    }

    #[test]
    fn non_linear_select() {
        let mut s = Steps::new(3).non_linear(true).disable_step(1);
        assert!(s.select(2));
        assert!(!s.select(1));
    }

    #[test]
    fn zero_count_and_stale_steps_are_clamped() {
        let mut s = Steps::new(0);
        assert_eq!(s.count(), 1);
        s.set_error(9, true);
        assert_eq!(s.status(0), StepStatus::Error);
        assert_eq!(s.status(7), StepStatus::Error, "out of range reads the last step");
        s.set_error(9, false);
        assert!(!s.error(0));
        assert_eq!(s.status(0), StepStatus::Active);
        let s = Steps::new(3).optional_step(9).disable_step(9);
        assert!(s.is_optional(2));
        assert_eq!(s.status(2), StepStatus::Disabled);
    }
}
