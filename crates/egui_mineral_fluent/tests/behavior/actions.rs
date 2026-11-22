use egui::{Key, Modifiers};
use egui_mineral::select::Check;
use egui_mineral_fluent::actions::{PushButton, SplitButton, TextLink, ToggleButton};

use crate::support::{FluentHarness, harness};

#[test]
fn click_reports_clicked() {
    let mut h = harness(0, |ui, n| {
        if ui.mineral(PushButton::new("b", "Go")).clicked() {
            *n += 1;
        }
    });
    h.run();
    h.click("b-button");
    assert_eq!(h.state().value, 1);
}

#[test]
fn disabled_button_ignores_clicks() {
    let mut h = harness(0, |ui, n| {
        if ui.mineral(PushButton::new("b", "Go").disabled(true)).clicked() {
            *n += 1;
        }
    });
    h.run();
    h.click("b-button");
    assert_eq!(h.state().value, 0);
    assert!(h.node("b-button").state.disabled);
}

#[test]
fn toggle_flips_the_bound_value_and_reports_changed() {
    let mut h = harness((false, 0), |ui, (on, changes)| {
        if ui.mineral(ToggleButton::new("t", "Bold", on)).changed() {
            *changes += 1;
        }
    });
    h.run();
    h.click("t-button");
    assert_eq!(h.state().value, (true, 1));
    assert_eq!(h.node("t-button").state.checked, Some(Check::Checked));
    h.click("t-button");
    assert_eq!(h.state().value, (false, 2));
}

#[test]
fn keyboard_activates_a_focused_button() {
    let mut h = harness(0, |ui, n| {
        if ui.mineral(PushButton::new("b", "Go")).clicked() {
            *n += 1;
        }
    });
    h.run();
    h.press(Key::Tab);
    h.press(Key::Enter);
    assert_eq!(h.state().value, 1);
}

#[test]
fn split_button_reports_each_part() {
    let mut h = harness((0, 0), |ui, (primary, menu)| {
        let r = SplitButton::new("s", "Save").show(ui);
        *primary += u32::from(r.clicked);
        *menu += u32::from(r.menu_clicked);
    });
    h.run();
    h.click("s-button");
    h.click("s-split-menu-button");
    assert_eq!(h.state().value, (1, 1));
    assert_eq!(h.node("s-split-menu-button").name, "More options");
}

#[test]
fn link_carries_its_url() {
    let h = harness((), |ui, ()| {
        ui.mineral(TextLink::new("l", "Docs").url("https://example.com"));
    });
    let n = h.node("l-link");
    assert_eq!(n.role, egui::accesskit::Role::Link);
    assert_eq!(n.value.as_deref(), Some("https://example.com"));
}

#[test]
fn marked_mnemonic_activates_and_shared_letters_cycle_focus() {
    let mut h = harness(0, |ui, clicks| {
        let (label, save_as) = egui_mineral::shortcut::Mnemonic::parse("Save &As");
        if let Some(m) = save_as
            && ui.mineral(PushButton::new("save-as", label).mnemonic(m)).clicked()
        {
            *clicks += 1;
        }
        ui.mineral(PushButton::new("bold", "Bold").mnemonic('b'));
        ui.mineral(PushButton::new("back", "Back").mnemonic('b'));
    });
    h.run();
    h.key_press_modifiers(Modifiers::ALT, Key::A);
    h.run();
    assert_eq!(h.state().value, 1, "Alt+A activates the marked button");
    for want in ["bold-button", "back-button", "bold-button"] {
        h.key_press_modifiers(Modifiers::ALT, Key::B);
        h.run();
        assert!(h.focused(want), "Alt+B focuses {want}");
    }
}
