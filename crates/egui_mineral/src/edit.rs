//! Pending edits of bound values. A field edits an [`EditBuffer`] draft; when to write the draft
//! to the bound value (every pass, or on commit only) is the field's policy.
//!
//! An edit starts when the field takes focus (or changes without focus) and ends with
//! [`EditBuffer::commit`] (Enter, focus loss, [`commit_pending`]) or [`EditBuffer::cancel`]
//! (Escape, restoring the value from before the edit).
//!
//! [`commit_pending`] flushes open edits for an app action such as save. A buffered field commits
//! on its first show after the call, so the model holds every draft once all fields have shown:
//! call it before the UI (e.g. right after `shortcut::take_action` at the start of the pass) and
//! read the model after the UI, in the same pass. Fields shown before the call commit next pass.

use egui::{Context, Id};

/// Focus state of a text field.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FieldFocus {
    /// Field holds keyboard focus.
    Focused,
    /// Field lost or never had focus.
    Unfocused,
}

impl FieldFocus {
    /// `Focused` when `focused`.
    pub const fn of(focused: bool) -> Self {
        if focused { Self::Focused } else { Self::Unfocused }
    }
}

/// End of an edit session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum EditEvent {
    /// Draft kept: the edit ended, or was flushed and stays open.
    Committed,
    /// Edit ended restoring the value from before it.
    Cancelled,
}

fn request_id(ctx: &Context) -> Id {
    Id::unique(("mineral::edit::commit_pending", ctx.viewport_id()))
}

/// Asks every open edit to commit on its field's next show; see the [module docs](self) for ordering.
pub fn commit_pending(ctx: &Context) {
    let id = request_id(ctx);
    ctx.data_mut(|d| *d.get_temp_mut_or_default::<u64>(id) += 1);
}

/// Draft of a bound value. Idle and unfocused, the draft follows the bound value.
#[derive(Clone, Debug, Default)]
pub struct EditBuffer<T> {
    draft: T,
    origin: Option<T>,
    seen: u64,
}

impl<T: Clone + PartialEq> EditBuffer<T> {
    /// Idle buffer showing `value`.
    pub fn new(value: T) -> Self {
        Self {
            draft: value,
            origin: None,
            seen: 0,
        }
    }

    /// Start of a pass: while idle, `Unfocused` adopts `bound` and `Focused` starts an edit from it.
    pub fn sync(&mut self, bound: &T, focus: FieldFocus) {
        if self.editing() {
            return;
        }
        match focus {
            FieldFocus::Unfocused => self.draft.clone_from(bound),
            FieldFocus::Focused => self.start(bound),
        }
    }

    /// Starts an edit from `bound` unless one is open; call before a change made without focus.
    pub fn start(&mut self, bound: &T) {
        if self.origin.is_none() {
            self.origin = Some(bound.clone());
        }
    }

    /// Value shown and edited; the bound one while idle.
    pub fn draft(&self) -> &T {
        &self.draft
    }

    /// Edit target; call [`Self::start`] first to make the change cancellable.
    pub fn draft_mut(&mut self) -> &mut T {
        &mut self.draft
    }

    /// Edit started and not yet committed or cancelled.
    pub fn editing(&self) -> bool {
        self.origin.is_some()
    }

    /// Ends the edit keeping the draft; `None` while idle.
    pub fn commit(&mut self) -> Option<EditEvent> {
        self.origin.take().map(|_| EditEvent::Committed)
    }

    /// Ends the edit restoring the value from before it; `None` while idle.
    pub fn cancel(&mut self) -> Option<EditEvent> {
        self.draft = self.origin.take()?;
        Some(EditEvent::Cancelled)
    }

    /// `true` once per [`commit_pending`] call while an edit is open; call once per pass.
    pub fn commit_requested(&mut self, ctx: &Context) -> bool {
        let id = request_id(ctx);
        let generation = ctx.data(|d| d.get_temp::<u64>(id)).unwrap_or(0);
        let requested = generation != self.seen && self.editing();
        self.seen = generation;
        requested
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_commit_cancel_and_flush() {
        let ctx = Context::default();
        let mut bound = String::from("a");
        let mut b = EditBuffer::new(bound.clone());

        b.sync(&bound, FieldFocus::Focused);
        b.draft_mut().push('b');
        assert!(!b.commit_requested(&ctx));
        commit_pending(&ctx);
        assert!(b.commit_requested(&ctx));
        assert!(!b.commit_requested(&ctx), "one commit per request");
        assert_eq!(b.commit(), Some(EditEvent::Committed));
        bound.clone_from(b.draft());

        b.sync(&bound, FieldFocus::Focused);
        b.draft_mut().push('c');
        assert_eq!(b.cancel(), Some(EditEvent::Cancelled));
        assert_eq!(b.draft(), "ab");
        assert_eq!(b.commit(), None);

        commit_pending(&ctx);
        b.sync(&String::from("z"), FieldFocus::Unfocused);
        assert!(!b.commit_requested(&ctx), "idle fields ignore requests");
        assert_eq!(b.draft(), "z");
    }
}
