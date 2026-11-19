use std::collections::BTreeSet;

use egui::Modifiers;

/// Single or multiple selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize)]
pub enum SelectionMode {
    /// At most one key; selecting replaces it.
    #[default]
    Single,
    /// Any number of keys; [`Selection::click`] supports Ctrl/Shift extension.
    Multiple,
}

/// Tri-state check of a checkbox, a tree node or a select-all header.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Check {
    /// AccessKit `Toggled::False`.
    Unchecked,
    /// AccessKit `Toggled::True`.
    Checked,
    /// Some but not all descendants checked; AccessKit `Toggled::Mixed`.
    Mixed,
}

impl From<bool> for Check {
    fn from(on: bool) -> Self {
        if on { Self::Checked } else { Self::Unchecked }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(
    try_from = "RawSelection<K>",
    bound(deserialize = "K: Ord + serde::Deserialize<'de>")
)]
/// Selected keys with an anchor for range selection.
pub struct Selection<K: Ord> {
    mode: SelectionMode,
    required: bool,
    selected: BTreeSet<K>,
    anchor: Option<K>,
}

#[derive(serde::Deserialize)]
#[serde(bound(deserialize = "K: Ord + serde::Deserialize<'de>"))]
struct RawSelection<K: Ord> {
    mode: SelectionMode,
    required: bool,
    selected: BTreeSet<K>,
    anchor: Option<K>,
}

impl<K: Ord> TryFrom<RawSelection<K>> for Selection<K> {
    type Error = String;

    fn try_from(r: RawSelection<K>) -> Result<Self, String> {
        if r.mode == SelectionMode::Single && r.selected.len() > 1 {
            return Err(format!(
                "Selection: single mode holds at most 1 key, got {}",
                r.selected.len()
            ));
        }
        Ok(Self {
            mode: r.mode,
            required: r.required,
            selected: r.selected,
            anchor: r.anchor,
        })
    }
}

/// Empty [`SelectionMode::Single`] selection, not required.
impl<K: Ord> Default for Selection<K> {
    fn default() -> Self {
        Self {
            mode: SelectionMode::default(),
            required: false,
            selected: BTreeSet::new(),
            anchor: None,
        }
    }
}

impl<K: Ord + Clone> Selection<K> {
    /// Empty single selection, not required.
    pub fn single() -> Self {
        Self::default()
    }

    /// Empty multiple selection.
    pub fn multiple() -> Self {
        Self {
            mode: SelectionMode::Multiple,
            ..Self::default()
        }
    }

    /// Single mode only: deselecting and clearing keep the selected key (tabs, radios, segmented buttons).
    #[must_use]
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Fixed at construction.
    pub fn mode(&self) -> SelectionMode {
        self.mode
    }

    /// Selects `keys` in order; single mode keeps the last.
    #[must_use]
    pub fn with(mut self, keys: impl IntoIterator<Item = K>) -> Self {
        for k in keys {
            self.select(k);
        }
        self
    }

    /// `true` when `key` is in the selection.
    pub fn is_selected(&self, key: &K) -> bool {
        self.selected.contains(key)
    }

    /// Ascending key order.
    pub fn selected(&self) -> impl Iterator<Item = &K> {
        self.selected.iter()
    }

    /// Smallest selected key; the only one in single mode.
    pub fn first(&self) -> Option<&K> {
        self.selected.first()
    }

    /// Number of selected keys.
    pub fn len(&self) -> usize {
        self.selected.len()
    }

    /// `true` with nothing selected.
    pub fn is_empty(&self) -> bool {
        self.selected.is_empty()
    }

    /// Adds `key` (replacing in single mode) and makes it the range anchor; `true` when the
    /// selected keys changed.
    pub fn select(&mut self, key: K) -> bool {
        self.anchor = Some(key.clone());
        if self.mode == SelectionMode::Single {
            let changed = self.selected.len() != 1 || !self.selected.contains(&key);
            self.selected.clear();
            self.selected.insert(key);
            changed
        } else {
            self.selected.insert(key)
        }
    }

    /// No-op for a required single selection; `true` when `key` was removed.
    pub fn deselect(&mut self, key: &K) -> bool {
        !(self.required && self.mode == SelectionMode::Single) && self.selected.remove(key)
    }

    /// [`Self::deselect`] when selected, else [`Self::select`]; `true` when the selected keys changed.
    pub fn toggle(&mut self, key: K) -> bool {
        if self.selected.contains(&key) {
            self.deselect(&key)
        } else {
            self.select(key)
        }
    }

    /// Drops all keys and the anchor; no-op when required. `true` when keys were dropped.
    pub fn clear(&mut self) -> bool {
        if self.required {
            return false;
        }
        self.anchor = None;
        let changed = !self.selected.is_empty();
        self.selected.clear();
        changed
    }

    /// Desktop listbox click: plain selects only `key`, Ctrl/Cmd toggles, Shift extends from the
    /// anchor through `order` (visual order of all keys). `true` when the selected keys changed.
    pub fn click(&mut self, key: K, modifiers: Modifiers, order: &[K]) -> bool {
        if self.mode == SelectionMode::Single {
            return self.select(key);
        }
        if modifiers.shift
            && let Some(anchor) = self.anchor.clone()
            && let (Some(a), Some(b)) = (
                order.iter().position(|k| *k == anchor),
                order.iter().position(|k| *k == key),
            )
        {
            let range = &order[a.min(b)..=a.max(b)];
            let mut changed = false;
            if !modifiers.command {
                let before = self.selected.len();
                self.selected.retain(|k| range.contains(k));
                changed = self.selected.len() != before;
            }
            for k in range {
                changed |= self.selected.insert(k.clone());
            }
            changed
        } else if modifiers.command {
            self.toggle(key)
        } else {
            let only = self.selected.len() == 1 && self.selected.contains(&key);
            self.selected.clear();
            self.select(key);
            !only
        }
    }
}

/// Collects into an empty [`SelectionMode::Multiple`] selection.
impl<K: Ord + Clone> FromIterator<K> for Selection<K> {
    fn from_iter<I: IntoIterator<Item = K>>(iter: I) -> Self {
        Self::multiple().with(iter)
    }
}

/// Selects each key in order; single mode keeps the last.
impl<K: Ord + Clone> Extend<K> for Selection<K> {
    fn extend<I: IntoIterator<Item = K>>(&mut self, iter: I) {
        iter.into_iter().for_each(|k| {
            self.select(k);
        });
    }
}

/// Ascending key order.
#[expect(clippy::into_iter_without_iter, reason = "`selected` is the named iterator")]
impl<'a, K: Ord> IntoIterator for &'a Selection<K> {
    type Item = &'a K;
    type IntoIter = std::collections::btree_set::Iter<'a, K>;

    fn into_iter(self) -> Self::IntoIter {
        self.selected.iter()
    }
}

/// Outcome of a single-selection group (radio group, tabs, navigation, stepper, button group).
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SelectionResponse {
    /// Group response; `mark_changed` when `changed`.
    pub response: egui::Response,
    /// `selected` changed this frame.
    pub changed: bool,
    /// Index picked this frame.
    pub selected: Option<usize>,
}

impl SelectionResponse {
    /// Marks `response` changed when `changed`.
    pub fn new(mut response: egui::Response, selected: Option<usize>, changed: bool) -> Self {
        if changed {
            response.mark_changed();
        }
        Self {
            response,
            changed,
            selected,
        }
    }

    /// Writes `picked` to `bound`; `changed` when it differs from the bound index.
    pub fn pick(response: egui::Response, bound: &mut usize, picked: Option<usize>) -> Self {
        let changed = picked.is_some_and(|i| i != *bound);
        if let Some(i) = picked {
            *bound = i;
        }
        Self::new(response, picked, changed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_extends_and_iterates() {
        let s: Selection<u8> = [3, 1, 2].into_iter().collect();
        assert_eq!(s.mode(), SelectionMode::Multiple);
        assert_eq!((&s).into_iter().copied().collect::<Vec<_>>(), [1, 2, 3]);
        let mut seen = Vec::new();
        for k in &s {
            seen.push(*k);
        }
        assert_eq!(seen, [1, 2, 3]);

        let mut multi = Selection::multiple();
        multi.extend([5, 4]);
        assert_eq!(multi.len(), 2);
        let mut single = Selection::single();
        single.extend([5, 4]);
        assert_eq!(
            single.selected().copied().collect::<Vec<_>>(),
            [4],
            "single keeps the last"
        );
        assert_eq!(Selection::<u8>::default().mode(), SelectionMode::Single);
    }

    #[test]
    fn shift_click_range_and_required_single() {
        let order = [1, 2, 3, 4, 5];
        let mut s = Selection::multiple();
        s.click(2, Modifiers::NONE, &order);
        s.click(4, Modifiers::SHIFT, &order);
        assert_eq!(s.selected().copied().collect::<Vec<_>>(), [2, 3, 4]);
        s.click(3, Modifiers::COMMAND, &order);
        assert_eq!(s.selected().copied().collect::<Vec<_>>(), [2, 4]);

        let mut tabs = Selection::single().required(true).with([0]);
        tabs.toggle(0);
        assert!(tabs.is_selected(&0));
    }

    #[test]
    fn restore_rejects_inconsistent_state() {
        let raw = |mode, keys: &[u8]| RawSelection {
            mode,
            required: false,
            selected: keys.iter().copied().collect(),
            anchor: None,
        };
        assert!(Selection::try_from(raw(SelectionMode::Multiple, &[1, 2])).is_ok());
        assert!(Selection::try_from(raw(SelectionMode::Single, &[1, 2])).is_err());
    }
}
