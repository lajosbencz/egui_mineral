//! Command palette model: grouped fuzzy rows, keyboard navigation, shortcut binding and the run step.

use std::{collections::HashMap, hash::Hash};

use egui::{Context, Id, KeyboardShortcut};

use crate::{
    combobox::{ComboAction, ComboKind, ComboList, ComboState},
    filter::Query,
    kit::state::{Memo, TempState},
    semantic::{KeyedSalt, StableId},
    shortcut::{self, ActionId, Scope},
};

/// One selectable entry of a palette.
pub trait Entry {
    /// Visible text, matched by the query.
    fn label(&self) -> &str;
    /// Header the entry is listed under.
    fn heading(&self) -> Option<&str>;
    /// Cannot be selected or run.
    fn is_disabled(&self) -> bool;
    /// App command the entry is a view of.
    fn action(&self) -> Option<&ActionId>;
    /// Shortcut bound to [`action`](Self::action) while the palette is shown.
    fn accelerator(&self) -> Option<KeyboardShortcut>;
}

/// Row of the filtered list.
#[derive(Debug, PartialEq)]
pub enum Row {
    /// Group header.
    Header(String),
    /// Matching entry with the matched character indices of its label.
    Item {
        /// Index into the entries.
        index: usize,
        /// Matched character indices.
        hits: Vec<usize>,
    },
}

/// Rows for `query`: groups in first-appearance order, matches in rank order within a group, headers only for
/// groups with a match; every entry, unranked, for a blank query.
pub fn rows<E: Entry>(entries: &[E], query: &str) -> Vec<Row> {
    let mut matched: Vec<(usize, Vec<usize>)> = if query.trim().is_empty() {
        (0..entries.len()).map(|i| (i, Vec::new())).collect()
    } else {
        Query::new(query)
            .rank(entries.iter().enumerate().map(|(i, c)| (i, c.label())))
            .into_iter()
            .map(|(i, m)| (i, m.indices))
            .collect()
    };
    let mut rank: HashMap<Option<&str>, usize> = HashMap::new();
    for c in entries {
        let next = rank.len();
        rank.entry(c.heading()).or_insert(next);
    }
    matched.sort_by_key(|(i, _)| rank[&entries[*i].heading()]);
    let mut rows = Vec::with_capacity(matched.len() + rank.len());
    let mut current = None;
    for (index, hits) in matched {
        let g = entries[index].heading();
        if current != Some(g) {
            current = Some(g);
            rows.extend(g.map(|g| Row::Header(g.to_owned())));
        }
        rows.push(Row::Item { index, hits });
    }
    rows
}

/// Row indices of the enabled items, in navigation order.
pub fn selectable<E: Entry>(rows: &[Row], entries: &[E]) -> Vec<usize> {
    rows.iter()
        .enumerate()
        .filter_map(|(r, row)| match row {
            Row::Item { index, .. } if entries.get(*index).is_some_and(|c| !c.is_disabled()) => Some(r),
            _ => None,
        })
        .collect()
}

/// Hashes what [`rows`] reads.
pub struct Labels<'a, E>(pub &'a [E]);

impl<E: Entry> Hash for Labels<'_, E> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for c in self.0 {
            (c.label(), c.heading()).hash(state);
        }
        self.0.len().hash(state);
    }
}

/// Palette state in temp memory, keyed by the palette id.
#[derive(Clone, Default)]
pub struct State {
    /// Position in [`selectable`] of the active item.
    pub active: usize,
    /// Open on the previous pass.
    pub was_open: bool,
    /// One-shot: set by keyboard moves, cleared after the pass that scrolled.
    pub reveal: bool,
    /// Rows for the last query.
    pub rows: Memo<Vec<Row>>,
}

impl TempState for State {}

impl State {
    /// Arrow, Home/End and Enter handling; the entry index Enter ran.
    pub fn navigate(&mut self, ctx: &Context, rows: &[Row], items: &[usize]) -> Option<usize> {
        let mut combo = ComboState::default();
        combo.open(Some(self.active).filter(|a| *a < items.len()), items.len());
        let list = ComboList {
            kind: ComboKind::Embedded,
            selected: None,
            len: items.len(),
            enabled: &|_| true,
        };
        let (mut moved, mut commit) = (false, None);
        for action in combo.update(ctx, list) {
            match action {
                ComboAction::Commit(i) => commit = Some(i),
                ComboAction::Moved(_) | ComboAction::Opened => moved = true,
                _ => {}
            }
        }
        if moved && let Some(a) = combo.active() {
            self.active = a;
            self.reveal = true;
        }
        match commit.and_then(|a| rows.get(*items.get(a)?)) {
            Some(&Row::Item { index, .. }) => Some(index),
            _ => None,
        }
    }
}

/// Binds the toggle `shortcut` (also inside the modal at `modal`) and every enabled entry's shortcut at `scope`,
/// toggling `open` on the former and closing it when a command fires.
pub fn bind_shortcuts<E: Entry>(
    ctx: &Context,
    salt: &KeyedSalt,
    toggle: Option<KeyboardShortcut>,
    entries: &[E],
    scope: Scope,
    modal: Id,
    open: &mut bool,
) {
    if let Some(k) = toggle {
        let action = ActionId::from(String::from(StableId::of(salt, "shortcut")));
        shortcut::bind(ctx, Scope::Global, k, action.clone());
        shortcut::bind(ctx, Scope::Modal(modal), k, action.clone());
        if shortcut::take_action(ctx, &action) {
            *open = !*open;
        }
    }
    let mut fired = false;
    for cmd in entries.iter().filter(|c| !c.is_disabled()) {
        if let Some(command) = cmd.action() {
            if let Some(k) = cmd.accelerator() {
                shortcut::bind(ctx, scope, k, command);
            }
            fired |= shortcut::fired(ctx, command);
        }
    }
    if fired {
        *open = false;
    }
}

/// Runs entry `run` after the surface was shown: a command entry fires its command (and is consumed from `run`),
/// and a run or `dismissed` closes the palette.
pub fn finish<E: Entry>(ctx: &Context, entries: &[E], run: &mut Option<usize>, dismissed: bool, open: &mut bool) {
    if let Some(command) = run.and_then(|i| entries[i].action()) {
        shortcut::fire(ctx, command);
        *run = None;
        *open = false;
    }
    if dismissed || run.is_some() {
        *open = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Cmd(&'static str, Option<&'static str>);

    impl Entry for Cmd {
        fn label(&self) -> &str {
            self.0
        }
        fn heading(&self) -> Option<&str> {
            self.1
        }
        fn is_disabled(&self) -> bool {
            false
        }
        fn action(&self) -> Option<&ActionId> {
            None
        }
        fn accelerator(&self) -> Option<KeyboardShortcut> {
            None
        }
    }

    fn shape(rows: &[Row]) -> Vec<String> {
        rows.iter()
            .map(|r| match r {
                Row::Header(g) => format!("[{g}]"),
                Row::Item { index, .. } => index.to_string(),
            })
            .collect()
    }

    #[test]
    fn rows_group_in_first_appearance_order_and_keep_match_order() {
        let commands = [
            Cmd("Open file", Some("File")),
            Cmd("Undo", None),
            Cmd("Save file", Some("File")),
            Cmd("Copy", Some("Edit")),
            Cmd("Redo", None),
        ];
        assert_eq!(
            shape(&rows(&commands, "")),
            ["[File]", "0", "2", "1", "4", "[Edit]", "3"]
        );
        let hits = rows(&commands, "file");
        assert_eq!(
            shape(&hits),
            ["[File]", "0", "2"],
            "groups without matches drop their header"
        );
        let Some(Row::Item { hits, .. }) = hits.get(1) else {
            unreachable!("an item row")
        };
        assert_eq!(hits.len(), 4, "match indices move into the row");
    }
}
