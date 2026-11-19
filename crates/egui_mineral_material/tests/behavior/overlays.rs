use egui::{Key, Modifiers, vec2};
use egui_mineral::{date::Time, input::synth, overlay::Dismiss, semantic::Role};
use egui_mineral_material::{
    actions::{FabMenu, FabMenuItem, PushButton},
    containment::{BottomSheet, Dialog, DialogAction},
    extras::Popover,
    extras::{CommandPalette, FloatingPanel, PaletteCommand, Tour, TourStep},
    icons::{ICON_ADD, ICON_HOME, ICON_MAIL},
    navigation::{NavDestination, NavigationDrawer, NavigationRail},
    selection::{DatePicker, TimePicker},
};

use crate::support::{Harness, MineralHarness, Mui, WIDE, flag_dismissals, still};

fn trigger(ui: &mut Mui<'_>, open: &mut bool) {
    if ui.mineral(PushButton::new("trigger", "Open")).clicked() {
        *open = true;
    }
}

fn assert_focus_returns<S>(mut h: Harness<'_, S>, open: impl Fn(&S) -> bool) {
    h.press(Key::Tab);
    assert!(h.focused("trigger-button"), "Tab focuses the trigger");
    h.press(Key::Enter);
    assert!(open(&h.state().value), "Enter opens");
    h.press(Key::Escape);
    assert!(!open(&h.state().value), "Escape closes");
    assert!(h.focused("trigger-button"), "focus returns to the trigger");
}

#[test]
fn modal_bottom_sheet_returns_focus_to_the_trigger() {
    let h = still(WIDE, false, |ui, open| {
        trigger(ui, open);
        BottomSheet::new("sheet").show_modal(ui.rt(), open, |ui| {
            ui.mineral(PushButton::new("inside", "Inside"));
        });
    });
    assert_focus_returns(h, |open| *open);
}

#[test]
fn command_palette_returns_focus_and_reports_dismissal() {
    let h = still(WIDE, (false, String::new(), None), |ui, (open, query, dismissed)| {
        trigger(ui, open);
        let r = CommandPalette::new("pal", [PaletteCommand::new("Save")], query).show_modal(ui.rt(), open);
        *dismissed = dismissed.or(r.dismissed);
    });
    assert_focus_returns(h, |s| s.0);
}

#[test]
fn command_palette_reports_the_clicked_entry() {
    let mut h = still(WIDE, (true, String::new(), None), |ui, (open, query, clicked)| {
        let commands = [PaletteCommand::new("Save"), PaletteCommand::new("Open")];
        let r = CommandPalette::new("pal", commands, query).show_modal(ui.rt(), open);
        *clicked = clicked.or(r.clicked);
    });
    h.run();
    h.click("pal-1-command");
    assert_eq!((h.state().value.0, h.state().value.2), (false, Some(1)));
}

#[test]
fn command_palette_applies_the_keys_of_a_pass_in_order_past_disabled_entries() {
    let mut h = still(WIDE, (true, String::new(), None), |ui, (open, query, clicked)| {
        let commands = [
            PaletteCommand::new("Save"),
            PaletteCommand::new("Print").disabled(true),
            PaletteCommand::new("Open"),
        ];
        let r = CommandPalette::new("pal", commands, query).show_modal(ui.rt(), open);
        *clicked = clicked.or(r.clicked);
    });
    h.run();
    // One pass: Up on the first entry stays there, then Down skips the disabled one.
    let keys = [Key::ArrowUp, Key::ArrowDown].map(|k| synth::key_tap(k, Modifiers::NONE));
    h.input_mut().events.extend(keys.into_iter().flatten());
    h.run();
    h.press(Key::Enter);
    assert_eq!((h.state().value.0, h.state().value.2), (false, Some(2)));
}

#[test]
fn time_picker_modal_confirms_on_enter_and_returns_focus() {
    // Third field: the dismissal of the last shown pass.
    let mut h = still(WIDE, (false, Time::default(), None), |ui, (open, time, shown)| {
        trigger(ui, open);
        let r = TimePicker::new("tp", "Time", time).show_modal(ui.rt(), open);
        if r.response.is_some() {
            *shown = Some(r.dismissed);
        }
    });
    h.press(Key::Tab);
    h.press(Key::Enter);
    assert!(h.state().value.0, "Enter on the trigger opens");
    h.state_mut().value.2 = None;
    h.press(Key::Enter);
    assert!(!h.state().value.0, "Enter confirms the open picker");
    assert_eq!(h.state().value.2, Some(None), "confirming is no dismissal");
    assert!(h.focused("trigger-button"), "focus returns to the trigger");
}

#[test]
fn picker_modals_emit_a_dialog_node_named_by_their_label() {
    let mut h = still(WIDE, (true, Time::default()), |ui, (open, time)| {
        TimePicker::new("tp", "Alarm", time).show_modal(ui.rt(), open);
    });
    h.run();
    let dialog = h.node("tp-dialog");
    assert_eq!(dialog.role, Role::Dialog);
    assert_eq!(dialog.name, "Alarm");
    assert!(
        dialog.rect.contains_rect(h.rect("tp-confirm-button")),
        "the node spans the surface"
    );
    h.click("tp-mode-button");
    assert_eq!(h.node("tp-dialog").name, "Alarm", "in every mode");

    let mut h = still(WIDE, (true, None), |ui, (open, date)| {
        DatePicker::new("dp", "Due date", date)
            .headline("Due")
            .show_modal(ui.rt(), open);
    });
    h.run();
    let dialog = h.node("dp-dialog");
    assert_eq!((dialog.role, dialog.name.as_str()), (Role::Dialog, "Due date"));
    assert!(
        dialog.rect.contains_rect(h.rect("dp-confirm-button")),
        "the node spans the surface"
    );
}

#[test]
fn fab_menu_reports_escape_and_outside_dismissal() {
    let mut h = still(WIDE, Vec::new(), |ui, seen: &mut Vec<Dismiss>| {
        let r = FabMenu::new("m", ICON_ADD, "Actions")
            .item(FabMenuItem::new(ICON_MAIL, "Mail"))
            .show(ui);
        seen.extend(r.dismissed);
    });
    h.click("m-fab");
    assert!(h.tree().find_key("m-fab-menu").is_some(), "the toggle opens the menu");
    h.press(Key::Escape);
    assert!(h.focused("m-fab"), "Escape returns focus to the toggle");
    h.click("m-fab");
    assert!(h.tree().find_key("m-fab-menu").is_some(), "the toggle reopens the menu");
    h.click_at(h.outside());
    assert_eq!(h.state().value, [Dismiss::Escape, Dismiss::Outside]);
    assert!(h.tree().find_key("m-fab-menu").is_none(), "closed");
}

#[test]
fn full_screen_dialog_reports_close_and_escape() {
    let mut h = still(WIDE, (true, None), |ui, (open, dismissed)| {
        let r = Dialog::full_screen("fs", "Edit").show_modal(ui.rt(), open, |_| ());
        *dismissed = dismissed.or(r.dismissed);
    });
    h.run();
    h.click("fs-close-button");
    assert_eq!(h.state().value, (false, Some(Dismiss::Close)));
    h.state_mut().value = (true, None);
    h.run();
    h.press(Key::Escape);
    assert_eq!(h.state().value, (false, Some(Dismiss::Escape)));
}

#[test]
fn floating_panel_reports_close_and_escape_from_inside() {
    let mut h = still(WIDE, (true, None), |ui, (open, dismissed)| {
        let r = FloatingPanel::new("fp", "Tools").show(ui.rt(), open, |ui| {
            ui.mineral(PushButton::new("inside", "Inside"));
        });
        *dismissed = dismissed.or(r.dismissed);
    });
    h.run();
    h.press(Key::Escape);
    assert_eq!(h.state().value, (true, None), "Escape with focus outside keeps it");
    let inside = h.widget("inside-button");
    h.ctx.memory_mut(|m| m.request_focus(inside));
    h.run();
    h.press(Key::Escape);
    assert_eq!(h.state().value, (false, Some(Dismiss::Escape)));
    h.state_mut().value = (true, None);
    h.run();
    h.click("fp-close-button");
    assert_eq!(h.state().value, (false, Some(Dismiss::Close)));
}

#[test]
fn tour_advances_finishes_and_dismisses() {
    let mut h = still(WIDE, (Some(0), false, None), |ui, (step, finished, dismissed)| {
        let target = ui.mineral(PushButton::new("target", "Target")).into_response().rect;
        let r = Tour::new("tour", step)
            .step(TourStep::new(target, "One"))
            .step(TourStep::new(target, "Two"))
            .show(ui.rt());
        *finished |= r.finished;
        *dismissed = dismissed.or(r.dismissed);
    });
    h.run();
    h.click("tour-next-button");
    assert_eq!(h.state().value.0, Some(1));
    h.click("tour-next-button");
    assert_eq!(h.state().value, (None, true, None), "the last step finishes");
    h.state_mut().value = (Some(0), false, None);
    h.run();
    h.press(Key::Escape);
    assert_eq!(h.state().value, (None, false, Some(Dismiss::Escape)));
    h.state_mut().value = (Some(1), false, None);
    h.run();
    h.click("tour-close-button");
    assert_eq!(h.state().value, (None, false, Some(Dismiss::Close)));
}

#[test]
fn modal_navigation_drawer_reports_dismissal_and_picks() {
    let h = still(WIDE, (true, None), |ui, (open, dismissed)| {
        let mut selected = 0;
        let r = NavigationDrawer::new("nav", &mut selected)
            .item(NavDestination::new(ICON_HOME, "Home"))
            .show_modal(ui.rt(), open);
        *dismissed = dismissed.or(r.dismissed);
    });
    let outside = h.outside();
    assert_eq!(
        flag_dismissals(h, outside),
        [Some(Dismiss::Escape), Some(Dismiss::Outside)]
    );
    let mut h = still(WIDE, (true, 0, None), |ui, (open, selected, picked)| {
        let r = NavigationDrawer::new("nav", selected)
            .item(NavDestination::new(ICON_HOME, "Home"))
            .item(NavDestination::new(ICON_MAIL, "Mail"))
            .show_modal(ui.rt(), open);
        *picked = picked.or(r.selection.and_then(|s| s.selected));
    });
    h.run();
    h.click("nav-1-nav-item");
    assert_eq!(h.state().value, (false, 1, Some(1)), "a pick selects and closes");
}

#[test]
fn modal_navigation_rail_reports_dismissal() {
    let h = still(WIDE, (true, None), |ui, (open, dismissed)| {
        let mut selected = 0;
        let r = NavigationRail::new("rail", &mut selected)
            .item(NavDestination::new(ICON_HOME, "Home"))
            .show_modal(ui, open);
        *dismissed = dismissed.or(r.dismissed);
    });
    let outside = h.outside();
    assert_eq!(
        flag_dismissals(h, outside),
        [Some(Dismiss::Escape), Some(Dismiss::Outside)]
    );
}

#[test]
fn escape_closes_only_the_modal_above_a_popover() {
    let mut h = still(WIDE, (false, false), |ui, (pop, dialog)| {
        let trigger = ui.mineral(PushButton::new("trigger", "Open")).into_response();
        Popover::new("pop", "Details").show(ui.rt(), &trigger, pop, |ui| {
            if ui.mineral(PushButton::new("more", "More")).clicked() {
                *dialog = true;
            }
        });
        if *dialog {
            Dialog::new("dlg", "Title").show_modal(ui.rt(), dialog, |_| ());
        }
    });
    h.click("trigger-button");
    h.click("more-button");
    assert_eq!(h.state().value, (true, true), "dialog open over the popover");
    h.press(Key::Escape);
    assert_eq!(h.state().value, (true, false), "Escape closes only the dialog");
}

#[test]
fn time_picker_dial_value_uses_locale_digits() {
    let mut h = still(WIDE, (true, Time::default()), |ui, (open, time)| {
        TimePicker::new("tp", "Time", time).show_modal(ui.rt(), open);
    });
    h.run();
    h.rt().i18n().set_locale("ar-EG").expect("locale");
    h.run();
    let shown = h.node("tp-hour-dial").value.expect("value");
    assert!(!shown.is_ascii(), "{shown:?} not in ar-EG digits");
}

#[test]
fn dialog_follows_its_content_width() {
    const LONG: &str = "Body text long enough to wrap at the maximum dialog width, so that the dialog grows to it \
                        instead of keeping the narrow width it had for the short text before.";
    let mut h = still(WIDE, (0_u8, 0.0), |ui, (content, width)| {
        let mut open = true;
        let dialog = Dialog::new("dlg", "Title").action(DialogAction::new("OK"));
        let r = dialog.show_modal(ui.rt(), &mut open, |ui| match content {
            0 => ui.add(egui::Label::new("A one-line label wider than the minimum dialog width").extend()),
            1 => ui.label("Short"),
            _ => ui.label(LONG),
        });
        *width = r.response.map_or(0.0, |r| r.rect.width());
    });
    h.run();
    let wide = h.state().value.1;
    h.state_mut().value.0 = 1;
    h.run();
    let short = h.state().value.1;
    assert!(short < wide, "shrinks with its content: {wide} -> {short}");
    h.state_mut().value.0 = 2;
    h.run();
    let long = h.state().value.1;
    // Wrapping at the narrow width would keep it at `short`.
    assert!(
        long > short * 1.5,
        "wrapped text grows it back toward the maximum: {short} -> {long}"
    );
}

#[test]
fn dialog_width_bounds_fix_or_cap_it() {
    for (bounds, content) in [(400.0..=400.0, "Short"), (280.0..=320.0, "A long line of body text. ")] {
        let mut h = still(WIDE, 0.0, |ui, width| {
            let mut open = true;
            let r = Dialog::new("dlg", "Title")
                .width(bounds.clone())
                .action(DialogAction::new("OK"))
                .show_modal(ui.rt(), &mut open, |ui| ui.label(content.repeat(4)));
            *width = r.response.map_or(0.0, |r| r.rect.width());
        });
        h.run();
        let w = h.state().value;
        assert!(bounds.contains(&w.round()), "{bounds:?}: {w}");
    }
}

#[test]
fn resizable_dialog_follows_its_dragged_edge_until_reset() {
    let mut h = still(WIDE, 0.0, |ui, width| {
        let mut open = true;
        let r = Dialog::new("dlg", "Title")
            .resizable(true)
            .action(DialogAction::new("OK"))
            .show_modal(ui.rt(), &mut open, |ui| ui.label("Short"));
        *width = r.response.map_or(0.0, |r| r.rect.width());
    });
    h.run();
    let natural = h.state().value;
    let edge = h.rect("dlg-resize").center();
    h.hover_at(edge);
    h.drag_at(edge);
    h.hover_at(edge + vec2(20.0, 0.0));
    h.hover_at(edge + vec2(30.0, 0.0));
    h.drop_at(edge + vec2(30.0, 0.0));
    h.run();
    let dragged = h.state().value;
    assert!(
        (dragged - natural - 60.0).abs() < 1.0,
        "the centered dialog grows on both sides: {natural} -> {dragged}"
    );
    let edge = h.rect("dlg-resize").center();
    for _ in 0..2 {
        h.click_at(edge);
    }
    h.run();
    assert!(
        (h.state().value - natural).abs() < 1.0,
        "a double click restores content sizing"
    );
}

#[test]
fn dialog_growing_each_frame_discards_only_the_opening_pass() {
    let mut h = still(WIDE, Vec::<(u64, u32)>::new(), |ui, passes| {
        let frame = ui.ctx().cumulative_frame_nr();
        match passes.last_mut() {
            Some((f, n)) if *f == frame => *n += 1,
            _ => passes.push((frame, 1)),
        }
        let mut open = true;
        Dialog::new("dlg", "Title")
            .action(DialogAction::new("OK"))
            .show_modal(ui.rt(), &mut open, |ui| ui.add_space(20.0 + 10.0 * frame as f32));
    });
    for _ in 0..8 {
        h.step();
    }
    let per_frame: Vec<u32> = h.state().value.iter().map(|&(_, n)| n).collect();
    let late = &per_frame[per_frame.len().saturating_sub(5)..];
    assert!(
        late.iter().all(|&n| n == 1),
        "passes per frame while growing: {per_frame:?}"
    );
}
