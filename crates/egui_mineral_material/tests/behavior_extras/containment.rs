use egui::{Key, Modifiers, PointerButton, Pos2, Rect, vec2};
use egui_mineral::{dnd::Reorder, input::synth};
use egui_mineral_material::{
    containment::ListItem,
    extras::ReorderableList,
    selection::{Menu, MenuItem},
};

use crate::support::{Harness, MineralHarness, WIDE, still};

fn secondary_click(h: &mut Harness<'_, Rows>, pos: Pos2) {
    h.hover_at(pos);
    h.event(synth::pointer_down(pos, PointerButton::Secondary));
    h.event(synth::pointer_up(pos, PointerButton::Secondary));
    h.run();
}

#[derive(Default)]
struct Rows {
    rects: Vec<Rect>,
    activated: Option<usize>,
    menu_open: bool,
    moved: Option<Reorder>,
}

#[test]
fn reorderable_rows_click_and_open_context_menu() {
    let items = ['a', 'b', 'c'];
    let mut h = still(WIDE, Rows::default(), |ui, s| {
        let r = ReorderableList::new("list", "List", &items)
            .show(ui, |_, c| ListItem::new(("row", *c), c.to_string()).clickable(true));
        assert_eq!(r.rows.len(), items.len());
        s.moved = s.moved.or(r.moved);
        s.rects = r.rows.iter().map(|w| w.rect).collect();
        for (i, row) in r.rows.iter().enumerate() {
            if row.output().activated {
                s.activated = Some(i);
            }
            if Menu::context_requested(ui.rt(), row) {
                Menu::new(("menu", i), "Menu")
                    .item(MenuItem::new("Item"))
                    .show_context(ui.rt(), row);
                s.menu_open |= Menu::is_context_open(&row.ctx, row);
            }
        }
    });
    let rows = h.state().value.rects.clone();
    h.click_at(rows[0].center());
    assert_eq!(h.state().value.activated, Some(0), "drag source keeps row clicks");
    assert!(!h.state().value.menu_open);
    secondary_click(&mut h, rows[1].center());
    assert!(h.state().value.menu_open, "context menu opens");
    h.press(Key::Escape);
    let (from, to) = (rows[0].center(), rows[2].center_bottom() - vec2(0.0, 1.0));
    h.hover_at(from);
    h.drag_at(from);
    h.hover_at(from.lerp(to, 0.5));
    h.hover_at(to);
    h.drop_at(to);
    h.run();
    assert_eq!(
        h.state().value.moved,
        Some(Reorder { from: 0, to: 2 }),
        "rows still drag"
    );
}

#[test]
fn clicked_row_keeps_focus_while_moving_with_alt_arrow() {
    let mut h = still(
        WIDE,
        (vec!['a', 'b', 'c', 'd'], Vec::new(), None),
        |ui, (items, rects, focused): &mut (Vec<char>, Vec<Rect>, Option<char>)| {
            let r = ReorderableList::new("list", "List", items)
                .show(ui, |_, c| ListItem::new(("row", *c), c.to_string()).clickable(true));
            *rects = r.rows.iter().map(|w| w.rect).collect();
            *focused = r.rows.iter().position(|w| w.has_focus()).map(|i| items[i]);
            if let Some(m) = r.moved {
                m.apply(items);
            }
        },
    );
    let first = h.state().value.1[0];
    h.click_at(first.center());
    for _ in 0..2 {
        h.key_press_modifiers(Modifiers::ALT, Key::ArrowDown);
        h.run();
    }
    assert_eq!(h.state().value.0, ['b', 'c', 'a', 'd'], "focus follows the moved row");
    h.key_press(Key::Tab);
    h.run();
    assert_eq!(h.state().value.2, Some('d'), "Tab takes the next row, no hidden stops");
}
