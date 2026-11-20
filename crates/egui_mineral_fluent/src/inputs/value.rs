use egui::{Id, LayerId};
use egui_mineral::{
    combobox::ComboState,
    select::{Selection, SelectionMode},
};

use super::listbox::{Flat, ListEntry};
use crate::widget::choice::Choice;

#[derive(Clone, Debug, Default)]
pub struct ListState {
    pub combo: ComboState,
    pub active_id: Option<Id>,
    pub popup: Option<LayerId>,
    pub filtering: bool,
}

pub(crate) enum Value<'a> {
    Many(&'a mut Selection<String>),
    One(&'a mut Option<String>),
    // Option values are their indices.
    Bound(Choice<'a>),
}

impl std::fmt::Debug for Value<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Value").finish_non_exhaustive()
    }
}

impl Value<'_> {
    pub(crate) fn multiselect(&self) -> bool {
        matches!(self, Self::Many(s) if s.mode() == SelectionMode::Multiple)
    }

    pub(crate) fn is_selected(&self, value: &str) -> bool {
        match self {
            Self::Many(s) => s.is_selected(&value.to_owned()),
            Self::One(v) => v.as_deref() == Some(value),
            Self::Bound(c) => c.selected().is_some_and(|i| value.parse() == Ok(i)),
        }
    }

    pub(crate) fn pick(&mut self, value: &str) -> bool {
        match self {
            Self::Many(s) if s.mode() == SelectionMode::Multiple => s.toggle(value.to_owned()),
            Self::Many(s) => s.select(value.to_owned()),
            Self::One(v) => {
                let changed = v.as_deref() != Some(value);
                **v = Some(value.to_owned());
                changed
            }
            Self::Bound(c) => value.parse().is_ok_and(|i| c.select(i)),
        }
    }
}

// Text of the selected options, in list order.
pub(crate) fn selected_text(entries: &[ListEntry], value: &Value<'_>) -> String {
    let flat = Flat::new(entries, &|_| true);
    flat.options
        .iter()
        .filter(|o| value.is_selected(&o.value))
        .map(|o| o.text.as_str())
        .collect::<Vec<_>>()
        .join(", ")
}

// First enabled option after `from` whose text starts with `typed`, wrapping.
pub(crate) fn typeahead(flat: &Flat<'_>, from: Option<usize>, typed: &str) -> Option<usize> {
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
