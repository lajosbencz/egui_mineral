use egui::{Key, Modifiers, PointerButton, Pos2, Rect, vec2};
use egui_mineral::{dnd::Reorder, input::synth, overlay::Dismiss, semantic::Role};
use egui_mineral_material::{
    actions::PushButton,
    containment::{
        BottomSheet, Carousel, CarouselLayout, CarouselPadding, Dialog, DialogAction, ListItem, ListItemOutput,
        ListSlot, SideSheet,
    },
    extras::ReorderableList,
    selection::{Menu, MenuItem},
};

use crate::support::{Harness, MineralHarness, SCREEN, WIDE, flag_dismissals, still};

#[test]
fn slot_controls_toggle_without_activating() {
    let mut h = still(SCREEN, ListItemOutput::default(), |ui, seen| {
        let r = ui.mineral(
            ListItem::new("row", "Row")
                .leading(
                    ListSlot::new()
                        .checkbox(false)
                        .icon(egui_mineral_material::icons::ICON_SAVE),
                )
                .trailing(ListSlot::new().text("3").switch(false))
                .clickable(true),
        );
        seen.activated |= r.output().activated;
        seen.leading_toggled |= r.output().leading_toggled;
        seen.trailing_toggled |= r.output().trailing_toggled;
    });
    let seen = |h: &mut Harness<'_, ListItemOutput>| {
        let s = std::mem::take(&mut h.state_mut().value);
        (s.leading_toggled, s.activated)
    };
    h.click("row-list-item-trailing-switch");
    assert!(h.state_mut().value.trailing_toggled, "the trailing control toggles");
    assert_eq!(seen(&mut h), (false, false));
    h.click("row-list-item-checkbox");
    assert_eq!(seen(&mut h), (true, false), "the control toggles");
    let control = h.rect("row-list-item-checkbox");
    let row = h.rect("row-list-item");
    h.click_at(Pos2::new(
        control.right() + (row.right() - control.right()) / 2.0,
        row.center().y,
    ));
    assert_eq!(seen(&mut h), (false, true), "the row activates");
    let id = h.widget("row-list-item");
    h.ctx.memory_mut(|m| m.request_focus(id));
    h.press(Key::Space);
    assert_eq!(
        seen(&mut h),
        (true, false),
        "Space toggles the control of a focused row"
    );
}

#[derive(Default)]
struct Rows {
    rects: Vec<Rect>,
    activated: Option<usize>,
    menu_open: bool,
    moved: Option<Reorder>,
}

fn secondary_click(h: &mut Harness<'_, Rows>, pos: Pos2) {
    h.hover_at(pos);
    h.event(synth::pointer_down(pos, PointerButton::Secondary));
    h.event(synth::pointer_up(pos, PointerButton::Secondary));
    h.run();
}

#[test]
fn reorderable_rows_click_and_open_context_menu() {
    let items = ['a', 'b', 'c'];
    let mut h = still(WIDE, Rows::default(), |ui, s| {
        let r = ReorderableList::new("list", "List", &items)
            .show(ui, |_, c| ListItem::new(("row", *c), c.to_string()).clickable(true));
        assert_eq!(r.rows.len(), items.len());
        s.moved = s.moved.or(r.moved);
        s.rects = r.rows.iter().map(|(_, w)| w.rect).collect();
        for (i, row) in r.rows.iter() {
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
            *rects = r.rows.iter().map(|(_, w)| w.rect).collect();
            *focused = r.rows.iter().position(|(_, w)| w.has_focus()).map(|i| items[i]);
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

#[test]
fn dialog_reports_escape_and_outside_dismissal() {
    let h = still(WIDE, (true, None), |ui, (open, dismissed)| {
        let r = Dialog::new("dlg", "Title")
            .action(DialogAction::new("OK"))
            .show_modal(ui.rt(), open, |_| ());
        *dismissed = dismissed.or(r.dismissed);
    });
    let outside = h.outside();
    assert_eq!(
        flag_dismissals(h, outside),
        [Some(Dismiss::Escape), Some(Dismiss::Outside)]
    );
}

#[test]
fn modal_side_sheet_reports_escape_and_outside_dismissal() {
    let h = still(WIDE, (true, None), |ui, (open, dismissed)| {
        let r = SideSheet::new("sheet").show_modal(ui.rt(), open, |_| ());
        *dismissed = dismissed.or(r.dismissed);
    });
    let outside = h.outside();
    assert_eq!(
        flag_dismissals(h, outside),
        [Some(Dismiss::Escape), Some(Dismiss::Outside)]
    );
}

#[test]
fn modal_bottom_sheet_reports_escape_and_outside_dismissal() {
    let h = still(WIDE, (true, None), |ui, (open, dismissed)| {
        let r = BottomSheet::new("sheet").show_modal(ui.rt(), open, |_| ());
        *dismissed = dismissed.or(r.dismissed);
    });
    let outside = h.outside();
    assert_eq!(
        flag_dismissals(h, outside),
        [Some(Dismiss::Escape), Some(Dismiss::Outside)]
    );
}

#[test]
fn modal_sheet_binds_in_its_own_scope_from_its_opening_pass() {
    let h = still(WIDE, (true, Vec::new()), |ui, (open, scopes)| {
        BottomSheet::new("sheet").show_modal(ui.rt(), open, |ui| {
            scopes.push(egui_mineral::shortcut::scope_of(ui.ctx(), ui.layer_id()));
        });
    });
    let scopes = &h.state().value.1;
    assert!(
        scopes
            .first()
            .is_some_and(|s| matches!(s, egui_mineral::shortcut::Scope::Modal(_))),
        "{scopes:?}"
    );
}

#[test]
fn bottom_sheet_handle_steps_below_the_lowest_detent_to_swipe_dismiss() {
    let mut h = still(WIDE, (true, None), |ui, (open, dismissed)| {
        let r = BottomSheet::new("sheet").show(ui.rt(), open, |_| ());
        *dismissed = dismissed.or(r.dismissed);
    });
    let handle = h.widget("sheet-drag-handle");
    h.ctx.memory_mut(|m| m.request_focus(handle));
    h.press(Key::ArrowDown);
    assert_eq!(h.state().value, (false, Some(Dismiss::Swipe)));
}

#[test]
fn modal_dialog_traps_tab_inside_and_returns_focus() {
    let mut h = still(WIDE, false, |ui, open| {
        if ui.mineral(PushButton::new("trigger", "Open")).clicked() {
            *open = true;
        }
        Dialog::new("dlg", "Title")
            .action(DialogAction::new("Cancel"))
            .action(DialogAction::new("Later"))
            .action(DialogAction::new("OK"))
            .show_modal(ui.rt(), open, |_| ());
    });
    let inside = ["dlg-0-action-button", "dlg-1-action-button", "dlg-2-action-button"];
    h.press(Key::Tab);
    assert!(h.focused("trigger-button"), "Tab focuses the trigger");
    h.press(Key::Enter);
    assert!(h.state().value, "Enter opens the dialog");
    assert!(h.focused(inside[0]), "focus moves to the first action");
    let xs = inside.map(|k| h.rect(k).left());
    assert!(xs.windows(2).all(|w| w[0] < w[1]), "actions run left to right: {xs:?}");
    for next in [inside[1], inside[2], inside[0]] {
        h.press(Key::Tab);
        assert!(h.focused(next), "Tab follows the visual order, expected {next}");
    }
    for _ in 0..2 * inside.len() {
        h.press(Key::Tab);
        assert!(inside.iter().any(|k| h.focused(k)), "Tab cycles within the dialog");
    }
    for closer in ["Escape", "action", "outside"] {
        if !h.state().value {
            h.press(Key::Enter);
            assert!(h.state().value, "{closer}: Enter reopens the dialog");
        }
        match closer {
            "Escape" => h.press(Key::Escape),
            "action" => h.click(inside[1]),
            _ => h.click_at(h.outside()),
        }
        assert!(!h.state().value, "{closer} closes the dialog");
        assert!(h.focused("trigger-button"), "{closer}: focus returns to the trigger");
    }
}

#[test]
fn carousel_content_padding_wider_than_the_strip_keeps_keylines_valid() {
    let layouts = [
        CarouselLayout::multi_browse(186.0),
        CarouselLayout::uncontained(100.0),
        CarouselLayout::centered_hero(),
        CarouselLayout::hero(),
        CarouselLayout::FullScreen,
    ];
    for layout in layouts {
        for width in [60.0, 400.0] {
            let mut h = still(vec2(width, 300.0), (), move |ui, ()| {
                Carousel::new("c", layout)
                    .content_padding(CarouselPadding {
                        start: 100.0,
                        end: 100.0,
                    })
                    .show(ui, 6, |ui, i| {
                        ui.label(i.to_string());
                    });
            });
            h.run();
            assert_eq!(
                h.node("c-0-item").role,
                Role::ListItem,
                "{layout:?} at {width}: first item shown"
            );
        }
    }
}

#[test]
fn carousel_survives_hostile_item_widths() {
    for w in [1e-30, -50.0, f32::NAN, f32::INFINITY] {
        let layouts = [
            CarouselLayout::multi_browse(w),
            CarouselLayout::uncontained(w),
            CarouselLayout::centered_hero().preferred_item_width(w),
        ];
        for layout in layouts {
            let mut h = still(vec2(400.0, 300.0), (), move |ui, ()| {
                Carousel::new("c", layout).show(ui, 6, |ui, i| {
                    ui.label(i.to_string());
                });
            });
            h.run();
            let items: Vec<_> = h
                .tree()
                .nodes
                .iter()
                .filter(|n| n.id.key().ends_with("-item"))
                .map(|n| n.rect)
                .collect();
            assert!(
                !items.is_empty() && items.iter().all(egui::Rect::is_finite),
                "{layout:?}: items placed"
            );
        }
    }
}

#[derive(Default)]
struct Strip {
    activated: Vec<usize>,
    position: f32,
}

fn hero_strip<'a>() -> Harness<'a, Strip> {
    still(vec2(400.0, 300.0), Strip::default(), |ui, s| {
        let layout = CarouselLayout::hero();
        let r = Carousel::new("c", layout).show(ui, 6, |ui, i| {
            ui.label(i.to_string());
        });
        s.activated.extend(r.activated);
        s.position = r.position;
    })
}

#[test]
fn carousel_click_activates_and_arrows_advance_the_current_item() {
    let mut h = hero_strip();
    h.click("c-0-item");
    assert_eq!(h.state().value.activated, [0]);
    h.press(Key::ArrowRight);
    h.step_until("strip settles", |h| {
        (h.state().value.position - 1.0).abs() < f32::EPSILON
    });
    assert!(h.focused("c-1-item"), "ArrowRight focuses the next item");
    assert_eq!(h.node("c-1-item").state.selected, Some(true), "and makes it current");
}

#[test]
fn carousel_drag_on_an_item_pans_and_snaps_to_the_next() {
    let mut h = hero_strip();
    let from = h.rect("c-0-item").center();
    h.hover_at(from);
    h.drag_at(from);
    h.step();
    for dx in 1..=8 {
        h.hover_at(from - vec2(dx as f32 * 25.0, 0.0));
        h.step();
    }
    let mid = h.state().value.position;
    assert!(mid > 0.0 && mid < 1.0, "the strip follows the drag: {mid}");
    h.drop_at(from - vec2(200.0, 0.0));
    h.step_until("strip settles", |h| {
        (h.state().value.position - 1.0).abs() < f32::EPSILON
    });
}

#[test]
fn carousel_shrinking_mid_drag_does_not_panic() {
    let mut h = still(vec2(400.0, 300.0), 6_usize, |ui, count| {
        let layout = CarouselLayout::hero();
        Carousel::new("c", layout).show(ui, *count, |ui, i| {
            ui.label(i.to_string());
        });
    });
    h.click("c-0-item");
    for i in 1..=4 {
        h.press(Key::ArrowRight);
        h.step_until("strip settles", |h| h.focused(&format!("c-{i}-item")));
    }
    let from = h.rect("c-4-item").center();
    h.hover_at(from);
    h.drag_at(from);
    h.step();
    for dx in 1..=4 {
        h.hover_at(from - vec2(dx as f32 * 25.0, 0.0));
        h.step();
    }
    h.state_mut().value = 2;
    h.step();
    h.drop_at(from - vec2(100.0, 0.0));
    h.run();
}

#[derive(Default)]
struct Page {
    offset: f32,
    position: f32,
}

fn page_with_carousel<'a>(layout: CarouselLayout, height: f32) -> Harness<'a, Page> {
    still(vec2(400.0, 300.0), Page::default(), move |ui, p| {
        let rt = ui.rt();
        let out = egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
            let mut mui = rt.ui(ui);
            let r = Carousel::new("c", layout).height(height).show(&mut mui, 6, |ui, i| {
                ui.label(i.to_string());
            });
            ui.allocate_space(vec2(1.0, 2000.0));
            r.position
        });
        p.offset = out.state.clamped_offset().y;
        p.position = out.inner;
    })
}

// eframe web order: pointerdown runs a pass before touchstart arrives.
fn touch_drag(h: &mut Harness<'_, Page>, from: Pos2, step: egui::Vec2, n: u32) {
    use egui::{Event, TouchPhase};
    h.input_mut()
        .events
        .push(synth::pointer_down(from, PointerButton::Primary));
    h.step();
    h.input_mut()
        .events
        .extend([synth::touch(from, TouchPhase::Start), Event::PointerMoved(from)]);
    h.step();
    for i in 1..=n {
        let at = from + step * i as f32;
        h.input_mut()
            .events
            .extend([synth::touch(at, TouchPhase::Move), Event::PointerMoved(at)]);
        h.step();
    }
    let end = from + step * n as f32;
    h.input_mut().events.extend([
        synth::pointer_up(end, PointerButton::Primary),
        Event::PointerGone,
        synth::touch(end, TouchPhase::End),
    ]);
    h.step();
}

const HERO: CarouselLayout = CarouselLayout::hero();

#[test]
fn carousel_vertical_touch_pan_scrolls_the_enclosing_page() {
    let mut h = page_with_carousel(HERO, 120.0);
    h.step();
    let from = h.rect("c-0-item").center();
    touch_drag(&mut h, from, vec2(0.0, -20.0), 6);
    h.run();
    let page = h.state().value.offset;
    assert!(page > 50.0, "page scrolled by the vertical pan: {page}");
    assert!(h.state().value.position.abs() < 0.01, "strip stayed put");
}

#[test]
fn carousel_horizontal_touch_pan_pages_the_strip_not_the_page() {
    let mut h = page_with_carousel(HERO, 120.0);
    h.step();
    let from = h.rect("c-0-item").center();
    touch_drag(&mut h, from, vec2(-25.0, 0.0), 6);
    h.step_until("strip settles", |h| {
        (h.state().value.position - 1.0).abs() < f32::EPSILON
    });
    assert!(h.state().value.offset.abs() < 0.01, "page stayed put");
}

#[test]
fn fullscreen_carousel_past_the_last_item_chains_the_pan_to_the_page() {
    let mut h = page_with_carousel(CarouselLayout::FullScreen, 200.0);
    h.step();
    let from = h.rect("c-0-item").center();
    for _ in 0..8 {
        touch_drag(&mut h, from, vec2(0.0, -20.0), 8);
        h.run();
    }
    let s = &h.state().value;
    assert!(s.position > 4.9, "strip reached the end: {}", s.position);
    assert!(s.offset > 50.0, "page took the rest: {}", s.offset);
}

#[test]
fn carousel_vertical_flick_keeps_the_page_coasting_after_release() {
    let mut h = page_with_carousel(HERO, 120.0);
    h.step();
    let from = h.rect("c-0-item").center();
    touch_drag(&mut h, from, vec2(0.0, -20.0), 6);
    let released = h.state().value.offset;
    h.run();
    let settled = h.state().value.offset;
    assert!(
        settled > released + 20.0,
        "page coasted after release: {released} -> {settled}"
    );
}

#[test]
fn swipeable_list_item_vertical_flick_scrolls_and_coasts_the_page() {
    use egui_mineral_material::{containment::SwipeAction, icons::ICON_DELETE};
    let mut h = still(vec2(400.0, 300.0), Page::default(), |ui, p| {
        let rt = ui.rt();
        let out = egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
            rt.ui(ui)
                .mineral(ListItem::new("row", "Row").swipe_end(SwipeAction::new(ICON_DELETE, "Delete")));
            ui.allocate_space(vec2(1.0, 2000.0));
        });
        p.offset = out.state.clamped_offset().y;
    });
    h.step();
    let from = h.rect("row-list-item").center();
    touch_drag(&mut h, from, vec2(0.0, -20.0), 6);
    let released = h.state().value.offset;
    h.run();
    let settled = h.state().value.offset;
    assert!(released > 50.0, "page followed the pan: {released}");
    assert!(settled > released + 20.0, "page coasted: {released} -> {settled}");
}
