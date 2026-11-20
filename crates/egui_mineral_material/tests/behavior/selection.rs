use egui::{Key, KeyboardShortcut, Modifiers};
use egui_mineral::{
    date::{Date, DateBounds, DateRange},
    overlay::Dismiss,
    range::{RangeModel, Track},
};
use egui_mineral_material::{
    MaterialUiExt as _,
    actions::PushButton,
    adaptive::Density,
    containment::ListItem,
    extras::MenuStrip,
    icons,
    selection::{CheckButton, DatePicker, DateRangePicker, Menu, MenuItem, Radio, Switch, ValueSlider},
};

use crate::support::{Harness, MineralHarness, SCREEN, WIDE, build, builder, harness, render_zones, still};

#[test]
fn date_range_keys_cross_months() {
    let mut h = harness(WIDE, DateRange::starting(Date::constant(2024, 5, 31)), |ui, range| {
        DateRangePicker::new("dk", "Dates", range).show(ui);
    });
    // Tab lands on the range start, the list's Tab stop.
    h.press(Key::Tab);
    h.press(Key::ArrowDown);
    h.press(Key::PageDown);
    assert!(
        h.focused("dk-2024-07-07-cell"),
        "down a week into June, then a month on"
    );
    assert!(
        h.rect("dk-grid").contains_rect(h.rect("dk-2024-07-07-cell")),
        "scrolled into view"
    );
}

#[test]
fn date_range_bounds_after_today_show_the_first_allowed_month() {
    let min = Some(Date::constant(2024, 8, 1));
    let today = Date::constant(2024, 5, 10);
    let before = DateRange::new(Date::constant(2024, 6, 3), Date::constant(2024, 6, 9));
    for range in [DateRange::default(), before] {
        let mut h = harness(WIDE, range, |ui, range| {
            DateRangePicker::new("dk", "Dates", range)
                .today(today)
                .bounds(DateBounds::new(min, None))
                .show(ui);
        });
        h.run();
        let tree = h.tree();
        let cells: Vec<_> = tree
            .nodes
            .iter()
            .map(|n| n.id.key())
            .filter(|k| k.ends_with("-cell"))
            .collect();
        assert_eq!(
            cells.first(),
            Some(&"dk-2024-08-01-cell"),
            "{range:?}: starts at the lower bound"
        );
        assert!(
            cells.iter().all(|k| *k >= "dk-2024-08-01-cell"),
            "{range:?}: no month before it"
        );
        assert_eq!(h.state().value, range, "showing never edits the bound range");
    }
}

#[test]
fn year_menu_opens_by_key_after_bounds_pass_the_shown_year() {
    let mut h = harness(WIDE, 2020, |ui, min_year: &mut i16| {
        let mut value = None;
        DatePicker::new("dp", "Date", &mut value)
            .today(Date::constant(2025, 6, 10))
            .bounds(DateBounds::new(Some(Date::constant(*min_year, 1, 1)), None))
            .show(ui);
    });
    h.run();
    h.state_mut().value = 2027;
    h.run();
    h.press(Key::Tab);
    assert!(h.focused("dp-year-button"));
    h.press(Key::Enter);
    h.press(Key::ArrowDown);
}

#[test]
fn menu_reports_every_dismissal() {
    let mut h = still(WIDE, Vec::new(), |ui, seen: &mut Vec<Dismiss>| {
        let anchor = ui.mineral(PushButton::new("anchor", "Open")).into_response();
        let r = Menu::new("menu", "Menu")
            .item(MenuItem::new("Item"))
            .show(ui.rt(), &anchor);
        seen.extend(r.dismissed);
    });
    let outside = h.outside();
    h.click("anchor-button");
    h.press(Key::Escape);
    h.click("anchor-button");
    h.click_at(outside);
    h.click("anchor-button");
    h.click("anchor-button");
    h.click("anchor-button");
    h.press(Key::Tab);
    assert_eq!(
        h.state().value,
        [Dismiss::Escape, Dismiss::Outside, Dismiss::Close, Dismiss::FocusOut]
    );
}

#[test]
fn menu_item_trailing_shortcut_is_exposed_and_icon_is_not() {
    let mut h = still(WIDE, (), |ui, (): &mut ()| {
        let anchor = ui.mineral(PushButton::new("anchor", "Open")).into_response();
        Menu::new("menu", "Menu")
            .item(
                MenuItem::new("Save")
                    .command("file.save")
                    .trailing(KeyboardShortcut::new(Modifiers::COMMAND, Key::S)),
            )
            .item(
                MenuItem::new("Share")
                    .leading(icons::ICON_MAIL)
                    .trailing(icons::ICON_SHARE),
            )
            .show(ui.rt(), &anchor);
    });
    h.click("anchor-button");
    assert!(h.node("menu-file-save-menu-item").shortcut.is_some());
    assert!(h.node("menu-1-menu-item").shortcut.is_none());
}

fn sliders(disabled: bool, range: RangeModel) -> Harness<'static, (f64, [f64; 2])> {
    let mut h = harness(SCREEN, (50.0, [20.0, 80.0]), move |ui, (value, values)| {
        ui.mineral(ValueSlider::new("one", "Volume", range, value).disabled(disabled));
        ui.mineral(ValueSlider::range("two", "Range", range, values));
        ui.mineral(PushButton::new("after", "After"));
    });
    h.press(Key::Tab);
    h
}

#[test]
fn focused_slider_keys_follow_the_range_model() {
    let range = RangeModel::new(0.0, 100.0).step(5.0).jump(20.0);
    let mut h = sliders(false, range);
    assert!(h.focused("one-slider"), "one Tab lands on the slider itself");
    for key in [Key::ArrowRight, Key::PageUp, Key::ArrowLeft, Key::Home, Key::End] {
        let before = h.state().value.0;
        let expected = range
            .on_key(before, key, Track::horizontal_ltr())
            .expect("a slider key");
        assert_ne!(expected, before, "{key:?} moves the value");
        h.press(key);
        assert_eq!(h.state().value.0, expected, "{key:?} from {before}");
    }
}

#[test]
fn slider_tab_stops_are_one_per_thumb() {
    let mut h = sliders(false, RangeModel::new(0.0, 100.0));
    h.press(Key::Tab);
    assert!(h.focused("two-slider-start"), "the single slider is one stop");
    h.press(Key::Tab);
    assert!(h.focused("two-slider-end"), "a range slider has one stop per thumb");
    h.press(Key::ArrowLeft);
    assert!(h.state().value.1[1] < 80.0, "the focused thumb takes the keys");
    h.press(Key::Tab);
    assert!(h.focused("after-button"));
}

#[test]
fn disabled_slider_ignores_keys() {
    let mut h = sliders(true, RangeModel::new(0.0, 100.0).step(5.0));
    for key in [Key::ArrowRight, Key::PageUp, Key::Home, Key::End] {
        h.press(key);
    }
    assert_eq!(h.state().value.0, 50.0);
}

#[test]
fn menu_keys_move_focus_and_enter_activates() {
    let mut h = still(WIDE, None, |ui, clicked: &mut Option<usize>| {
        let anchor = ui.mineral(PushButton::new("anchor", "Open")).into_response();
        let r = Menu::new("menu", "Menu")
            .item(MenuItem::new("Apple"))
            .item(MenuItem::new("Banana").disabled(true))
            .item(MenuItem::new("Cherry"))
            .item(MenuItem::new("Date"))
            .show(ui.rt(), &anchor);
        *clicked = clicked.or(r.clicked.as_ref().map(egui_mineral::action::Clicked::index));
    });
    let item = |i: usize| format!("menu-{i}-menu-item");
    h.click("anchor-button");
    assert!(h.node(&item(1)).state.disabled);
    assert!(!h.focused(&item(0)), "a pointer open leaves focus on the anchor");
    h.press(Key::ArrowDown);
    assert!(h.focused(&item(0)), "down moves into the menu");
    for (key, at, why) in [
        (Key::ArrowDown, 2, "down skips the disabled item"),
        (Key::ArrowDown, 3, "down"),
        (Key::ArrowDown, 0, "down wraps"),
        (Key::ArrowUp, 3, "up wraps"),
        (Key::Home, 0, "Home"),
        (Key::End, 3, "End"),
    ] {
        h.press(key);
        assert!(h.focused(&item(at)), "{why}");
    }
    h.type_text("c");
    assert!(h.focused(&item(2)), "typeahead focuses the matching item");
    h.press(Key::Enter);
    assert_eq!(h.state().value, Some(2), "Enter fires the focused item");
}

#[test]
fn escape_closes_the_focused_submenu_first() {
    let mut h = still(WIDE, Vec::new(), |ui, seen: &mut Vec<Dismiss>| {
        let anchor = ui.mineral(PushButton::new("anchor", "Open")).into_response();
        let r = Menu::new("menu", "Menu")
            .item(MenuItem::new("Share").submenu(|m| m.item(MenuItem::new("Mail")).item(MenuItem::new("Link"))))
            .item(MenuItem::new("Print"))
            .show(ui.rt(), &anchor);
        seen.extend(r.dismissed);
    });
    let item = |i: usize| format!("menu-{i}-menu-item");
    h.click("anchor-button");
    h.press(Key::ArrowDown);
    h.press(Key::ArrowRight);
    assert!(h.focused(&item(1)), "the forward key opens the submenu");
    h.press(Key::Escape);
    assert_eq!(
        h.node(&item(0)).state.expanded,
        Some(false),
        "Escape closes the submenu"
    );
    assert!(h.focused(&item(0)), "focus returns to the parent item");
    assert!(h.state().value.is_empty(), "the menu stays open");
    h.press(Key::Escape);
    assert_eq!(h.state().value, [Dismiss::Escape]);
    assert!(h.focused("anchor-button"));
}

#[test]
fn modal_picker_footer_tabs_in_reading_order_in_both_directions() {
    for (locale, rtl) in [("en-US", false), ("ar-EG", true)] {
        let setup = move |rt: &egui_mineral_material::Rt| {
            rt.core().motion().set_reduced(true);
            rt.i18n().set_locale(locale).expect("locale");
        };
        let mut h = build(builder(WIDE), Some(Box::new(setup)), true, |ui, open: &mut bool| {
            let mut value = Some(Date::constant(2025, 6, 10));
            DatePicker::new("dp", "Date", &mut value).show_modal(ui.rt(), open);
        });
        h.run();
        let (cancel, confirm) = (h.rect("dp-cancel-button"), h.rect("dp-confirm-button"));
        assert_eq!(
            confirm.left() > cancel.left(),
            !rtl,
            "{locale}: confirm sits at the end"
        );
        h.ctx.memory_mut(|m| m.request_focus(h.widget("dp-cancel-button")));
        h.press(Key::Tab);
        assert!(
            h.focused("dp-confirm-button"),
            "{locale}: Tab goes from cancel to confirm"
        );
    }
}

#[test]
fn keyboard_open_focuses_the_first_item() {
    let mut h = still(WIDE, (), |ui, ()| {
        let anchor = ui.mineral(PushButton::new("anchor", "Open")).into_response();
        Menu::new("menu", "Menu")
            .item(MenuItem::new("Apple"))
            .show(ui.rt(), &anchor);
    });
    h.click("anchor-button");
    h.press(Key::Escape);
    assert!(h.focused("anchor-button"));
    h.press(Key::Enter);
    assert!(
        h.focused("menu-0-menu-item"),
        "Enter on the anchor opens on the first item"
    );
}

#[test]
fn context_menu_opens_on_the_menu_key() {
    let mut h = still(WIDE, (), |ui, ()| {
        let target = ui.mineral(PushButton::new("target", "Target")).into_response();
        Menu::new("ctx", "Actions")
            .item(MenuItem::new("Rename"))
            .show_context(ui.rt(), &target);
    });
    h.click("target-button");
    h.press(Key::Menu);
    assert!(h.focused("ctx-0-menu-item"), "the Menu key opens it from the keyboard");
}

#[test]
fn menu_shown_at_a_rect_follows_the_app_open_state() {
    let mut h = still(
        WIDE,
        (false, None),
        |ui, (open, clicked): &mut (bool, Option<usize>)| {
            let more = ui.mineral(PushButton::new("more", "More")).into_response();
            if more.clicked() {
                *open = !*open;
            }
            let r = Menu::new("at", "More")
                .item(MenuItem::new("Copy"))
                .item(MenuItem::new("Paste"))
                .show_at(ui.rt(), more.rect, open);
            *clicked = clicked.or(r.clicked.as_ref().map(egui_mineral::action::Clicked::index));
        },
    );
    h.click("more-button");
    assert!(h.state().value.0, "opened by the app");
    let (menu, more) = (h.rect("at-1-menu-item"), h.rect("more-button"));
    assert!(menu.top() >= more.bottom() - 0.5, "below its anchor");
    h.click("more-button");
    assert!(!h.state().value.0, "a press on the anchor is not an outside press");
    h.click("more-button");
    h.click("at-1-menu-item");
    assert_eq!(h.state().value, (false, Some(1)), "picking an item closes it");
}

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

#[test]
fn escape_after_switching_menus_returns_focus_to_the_open_title() {
    let mut h = still(WIDE, (), |ui, ()| {
        MenuStrip::new("strip")
            .menu(Menu::new("file", "File").item(MenuItem::new("Open")))
            .menu(Menu::new("edit", "Edit").item(MenuItem::new("Copy")))
            .show(ui);
        ui.mineral(PushButton::new("other", "Other"));
    });
    h.click("other-button");
    h.press(Key::F10);
    h.press(Key::ArrowRight);
    assert!(h.focused("edit-menubar-item"), "Right roves to the next title");
    h.press(Key::ArrowDown);
    h.press(Key::ArrowLeft);
    h.press(Key::Escape);
    h.run();
    assert!(
        h.focused("file-menubar-item"),
        "Escape returns focus to the open menu's title"
    );
}

#[test]
fn menu_item_tooltip_describes_the_item() {
    use egui_kittest::kittest::{NodeT as _, Queryable as _};
    let mut h = still(WIDE, (), |ui, ()| {
        let anchor = ui.mineral(PushButton::new("anchor", "Open")).into_response();
        Menu::new("menu", "Menu")
            .item(MenuItem::new("Export").tooltip("Writes a PDF"))
            .show(ui.rt(), &anchor);
    });
    h.click("anchor-button");
    let description = h.get_by_label("Export").accesskit_node().description();
    assert_eq!(description.as_deref(), Some("Writes a PDF"));
}

#[test]
fn long_checkbox_label_wraps_inside_the_width_and_stays_clickable() {
    let label = "Export every selected layer, including hidden ones, as separate files";
    let mut h = harness(egui::vec2(220.0, 300.0), false, move |ui, checked| {
        ui.mineral(egui_mineral_material::selection::CheckButton::new("cb", label, checked));
    });
    let r = h.rect("cb-checkbox");
    assert!(r.width() <= 220.0, "{r:?}");
    assert!(r.height() > 48.0, "wrapped onto more lines: {r:?}");
    h.click_at(egui::pos2(r.center().x, r.bottom() - 4.0));
    assert!(h.state().value, "the last line toggles");
}

#[test]
fn a_squeezed_switch_keeps_its_track() {
    use egui_mineral::widget::SizeExt;
    let h = harness(egui::vec2(300.0, 100.0), false, |ui, on| {
        ui.mineral(
            egui_mineral_material::selection::Switch::new("sw", "Enabled", on)
                .label_visible(false)
                .preferred_width(20.0),
        );
    });
    assert!(h.rect("sw-switch").width() >= 52.0, "{:?}", h.rect("sw-switch"));
}

#[test]
fn control_rows_fit_list_rows_at_every_density() {
    for step in [0, -3, -6] {
        let density = Density::new(step);
        let height = |control: usize| {
            render_zones(|ui| {
                ui.with_density(density, |ui| {
                    let mut on = false;
                    match control {
                        0 => ui.mineral(CheckButton::new("c", "Label", &mut on)).zones(),
                        1 => ui.mineral(Radio::new("r", "Label")).zones(),
                        2 => ui.mineral(Switch::new("s", "Label", &mut on)).zones(),
                        _ => ui.mineral(ListItem::new("l", "Label")).zones(),
                    }
                })
            })
            .zones
            .map_or(f32::NAN, |z| z.outer.height())
        };
        let row = height(3);
        for control in 0..3 {
            assert!(
                height(control) <= row,
                "control {control} at density {step}: {} > {row}",
                height(control)
            );
        }
    }
    assert_eq!(egui_mineral_material::adaptive::min_interactive(Density::new(-1)), 44.0);
}

#[test]
fn menu_reports_item_keys_through_submenus() {
    let mut h = still(WIDE, Vec::new(), |ui, seen: &mut Vec<(usize, Option<String>)>| {
        let anchor = ui.mineral(PushButton::new("anchor", "Open")).into_response();
        let r = Menu::new("menu", "Menu")
            .item(MenuItem::new("Plain"))
            .item(MenuItem::new("More").submenu(|m| m.item(MenuItem::new("Nested").key("nested"))))
            .item(MenuItem::new("Last").key("last"))
            .show(ui.rt(), &anchor);
        seen.extend(r.clicked.map(|c| (c.index(), c.key::<&str>().map(|k| (*k).to_owned()))));
    });
    h.click("anchor-button");
    h.click("menu-0-menu-item");
    h.click("anchor-button");
    h.click("menu-3-menu-item");
    assert_eq!(h.state().value, [(0, None), (3, Some("last".to_owned()))]);
}
