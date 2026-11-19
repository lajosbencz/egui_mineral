use egui::Key;
use egui_mineral::semantic::Role;
use egui_mineral_fluent::{
    actions::PushButton,
    icons::ICON_CUT,
    navigation::{Menu, MenuItem},
};

use crate::support::{FluentHarness, Harness, harness};

#[derive(Default)]
struct MenuState {
    open: bool,
    log: Vec<&'static str>,
    checked: bool,
    radio: u8,
}

fn menu() -> Harness<'static, MenuState> {
    harness(MenuState::default(), |ui, s| {
        let rt = ui.rt();
        let trigger = ui.mineral(PushButton::new("trig", "Open")).response().clone();
        let open = &mut s.open;
        let log = &mut s.log;
        let (checked, radio) = (&mut s.checked, &mut s.radio);
        Menu::new("m", "Actions").show(rt, &trigger, open, |m| {
            if m.item(MenuItem::new("Cut").icon(ICON_CUT)).clicked {
                log.push("cut");
            }
            m.item(MenuItem::new("Off").disabled(true));
            m.divider();
            m.checkbox_item("Wrap", checked);
            m.group("Size", |m| {
                m.radio_item("Small", radio, 0);
                m.radio_item("Large", radio, 1);
            });
            m.submenu("More", |m| {
                if m.item("Deep").clicked {
                    log.push("deep");
                }
            });
        });
    })
}

#[test]
fn menu_opens_from_its_trigger_and_exposes_roles() {
    let mut h = menu();
    h.run();
    h.click("trig-button");
    h.run();
    assert!(h.state().value.open);
    assert_eq!(h.node("m-menu").role, Role::Menu);
    assert_eq!(h.node("m-0-menuitem").role, Role::MenuItem);
    assert!(h.node("m-1-menuitem").state.disabled);
    assert_eq!(h.node("m-2-menuitem").role, Role::MenuItemCheckBox);
    assert_eq!(h.node("m-3-menuitem").role, Role::MenuItemRadio);
}

#[test]
fn item_click_runs_and_closes_but_checkable_rows_keep_it_open() {
    let mut h = menu();
    h.run();
    h.click("trig-button");
    h.run();
    h.click("m-2-menuitem");
    assert!(h.state().value.checked && h.state().value.open);
    h.click("m-4-menuitem");
    assert_eq!(h.state().value.radio, 1);
    assert!(h.state().value.open);
    h.click("m-0-menuitem");
    assert_eq!(h.state().value.log, ["cut"]);
    assert!(!h.state().value.open);
}

#[test]
fn escape_closes_the_menu() {
    let mut h = menu();
    h.run();
    h.click("trig-button");
    h.run();
    h.press(Key::Escape);
    assert!(!h.state().value.open);
}

#[test]
fn opening_focuses_the_first_row_and_arrows_skip_disabled_rows() {
    let mut h = menu();
    h.run();
    h.click("trig-button");
    h.run();
    h.run();
    h.press(Key::ArrowDown);
    h.run();
    h.press(Key::Enter);
    assert!(h.state().value.checked);
    assert_eq!(h.state().value.log, Vec::<&str>::new());
}

#[test]
fn arrows_wrap_and_the_forward_arrow_opens_a_submenu() {
    let mut h = menu();
    h.run();
    h.click("trig-button");
    h.run();
    h.press(Key::ArrowUp);
    h.press(Key::ArrowRight);
    h.run();
    assert_eq!(h.node("m-5-menuitem").state.expanded, Some(true));
    assert!(h.tree().find_key("m-sub-5-0-menuitem").is_some());
}

fn open_submenu(h: &mut Harness<'static, MenuState>) {
    h.run();
    h.click("trig-button");
    h.run();
    h.press(Key::ArrowUp);
    h.press(Key::ArrowRight);
    h.run();
    h.run();
}

#[test]
fn clicking_a_submenu_row_activates_it_and_closes_the_menu() {
    let mut h = menu();
    open_submenu(&mut h);
    h.click("m-sub-5-0-menuitem");
    assert_eq!(h.state().value.log, ["deep"]);
    assert!(!h.state().value.open);
}

#[test]
fn enter_on_a_submenu_row_activates_it() {
    let mut h = menu();
    open_submenu(&mut h);
    h.press(Key::Enter);
    assert_eq!(h.state().value.log, ["deep"]);
    assert!(!h.state().value.open);
}

fn type_char(h: &mut Harness<'static, MenuState>, c: &str) {
    h.event(egui::Event::Text(c.to_owned()));
    h.run();
}

fn typeahead_menu() -> Harness<'static, MenuState> {
    harness(MenuState::default(), |ui, s| {
        let rt = ui.rt();
        let trigger = ui.mineral(PushButton::new("trig", "Open")).response().clone();
        let log = &mut s.log;
        Menu::new("t", "Actions").show(rt, &trigger, &mut s.open, |m| {
            for (name, tag) in [("Cut", "cut"), ("Copy", "copy"), ("Paste", "paste"), ("Crop", "crop")] {
                if m.item(MenuItem::new(name).disabled(name == "Crop")).clicked {
                    log.push(tag);
                }
            }
        });
    })
}

fn opened_typeahead_menu() -> Harness<'static, MenuState> {
    let mut h = typeahead_menu();
    h.run();
    h.click("trig-button");
    h.run();
    h.run();
    h
}

#[test]
fn typing_a_character_focuses_the_next_row_starting_with_it() {
    let mut h = opened_typeahead_menu();
    type_char(&mut h, "c");
    h.press(Key::Enter);
    assert_eq!(h.state().value.log, ["copy"]);
}

#[test]
fn typeahead_wraps_skips_disabled_rows_and_ignores_case() {
    let mut h = opened_typeahead_menu();
    type_char(&mut h, "C");
    type_char(&mut h, "c");
    h.press(Key::Enter);
    assert_eq!(h.state().value.log, ["cut"]);
}

#[test]
fn typeahead_ignores_unmatched_characters() {
    let mut h = opened_typeahead_menu();
    type_char(&mut h, "z");
    h.press(Key::Enter);
    assert_eq!(h.state().value.log, ["cut"]);
}

fn split_menu() -> Harness<'static, MenuState> {
    harness(MenuState::default(), |ui, s| {
        let rt = ui.rt();
        let trigger = ui.mineral(PushButton::new("trig", "Open")).response().clone();
        let open = &mut s.open;
        let log = &mut s.log;
        Menu::new("m", "Actions").show(rt, &trigger, open, |m| {
            m.item("Cut");
            let r = m.split_item("Paste", |m| {
                if m.item("Plain").clicked {
                    log.push("plain");
                }
            });
            if r.clicked {
                log.push("paste");
            }
            if m.item("Last").clicked {
                log.push("last");
            }
        });
    })
}

fn opened_split_menu() -> Harness<'static, MenuState> {
    let mut h = split_menu();
    h.run();
    h.click("trig-button");
    h.run();
    h.run();
    h
}

#[test]
fn split_main_part_activates_and_closes_the_menu() {
    let mut h = opened_split_menu();
    assert_eq!(h.node("m-split-1-menusplit").state.expanded, Some(false));
    h.click("m-1-menuitem");
    assert_eq!(h.state().value.log, ["paste"]);
    assert!(!h.state().value.open);
}

#[test]
fn split_trigger_opens_the_submenu_beside_the_row() {
    let mut h = opened_split_menu();
    h.click("m-split-1-menusplit");
    h.run();
    h.run();
    assert!(h.state().value.open);
    assert_eq!(h.node("m-split-1-menusplit").state.expanded, Some(true));
    assert_eq!(h.state().value.log, Vec::<&str>::new());
    h.click("m-sub-1-0-menuitem");
    assert_eq!(h.state().value.log, ["plain"]);
    assert!(!h.state().value.open);
}

#[test]
fn split_arrows_move_between_main_and_trigger_and_open_from_the_trigger() {
    let mut h = opened_split_menu();
    h.press(Key::ArrowDown);
    h.press(Key::ArrowRight);
    assert!(
        h.node("m-split-1-menusplit").state.focused,
        "forward arrow reaches the trigger"
    );
    h.press(Key::ArrowLeft);
    assert!(!h.node("m-split-1-menusplit").state.focused);
    h.press(Key::Enter);
    assert_eq!(h.state().value.log, ["paste"], "back on the main part");

    let mut h = opened_split_menu();
    h.press(Key::ArrowDown);
    h.press(Key::ArrowRight);
    h.press(Key::ArrowDown);
    h.press(Key::Enter);
    assert_eq!(
        h.state().value.log,
        ["last"],
        "Down from the trigger goes to the next row"
    );

    let mut h = opened_split_menu();
    h.press(Key::ArrowDown);
    h.press(Key::ArrowRight);
    h.press(Key::ArrowRight);
    h.run();
    h.run();
    assert_eq!(h.node("m-split-1-menusplit").state.expanded, Some(true));
    h.press(Key::Enter);
    assert_eq!(h.state().value.log, ["plain"], "the first submenu row has focus");
}

#[test]
fn menu_opened_by_a_menu_pick_survives_the_picking_press() {
    let mut h = harness((false, false), |ui, (first, second)| {
        let rt = ui.rt();
        let more = ui.mineral(PushButton::new("more", "More")).response().clone();
        let mut picked = false;
        Menu::new("a", "More").show(rt, &more, first, |m| {
            picked = m.item("Rename").clicked;
        });
        *second |= picked;
        ui.add_space(200.0);
        let target = ui.mineral(PushButton::new("target", "Target")).response().clone();
        Menu::new("b", "Target").show(rt, &target, second, |m| {
            m.item("Copy");
        });
    });
    h.run();
    h.click("more-button");
    h.run();
    h.click("a-0-menuitem");
    assert!(h.state().value.1, "still open after the picking pass");
}
