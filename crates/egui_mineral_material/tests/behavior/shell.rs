use egui::{Id, Key, Rect, UiBuilder, Vec2, vec2};
use egui_mineral_material::{
    IconAction,
    actions::{PushButton, Segment, SegmentedButton},
    containment::{Dialog, DialogAction, SideSheet},
    extras::SettingRow,
    icons::ICON_STAR,
    navigation::{DockedToolbar, TopAppBar},
    selection::Switch,
};

use crate::support::{Harness, MineralHarness, Mui, SCREEN, WIDE, harness, still};

#[derive(Default)]
struct Bar {
    inline: Vec<bool>,
    overflow: bool,
    clicked: Option<usize>,
}

fn bar(ui: &mut Mui<'_>, n: usize, out: &mut Bar) {
    let r = TopAppBar::new("bar", "Title")
        .actions((0..n).map(|i| (i, IconAction::new(ICON_STAR, format!("Action {i}")))))
        .show(ui);
    out.inline = (0..n).map(|i| r.actions.get(&i).is_some()).collect();
    out.overflow = r.overflow.is_some();
    out.clicked = out.clicked.or(r.clicked);
}

#[test]
fn app_bar_overflow_partition_by_window_class() {
    // (window class, action count, inline count): COMPACT_ACTIONS = 3, MAX_ACTIONS = 5 in navigation/top_app_bar.rs,
    // the overflow button takes the last slot.
    for (expanded, n, inline) in [(false, 3, 3), (false, 5, 2), (true, 5, 5), (true, 6, 4)] {
        let mut h = harness(SCREEN, Bar::default(), |ui, out| bar(ui, n, out));
        if expanded {
            let width = h.rt().core().policies().breakpoints[1];
            h.set_size(vec2(width, SCREEN.y));
            h.run();
        }
        let got = &h.state().value;
        let shown = got.inline.iter().take_while(|s| **s).count();
        assert_eq!(
            (shown, got.inline.iter().filter(|s| **s).count()),
            (inline, inline),
            "expanded {expanded}, {n} actions"
        );
        assert_eq!(got.overflow, inline < n, "overflow button iff actions overflow");
    }
}

#[test]
fn overflowed_action_keeps_index_and_agent_key() {
    let mut h = harness(SCREEN, Bar::default(), |ui, out| bar(ui, 5, out));
    h.click("bar-overflow-button");
    assert_eq!(h.state().value.clicked, None);
    h.click("bar-3-action-button");
    assert_eq!(h.state().value.clicked, Some(3));
}

#[derive(Default)]
struct Form {
    open: bool,
    name: String,
    notes: String,
    action: Option<usize>,
}

fn enter_with_focus(focus: Option<&str>) -> Option<usize> {
    let form = Form {
        open: true,
        ..Form::default()
    };
    let mut h = harness(WIDE, form, |ui, form| {
        let r = Dialog::new("dlg", "Rename")
            .action(0, DialogAction::new("Cancel"))
            .action(1, DialogAction::new("Save"))
            .default_action(1)
            .show_modal(ui.rt(), &mut form.open, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut form.name)
                        .id(Id::unique("name"))
                        .hint_text("Name"),
                );
                ui.add(
                    egui::TextEdit::multiline(&mut form.notes)
                        .id(Id::unique("notes"))
                        .hint_text("Notes"),
                );
            });
        form.action = form
            .action
            .or(r.clicked);
    });
    if let Some(f) = focus {
        let id = if f.ends_with("-button") {
            h.widget(f)
        } else {
            Id::unique(f)
        };
        h.ctx.memory_mut(|m| m.request_focus(id));
        h.run();
    }
    h.press(Key::Enter);
    h.state().value.action
}

#[test]
fn dialog_enter_triggers_default_unless_focus_handles_it() {
    assert_eq!(enter_with_focus(None), Some(1), "no focus");
    assert_eq!(enter_with_focus(Some("name")), Some(1), "single-line edit");
    assert_eq!(enter_with_focus(Some("notes")), None, "multi-line edit keeps Enter");
    assert_eq!(
        enter_with_focus(Some("dlg-0-action-button")),
        Some(0),
        "focused button activates"
    );
}

#[test]
fn standard_side_sheet_docks_and_reserves_width() {
    let mut h = still(WIDE, (false, 0.0), |ui, (open, rest)| {
        SideSheet::new("sheet").show(ui, open, |_| {});
        *rest = ui.available_rect_before_wrap().width();
    });
    let full = h.state().value.1;
    assert!(
        h.tree().find_key("sheet-side-sheet").is_none(),
        "closed sheet is not shown"
    );
    h.state_mut().value.0 = true;
    h.run();
    let sheet = h.rect("sheet-side-sheet").width();
    assert_eq!(h.state().value.1, full - sheet, "content keeps the rest");
    h.state_mut().value.0 = false;
    h.run();
    assert_eq!(h.state().value.1, full, "closed sheet reserves nothing");
}

#[test]
fn setting_row_stacks_below_first_breakpoint() {
    let h = harness(WIDE, Vec::new(), |ui, stacked: &mut Vec<bool>| {
        stacked.clear();
        let rt = ui.rt();
        let breakpoint = rt.core().policies().breakpoints[0];
        for (i, w) in [breakpoint / 2.0, breakpoint + 1.0].into_iter().enumerate() {
            let rect = Rect::from_min_size(ui.cursor().min, Vec2::new(w, WIDE.y / 4.0));
            ui.scope_builder(UiBuilder::new().max_rect(rect), |ui| {
                let mut on = false;
                let r = SettingRow::new(("row", i), "Wi-Fi").show(&mut rt.ui(ui), |ui, label| {
                    ui.mineral(Switch::new(("switch", i), label, &mut on).label_visible(false))
                });
                stacked.push(r.stacked);
            });
        }
    });
    assert_eq!(h.state().value, [true, false]);
    let names = h.tree().nodes.iter().filter(|n| n.name == "Wi-Fi").count();
    assert_eq!(names, 2, "one named node (the switch) per row");
}

#[test]
fn resizable_side_sheet_drags_and_steps_within_its_range() {
    let mut h = still(WIDE, true, |ui, open| {
        SideSheet::new("sheet")
            .width(300.0..=420.0)
            .resizable(true)
            .show(ui, open, |_| {});
    });
    h.step_until("the sheet slides in", |h| {
        (h.rect("sheet-side-sheet").width() - 360.0).abs() < 0.5
    });
    let edge = h.rect("sheet-resize");
    h.hover_at(edge.center());
    h.drag_at(edge.center());
    h.hover_at(edge.center() - egui::vec2(20.0, 0.0));
    h.hover_at(edge.center() - egui::vec2(40.0, 0.0));
    h.drop_at(edge.center() - egui::vec2(40.0, 0.0));
    h.run();
    assert_eq!(
        h.rect("sheet-side-sheet").width(),
        400.0,
        "dragging toward the start grows it"
    );
    for _ in 0..4 {
        if !h.focused("sheet-resize") {
            h.press(Key::Tab);
        }
    }
    assert!(h.focused("sheet-resize"), "the handle is a Tab stop");
    h.press(Key::End);
    assert_eq!(h.rect("sheet-side-sheet").width(), 420.0, "End reaches the maximum");
    h.press(Key::ArrowRight);
    assert!(h.rect("sheet-side-sheet").width() < 420.0, "toward the end shrinks it");
}

#[test]
fn start_side_sheet_opens_at_its_initial_width_and_resizes() {
    use egui_mineral::layout::Edge;
    let width = |h: &Harness<'_, bool>| h.rect("sheet-side-sheet").width();
    let mut h = still(WIDE, true, |ui, open| {
        SideSheet::new("sheet")
            .edge(Edge::Start)
            .initial_width(280.0)
            .resizable(true)
            .show(ui, open, |_| {});
    });
    h.step_until("the sheet slides in", |h| {
        h.tree().find_key("sheet-resize").is_some() && (width(h) - 280.0).abs() < 0.5
    });
    let edge = h.rect("sheet-resize");
    h.hover_at(edge.center());
    h.drag_at(edge.center());
    h.hover_at(edge.center() + egui::vec2(20.0, 0.0));
    h.hover_at(edge.center() + egui::vec2(40.0, 0.0));
    h.drop_at(edge.center() + egui::vec2(40.0, 0.0));
    h.run();
    assert_eq!(width(&h), 320.0, "dragging toward the end grows it");
    for _ in 0..4 {
        if !h.focused("sheet-resize") {
            h.press(Key::Tab);
        }
    }
    h.press(Key::ArrowRight);
    assert!(width(&h) > 320.0, "toward the end grows it");
    h.press(Key::Home);
    assert_eq!(width(&h), 256.0, "the persisted width replaces the initial one");
}

#[test]
fn resizable_modal_side_sheet_drags_clamps_and_persists() {
    let mut h = still(WIDE, true, |ui, open| {
        SideSheet::new("sheet")
            .width(300.0..=380.0)
            .resizable(true)
            .show_modal(ui.rt(), open, |_| {});
    });
    h.step_until("the sheet slides in", |h| {
        h.tree().find_key("sheet-resize").is_some() && (h.rect("sheet-side-sheet").width() - 360.0).abs() < 0.5
    });
    let edge = h.rect("sheet-resize");
    h.hover_at(edge.center());
    h.drag_at(edge.center());
    h.hover_at(edge.center() - egui::vec2(200.0, 0.0));
    h.drop_at(edge.center() - egui::vec2(200.0, 0.0));
    h.run();
    assert_eq!(h.rect("sheet-side-sheet").width(), 380.0, "drag clamps to the maximum");
    h.run();
    h.run();
    assert_eq!(h.rect("sheet-side-sheet").width(), 380.0, "the width persists");
    for _ in 0..8 {
        if !h.focused("sheet-resize") {
            h.press(Key::Tab);
        }
    }
    assert!(h.focused("sheet-resize"), "the handle is a Tab stop");
    h.press(Key::Home);
    assert_eq!(h.rect("sheet-side-sheet").width(), 300.0, "Home clamps to the minimum");
}

#[test]
fn side_sheet_edge_docks_and_slides_at_the_start_in_both_directions() {
    use egui_mineral::{i18n::Direction, layout::Edge};
    for (dir, left) in [(Direction::Ltr, true), (Direction::Rtl, false)] {
        let pass = crate::support::render_directed(dir, WIDE, |ui| {
            let mut open = true;
            SideSheet::new("sheet")
                .edge(Edge::Start)
                .width(200.0..=200.0)
                .show(ui, &mut open, |_| {});
        });
        let sheet = pass.tree.find_key("sheet-side-sheet").expect("sheet").rect;
        let near_left = sheet.center().x < WIDE.x / 2.0;
        assert_eq!(near_left, left, "{dir:?}: sheet at the reading start, {sheet:?}");
    }
    let pass = crate::support::render_directed(Direction::Ltr, WIDE, |ui| {
        let mut open = true;
        SideSheet::new("sheet")
            .edge(Edge::Start)
            .width(200.0..=200.0)
            .show_modal(ui.rt(), &mut open, |_| {});
    });
    let sheet = pass.tree.find_key("sheet-side-sheet").expect("modal sheet").rect;
    assert!(sheet.left().abs() < 1.0, "modal at the start: {sheet:?}");
    let pass = crate::support::render_directed(Direction::Ltr, WIDE, |ui| {
        let mut open = true;
        SideSheet::new("sheet").show_modal(ui.rt(), &mut open, |_| {});
    });
    let sheet = pass.tree.find_key("sheet-side-sheet").expect("modal sheet").rect;
    assert!(
        (sheet.right() - WIDE.x).abs() < 1.0,
        "modal default at the end: {sheet:?}"
    );
}

#[test]
fn side_sheet_without_scroll_gives_content_the_full_height_and_padding() {
    use egui::Margin;
    let mut h = still(WIDE, (true, Vec2::ZERO, Vec2::ZERO), |ui, (open, bare, padded)| {
        SideSheet::new("a")
            .scrollable(false)
            .content_padding(Margin::ZERO)
            .headline("H")
            .show(ui, open, |ui| *bare = ui.available_size());
        SideSheet::new("b")
            .scrollable(false)
            .content_padding(Margin::same(10))
            .headline("H")
            .show(ui, open, |ui| *padded = ui.available_size());
    });
    h.run();
    let (bare, padded) = (h.state().value.1, h.state().value.2);
    let sheet = h.rect("a-side-sheet");
    assert!(
        (bare.y - (sheet.height() - 64.0)).abs() < 1.0,
        "full remaining height: {bare:?} in {sheet:?}"
    );
    assert!((bare.x - sheet.width()).abs() < 1.0, "no side padding: {bare:?}");
    assert!((bare.x - padded.x - 20.0).abs() < 1.0 && (bare.y - padded.y - 20.0).abs() < 1.0);
}

#[test]
fn toolbar_custom_items_share_the_row_height_and_the_roving_order() {
    let mut h = still(WIDE, 0usize, |ui, seg| {
        DockedToolbar::new("t", "Edit")
            .action(0, IconAction::new(ICON_STAR, "A"))
            .action(1, IconAction::new(ICON_STAR, "B"))
            .custom("seg", |ui| {
                SegmentedButton::new("seg", "Mode", seg)
                    .items(
                        ([Segment::new("Field"), Segment::new("Both"), Segment::new("Intensity")])
                            .into_iter()
                            .enumerate(),
                    )
                    .show(ui)
                    .response
            })
            .custom("go", |ui| ui.mineral(PushButton::new("go", "Go")).response().clone())
            .show(ui);
    });
    h.run();
    let bar = h.rect("t-toolbar");
    for key in ["seg-0-segment", "seg-2-segment", "go-button"] {
        let r = h.rect(key);
        assert!(bar.contains_rect(r), "{key} {r:?} inside {bar:?}");
        assert!((r.center().y - bar.center().y).abs() < 0.5, "{key} centered in the row");
    }
    assert!(
        h.rect("seg-0-segment").right() <= h.rect("go-button").left(),
        "call order"
    );
    h.press(Key::Tab);
    for _ in 0..4 {
        if !h.focused("t-1-button") {
            h.press(Key::Tab);
        }
    }
    assert!(h.focused("t-1-button"));
    h.press(Key::ArrowRight);
    assert!(h.focused("go-button"), "arrows rove into the custom item");
    h.press(Key::ArrowLeft);
    assert!(h.focused("t-1-button"));
}

#[test]
fn docked_toolbar_hides_custom_items_that_do_not_fit() {
    let mut h = still(vec2(200.0, 200.0), (), |ui, ()| {
        DockedToolbar::new("t", "Edit")
            .action(0, IconAction::new(ICON_STAR, "A"))
            .custom("go", |ui| ui.mineral(PushButton::new("go", "Go")).response().clone())
            .custom("wide", |ui| {
                ui.mineral(PushButton::new("wide", "A very wide button label"))
                    .response()
                    .clone()
            })
            .show(ui);
    });
    h.run();
    h.run();
    assert!(h.tree().find_key("wide-button").is_none(), "overflowed item is hidden");
    assert!(h.tree().find_key("go-button").is_some(), "the one that fits stays");
}

#[test]
fn top_app_bar_reports_action_keys_inline_and_overflowed() {
    let state: (Vec<String>, bool) = (Vec::new(), false);
    let mut h = harness(SCREEN, state, |ui, (keys, anchored)| {
        let r = TopAppBar::new("bar", "Title")
            .actions((0..5).map(|i| (format!("k{i}"), IconAction::new(ICON_STAR, format!("Action {i}")))))
            .show(ui);
        *anchored = r.actions.get(&"k0".to_owned()).is_some() && r.actions.get(&"k4".to_owned()).is_none();
        keys.extend(r.clicked);
    });
    h.click("bar-k0-action-button");
    h.click("bar-overflow-button");
    h.click("bar-k4-action-button");
    let (keys, anchored) = &h.state().value;
    assert_eq!(keys, &["k0", "k4"]);
    assert!(anchored, "inline actions resolve by key, overflowed ones do not");
}
