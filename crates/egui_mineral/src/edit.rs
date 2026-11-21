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

/// Draft of a bound value. Idle and unfocused, the draft follows the bound value; while editing,
/// it adopts a bound value the app changed, told apart from the field's own writes by
/// [`EditBuffer::wrote`].
#[derive(Clone, Debug, Default)]
pub struct EditBuffer<T> {
    draft: T,
    origin: Option<T>,
    known: Option<T>,
    seen: u64,
}

impl<T: Clone + PartialEq> EditBuffer<T> {
    /// Idle buffer showing `value`.
    pub fn new(value: T) -> Self {
        Self {
            draft: value,
            origin: None,
            known: None,
            seen: 0,
        }
    }

    /// Start of a pass: while idle, the draft adopts `bound` and `Focused` starts an edit from it;
    /// while editing, it adopts `bound` when that differs from the last value the field wrote (or
    /// started from). `true` when the draft changed, so derived text needs a refresh.
    pub fn sync(&mut self, bound: &T, focus: FieldFocus) -> bool {
        let adopt = if self.editing() {
            self.known.as_ref() != Some(bound)
        } else {
            if focus == FieldFocus::Focused {
                self.start(bound);
            }
            true
        };
        let changed = adopt && self.draft != *bound;
        if adopt {
            self.draft.clone_from(bound);
            self.known = Some(bound.clone());
        }
        changed
    }

    /// Starts an edit from `bound` unless one is open; call before a change made without focus.
    pub fn start(&mut self, bound: &T) {
        if self.origin.is_none() {
            self.origin = Some(bound.clone());
            self.known = Some(bound.clone());
        }
    }

    /// Records `value` as written to the bound value by the field itself; call after each write.
    pub fn wrote(&mut self, value: &T) {
        self.known = Some(value.clone());
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

/// Selects all of `text` in the egui `TextEdit` `id` from its next pass, e.g. so typing replaces a value.
pub fn select_all(ctx: &Context, id: egui::Id, text: &str) {
    use egui::text::{CCursor, CCursorRange};
    let mut s = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();
    s.cursor.set_char_range(Some(CCursorRange::two(
        CCursor::new(0),
        CCursor::new(text.chars().count()),
    )));
    s.store(ctx, id);
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

    #[test]
    fn editing_adopts_bound_changes_the_field_did_not_write() {
        let mut b = EditBuffer::new(5.0);
        assert!(!b.sync(&5.0, FieldFocus::Focused));
        *b.draft_mut() = 0.0;
        b.wrote(&0.0);
        assert!(!b.sync(&0.0, FieldFocus::Focused), "its own write keeps the draft");
        *b.draft_mut() = 0.25;
        assert!(!b.sync(&0.0, FieldFocus::Focused), "an unwritten draft stays");
        assert_eq!(*b.draft(), 0.25);
        assert!(b.sync(&0.5, FieldFocus::Focused), "the app clamped the bound value");
        assert_eq!((*b.draft(), b.editing()), (0.5, true));
        assert_eq!(b.cancel(), Some(EditEvent::Cancelled));
        assert_eq!(*b.draft(), 5.0);
    }
}
