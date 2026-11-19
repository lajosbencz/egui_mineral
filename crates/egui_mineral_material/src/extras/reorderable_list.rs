use crate::Material;
use crate::MaterialExt;
use egui::{AsIdSalt, Key, Sense};
use egui_mineral::semantic::AgentKey;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    dnd::{self, Reorder, RowView},
    semantic::{self, Role, SemanticNode},
    widget::WidgetResponse,
};

use crate::containment::{ListItem, ListItemOutput};
use crate::widget::row_parts;
use egui_mineral::kit;

/// Result of [`ReorderableList::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ReorderableListResponse {
    /// List response.
    pub response: egui::Response,
    /// Move committed this frame; the app applies it to `items`.
    pub moved: Option<Reorder>,
    /// Per-item responses, index-aligned with the items.
    pub rows: Vec<WidgetResponse<ListItemOutput>>,
}

/// List whose rows reorder by drag or Alt+arrows; non-MD3 (Flutter `ReorderableListView` pattern).
/// Rows are scoped by the item's hash ([`dnd::reorder_list`]); items must hash uniquely.
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct ReorderableList<'a, T> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    items: &'a [T],
    disabled: bool,
}

impl<'a, T: AsIdSalt> ReorderableList<'a, T> {
    /// `label` names the list for assistive technology.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, items: &'a [T]) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            key: AgentKey::Kind("reorderable-list"),
            salt: id_salt,
            label: label.into(),
            items,
            disabled: false,
        }
    }

    crate::widget::setters! {
        /// Disables every row and reordering.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-reorderable-list` agent key.
        agent_key(key)
    }

    /// `item` builds the row for each `(index, item)`.
    pub fn show(
        self,
        ui: &mut MineralUi<'_, Material>,
        mut item: impl FnMut(usize, &T) -> ListItem,
    ) -> ReorderableListResponse {
        let rt = ui.rt();
        let out = ui.scope(|ui| {
            if self.disabled {
                let rows = self
                    .items
                    .iter()
                    .enumerate()
                    .map(|(i, it)| {
                        let row = item(i, it).drag_indicator(&rt.icons()).disabled(true);
                        ui.push_id(it, |ui| rt.ui(ui).mineral(row)).inner
                    })
                    .collect();
                (None, rows)
            } else {
                reorderable_list(&mut rt.ui(ui), &self.salt, self.items, item)
            }
        });
        let response = out.response.interact(Sense::hover());
        semantic::emit(ui.ctx(), Some(response.id), response.rect, || {
            let mut n = SemanticNode::new(self.key.id(&self.salt), Role::List, &self.label);
            n.state.disabled = self.disabled;
            n
        });
        let (moved, rows) = out.inner;
        ReorderableListResponse { response, moved, rows }
    }
}

fn reorderable_list<T: AsIdSalt>(
    ui: &mut MineralUi<'_, Material>,
    salt: &KeyedSalt,
    items: &[T],
    mut item: impl FnMut(usize, &T) -> ListItem,
) -> (Option<Reorder>, Vec<WidgetResponse<ListItemOutput>>) {
    let rt = ui.rt();
    let mut focused = None;
    let mut rows = Vec::with_capacity(items.len());
    let t = rt.tokens();
    let res = dnd::reorder_list(ui, salt.clone(), items, |ui, i, it, view| {
        let w = item(i, it).drag_indicator(&rt.icons());
        match view {
            RowView::Idle => {
                let r = rt.ui(ui).mineral(w);
                if r.has_focus() {
                    focused = Some(i);
                }
                rows.push(r);
            }
            RowView::Dragged => {
                ui.multiply_opacity(t.state.disabled_content);
                rows.push(rt.ui(ui).mineral(w));
            }
            RowView::Preview => {
                rt.ui(ui).mineral(w.dragged(true));
            }
        }
    });
    if let Some(line) = res.indicator {
        ui.painter().hline(line.x, line.y, row_parts::drop_indicator(&t));
    }
    let moved = res.moved;
    let keyed = focused.and_then(|i| {
        kit::input::key_presses(ui.ctx())
            .into_iter()
            .filter(|(_, m)| m.alt)
            .find_map(|(k, _)| match k {
                Key::ArrowUp if i > 0 => Some(Reorder { from: i, to: i - 1 }),
                Key::ArrowDown if i + 1 < items.len() => Some(Reorder { from: i, to: i + 1 }),
                _ => None,
            })
    });
    (moved.or(keyed), rows)
}
