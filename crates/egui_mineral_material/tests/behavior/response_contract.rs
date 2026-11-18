use egui_mineral_material::{
    actions::{IconButton, PushButton, ToggleButton},
    icons,
    navigation::{NavDestination, NavigationBar, Tab, Tabs},
    selection::{CheckButton, Chip, Switch},
};

use crate::support::{MineralHarness, Mui, SCREEN, still};

type Seen = Vec<(bool, bool)>;

fn click_key(key: &str, mut show: impl FnMut(&mut Mui<'_>) -> (bool, bool)) -> Seen {
    let mut h = still(SCREEN, Seen::new(), |ui, seen| seen.push(show(ui)));
    h.state_mut().value.clear();
    h.click(key);
    h.state().value.clone()
}

fn assert_agree(seen: &Seen) {
    assert!(seen.iter().all(|(field, response)| field == response), "{seen:?}");
    assert!(seen.iter().any(|(field, _)| *field), "no click registered: {seen:?}");
}

#[test]
fn tabs_response_changed_matches_field() {
    let mut selected = 0;
    let seen = click_key("tabs-1-tab", |ui| {
        let r = Tabs::new("tabs", "Pages", &mut selected)
            .item(Tab::new("One"))
            .item(Tab::new("Two"))
            .show(ui);
        (r.changed, r.response.changed())
    });
    assert_agree(&seen);
    assert_eq!(selected, 1);
}

#[test]
fn navigation_bar_response_changed_matches_field() {
    let mut selected = 0;
    let seen = click_key("bar-1-nav-item", |ui| {
        let r = NavigationBar::new("bar", &mut selected)
            .item(NavDestination::new(icons::ICON_HOME, "Home"))
            .item(NavDestination::new(icons::ICON_SEARCH, "Search"))
            .show(ui);
        (r.changed, r.response.changed())
    });
    assert_agree(&seen);
    assert_eq!(selected, 1);
}

#[test]
fn button_click_is_clicked_not_changed() {
    let mut seen = Vec::new();
    click_key("btn-button", |ui| {
        let r = ui.mineral(PushButton::new("btn", "Press me"));
        seen.push((r.clicked(), r.changed(), r.response().changed()));
        (false, false)
    });
    assert!(seen.iter().any(|s| s.0), "no click registered: {seen:?}");
    assert!(seen.iter().all(|s| !s.1 && !s.2), "{seen:?}");
}

fn toggle_click(key: &str, mut show: impl FnMut(&mut Mui<'_>, &mut bool) -> (bool, bool)) -> (bool, Seen) {
    let mut h = still(SCREEN, (false, Seen::new()), |ui, (on, seen)| {
        let pair = show(ui, on);
        seen.push(pair);
    });
    h.state_mut().value.1.clear();
    h.click(key);
    let (on, seen) = &h.state().value;
    (*on, seen.clone())
}

#[test]
fn toggle_click_flips_value_and_reports_changed() {
    type Show = fn(&mut Mui<'_>, &mut bool) -> (bool, bool);
    let toggles: [(&str, Show); 5] = [
        ("t-button", |ui, on| {
            let r = ui.mineral(ToggleButton::new("t", "Toggle", on));
            (r.changed(), r.response().changed())
        }),
        ("t-button", |ui, on| {
            let r = ui.mineral(IconButton::toggle("t", icons::ICON_STAR, "Toggle", on));
            (r.changed(), r.response().changed())
        }),
        ("t-chip", |ui, on| {
            let r = ui.mineral(Chip::filter("t", "Toggle", on));
            (r.changed(), r.response().changed())
        }),
        ("t-checkbox", |ui, on| {
            let r = ui.mineral(CheckButton::new("t", "Toggle", on));
            (r.changed(), r.response().changed())
        }),
        ("t-switch", |ui, on| {
            let r = ui.mineral(Switch::new("t", "Toggle", on));
            (r.changed(), r.response().changed())
        }),
    ];
    for (i, (key, show)) in toggles.into_iter().enumerate() {
        let (on, seen) = toggle_click(key, show);
        assert!(on, "toggle {i} flips the bound value");
        assert_agree(&seen);
    }
}

#[test]
fn assist_chip_click_is_clicked_not_changed() {
    let mut seen = Vec::new();
    click_key("c-chip", |ui| {
        let r = ui.mineral(Chip::assist("c", "Assist"));
        seen.push((r.clicked(), r.changed(), r.response().changed()));
        (false, false)
    });
    assert!(seen.iter().any(|s| s.0), "no click registered: {seen:?}");
    assert!(seen.iter().all(|s| !s.1 && !s.2), "{seen:?}");
}
