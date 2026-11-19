use egui::Key;
use egui_mineral_material::navigation::SearchBar;

use crate::support::{MineralHarness, SCREEN, harness};

#[test]
fn search_bar_reports_submit_on_enter() {
    let mut h = harness(SCREEN, (String::new(), false), |ui, (query, submitted)| {
        *submitted |= ui.mineral(SearchBar::new("search", query)).output().submitted;
    });
    h.press(Key::Tab);
    h.type_text("rust");
    assert_eq!(h.state().value, ("rust".to_owned(), false));
    h.press(Key::Enter);
    assert_eq!(h.state().value, ("rust".to_owned(), true));
}
