use egui::{Id, LayerId};
use egui_mineral::{combobox::ComboState, kit::choice::Choice};

use super::listbox::{Entry, Flat};

#[derive(Clone, Debug, Default)]
pub struct ListState {
    pub combo: ComboState,
    pub active_id: Option<Id>,
    pub popup: Option<LayerId>,
    pub filtering: bool,
}

pub(crate) fn selected_text<K: PartialEq + Clone>(entries: &[Entry<K>], value: &Choice<'_, K>) -> String {
    let flat = Flat::new(entries, &|_, _| true);
    flat.options
        .iter()
        .filter(|o| value.contains(&o.value))
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
