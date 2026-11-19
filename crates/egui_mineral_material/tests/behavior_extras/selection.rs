use egui::{Key, Modifiers};
use egui_mineral_material::{
    actions::PushButton,
    extras::MenuStrip,
    selection::{Menu, MenuItem},
};

use crate::support::{MineralHarness, WIDE, still};

#[test]
fn f10_and_a_lone_alt_toggle_focus_to_the_menu_strip() {
    let mut h = still(WIDE, (), |ui, ()| {
        MenuStrip::new("strip")
            .menu(Menu::new("file", "File").item(MenuItem::new("Open")))
            .menu(Menu::new("edit", "Edit").item(MenuItem::new("Copy")))
            .show(ui);
        ui.mineral(PushButton::new("other", "Other"));
    });
    h.click("other-button");
    h.press(Key::F10);
    assert!(h.focused("file-menubar-item"), "F10 focuses the first title");
    h.press(Key::F10);
    assert!(h.focused("other-button"), "F10 again returns focus");
    for modifiers in [Modifiers::ALT, Modifiers::NONE] {
        h.event(egui::Event::ModifiersChanged(modifiers));
        h.run();
    }
    assert!(h.focused("file-menubar-item"), "a lone Alt focuses the first title");
}
