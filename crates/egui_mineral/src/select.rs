use std::collections::BTreeSet;

use egui::Modifiers;

/// Bound of every app key an item, row or panel carries: hashed into egui ids, written into agent keys, ordered in
/// a [`Selection`]. Implemented for every such type; `#[derive(KeySalt)]` covers app enums.
pub trait ItemKey: crate::semantic::KeySalt + egui::AsIdSalt + Ord + Clone {}

impl<K: crate::semantic::KeySalt + egui::AsIdSalt + Ord + Clone> ItemKey for K {}

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
/// Selected keys, the anchor range selection extends from and the current (focused) key, which keyboard focus
/// moves without changing the selection.
pub struct Selection<K> {
    mode: SelectionMode,
    required: bool,
    selected: BTreeSet<K>,
    anchor: Option<K>,
    current: Option<K>,
}

#[derive(serde::Deserialize)]
#[serde(bound(deserialize = "K: Ord + serde::Deserialize<'de>"))]
struct RawSelection<K: Ord> {
    mode: SelectionMode,
    required: bool,
    selected: BTreeSet<K>,
    anchor: Option<K>,
    current: Option<K>,
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
            current: r.current,
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
            current: None,
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

    /// Focused key: the last one selected, clicked or moved to with [`Self::set_current`].
    pub fn current(&self) -> Option<&K> {
        self.current.as_ref()
    }

    /// Moves focus to `key` without changing the selected keys or the anchor (Ctrl+arrows, plain arrows of a
    /// multiple selection).
    pub fn set_current(&mut self, key: K) {
        self.current = Some(key);
    }

    /// Range anchor of Shift extension.
    pub fn anchor(&self) -> Option<&K> {
        self.anchor.as_ref()
    }

    /// Adds `key` (replacing in single mode) and makes it the range anchor and current key; `true` when the
    /// selected keys changed.
    pub fn select(&mut self, key: K) -> bool {
        self.anchor = Some(key.clone());
        self.current = Some(key.clone());
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
    /// anchor through `order` (visual order of all keys) like [`Self::extend_to`], Ctrl/Cmd+Shift adds the range. Shift with an anchor outside `order` (another page)
    /// adds `key` and makes it the anchor. `true` when the selected keys changed.
    pub fn click(&mut self, key: K, modifiers: Modifiers, order: &[K]) -> bool {
        if self.mode == SelectionMode::Single {
            return self.select(key);
        }
        if modifiers.shift {
            if let Some(changed) = self.range_to(&key, order, modifiers.command) {
                self.current = Some(key);
                changed
            } else {
                self.select(key)
            }
        } else if modifiers.command {
            self.current = Some(key.clone());
            self.toggle(key)
        } else {
            let only = self.selected.len() == 1 && self.selected.contains(&key);
            self.selected.clear();
            self.select(key);
            !only
        }
    }

    /// Pointer press of a draggable item: [`Self::click`], except that a plain press on a selected key of a
    /// multiple selection only makes it current and the anchor, so the selection can be dragged. The
    /// [`Self::release`] of a press that did not drag then narrows it.
    pub fn press(&mut self, key: K, modifiers: Modifiers, order: &[K]) -> bool {
        let plain = !modifiers.shift && !modifiers.command;
        if plain && self.mode == SelectionMode::Multiple && self.selected.contains(&key) {
            self.anchor = Some(key.clone());
            self.current = Some(key);
            return false;
        }
        self.click(key, modifiers, order)
    }

    /// Pointer release that ends a [`Self::press`] without a drag: a plain release on a selected key selects only
    /// it. `true` when the selected keys changed.
    pub fn release(&mut self, key: K, modifiers: Modifiers) -> bool {
        let plain = !modifiers.shift && !modifiers.command;
        if !plain || !self.selected.contains(&key) || self.selected.len() == 1 {
            return false;
        }
        self.selected.clear();
        self.select(key);
        true
    }

    /// Shift+arrow: swaps the anchor's previous range for the one through `key` in `order`, keeping other keys,
    /// and makes `key` current; selects `key` alone without an anchor in `order`. `true` when the selected keys changed.
    pub fn extend_to(&mut self, key: K, order: &[K]) -> bool {
        if self.mode == SelectionMode::Multiple
            && let Some(changed) = self.range_to(&key, order, false)
        {
            self.current = Some(key);
            return changed;
        }
        self.click(key, Modifiers::NONE, order)
    }

    /// Row `input` on `key` under `behavior`; `order` is the visual order Shift reads. `true` when the selected
    /// keys changed.
    ///
    /// [`SelectionBehavior::Replace`]: a mouse press [`Self::press`]es and the click ending it [`Self::release`]s;
    /// Space [`Self::click`]s (Ctrl/Cmd+Space toggles); a touch tap toggles while anything is selected and a long
    /// press adds the row, so an empty selection leaves taps to the row action; Enter is the row action's.
    /// [`SelectionBehavior::Toggle`]: clicks, taps, Space and Enter toggle, Shift adds the range from the anchor.
    pub fn apply(&mut self, key: K, input: RowInput, behavior: SelectionBehavior, order: &[K]) -> bool {
        let single = self.mode == SelectionMode::Single;
        match behavior {
            SelectionBehavior::Replace => {
                let mut changed = input.press.is_some_and(|m| self.press(key.clone(), m, order));
                if input.long_press && !self.is_selected(&key) {
                    changed |= self.select(key.clone());
                }
                changed
                    | match input.click {
                        Some((m, RowSource::Mouse)) => self.release(key, m),
                        Some((_, RowSource::Touch)) if self.is_empty() => false,
                        Some((_, RowSource::Touch)) if single => self.select(key),
                        Some((_, RowSource::Touch)) => self.toggle(key),
                        Some((m, RowSource::Space)) if m.command && !single => self.toggle(key),
                        Some((m, RowSource::Space)) => self.click(key, m, order),
                        _ => false,
                    }
            }
            SelectionBehavior::Toggle => match input.click {
                None => false,
                Some(_) if single => self.select(key),
                Some((m, _)) if m.shift => self.click(key, Modifiers::SHIFT | Modifiers::COMMAND, order),
                Some(_) => self.toggle(key),
            },
        }
    }

    /// `input` runs the row's action, judged before [`Self::apply`]: Enter, and under
    /// [`SelectionBehavior::Replace`] a touch tap while nothing is selected. Kits add their own (double click).
    pub fn activates(&self, input: RowInput, behavior: SelectionBehavior) -> bool {
        match input.click {
            Some((_, RowSource::Enter)) => true,
            Some((_, RowSource::Touch)) => behavior == SelectionBehavior::Replace && self.is_empty(),
            _ => false,
        }
    }

    /// Keyboard focus moved to `to` with `modifiers`: Shift extends a multiple selection to it ([`Self::extend_to`]),
    /// Ctrl/Cmd only makes it current; a plain move selects only `to` under [`SelectionBehavior::Replace`] and makes
    /// it current under [`SelectionBehavior::Toggle`]. `true` when the selected keys changed.
    pub fn follow_focus(&mut self, to: K, modifiers: Modifiers, behavior: SelectionBehavior, order: &[K]) -> bool {
        if modifiers.shift && self.mode == SelectionMode::Multiple {
            self.extend_to(to, order)
        } else if modifiers.command || behavior == SelectionBehavior::Toggle {
            self.set_current(to);
            false
        } else {
            self.click(to, Modifiers::NONE, order)
        }
    }

    /// Every key of `order` in a multiple selection ([`select_all_shortcut`]); `true` when the selected keys
    /// changed.
    pub fn select_all(&mut self, order: &[K]) -> bool {
        if self.mode != SelectionMode::Multiple {
            return false;
        }
        order
            .iter()
            .fold(false, |changed, k| self.selected.insert(k.clone()) | changed)
    }

    // Anchor through `key` in `order` replaces the anchor's previous range (through `current`), keeping other
    // keys (React Aria `extendSelection`); `keep` adds without dropping it. `None` when the anchor or `key` is
    // missing from `order`.
    fn range_to(&mut self, key: &K, order: &[K], keep: bool) -> Option<bool> {
        let at = |k: &K| order.iter().position(|o| o == k);
        let a = at(self.anchor.as_ref()?)?;
        let b = at(key)?;
        let before = self.selected.clone();
        if !keep && let Some(c) = self.current.as_ref().and_then(at) {
            for k in &order[a.min(c)..=a.max(c)] {
                self.selected.remove(k);
            }
        }
        self.selected.extend(order[a.min(b)..=a.max(b)].iter().cloned());
        Some(self.selected != before)
    }
}

/// How clicks and plain arrows act on a collection's [`Selection`] ([`Selection::apply`], [`Selection::follow_focus`]),
/// after React Aria's `selectionBehavior`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SelectionBehavior {
    /// Clicks, taps and Space toggle rows like a checkbox group; plain arrows only move focus. Touch-first lists
    /// and tables.
    #[default]
    Toggle,
    /// Clicks and plain arrows select only the row, Ctrl/Cmd toggles, Shift extends; on touch a long press starts
    /// a selection that taps then toggle. Desktop trees and grids.
    Replace,
}

/// Source of a [`RowInput`] click.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowSource {
    /// Mouse or pen click; ends a [`RowInput::press`].
    Mouse,
    /// Touch tap.
    Touch,
    /// Space on the focused row.
    Space,
    /// Enter on the focused row.
    Enter,
}

/// Press, click and long press of a selectable row this pass, applied by [`Selection::apply`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RowInput {
    /// Mouse press that started on the row; touch never presses, so a scroll never selects.
    pub press: Option<Modifiers>,
    /// Click and its source.
    pub click: Option<(Modifiers, RowSource)>,
    /// Touch long press on the row.
    pub long_press: bool,
}

impl RowInput {
    /// Of `response` this pass, from egui's press and click for the mouse and keys and from the gestures of `input`
    /// for touch. Reads the press from this pass's input, not `is_pointer_button_down_on`: a fast click presses and
    /// releases in one pass.
    pub fn of(input: &crate::input::InputContext, response: &egui::Response) -> Self {
        let touch = input.modality == crate::input::PointerKind::Touch;
        let (modifiers, origin, space) = response.ctx.input(|i| {
            let origin = i.pointer.primary_pressed().then(|| i.pointer.press_origin()).flatten();
            (i.modifiers, origin, i.key_pressed(egui::Key::Space))
        });
        let pressed = !touch && response.hovered() && origin.is_some_and(|p| response.rect.contains(p));
        let by_pointer = response.clicked_by(egui::PointerButton::Primary);
        let click = if touch && input.tapped(response).is_some() {
            Some(RowSource::Touch)
        } else if !response.clicked() || (touch && by_pointer) {
            None
        } else if by_pointer {
            Some(RowSource::Mouse)
        } else if space {
            Some(RowSource::Space)
        } else {
            Some(RowSource::Enter)
        };
        Self {
            press: pressed.then_some(modifiers),
            click: click.map(|c| (modifiers, c)),
            long_press: input.long_pressed(response).is_some(),
        }
    }

    /// Each part of `self`, else of `other`; rows made of several responses.
    #[must_use]
    pub fn or(self, other: Self) -> Self {
        Self {
            press: self.press.or(other.press),
            click: self.click.or(other.click),
            long_press: self.long_press || other.long_press,
        }
    }

    /// No press, click or long press.
    pub fn is_empty(&self) -> bool {
        self.press.is_none() && self.click.is_none() && !self.long_press
    }

    /// Shift held, so applying it reads the visual order.
    pub fn shift(&self) -> bool {
        self.press.or(self.click.map(|c| c.0)).is_some_and(|m| m.shift)
    }
}

/// Ctrl/Cmd+A for the collection whose `Ui` has `scope` as its `unique_id`: bound in
/// [`Scope::Focus`](crate::shortcut::Scope::Focus) for the next pass, so it wins over app-wide bindings while focus is inside and
/// is consumed; `true` when it fired this pass. Left unbound while a text edit has focus, which keeps its own
/// select-all.
pub fn select_all_shortcut(ctx: &egui::Context, scope: egui::Id) -> bool {
    use crate::shortcut;
    let action = shortcut::ActionId::widget(scope.with("mineral::select-all"));
    if !ctx.text_edit_focused() {
        let keys = egui::KeyboardShortcut::new(Modifiers::COMMAND, egui::Key::A);
        shortcut::bind(ctx, shortcut::Scope::Focus(scope), keys, action.clone());
    }
    shortcut::take_action(ctx, &action)
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

/// Outcome of a selection group (radio group, tabs, navigation, stepper, segmented button); built by
/// [`Choice::pick`](crate::kit::choice::Choice::pick).
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SelectionResponse<K> {
    /// Group response; `mark_changed` when `changed`.
    pub response: egui::Response,
    /// Agrees with `response.changed()`: the bound value changed this frame.
    pub changed: bool,
    /// Value of the item picked this frame, also when it was already selected.
    pub selected: Option<K>,
}

impl<K> SelectionResponse<K> {
    /// Reports a pick of `selected` that changed the group's own model (e.g. [`Steps`](crate::steps::Steps)); `None`
    /// reports no pick. Groups bound to an app value use [`Choice::pick`](crate::kit::choice::Choice::pick).
    pub fn changed_to(response: egui::Response, selected: Option<K>) -> Self {
        let changed = selected.is_some();
        Self::new(response, selected, changed)
    }

    pub(crate) fn new(mut response: egui::Response, selected: Option<K>, changed: bool) -> Self {
        if changed {
            response.mark_changed();
        }
        Self {
            response,
            changed,
            selected,
        }
    }
}

/// Mark of the checked item of a radio group in a menu or list; stylekits pick their spec's default.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RadioMark {
    /// Checkmark, as checkbox items.
    Check,
    /// Filled dot, as a radio button.
    Dot,
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
    fn press_keeps_a_selection_to_drag_and_release_narrows_it() {
        let order = [1, 2, 3, 4];
        let mut s = Selection::multiple().with([1, 2, 3]);
        assert!(
            !s.press(2, Modifiers::NONE, &order),
            "pressing a selected key keeps the set"
        );
        assert_eq!(s.current(), Some(&2));
        assert_eq!(s.len(), 3);
        assert!(s.release(2, Modifiers::NONE), "a release without a drag narrows it");
        assert_eq!(s.selected().copied().collect::<Vec<_>>(), [2]);
        assert!(
            s.press(4, Modifiers::NONE, &order),
            "an unselected key selects on press"
        );
        assert!(!s.release(4, Modifiers::NONE));
        assert!(s.press(1, Modifiers::COMMAND, &order));
        assert!(!s.release(1, Modifiers::COMMAND), "modified releases change nothing");
        assert_eq!(s.selected().copied().collect::<Vec<_>>(), [1, 4]);
    }

    fn input(click: Option<(Modifiers, RowSource)>) -> RowInput {
        RowInput {
            click,
            ..RowInput::default()
        }
    }

    fn keys(s: &Selection<u8>) -> Vec<u8> {
        s.selected().copied().collect()
    }

    #[test]
    fn replace_presses_on_the_mouse_and_leaves_enter_to_the_action() {
        use SelectionBehavior::Replace;
        let order = [1, 2, 3];
        let mut s = Selection::multiple().with([1, 2]);
        let press = RowInput {
            press: Some(Modifiers::NONE),
            ..RowInput::default()
        };
        assert!(!s.apply(2, press, Replace, &order), "a press keeps the set to drag");
        assert!(s.apply(2, input(Some((Modifiers::NONE, RowSource::Mouse))), Replace, &order));
        assert_eq!(keys(&s), [2], "the release without a drag narrows it");
        assert!(!s.apply(3, input(Some((Modifiers::NONE, RowSource::Enter))), Replace, &order));
        let shift_space = input(Some((Modifiers::SHIFT, RowSource::Space)));
        assert!(shift_space.shift());
        assert!(s.apply(3, shift_space, Replace, &order));
        assert_eq!(keys(&s), [2, 3]);
        assert!(s.apply(2, input(Some((Modifiers::COMMAND, RowSource::Space))), Replace, &order));
        assert_eq!(keys(&s), [3], "Ctrl+Space toggles");
    }

    #[test]
    fn replace_touch_taps_act_until_a_long_press_starts_a_selection() {
        use SelectionBehavior::Replace;
        let order = [1, 2, 3];
        let tap = input(Some((Modifiers::NONE, RowSource::Touch)));
        let mut s = Selection::multiple();
        assert!(
            !s.apply(1, tap, Replace, &order),
            "an empty selection leaves taps to the action"
        );
        let long = RowInput {
            long_press: true,
            ..RowInput::default()
        };
        assert!(s.apply(1, long, Replace, &order));
        assert!(s.apply(3, tap, Replace, &order), "then taps toggle");
        assert_eq!(keys(&s), [1, 3]);
        assert!(s.apply(1, tap, Replace, &order) && s.apply(3, tap, Replace, &order));
        assert!(s.is_empty(), "deselecting everything ends it");
        assert!(!s.apply(2, tap, Replace, &order));
    }

    #[test]
    fn toggle_clicks_and_space_toggle_and_shift_adds_ranges() {
        use SelectionBehavior::Toggle;
        let order = [1, 2, 3, 4];
        let mut s = Selection::multiple();
        let press = RowInput {
            press: Some(Modifiers::NONE),
            ..RowInput::default()
        };
        assert!(
            !s.apply(1, press, Toggle, &order),
            "toggle acts on the click, so a drag changes nothing"
        );
        assert!(s.apply(1, input(Some((Modifiers::NONE, RowSource::Mouse))), Toggle, &order));
        assert!(s.apply(2, input(Some((Modifiers::NONE, RowSource::Touch))), Toggle, &order));
        assert!(s.apply(1, input(Some((Modifiers::NONE, RowSource::Space))), Toggle, &order));
        assert_eq!(keys(&s), [2]);
        assert!(s.apply(3, input(Some((Modifiers::NONE, RowSource::Enter))), Toggle, &order));
        assert!(s.apply(3, input(Some((Modifiers::NONE, RowSource::Enter))), Toggle, &order));
        assert!(s.apply(4, input(Some((Modifiers::SHIFT, RowSource::Mouse))), Toggle, &order));
        assert_eq!(keys(&s), [2, 3, 4], "Shift adds the anchor's range");
        let mut single = Selection::single();
        assert!(single.apply(3, input(Some((Modifiers::NONE, RowSource::Mouse))), Toggle, &order));
        assert!(!single.apply(3, input(Some((Modifiers::NONE, RowSource::Mouse))), Toggle, &order));
    }

    #[test]
    fn plain_moves_select_under_replace_and_only_focus_under_toggle() {
        use SelectionBehavior::{Replace, Toggle};
        let order = [1, 2, 3, 4, 5];
        let mut s = Selection::multiple();
        s.click(2, Modifiers::NONE, &order);
        assert!(!s.follow_focus(4, Modifiers::NONE, Toggle, &order));
        assert_eq!((s.current(), s.anchor()), (Some(&4), Some(&2)));
        assert!(s.follow_focus(4, Modifiers::SHIFT, Toggle, &order));
        assert_eq!(keys(&s), [2, 3, 4]);
        assert!(
            !s.follow_focus(5, Modifiers::COMMAND, Replace, &order),
            "Ctrl moves focus only"
        );
        assert!(s.follow_focus(1, Modifiers::NONE, Replace, &order));
        assert_eq!(keys(&s), [1]);
        assert!(s.select_all(&order));
        assert_eq!(s.len(), 5);
        let mut single = Selection::single();
        assert!(!single.select_all(&order));
        assert!(
            single.follow_focus(3, Modifiers::NONE, Replace, &order),
            "selection follows focus"
        );
        assert!(!single.follow_focus(4, Modifiers::NONE, Toggle, &order));
        assert_eq!(single.first(), Some(&3));
    }

    #[test]
    fn shift_with_an_anchor_on_another_page_adds_instead_of_clearing() {
        let mut s = Selection::multiple();
        s.click(1, Modifiers::NONE, &[1, 2]);
        assert!(s.click(7, Modifiers::SHIFT, &[6, 7, 8]));
        assert_eq!(keys(&s), [1, 7]);
        assert_eq!(s.anchor(), Some(&7));
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
            current: None,
        };
        assert!(Selection::try_from(raw(SelectionMode::Multiple, &[1, 2])).is_ok());
        assert!(Selection::try_from(raw(SelectionMode::Single, &[1, 2])).is_err());
    }
}
