use egui_mineral_material::{
    actions::SplitButton,
    selection::{Menu, MenuItem},
};

use crate::support::{MineralHarness, WIDE, still};

#[test]
fn split_button_reports_the_leading_click_and_menu_pick() {
    let mut h = still(WIDE, (0, None, false), |ui, (leading, picked, open)| {
        let menu = Menu::new("more", "More")
            .item(MenuItem::new("Draft"))
            .item(MenuItem::new("Copy"));
        let r = SplitButton::new("send", "Send", menu).show(ui);
        *leading += usize::from(r.leading_clicked);
        *picked = picked.or(r.clicked);
        *open = r.open;
    });
    h.click("send-button");
    assert_eq!(h.state().value, (1, None, false));
    h.click("send-split-menu-button");
    assert!(h.state().value.2, "the trailing button opens the menu");
    h.click("more-1-menu-item");
    assert_eq!(h.state().value, (1, Some(1), false), "a pick closes the menu");
}
