use std::collections::BTreeSet;

use crate::select::ItemKey;

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

/// Step progression over app keys `K` in flow order: current step, completion, skips, errors, optional and
/// disabled steps. A key the flow lacks is reported and reads as `Pending` or `false`.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(
    try_from = "RawSteps<K>",
    bound(deserialize = "K: ItemKey + serde::Deserialize<'de>")
)]
pub struct Steps<K = usize> {
    keys: Vec<K>,
    current: usize,
    linear: bool,
    complete: BTreeSet<usize>,
    skipped: BTreeSet<usize>,
    errors: BTreeSet<usize>,
    optional: BTreeSet<usize>,
    disabled: BTreeSet<usize>,
}

#[derive(serde::Deserialize)]
#[serde(bound(deserialize = "K: serde::Deserialize<'de>"))]
struct RawSteps<K> {
    keys: Vec<K>,
    current: usize,
    linear: bool,
    complete: BTreeSet<usize>,
    skipped: BTreeSet<usize>,
    errors: BTreeSet<usize>,
    optional: BTreeSet<usize>,
    disabled: BTreeSet<usize>,
}

impl<K: ItemKey> TryFrom<RawSteps<K>> for Steps<K> {
    type Error = String;

    fn try_from(r: RawSteps<K>) -> Result<Self, String> {
        let count = r.keys.len();
        if r.keys.iter().enumerate().any(|(i, k)| r.keys[..i].contains(k)) {
            return Err("Steps: repeated step key".into());
        }
        let sets = [
            ("complete", &r.complete),
            ("skipped", &r.skipped),
            ("errors", &r.errors),
            ("optional", &r.optional),
            ("disabled", &r.disabled),
        ];
        for (name, set) in sets {
            if let Some(i) = set.iter().find(|i| **i >= count) {
                return Err(format!("Steps: {name} index {i} out of range 0..{count}"));
            }
        }
        if r.current >= count.max(1) {
            return Err(format!("Steps: current {} out of range 0..{count}", r.current));
        }
        Ok(Self {
            keys: r.keys,
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

impl<K: ItemKey> Steps<K> {
    /// Linear flow over `keys` in order, starting at the first (`Steps::new(0..4)` for plain indices); a repeated
    /// key is reported and dropped.
    pub fn new(keys: impl IntoIterator<Item = K>) -> Self {
        let mut unique: Vec<K> = Vec::new();
        for key in keys {
            if unique.contains(&key) {
                crate::kit::sanitize::report(format_args!("Steps: repeated step key"));
            } else {
                unique.push(key);
            }
        }
        Self {
            keys: unique,
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
    pub fn optional_step(mut self, step: &K) -> Self {
        if let Some(i) = self.index(step) {
            self.optional.insert(i);
        }
        self
    }

    /// Disabled steps are passed over by navigation. The first step stays current only if enabled.
    #[must_use]
    pub fn disable_step(mut self, step: &K) -> Self {
        if let Some(i) = self.index(step) {
            self.disabled.insert(i);
            if self.current == i {
                self.current = self.enabled_from(i, true).unwrap_or(i);
            }
        }
        self
    }

    #[track_caller]
    fn index(&self, step: &K) -> Option<usize> {
        let found = self.keys.iter().position(|k| k == step);
        if found.is_none() {
            crate::kit::sanitize::report(format_args!("Steps: step key not in the flow"));
        }
        found
    }

    /// Steps in flow order.
    pub fn keys(&self) -> &[K] {
        &self.keys
    }

    /// Number of steps.
    pub fn count(&self) -> usize {
        self.keys.len()
    }

    /// Current step; never a disabled one unless all are disabled. `None` for an empty flow.
    pub fn current(&self) -> Option<&K> {
        self.keys.get(self.current)
    }

    /// `true` when `step` is marked by [`Self::optional_step`].
    pub fn is_optional(&self, step: &K) -> bool {
        self.index(step).is_some_and(|i| self.optional.contains(&i))
    }

    /// Error flag of `step`; outranks every status but `Disabled` until cleared or [`Self::forward`].
    pub fn set_error(&mut self, step: &K, error: bool) {
        let Some(i) = self.index(step) else { return };
        if error {
            self.errors.insert(i);
        } else {
            self.errors.remove(&i);
        }
    }

    /// Error flag of `step`, regardless of the [`Self::status`] it shows.
    pub fn error(&self, step: &K) -> bool {
        self.index(step).is_some_and(|i| self.errors.contains(&i))
    }

    /// Priority: `Disabled`, `Error`, `Active`, `Complete`, `Skipped`, `Pending`.
    pub fn status(&self, step: &K) -> StepStatus {
        let Some(i) = self.index(step) else {
            return StepStatus::Pending;
        };
        if self.disabled.contains(&i) {
            StepStatus::Disabled
        } else if self.errors.contains(&i) {
            StepStatus::Error
        } else if i == self.current && !self.is_finished() {
            StepStatus::Active
        } else if self.complete.contains(&i) {
            StepStatus::Complete
        } else if self.skipped.contains(&i) {
            StepStatus::Skipped
        } else {
            StepStatus::Pending
        }
    }

    fn enabled_from(&self, step: usize, forward: bool) -> Option<usize> {
        if forward {
            (step..self.keys.len()).find(|s| !self.disabled.contains(s))
        } else {
            let last = self.keys.len().checked_sub(1)?;
            (0..=step.min(last)).rev().find(|s| !self.disabled.contains(s))
        }
    }

    fn passed(&self, step: usize) -> bool {
        self.disabled.contains(&step) || self.complete.contains(&step) || self.skipped.contains(&step)
    }

    fn selectable(&self, i: usize) -> bool {
        !self.disabled.contains(&i) && (!self.linear || (0..i).all(|s| self.passed(s)))
    }

    /// Enabled and, in a linear flow, every earlier step complete, skipped or disabled.
    pub fn can_select(&self, step: &K) -> bool {
        self.index(step).is_some_and(|i| self.selectable(i))
    }

    /// Moves to `step` when [`Self::can_select`]; `false` otherwise.
    pub fn select(&mut self, step: &K) -> bool {
        match self.index(step).filter(|i| self.selectable(*i)) {
            Some(i) => {
                self.current = i;
                true
            }
            None => false,
        }
    }

    /// Completes the current step and moves to the next enabled one; `false` at the last.
    pub fn forward(&mut self) -> bool {
        if self.keys.is_empty() {
            return false;
        }
        self.complete.insert(self.current);
        self.skipped.remove(&self.current);
        self.errors.remove(&self.current);
        self.advance()
    }

    /// Skips an optional current step; `false` and no change for a required one.
    pub fn skip(&mut self) -> bool {
        if !self.optional.contains(&self.current) {
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
        (0..self.keys.len()).all(|s| self.passed(s))
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
        let mut s = Steps::new(0..4).optional_step(&1).disable_step(&2);
        assert_eq!(s.status(&0), StepStatus::Active);
        assert!(!s.can_select(&1), "linear: step 0 not done");
        assert!(!s.skip(), "step 0 is not optional");
        assert!(s.forward());
        assert!(s.skip());
        assert_eq!(s.current(), Some(&3), "disabled step passed over");
        assert_eq!(s.status(&1), StepStatus::Skipped);
        assert_eq!(s.status(&2), StepStatus::Disabled);
        s.set_error(&3, true);
        assert!(s.error(&3));
        assert_eq!(s.status(&3), StepStatus::Error);
        assert!(!s.forward(), "last step");
        assert!(s.is_finished());
        assert_eq!(s.status(&3), StepStatus::Complete);
        assert!(s.back());
        assert_eq!(s.current(), Some(&1));
        s.reset();
        assert_eq!(s.current(), Some(&0));
        assert!(!s.is_finished());
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, crate::semantic::KeySalt)]
    enum Wizard {
        Account,
        Billing,
        Confirm,
    }

    #[test]
    fn keyed_flow_selects_by_key_and_ignores_unknown_ones() {
        let mut s = Steps::new([Wizard::Account, Wizard::Billing, Wizard::Account, Wizard::Confirm])
            .non_linear(true)
            .disable_step(&Wizard::Billing);
        assert_eq!(
            s.keys(),
            [Wizard::Account, Wizard::Billing, Wizard::Confirm],
            "repeats dropped"
        );
        assert!(s.select(&Wizard::Confirm));
        assert!(!s.select(&Wizard::Billing));
        assert_eq!(s.current(), Some(&Wizard::Confirm));
        let mut s = Steps::new(0..3);
        s.set_error(&9, true);
        assert_eq!(s.status(&9), StepStatus::Pending);
        assert!(!s.select(&9));
        assert!(!(0..3).any(|i| s.error(&i)));
    }

    #[test]
    fn empty_flow_has_no_current_step() {
        let mut s = Steps::<u8>::new([]);
        assert_eq!(s.current(), None);
        assert!(!s.forward());
        assert!(!s.back());
        assert!(!s.skip());
        assert!(s.is_finished());
        s.reset();
        assert_eq!(s.current(), None);
    }
}
