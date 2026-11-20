use egui::{Id, LayerId};
use egui_mineral::{
    combobox::ComboState,
    select::{Selection, SelectionMode},
};

use super::listbox::{Flat, ListEntry};

#[derive(Clone, Debug, Default)]
pub struct ListState {
    pub combo: ComboState,
    pub active_id: Option<Id>,
    pub popup: Option<LayerId>,
    pub filtering: bool,
}

// Binding of a list's picked option values.
pub(crate) trait Pick<K> {
    fn multiselect(&self) -> bool {
        false
    }

    fn is_selected(&self, value: &K) -> bool;

    // Whether the binding changed.
    fn pick(&mut self, value: &K) -> bool;
}

impl<K: Ord + Clone> Pick<K> for Selection<K> {
    fn multiselect(&self) -> bool {
        self.mode() == SelectionMode::Multiple
    }

    fn is_selected(&self, value: &K) -> bool {
        Selection::is_selected(self, value)
    }

    fn pick(&mut self, value: &K) -> bool {
        if self.mode() == SelectionMode::Multiple {
            self.toggle(value.clone())
        } else {
            self.select(value.clone())
        }
    }
}

impl<K: PartialEq + Clone> Pick<K> for Option<K> {
    fn is_selected(&self, value: &K) -> bool {
        self.as_ref() == Some(value)
    }

    fn pick(&mut self, value: &K) -> bool {
        let changed = self.as_ref() != Some(value);
        *self = Some(value.clone());
        changed
    }
}

// Always holds one of the option values; nothing is selected while it matches none.
pub(crate) struct Exact<'a, K>(pub &'a mut K);

impl<K: PartialEq + Clone> Pick<K> for Exact<'_, K> {
    fn is_selected(&self, value: &K) -> bool {
        self.0 == value
    }

    fn pick(&mut self, value: &K) -> bool {
        let changed = self.0 != value;
        *self.0 = value.clone();
        changed
    }
}

impl<K, P: Pick<K> + ?Sized> Pick<K> for &mut P {
    fn multiselect(&self) -> bool {
        (**self).multiselect()
    }

    fn is_selected(&self, value: &K) -> bool {
        (**self).is_selected(value)
    }

    fn pick(&mut self, value: &K) -> bool {
        (**self).pick(value)
    }
}

pub(crate) struct Value<'a, K>(Box<dyn Pick<K> + 'a>);

impl<'a, K> Value<'a, K> {
    pub(crate) fn new(pick: impl Pick<K> + 'a) -> Self {
        Self(Box::new(pick))
    }
}

impl<K> std::fmt::Debug for Value<'_, K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Value").finish_non_exhaustive()
    }
}

impl<'a, K> std::ops::Deref for Value<'a, K> {
    type Target = dyn Pick<K> + 'a;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<K> std::ops::DerefMut for Value<'_, K> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.0
    }
}

pub(crate) fn selected_text<K>(entries: &[ListEntry<K>], value: &dyn Pick<K>) -> String {
    let flat = Flat::new(entries, &|_, _| true);
    flat.options
        .iter()
        .filter(|o| value.is_selected(&o.value))
        .map(|o| o.text.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

// First enabled option after `from` whose text starts with `typed`, wrapping.
pub(crate) fn typeahead<K>(flat: &Flat<'_, K>, from: Option<usize>, typed: &str) -> Option<usize> {
    let lower = typed.to_lowercase();
    let n = flat.len();
    let start = from.map_or(0, |i| i.saturating_add(1));
    (0..n).map(|k| start.saturating_add(k) % n).find(|i| {
        flat.enabled(*i)
            && flat
                .value(*i)
                .is_some_and(|o| o.text.to_lowercase().starts_with(&lower))
    })
}

// A press or its release: focus left to a click on the popup must not close it before the click lands.
pub(crate) fn pointer_busy(ctx: &egui::Context) -> bool {
    ctx.input(|i| i.pointer.any_down() || i.pointer.any_released())
}

pub(crate) fn typed_text(ctx: &egui::Context) -> String {
    ctx.input(|i| {
        i.events
            .iter()
            .filter_map(|e| match e {
                egui::Event::Text(t) if !t.trim().is_empty() => Some(t.as_str()),
                _ => None,
            })
            .collect()
    })
}

// Keys of the open or closed list; the committed index and whether the active row moved.
pub(crate) fn drive(
    ctx: &egui::Context,
    combo: &mut egui_mineral::combobox::ComboState,
    list: egui_mineral::combobox::ComboList<'_>,
) -> (Option<usize>, bool) {
    use egui_mineral::combobox::ComboAction;
    let mut chosen = None;
    let mut moved = false;
    for action in combo.update(ctx, list) {
        match action {
            ComboAction::Commit(i) => chosen = Some(i),
            ComboAction::Moved(_) | ComboAction::Opened => moved = true,
            _ => {}
        }
    }
    (chosen, moved)
}
