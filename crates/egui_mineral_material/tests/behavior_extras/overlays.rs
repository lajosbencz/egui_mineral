use egui::{Key, Modifiers};
use egui_mineral::{input::synth, overlay::Dismiss};
use egui_mineral_material::{
    actions::PushButton,
    containment::Dialog,
    extras::Popover,
    extras::{CommandPalette, FloatingPanel, PaletteCommand, Tour, TourStep},
};

use crate::support::{Harness, MineralHarness, Mui, WIDE, still};

fn assert_focus_returns<S>(mut h: Harness<'_, S>, open: impl Fn(&S) -> bool) {
    h.press(Key::Tab);
    assert!(h.focused("trigger-button"), "Tab focuses the trigger");
    h.press(Key::Enter);
    assert!(open(&h.state().value), "Enter opens");
    h.press(Key::Escape);
    assert!(!open(&h.state().value), "Escape closes");
    assert!(h.focused("trigger-button"), "focus returns to the trigger");
}

fn trigger(ui: &mut Mui<'_>, open: &mut bool) {
    if ui.mineral(PushButton::new("trigger", "Open")).clicked() {
        *open = true;
    }
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
