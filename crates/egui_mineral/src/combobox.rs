//! <https://www.w3.org/WAI/ARIA/apg/patterns/combobox> (active-descendant model)
// docs/rules.md rule 10: indices here outlive the frame.
#![deny(clippy::indexing_slicing)]
#![warn(clippy::arithmetic_side_effects)]

use egui::{Context, Key, Modifiers};

use crate::filter::Query;

/// Transition of a [`ComboState`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ComboAction {
    /// List opened; `active` may be `None`.
    Opened,
    /// List closed without a commit (Escape, or nothing active).
    Closed,
    /// Active descendant moved to the option index; selection unchanged.
    Moved(usize),
    /// Option index chosen (Enter, Tab, Alt+Up, Space when select-only); the list closed.
    Commit(usize),
}

/// APG combobox variants: select-only or editable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ComboKind {
    /// Button-like trigger; Space and Home/End act on the list.
    SelectOnly,
    /// Leaves Space and Home/End to the text field.
    Editable,
    /// [`Self::Editable`] whose list its host keeps shown and dismisses (command palette, search
    /// view): Escape, Tab and Enter with no active option are left to the host.
    Embedded,
}

/// Inputs of [`ComboState::update`]: kind, selection, item count and enabled test.
#[derive(Clone, Copy)]
pub struct ComboList<'a> {
    /// Picks the APG keyboard variant.
    pub kind: ComboKind,
    /// Becomes active when a key opens the list.
    pub selected: Option<usize>,
    /// Option count; indexes are `0..len`.
    pub len: usize,
    /// Disabled options are skipped by navigation and never committed.
    pub enabled: &'a dyn Fn(usize) -> bool,
}

crate::kit::opaque_debug!(ComboList<'_>);

impl ComboList<'_> {
    fn first(&self, mut range: impl Iterator<Item = usize>) -> Option<usize> {
        range.find(|i| (self.enabled)(*i))
    }

    fn first_enabled(&self) -> Option<usize> {
        self.first(0..self.len)
    }

    fn last_enabled(&self) -> Option<usize> {
        self.first((0..self.len).rev())
    }

    fn usable(&self, i: Option<usize>) -> Option<usize> {
        i.filter(|i| *i < self.len && (self.enabled)(*i))
    }

    fn page(&self, cur: Option<usize>, target: usize, down: bool) -> Option<usize> {
        if self.len == 0 {
            None
        } else if down {
            let lo = cur.map_or(0, |i| i.saturating_add(1));
            self.first(target..self.len).or_else(|| self.first((lo..target).rev()))
        } else {
            let hi = cur.unwrap_or(self.len);
            self.first((0..=target).rev())
                .or_else(|| self.first(target.saturating_add(1)..hi))
        }
    }
}

/// APG combobox behavior: open state, active descendant and keyboard handling; no painting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ComboState {
    open: bool,
    active: Option<usize>,
    jump: usize,
}

impl Default for ComboState {
    fn default() -> Self {
        Self {
            open: false,
            active: None,
            jump: 10,
        }
    }
}

impl ComboState {
    /// Rows a PageUp/PageDown press moves; the visible row count. 0 counts as 1.
    #[must_use]
    pub fn jump(mut self, jump: usize) -> Self {
        self.jump = crate::kit::sanitize::at_least(jump, 1.., "ComboState jump");
        self
    }

    /// `true` while the listbox shows.
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Highlighted option index; `None` while closed or nothing is highlighted.
    pub fn active(&self) -> Option<usize> {
        self.active
    }

    /// Pointer hover or filtering moves the highlight here; an `active` not below `len` moves to the
    /// last option.
    pub fn set_active(&mut self, active: Option<usize>, len: usize) {
        self.active = in_range(active, len);
    }

    /// Clears the highlight when it is not below `len`; call when the option count changes.
    pub fn retain_active(&mut self, len: usize) {
        self.active = self.active.filter(|i| *i < len);
    }

    fn move_to(&mut self, i: usize) -> ComboAction {
        self.active = Some(i);
        ComboAction::Moved(i)
    }

    /// Opens with `active` highlighted; an `active` not below `len` highlights the last option.
    pub fn open(&mut self, active: Option<usize>, len: usize) -> ComboAction {
        self.open = true;
        self.active = in_range(active, len);
        ComboAction::Opened
    }

    /// Opens with `selected` active when enabled, else the first enabled option.
    pub fn open_selected(&mut self, listbox: ComboList<'_>) -> ComboAction {
        let active = listbox.usable(listbox.selected).or_else(|| listbox.first_enabled());
        self.open(active, listbox.len)
    }

    /// Clears the highlight; reports [`ComboAction::Closed`].
    pub fn close(&mut self) -> ComboAction {
        self.open = false;
        self.active = None;
        ComboAction::Closed
    }

    /// One key press per the APG combobox pattern; `None` when the key does nothing.
    pub fn on_key(&mut self, key: Key, modifiers: Modifiers, listbox: ComboList<'_>) -> Option<ComboAction> {
        let ComboList {
            kind, selected, len, ..
        } = listbox;
        let editable = kind != ComboKind::SelectOnly;
        let embedded = kind == ComboKind::Embedded;
        if !self.open {
            let from_selected = |fallback: Option<usize>| listbox.usable(selected).or(fallback);
            return match key {
                Key::ArrowDown | Key::ArrowUp if modifiers.alt => Some(self.open_selected(listbox)),
                Key::ArrowDown => Some(self.open_selected(listbox)),
                Key::ArrowUp => Some(self.open(from_selected(listbox.last_enabled()), len)),
                Key::Enter | Key::Space if !editable => Some(self.open_selected(listbox)),
                Key::Home if !editable => Some(self.open(listbox.first_enabled(), len)),
                Key::End if !editable => Some(self.open(listbox.last_enabled(), len)),
                _ => None,
            };
        }
        // The list may have shrunk or disabled `active` since it was set.
        self.active = listbox.usable(self.active);
        let cur = self.active;
        let commit = |s: &mut Self| match cur {
            Some(i) => {
                s.close();
                ComboAction::Commit(i)
            }
            None => s.close(),
        };
        let commit_key = key == Key::Enter || key == Key::ArrowUp && modifiers.alt;
        if embedded && (matches!(key, Key::Escape | Key::Tab) || commit_key && cur.is_none()) {
            return None;
        }
        let target = match key {
            Key::Escape => return Some(self.close()),
            Key::ArrowUp if modifiers.alt => return Some(commit(self)),
            Key::Enter | Key::Tab => return Some(commit(self)),
            Key::Space if !editable => return cur.is_some().then(|| commit(self)),
            Key::ArrowDown => cur.map_or_else(
                || listbox.first_enabled(),
                |i| listbox.first(i.saturating_add(1)..len).or(cur),
            ),
            Key::ArrowUp => cur.map_or_else(|| listbox.last_enabled(), |i| listbox.first((0..i).rev()).or(cur)),
            Key::PageDown => {
                let t = cur
                    .map_or(0, |i| i.saturating_add(self.jump))
                    .min(len.saturating_sub(1));
                listbox.page(cur, t, true).or(cur)
            }
            Key::PageUp => {
                let t = cur.map_or(0, |i| i.saturating_sub(self.jump));
                listbox.page(cur, t, false).or(cur)
            }
            Key::Home if !editable => listbox.first_enabled(),
            Key::End if !editable => listbox.last_enabled(),
            _ => return None,
        };
        target.map(|i| self.move_to(i))
    }

    /// Consumes this pass's combobox keys from `ctx` (so text fields and focus navigation do not
    /// see them) and applies them in order. Tab commits but still moves focus.
    pub fn update(&mut self, ctx: &Context, listbox: ComboList<'_>) -> Vec<ComboAction> {
        let keys: Vec<(Key, Modifiers)> = crate::kit::input::key_presses(ctx)
            .into_iter()
            .filter(|(_, m)| !m.command && !m.ctrl)
            .collect();
        let mut out = Vec::new();
        for (key, mods) in keys {
            if key == Key::Tab && !self.open {
                continue;
            }
            if let Some(a) = self.on_key(key, mods, listbox) {
                out.push(a);
                if key != Key::Tab {
                    ctx.input_mut(|i| i.consume_key(mods, key));
                }
            }
        }
        out
    }
}

#[track_caller]
fn in_range(active: Option<usize>, len: usize) -> Option<usize> {
    active.and_then(|i| crate::kit::sanitize::index(i, len, "ComboState option"))
}

/// Row of a filtered combobox list: a matched option or the create entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComboItem {
    /// Matched option.
    Option {
        /// Index into the input labels.
        index: usize,
        /// Matched char indices for highlighting; empty for an empty query.
        hits: Vec<usize>,
    },
    /// Entry that creates an option from the query.
    Create,
}

/// Options matching `query`: all, in order, when empty; best first otherwise.
pub fn combo_items<'a>(query: &str, labels: impl IntoIterator<Item = &'a str>) -> Vec<ComboItem> {
    options(query.trim(), labels)
}

/// [`combo_items`] followed by a [`ComboItem::Create`] row when no option equals the trimmed query
/// case-insensitively.
pub fn creatable_combo_items<'a>(
    query: &str,
    labels: impl IntoIterator<Item = &'a str, IntoIter: Clone>,
) -> Vec<ComboItem> {
    let q = query.trim();
    let labels = labels.into_iter();
    let mut out = options(q, labels.clone());
    if !q.is_empty() && !labels.into_iter().any(|l| lowercase(l.trim()).eq(lowercase(q))) {
        out.push(ComboItem::Create);
    }
    out
}

fn lowercase(s: &str) -> impl Iterator<Item = char> + '_ {
    s.chars().flat_map(char::to_lowercase)
}

fn options<'a>(q: &str, labels: impl IntoIterator<Item = &'a str>) -> Vec<ComboItem> {
    if q.is_empty() {
        (0..labels.into_iter().count())
            .map(|index| ComboItem::Option {
                index,
                hits: Vec::new(),
            })
            .collect()
    } else {
        Query::new(q)
            .rank(labels.into_iter().enumerate())
            .into_iter()
            .map(|(index, m)| ComboItem::Option { index, hits: m.indices })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(kind: ComboKind, selected: Option<usize>) -> ComboList<'static> {
        ComboList {
            kind,
            selected,
            len: 5,
            enabled: &|_| true,
        }
    }

    #[test]
    fn stale_active_is_not_committed_after_shrink() {
        let mut s = ComboState::default();
        s.open(Some(4), 5);
        let shrunk = ComboList {
            kind: ComboKind::Editable,
            selected: None,
            len: 2,
            enabled: &|_| true,
        };
        assert_eq!(s.on_key(Key::Enter, Modifiers::NONE, shrunk), Some(ComboAction::Closed));
    }

    #[test]
    fn apg_keys() {
        let mut s = ComboState::default();
        assert_eq!(
            s.on_key(Key::ArrowDown, Modifiers::NONE, list(ComboKind::SelectOnly, Some(2))),
            Some(ComboAction::Opened)
        );
        assert_eq!(s.active(), Some(2));
        assert_eq!(
            s.on_key(Key::End, Modifiers::NONE, list(ComboKind::SelectOnly, None)),
            Some(ComboAction::Moved(4))
        );
        assert_eq!(
            s.on_key(Key::ArrowDown, Modifiers::NONE, list(ComboKind::SelectOnly, None)),
            Some(ComboAction::Moved(4))
        );
        assert_eq!(
            s.on_key(Key::PageUp, Modifiers::NONE, list(ComboKind::SelectOnly, None)),
            Some(ComboAction::Moved(0))
        );
        assert_eq!(
            s.on_key(Key::Enter, Modifiers::NONE, list(ComboKind::SelectOnly, None)),
            Some(ComboAction::Commit(0))
        );
        assert!(!s.is_open());
        assert_eq!(
            s.on_key(Key::Space, Modifiers::NONE, list(ComboKind::Editable, None)),
            None,
            "editable types spaces"
        );
        s.on_key(Key::ArrowUp, Modifiers::NONE, list(ComboKind::Editable, None));
        assert_eq!(s.active(), Some(4));
        assert_eq!(
            s.on_key(Key::Escape, Modifiers::NONE, list(ComboKind::Editable, None)),
            Some(ComboAction::Closed)
        );
    }

    #[test]
    fn disabled_options_are_skipped_and_never_committed() {
        let enabled = |i: usize| i % 2 == 1;
        let l = ComboList {
            kind: ComboKind::SelectOnly,
            selected: Some(2),
            len: 5,
            enabled: &enabled,
        };
        let mut s = ComboState::default().jump(2);
        assert_eq!(s.on_key(Key::ArrowDown, Modifiers::NONE, l), Some(ComboAction::Opened));
        assert_eq!(s.active(), Some(1), "disabled selection opens on the first enabled");
        assert_eq!(
            s.on_key(Key::ArrowDown, Modifiers::NONE, l),
            Some(ComboAction::Moved(3))
        );
        assert_eq!(
            s.on_key(Key::ArrowDown, Modifiers::NONE, l),
            Some(ComboAction::Moved(3))
        );
        assert_eq!(s.on_key(Key::Home, Modifiers::NONE, l), Some(ComboAction::Moved(1)));
        assert_eq!(s.on_key(Key::PageDown, Modifiers::NONE, l), Some(ComboAction::Moved(3)));
        assert_eq!(s.on_key(Key::PageUp, Modifiers::NONE, l), Some(ComboAction::Moved(1)));
        s.set_active(Some(2), l.len);
        assert_eq!(s.on_key(Key::Enter, Modifiers::NONE, l), Some(ComboAction::Closed));
    }

    #[test]
    fn filtered_items_and_create() {
        let labels = ["Apple", "Banana", "Cherry"];
        assert_eq!(creatable_combo_items("", labels).len(), 3);
        let items = creatable_combo_items("an", labels);
        assert!(matches!(items[0], ComboItem::Option { index: 1, .. }));
        assert_eq!(items.last(), Some(&ComboItem::Create));
        assert!(!creatable_combo_items(" apple ", labels).contains(&ComboItem::Create));
    }

    #[test]
    fn stale_active_index_clamps_to_the_last_option() {
        let mut s = ComboState::default().jump(0);
        s.set_active(Some(9), 3);
        assert_eq!(s.active(), Some(2));
        s.set_active(Some(9), 0);
        assert_eq!(s.active(), None);
        s.open(Some(7), 4);
        assert_eq!(s.active(), Some(3));
    }
}
