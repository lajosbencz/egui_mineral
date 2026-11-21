//! Payload transport is egui's own: drop targets use `Ui::dnd_drop_zone` directly.
// docs/rules.md rule 10: indices here outlive the frame.
#![deny(clippy::indexing_slicing)]
#![warn(clippy::arithmetic_side_effects)]

use egui::{CursorIcon, Id, InnerResponse, Order, Rangef, Sense, UiBuilder, Vec2};

use crate::{kit::state::TempState, semantic::KeyedSalt, widget::MineralUi};

/// Result of [`drag_source`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct DragSourceResponse<R> {
    /// Spans the contents; drag sensed.
    pub response: egui::Response,
    /// Value of the contents closure.
    pub inner: R,
    /// Being dragged this pass; also passed to the contents closure.
    pub dragged: bool,
}

/// `id_salt` names the contents' scope app-wide, independent of `ui`, so a source keeps its drag when it moves.
/// Drag sense sits below the contents, so clickable contents keep their clicks.
pub fn drag_source<K, T: Send + Sync + 'static, R>(
    ui: &mut MineralUi<'_, K>,
    id_salt: impl Into<KeyedSalt>,
    payload: impl FnOnce() -> T,
    preview: impl FnOnce(&mut MineralUi<'_, K>),
    add_contents: impl FnOnce(&mut MineralUi<'_, K>, bool) -> R,
) -> DragSourceResponse<R> {
    let id = root_id(ui.core(), &id_salt.into());
    drag_source_at(ui, id, payload, preview, add_contents)
}

fn root_id(core: &crate::runtime::Core, salt: &KeyedSalt) -> Id {
    crate::kit::scoped_id(core, Id::unique(("mineral::dnd", salt.id_salt())))
}

#[expect(clippy::arithmetic_side_effects, reason = "Pos2/Vec2 float math only")]
fn drag_source_at<K, T: Send + Sync + 'static, R>(
    ui: &mut MineralUi<'_, K>,
    id: Id,
    payload: impl FnOnce() -> T,
    preview: impl FnOnce(&mut MineralUi<'_, K>),
    add_contents: impl FnOnce(&mut MineralUi<'_, K>, bool) -> R,
) -> DragSourceResponse<R> {
    let rt = ui.rt();
    let ctx = rt.ctx();
    let dragged = ctx.is_being_dragged(id);
    let InnerResponse { inner, response } = ui
        // Not focusable: keyboard moves belong to the content. CLICK makes egui wait for a decided drag instead
        // of dragging on press, so a click on the contents shows no preview.
        .scope_builder(UiBuilder::new().scope_id(id).sense(Sense::CLICK | Sense::DRAG), |ui| {
            add_contents(&mut rt.ui(ui), dragged)
        });
    let grab = id.with("mineral::dnd_grab");
    if response.drag_started() {
        response.dnd_set_drag_payload(payload());
        let origin = ctx.input(|i| i.pointer.press_origin()).unwrap_or(response.rect.min);
        (response.rect.min - origin).store(ctx, grab);
    }
    if dragged {
        ctx.set_cursor_icon(CursorIcon::Grabbing);
        if let Some(pos) = ctx.pointer_interact_pos() {
            let offset = Vec2::load(ctx, grab);
            crate::kit::layer::area(rt.core(), id.with("mineral::dnd_preview"))
                .order(Order::Tooltip)
                .fixed_pos(pos + offset)
                .interactable(false)
                .show(ctx, |ui| preview(&mut rt.ui(ui)));
        }
    } else if response.hovered() {
        ctx.set_cursor_icon(CursorIcon::Grab);
    }
    DragSourceResponse {
        response,
        inner,
        dragged,
    }
}

/// Completed list move; both fields are item indexes, `from != to`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Reorder {
    /// Index of the moved item before the move.
    pub from: usize,
    /// Index of the moved item after the move (`Vec::remove(from)` then `Vec::insert(to, ..)`).
    pub to: usize,
}

impl Reorder {
    /// Moves the item at `from` to `to`, shifting the items between; an index past the end counts as
    /// the last item, and nothing moves in an empty slice.
    pub fn apply<T>(self, items: &mut [T]) {
        let len = items.len();
        let (Some(from), Some(to)) = (
            crate::kit::sanitize::index(self.from, len, "Reorder from"),
            crate::kit::sanitize::index(self.to, len, "Reorder to"),
        ) else {
            return;
        };
        if from < to {
            if let Some(span) = items.get_mut(from..=to) {
                span.rotate_left(1);
            }
        } else if let Some(span) = items.get_mut(to..=from) {
            span.rotate_right(1);
        }
    }
}

#[derive(Clone, Copy)]
struct ReorderPayload {
    list: Id,
    item: Id,
}

/// Drag state of one row of a reorderable list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RowView {
    /// Row in place, not dragged.
    Idle,
    /// Row in place while its drag is active; stylekits dim it.
    Dragged,
    /// Copy rendered under the pointer during the drag.
    Preview,
}

/// Insertion line of a reorderable list while an item is dragged.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct DropIndicator {
    /// Horizontal extent of the line.
    pub x: Rangef,
    /// Vertical position of the line.
    pub y: f32,
}

/// Result of one reorderable-list pass: the finished move and the drop indicator.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct ReorderOutput {
    /// `Some` on the pass the drop lands.
    pub moved: Option<Reorder>,
    /// Insertion line while this list's item is dragged.
    pub indicator: Option<DropIndicator>,
}

/// Draws `items` as rows draggable for reordering; reports the finished move and drop indicator. `id_salt` names
/// the list app-wide.
/// Rows are scoped by the item's hash (an [`egui::AsIdSalt`], as in `egui_dnd`), so ids, focus and state follow an item when it moves;
/// items must hash uniquely. The dragged item is looked up by that hash on release, so a list that
/// changed mid-drag moves the right item, and a removed one drops the move.
pub fn reorder_list<K, T: egui::AsIdSalt>(
    ui: &mut MineralUi<'_, K>,
    id_salt: impl Into<KeyedSalt>,
    items: &[T],
    row: impl FnMut(&mut MineralUi<'_, K>, usize, &T, RowView),
) -> ReorderOutput {
    let id = root_id(ui.core(), &id_salt.into());
    let row = std::cell::RefCell::new(row);
    let mut gaps = Vec::with_capacity(items.len().saturating_add(1));
    let mut out = ReorderOutput {
        moved: None,
        indicator: None,
    };
    for (i, item) in items.iter().enumerate() {
        let r = drag_source_at(
            ui,
            id.with(item),
            || ReorderPayload {
                list: id,
                item: id.with(item),
            },
            |ui| (row.borrow_mut())(ui, i, item, RowView::Preview),
            |ui, dragged| {
                let view = if dragged { RowView::Dragged } else { RowView::Idle };
                (row.borrow_mut())(ui, i, item, view);
            },
        );
        gaps.push(r.response.rect.top());
        if i.saturating_add(1) == items.len() {
            gaps.push(r.response.rect.bottom());
        }
    }
    let dragged = egui::DragAndDrop::payload::<ReorderPayload>(ui.ctx())
        .filter(|p| p.list == id)
        .and_then(|p| items.iter().position(|item| id.with(item) == p.item));
    if let (Some(from), Some(pos)) = (dragged, ui.ctx().pointer_interact_pos()) {
        let slot = gaps
            .iter()
            .enumerate()
            .min_by(|a, b| (a.1 - pos.y).abs().total_cmp(&(b.1 - pos.y).abs()));
        if let Some((slot, y)) = slot {
            out.indicator = Some(DropIndicator {
                x: ui.min_rect().x_range(),
                y: *y,
            });
            if ui.input(|i| i.pointer.any_released()) {
                egui::DragAndDrop::clear_payload(ui.ctx());
                // Gap `slot` lies before item `slot`; past `from` it shifts down by the removed item.
                let to = if slot > from { slot.saturating_sub(1) } else { slot };
                if to != from {
                    out.moved = Some(Reorder { from, to });
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use egui::Event;

    use super::*;

    #[test]
    fn reorder_apply_clamps_out_of_range_indexes() {
        let mut v = vec!['a', 'b', 'c'];
        Reorder { from: 0, to: 9 }.apply(&mut v);
        assert_eq!(v, ['b', 'c', 'a'], "to past the end is the last item");
        Reorder { from: 9, to: 0 }.apply(&mut v);
        assert_eq!(v, ['a', 'b', 'c'], "from past the end is the last item");
        Reorder { from: 1, to: 2 }.apply(&mut Vec::<char>::new());
    }

    #[test]
    fn reorder_apply() {
        let mut v = vec!['a', 'b', 'c', 'd'];
        Reorder { from: 0, to: 2 }.apply(&mut v);
        assert_eq!(v, ['b', 'c', 'a', 'd']);
        Reorder { from: 3, to: 0 }.apply(&mut v);
        assert_eq!(v, ['d', 'b', 'c', 'a']);
        Reorder { from: 0, to: 3 }.apply(&mut v);
        assert_eq!(v, ['b', 'c', 'a', 'd'], "to the end");
    }

    #[derive(Default)]
    struct List {
        items: Vec<u32>,
        rects: Vec<egui::Rect>,
        moved: Option<Reorder>,
    }

    fn drag_with_list_change(grab: usize, change: impl FnOnce(&mut Vec<u32>), release_row: usize) -> Option<Reorder> {
        let mut h = crate::test_support::harness(
            |ui, s: &mut List| {
                let items = s.items.clone();
                s.rects = vec![egui::Rect::NOTHING; items.len()];
                let rects = std::cell::RefCell::new(std::mem::take(&mut s.rects));
                let out = reorder_list(ui, "list", &items, |ui, i, _, view| {
                    let r = ui.allocate_exact_size(egui::vec2(100.0, 20.0), egui::Sense::hover()).0;
                    if view == RowView::Idle
                        && let Some(slot) = rects.borrow_mut().get_mut(i)
                    {
                        *slot = r;
                    }
                });
                s.rects = rects.into_inner();
                s.moved = s.moved.or(out.moved);
            },
            List {
                items: vec![1, 2, 3, 4],
                ..List::default()
            },
        );
        h.step();
        let center =
            |h: &egui_kittest::Harness<'_, crate::test_support::Run<List>>, i: usize| h.state().value.rects[i].center();
        let from = center(&h, grab);
        h.input_mut().events.extend(crate::test_support::press(from));
        h.step();
        h.event(Event::PointerMoved(egui::pos2(from.x, from.y + 30.0)));
        h.step();
        change(&mut h.state_mut().value.items);
        h.step();
        let to = center(&h, release_row.min(h.state().value.rects.len().saturating_sub(1)));
        h.event(Event::PointerMoved(to));
        h.event(crate::input::synth::pointer_up(to, egui::PointerButton::Primary));
        h.step();
        h.state().value.moved
    }

    #[test]
    fn dragged_item_removed_mid_drag_drops_the_move() {
        let moved = drag_with_list_change(3, |items| items.truncate(2), 0);
        assert_eq!(moved, None);
    }

    #[test]
    fn list_changes_mid_drag_move_the_item_by_identity() {
        let moved = drag_with_list_change(
            2,
            |items| {
                items.remove(0);
            },
            0,
        );
        assert_eq!(moved, Some(Reorder { from: 1, to: 0 }), "item 3 is now at index 1");
    }

    #[test]
    fn a_press_is_no_drag_until_the_pointer_travels() {
        // (dragged, clicked, contents rect)
        let mut h = crate::test_support::harness(
            |ui, s: &mut (bool, u32, egui::Rect)| {
                let r = drag_source(
                    ui,
                    "src",
                    || 7u8,
                    |_| (),
                    |ui, _| {
                        let rect = ui.allocate_exact_size(egui::vec2(100.0, 20.0), egui::Sense::hover()).0;
                        ui.interact(rect, ui.scope_id().with("row"), egui::Sense::click())
                    },
                );
                s.0 = r.dragged;
                s.1 += u32::from(r.inner.clicked());
                s.2 = r.inner.rect;
            },
            (false, 0, egui::Rect::NOTHING),
        );
        h.step();
        let at = h.state().value.2.center();
        h.input_mut().events.extend(crate::test_support::press(at));
        h.step();
        h.step();
        assert!(!h.state().value.0, "pressed, not dragged");
        assert!(!egui::DragAndDrop::has_any_payload(&h.ctx), "no payload on a press");
        h.event(crate::input::synth::pointer_up(at, egui::PointerButton::Primary));
        h.step();
        assert_eq!(h.state().value.1, 1, "the contents keep the click");
        h.input_mut().events.extend(crate::test_support::press(at));
        h.step();
        h.event(Event::PointerMoved(egui::pos2(at.x, at.y + 30.0)));
        h.step();
        h.step();
        assert!(h.state().value.0, "dragged past the click distance");
        assert!(egui::DragAndDrop::has_payload_of_type::<u8>(&h.ctx));
    }
}
