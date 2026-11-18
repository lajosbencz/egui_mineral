use std::{any::Any, collections::VecDeque};

/// Reversible edit of state `S`.
pub trait UndoCommand<S>: Any + Send {
    /// Performs the edit; also called again on redo.
    fn apply(&mut self, state: &mut S);
    /// Exactly undoes [`Self::apply`].
    fn revert(&mut self, state: &mut S);
    /// Human-readable name for undo/redo menu items.
    fn label(&self) -> &str;

    /// Absorbs an already-applied follow-up command (e.g. consecutive keystrokes).
    /// Downcast `_next` to `Self`; return `true` if merged.
    fn merge(&mut self, _next: &dyn Any) -> bool {
        false
    }
}

struct Group<S> {
    label: String,
    commands: Vec<Box<dyn UndoCommand<S>>>,
}

impl<S: 'static> UndoCommand<S> for Group<S> {
    fn apply(&mut self, state: &mut S) {
        self.commands.iter_mut().for_each(|c| c.apply(state));
    }

    fn revert(&mut self, state: &mut S) {
        self.commands.iter_mut().rev().for_each(|c| c.revert(state));
    }

    fn label(&self) -> &str {
        &self.label
    }
}

/// Shortcut sets for [`UndoStack::handle_shortcuts`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UndoShortcuts<'a> {
    /// Shortcuts that undo.
    pub undo: &'a [egui::KeyboardShortcut],
    /// Shortcuts that redo.
    pub redo: &'a [egui::KeyboardShortcut],
}

/// Undo and redo history of [`UndoCommand`]s with grouping and a size limit.
pub struct UndoStack<S> {
    done: VecDeque<Box<dyn UndoCommand<S>>>,
    undone: Vec<Box<dyn UndoCommand<S>>>,
    group: Option<Group<S>>,
    limit: usize,
    sealed: bool,
}

impl<S> std::fmt::Debug for UndoStack<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UndoStack")
            .field("done", &self.done.len())
            .field("undone", &self.undone.len())
            .finish_non_exhaustive()
    }
}

impl<S: 'static> UndoStack<S> {
    /// Keeps at most `limit` undo steps (a group counts as one); 0 counts as 1.
    pub fn new(limit: usize) -> Self {
        let limit = crate::kit::sanitize::at_least(limit, 1.., "UndoStack limit");
        Self {
            done: VecDeque::new(),
            undone: Vec::new(),
            group: None,
            limit,
            sealed: true,
        }
    }

    /// Applies `cmd`, clears redo and merges into the previous command unless sealed.
    pub fn execute(&mut self, mut cmd: impl UndoCommand<S>, state: &mut S) {
        cmd.apply(state);
        self.undone.clear();
        let last = match &mut self.group {
            Some(g) => g.commands.last_mut(),
            None => self.done.back_mut(),
        };
        if !self.sealed
            && let Some(last) = last
            && last.merge(&cmd)
        {
            return;
        }
        if let Some(g) = &mut self.group {
            g.commands.push(Box::new(cmd));
        } else {
            self.done.push_back(Box::new(cmd));
            if self.done.len() > self.limit {
                self.done.pop_front();
            }
        }
        self.sealed = false;
    }

    /// Stops the next command from merging into the last one.
    pub fn seal(&mut self) {
        self.sealed = true;
    }

    /// Collects commands into one undo step until [`Self::end_group`]; nested calls are ignored.
    pub fn begin_group(&mut self, label: impl Into<String>) {
        if self.group.is_none() {
            self.group = Some(Group {
                label: label.into(),
                commands: Vec::new(),
            });
            self.sealed = true;
        }
    }

    /// Closes the open group; an empty group adds no step.
    pub fn end_group(&mut self) {
        if let Some(g) = self.group.take()
            && !g.commands.is_empty()
        {
            self.done.push_back(Box::new(g));
            if self.done.len() > self.limit {
                self.done.pop_front();
            }
        }
        self.sealed = true;
    }

    /// Reverts the latest step, closing an open group first; `false` when there is none.
    pub fn undo(&mut self, state: &mut S) -> bool {
        self.end_group();
        let Some(mut cmd) = self.done.pop_back() else {
            return false;
        };
        cmd.revert(state);
        self.undone.push(cmd);
        true
    }

    /// Re-applies the latest undone step; `false` when there is none.
    pub fn redo(&mut self, state: &mut S) -> bool {
        self.end_group();
        let Some(mut cmd) = self.undone.pop() else { return false };
        cmd.apply(state);
        self.done.push_back(cmd);
        true
    }

    /// Includes commands of an open group.
    pub fn can_undo(&self) -> bool {
        !self.done.is_empty() || self.group.as_ref().is_some_and(|g| !g.commands.is_empty())
    }

    /// `false` after any new [`Self::execute`].
    pub fn can_redo(&self) -> bool {
        !self.undone.is_empty()
    }

    /// Label of the step [`Self::undo`] reverts; ignores an open group.
    pub fn undo_label(&self) -> Option<&str> {
        self.done.back().map(|c| c.label())
    }

    /// Label of the step [`Self::redo`] re-applies.
    pub fn redo_label(&self) -> Option<&str> {
        self.undone.last().map(|c| c.label())
    }

    /// Consumes the first matching shortcut (redo checked first) and undoes or redoes; `true` when state changed.
    pub fn handle_shortcuts(&mut self, ctx: &egui::Context, state: &mut S, shortcuts: UndoShortcuts<'_>) -> bool {
        // Redo first: Cmd+Shift+Z also matches a Cmd+Z undo shortcut.
        let (u, r) = ctx.input_mut(|i| {
            let r = shortcuts.redo.iter().any(|s| i.consume_shortcut(s));
            (!r && shortcuts.undo.iter().any(|s| i.consume_shortcut(s)), r)
        });
        (u && self.undo(state)) || (r && self.redo(state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Push(String);

    impl UndoCommand<String> for Push {
        fn apply(&mut self, s: &mut String) {
            s.push_str(&self.0);
        }

        fn revert(&mut self, s: &mut String) {
            s.truncate(s.len() - self.0.len());
        }

        fn label(&self) -> &'static str {
            "type"
        }

        fn merge(&mut self, next: &dyn Any) -> bool {
            next.downcast_ref::<Self>().is_some_and(|n| {
                self.0.push_str(&n.0);
                true
            })
        }
    }

    #[test]
    fn merge_seal_group() {
        let mut s = String::new();
        let mut u = UndoStack::new(16);
        u.execute(Push("a".into()), &mut s);
        u.execute(Push("b".into()), &mut s);
        u.seal();
        u.execute(Push("c".into()), &mut s);
        assert!(u.undo(&mut s));
        assert_eq!(s, "ab");
        assert!(u.undo(&mut s));
        assert_eq!(s, "");
        assert!(u.redo(&mut s));
        assert_eq!(s, "ab");

        u.begin_group("batch");
        u.execute(Push("x".into()), &mut s);
        u.seal();
        u.execute(Push("y".into()), &mut s);
        u.end_group();
        assert_eq!(s, "abxy");
        assert!(u.undo(&mut s));
        assert_eq!(s, "ab");
    }

    fn push(u: &mut UndoStack<String>, s: &mut String, text: &str) {
        u.execute(Push(text.into()), s);
        u.seal();
    }

    #[test]
    fn limit_evicts_the_oldest_step() {
        let (mut s, mut u) = (String::new(), UndoStack::new(2));
        for t in ["a", "b", "c"] {
            push(&mut u, &mut s, t);
        }
        assert!(u.undo(&mut s) && u.undo(&mut s));
        assert!(!u.undo(&mut s));
        assert_eq!(s, "a");
    }

    #[test]
    fn execute_after_undo_clears_redo() {
        let (mut s, mut u) = (String::new(), UndoStack::new(16));
        push(&mut u, &mut s, "a");
        assert!(u.undo(&mut s));
        assert!(u.can_redo());
        push(&mut u, &mut s, "b");
        assert!(!u.can_redo() && !u.redo(&mut s));
        assert_eq!(s, "b");
    }

    #[test]
    fn group_counts_as_one_step() {
        let (mut s, mut u) = (String::new(), UndoStack::new(2));
        push(&mut u, &mut s, "a");
        u.begin_group("batch");
        push(&mut u, &mut s, "x");
        push(&mut u, &mut s, "y");
        u.end_group();
        assert!(u.undo(&mut s));
        assert_eq!(s, "a");
        assert!(u.undo(&mut s));
        assert!(!u.undo(&mut s));
    }

    #[test]
    fn zero_limit_keeps_one_step() {
        let u = UndoStack::<String>::new(0);
        assert!(!u.can_undo());
    }
}
