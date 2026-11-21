use egui::{Event, Key};
use egui_mineral::select::{Check, Selection};
use egui_mineral_fluent::inputs::ColorPicker;
use egui_mineral_fluent::inputs::{Swatch, SwatchPicker};
use egui_mineral_fluent::{
    egui_mineral::{number::ScrubRate, range::RangeModel},
    inputs::{
        Checkbox, Combobox, Dropdown, Field, InfoLabel, Input, ListEntry, ListOption, OptionGroup, RadioGroup,
        RadioOption, RangeSlider, Rating, SearchBox, Select, Slider, SpinButton, Switch, TimePicker, Validation,
    },
};

use egui_mineral_fluent::{
    egui_mineral::{
        date::{Date, Weekday},
        semantic::Role,
    },
    inputs::{Calendar, DatePicker, TagPicker, TagPickerControl},
};

use crate::support::{FluentHarness, harness};

#[test]
fn checkbox_flips_and_mixed_resolves_to_checked() {
    let mut h = harness((false, 0), |ui, (on, changes)| {
        changes_if(ui.mineral(Checkbox::new("c", "Check", on)).changed(), changes);
    });
    h.run();
    h.click("c-checkbox");
    assert_eq!(h.state().value, (true, 1));
    assert_eq!(h.node("c-checkbox").state.checked, Some(Check::Checked));

    let mut m = harness(false, |ui, on| {
        ui.mineral(Checkbox::new("c", "Check", on).mixed(true));
    });
    m.run();
    assert_eq!(m.node("c-checkbox").state.checked, Some(Check::Mixed));
    m.click("c-checkbox");
    assert!(m.state().value, "the next activation checks the box");
}

fn changes_if(changed: bool, count: &mut u32) {
    *count += u32::from(changed);
}

#[test]
fn disabled_switch_ignores_clicks() {
    let mut h = harness(false, |ui, on| {
        ui.mineral(Switch::new("s", "Switch", on).disabled(true));
    });
    h.run();
    h.click("s-switch");
    assert!(!h.state().value);
    assert!(h.node("s-switch").state.disabled);
}

#[test]
fn switch_toggles() {
    let mut h = harness(false, |ui, on| {
        ui.mineral(Switch::new("s", "Switch", on));
    });
    h.run();
    h.click("s-switch");
    assert!(h.state().value);
    assert_eq!(h.node("s-switch").state.checked, Some(Check::Checked));
}

#[test]
fn radio_group_picks_by_click_and_arrow_key() {
    let mut h = harness(0_usize, |ui, v| {
        RadioGroup::new("g", "Fruit", v)
            .items(
                ["A", "B", "C"]
                    .into_iter()
                    .enumerate()
                    .map(|(i, l)| (i, RadioOption::new(l))),
            )
            .show(ui);
    });
    h.run();
    h.click("g-2-radio");
    assert_eq!(h.state().value, 2);
    h.press(Key::ArrowUp);
    assert_eq!(h.state().value, 1, "arrow keys move the selection");
    assert_eq!(h.node("g-1-radio").state.checked, Some(Check::Checked));
}

#[test]
fn slider_steps_by_keyboard() {
    let mut h = harness(40.0, |ui, v| {
        ui.mineral(Slider::new("s", "Volume", RangeModel::new(0.0, 100.0).step(10.0), v));
    });
    h.run();
    h.press(Key::Tab);
    h.press(Key::ArrowRight);
    assert_eq!(h.state().value, 50.0);
    h.press(Key::End);
    assert_eq!(h.state().value, 100.0);
}

#[test]
fn range_slider_exposes_a_node_per_thumb_and_keeps_them_ordered() {
    let mut h = harness([20.0, 30.0], |ui, v| {
        ui.mineral(RangeSlider::new(
            "r",
            "Price",
            RangeModel::new(0.0, 100.0).step(10.0),
            v,
        ));
    });
    h.run();
    assert_eq!(h.node("r-slider-start").name, "Price start");
    assert_eq!(h.node("r-slider-end").name, "Price end");
    h.press(Key::Tab);
    h.press(Key::ArrowRight);
    assert_eq!(h.state().value, [30.0, 30.0], "Tab reaches the start thumb");
    h.press(Key::End);
    let v = h.state().value;
    assert_eq!(v, [30.0, 30.0], "the start thumb stops at the end thumb");
    h.press(Key::Tab);
    h.press(Key::End);
    assert_eq!(h.state().value, [30.0, 100.0], "the end thumb is its own Tab stop");
}

#[test]
fn range_slider_binds_integers_in_whole_steps() {
    let mut h = harness([2u8, 5], |ui, v| {
        ui.mineral(RangeSlider::new("r", "Count", RangeModel::new(0.0, 10.0).step(0.5), v));
    });
    h.run();
    let track = h.rect("r-range-slider");
    h.click_at(track.lerp_inside(egui::vec2(0.3, 0.5)));
    assert_eq!(h.state().value, [3, 5], "the nearer thumb lands on a whole value");
}

#[test]
fn spin_button_steps_with_arrow_keys_and_reports_changed() {
    let mut h = harness(10.0, |ui, v| {
        ui.mineral(SpinButton::new("s", "Qty", RangeModel::new(0.0, 20.0).step(1.0), v));
    });
    h.run();
    h.press(Key::Tab);
    h.press(Key::ArrowUp);
    assert_eq!(h.state().value, 11.0);
    h.press(Key::ArrowDown);
    h.press(Key::ArrowDown);
    assert_eq!(h.state().value, 9.0);
}

#[test]
fn optional_spin_button_clears_to_none_and_reports_clamps() {
    let mut h = harness((Some(3.0), false), |ui, (v, clamped)| {
        let r = ui.mineral(SpinButton::optional("s", "Qty", RangeModel::new(0.0, 5.0).step(1.0), v));
        *clamped |= r.output().clamped;
    });
    h.run();
    h.press(Key::Tab);
    h.key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
    h.press(Key::Backspace);
    h.press(Key::ArrowUp);
    assert_eq!(h.state().value.0, Some(0.0), "first step from blank");
    h.event(Event::Text("9".into()));
    h.press(Key::Enter);
    assert_eq!(h.state().value, (Some(5.0), true), "09 is past the bound");
    h.click("s-spin-button");
    h.key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
    h.press(Key::Backspace);
    h.press(Key::Enter);
    assert_eq!(h.state().value.0, None, "blank text binds None");
}

#[test]
fn spin_button_scrubs_and_selects_after_a_step() {
    let mut h = harness(5.0, |ui, v| {
        let range = RangeModel::new(0.0, 100.0).step(1.0);
        ui.mineral(
            SpinButton::new("s", "Qty", range, v)
                .scrub(ScrubRate::pixels(4.0, 50.0))
                .select_on_step(true),
        );
    });
    h.run();
    let from = h.rect("s-spin-button").left_center() + egui::vec2(12.0, 0.0);
    h.hover_at(from);
    h.drag_at(from);
    for dx in [8.0, 20.0, 40.0] {
        h.hover_at(from + egui::vec2(dx, 0.0));
    }
    h.drop_at(from + egui::vec2(40.0, 0.0));
    h.run();
    assert_eq!(h.state().value, 15.0, "40 points at 4 per step");
    h.press(Key::Tab);
    h.press(Key::ArrowUp);
    h.event(Event::Text("7".into()));
    h.press(Key::Enter);
    assert_eq!(h.state().value, 7.0, "the step selected the text");
}

#[test]
fn rating_picks_by_click_and_keys() {
    let mut h = harness(1.0, |ui, v| {
        ui.mineral(Rating::new("r", "Rating", v));
    });
    h.run();
    let r = h.rect("r-rating");
    h.click_at(r.right_center() - egui::vec2(1.0, 0.0));
    assert_eq!(h.state().value, 5.0);
    h.press(Key::ArrowLeft);
    assert_eq!(h.state().value, 4.0);
}

#[test]
fn input_takes_typed_text() {
    let mut h = harness(String::new(), |ui, text| {
        ui.mineral(Input::new("i", "Name", text));
    });
    h.run();
    h.click("i-input");
    h.event(Event::Text("ab".into()));
    h.run();
    assert_eq!(h.state().value, "ab");
}

#[test]
fn search_box_clears_by_escape_and_by_its_dismiss_button() {
    let mut h = harness(String::from("abc"), |ui, text| {
        ui.mineral(SearchBox::new("s", "Search", text));
    });
    h.run();
    assert!(h.tree().find_key("s-clear-button").is_none(), "hidden until focused");
    h.click("s-search-box");
    h.run();
    assert_eq!(h.node("s-clear-button").name, "Clear text");
    h.click("s-clear-button");
    assert_eq!(h.state().value, "");
}

#[test]
fn field_hands_its_state_to_the_control_and_names_its_messages() {
    let h = harness(String::new(), |ui, text| {
        Field::new("f")
            .label("Email")
            .hint("Work address")
            .validation(Validation::Error, "Invalid")
            .show(ui, |ui, cx| {
                ui.mineral(Input::new("f-in", "Email", text).validation(cx.validation))
            });
    });
    assert!(h.node("f-in-input").state.invalid);
    assert_eq!(h.node("f-validation-message").name, "Invalid");
    assert_eq!(h.node("f-hint").name, "Work address");
    assert_eq!(h.node("f-label").name, "Email");
}

#[test]
fn info_label_button_toggles_expanded() {
    let mut h = harness((), |ui, ()| {
        ui.mineral(InfoLabel::new("il", "Label", "Help"));
    });
    h.run();
    assert_eq!(h.node("il-info-button").state.expanded, Some(false));
    h.click("il-info-button");
    assert_eq!(h.node("il-info-button").state.expanded, Some(true));
}

fn animals() -> Vec<ListEntry<&'static str>> {
    vec![
        OptionGroup::new("Land")
            .items([("cat", ListOption::new("Cat")), ("dog", ListOption::new("Dog"))])
            .into(),
        ("bird", ListOption::new("Bird")).into(),
    ]
}

#[test]
fn dropdown_opens_picks_and_closes() {
    let mut h = harness(Selection::<&str>::single(), |ui, sel| {
        ui.mineral(Dropdown::multi("d", "Pet", sel).items(animals()));
    });
    h.run();
    assert_eq!(h.node("d-dropdown").state.expanded, Some(false));
    h.click("d-dropdown");
    h.run();
    assert_eq!(h.node("d-dropdown").state.expanded, Some(true));
    h.click("d-dropdown-dog-option");
    h.run();
    assert!(h.state().value.is_selected(&"dog"));
    assert_eq!(h.node("d-dropdown").state.expanded, Some(false));
    assert_eq!(h.node("d-dropdown").value.as_deref(), Some("Dog"));
}

#[test]
fn dropdown_keyboard_opens_and_commits() {
    let mut h = harness(Selection::<&str>::single(), |ui, sel| {
        ui.mineral(Dropdown::multi("d", "Pet", sel).items(animals()));
    });
    h.run();
    h.click("d-dropdown");
    h.press(Key::ArrowDown);
    h.press(Key::Enter);
    assert_eq!(h.node("d-dropdown").state.expanded, Some(false));
    assert_eq!(h.state().value.len(), 1);
}

#[test]
fn multiselect_dropdown_keeps_the_list_open_and_joins_texts() {
    let mut h = harness(Selection::<&str>::multiple(), |ui, sel| {
        ui.mineral(Dropdown::multi("d", "Pet", sel).items(animals()));
    });
    h.run();
    h.click("d-dropdown");
    h.click("d-dropdown-cat-option");
    h.click("d-dropdown-bird-option");
    assert_eq!(h.node("d-dropdown").state.expanded, Some(true));
    assert_eq!(h.node("d-dropdown").value.as_deref(), Some("Cat, Bird"));
}

#[test]
fn disabled_dropdown_does_not_open() {
    let mut h = harness(Selection::<&str>::single(), |ui, sel| {
        ui.mineral(Dropdown::multi("d", "Pet", sel).items(animals()).disabled(true));
    });
    h.run();
    h.click("d-dropdown");
    assert_eq!(h.node("d-dropdown").state.expanded, Some(false));
    assert!(h.node("d-dropdown").state.disabled);
}

#[test]
fn select_binds_an_option_value() {
    let mut h = harness(None::<&str>, |ui, v| {
        ui.mineral(Select::optional("s", "Pet", v).items(animals()));
    });
    h.run();
    h.click("s-select");
    h.click("s-select-bird-option");
    assert_eq!(h.state().value, Some("bird"));
}

#[test]
fn combobox_hides_options_that_miss_the_typed_text() {
    let mut h = harness((Selection::<&str>::single(), String::new()), |ui, (sel, text)| {
        ui.mineral(Combobox::multi("c", "Pet", sel, text).items(animals()));
    });
    h.run();
    h.click("c-combobox");
    h.event(Event::Text("Bi".to_owned()));
    h.run();
    h.run();
    let options = h
        .tree()
        .nodes
        .iter()
        .filter(|n| n.role == egui_mineral_fluent::egui_mineral::semantic::Role::ListBoxOption)
        .count();
    assert_eq!(options, 1);
}

#[test]
fn combobox_filters_by_typed_text_and_picks() {
    let mut h = harness((Selection::<&str>::single(), String::new()), |ui, (sel, text)| {
        ui.mineral(Combobox::multi("c", "Pet", sel, text).items(animals()));
    });
    h.run();
    h.click("c-combobox");
    h.run();
    assert_eq!(h.node("c-combobox").state.expanded, Some(true));
    h.click("c-combobox-bird-option");
    assert_eq!(h.state().value.1, "Bird");
    assert!(h.state().value.0.is_selected(&"bird"));
}

#[test]
fn time_picker_lists_the_increment_and_picks_a_time() {
    let mut h = harness(None::<egui_mineral_fluent::egui_mineral::date::Time>, |ui, v| {
        ui.mineral(TimePicker::new("t", "Time", v).start_hour(9).end_hour(11).increment(30));
    });
    h.run();
    h.click("t-time-picker");
    h.run();
    let options = h
        .tree()
        .nodes
        .iter()
        .filter(|n| n.role == egui_mineral_fluent::egui_mineral::semantic::Role::ListBoxOption)
        .count();
    assert_eq!(options, 4);
    h.click("t-time-picker-09-30-00-option");
    let picked = h.state().value.expect("a time");
    assert_eq!((picked.hour(), picked.minute()), (9, 30));
}

fn swatches() -> Vec<(&'static str, Swatch)> {
    vec![
        ("a", Swatch::color("A", egui::Color32::RED)),
        ("b", Swatch::color("B", egui::Color32::GREEN)),
        ("c", Swatch::color("C", egui::Color32::BLUE).disabled(true)),
    ]
}

#[test]
fn swatch_picker_picks_by_click_and_skips_disabled() {
    let mut h = harness(None::<&str>, |ui, v| {
        SwatchPicker::optional("s", "Color", v).items(swatches()).show(ui);
    });
    h.run();
    h.click("s-b-swatch");
    assert_eq!(h.state().value, Some("b"));
    assert_eq!(h.node("s-b-swatch").state.checked, Some(Check::Checked));
    h.click("s-c-swatch");
    assert_eq!(h.state().value, Some("b"));
    assert!(h.node("s-c-swatch").state.disabled);
}

#[test]
fn swatch_picker_arrow_key_moves_the_selection() {
    let mut h = harness(Some("a"), |ui, v| {
        SwatchPicker::optional("s", "Color", v).items(swatches()).show(ui);
    });
    h.run();
    h.click("s-a-swatch");
    h.press(Key::ArrowRight);
    assert_eq!(h.state().value, Some("b"));
}

#[test]
fn color_picker_area_drag_sets_saturation_and_value_and_arrows_step_hue() {
    let mut h = harness(egui::ecolor::Hsva::new(0.5, 0.5, 0.5, 1.0), |ui, v| {
        ui.mineral(ColorPicker::new("c", "Color", v).alpha(true));
    });
    h.run();
    let area = h.rect("c-color-area");
    h.click_at(area.right_top());
    let v = h.state().value;
    assert!(v.s > 0.99 && v.v > 0.99, "{v:?}");
    h.click("c-hue-slider");
    h.press(Key::ArrowRight);
    let hue = h.state().value.h;
    assert!((hue - 0.51).abs() < 0.02, "{hue}");
    assert_eq!(
        h.node("c-alpha-slider").role,
        egui_mineral_fluent::egui_mineral::semantic::Role::Slider
    );
}

fn day(d: i8) -> Date {
    Date::new(2026, 11, d).expect("valid date")
}

fn calendar_harness<'a>() -> crate::support::Harness<'a, Option<Date>> {
    harness(None::<Date>, |ui, v| {
        ui.mineral(
            Calendar::new("c", "Day", v)
                .today(day(18))
                .first_weekday(Weekday::Monday),
        );
    })
}

#[test]
fn calendar_click_selects_a_day_and_arrows_move_the_focused_day() {
    let mut h = calendar_harness();
    h.run();
    h.click("c-calendar-2026-11-12-day");
    assert_eq!(h.state().value, Some(day(12)));
    let mut f = harness(None::<Date>, |ui, v| {
        ui.mineral(
            Calendar::new("c", "Day", v)
                .today(day(18))
                .first_weekday(Weekday::Monday)
                .autofocus(true),
        );
    });
    f.run();
    f.press(Key::ArrowRight);
    f.press(Key::ArrowDown);
    f.press(Key::Enter);
    assert_eq!(f.state().value, Some(day(26)));
    assert_eq!(h.node("c-calendar-2026-11-18-day").value.as_deref(), Some("today"));
}

#[test]
fn calendar_header_opens_the_month_and_year_pickers() {
    let mut h = calendar_harness();
    h.run();
    h.click("c-calendar-title");
    h.click("c-calendar-2-month");
    h.click("c-calendar-2026-03-15-day");
    assert_eq!(h.state().value, Some(Date::new(2026, 3, 15).expect("valid date")));
    h.click("c-calendar-title");
    h.click("c-calendar-title");
    h.click("c-calendar-4-year");
    h.click("c-calendar-0-month");
    h.click("c-calendar-2024-01-10-day");
    assert_eq!(h.state().value.map(Date::year), Some(2024));
}

#[test]
fn calendar_navigation_buttons_step_months_and_go_to_today_returns() {
    let mut h = calendar_harness();
    h.run();
    h.click("c-calendar-nav-1");
    h.click("c-calendar-2026-12-10-day");
    assert_eq!(h.state().value, Some(Date::new(2026, 12, 10).expect("valid date")));
    h.click("c-calendar-go-to-today");
    h.click("c-calendar-2026-11-03-day");
    assert_eq!(h.state().value, Some(day(3)));
}

#[test]
fn tag_picker_picks_removes_and_backspace_drops_the_last_tag() {
    let animals = || vec![("a", ListOption::new("Ant")), ("b", ListOption::new("Bee"))];
    let mut h = harness(
        (Selection::<&str>::multiple(), String::new()),
        move |ui, (sel, text)| {
            ui.mineral(TagPicker::new("t", "Pets", sel, text).items(animals()));
        },
    );
    h.run();
    h.click("t-tag-picker");
    let options = |h: &crate::support::Harness<'_, (Selection<&str>, String)>| {
        h.tree().nodes.iter().filter(|n| n.role == Role::ListBoxOption).count()
    };
    assert_eq!(options(&h), 2);
    h.click("t-tag-picker-a-option");
    assert!(h.state().value.0.is_selected(&"a"));
    h.click("t-tag-picker");
    assert_eq!(options(&h), 1, "selected options leave the list");
    h.click("t-tag-picker-b-option");
    assert_eq!(h.state().value.0.len(), 2);
    h.click("t-tag-picker-a-dismiss");
    assert!(!h.state().value.0.is_selected(&"a"));
    assert_eq!(h.state().value.0.len(), 1);
    h.click("t-tag-picker");
    h.press(Key::Backspace);
    assert!(
        h.state().value.0.is_empty(),
        "Backspace in the empty input removes the last tag"
    );
}

#[test]
fn date_picker_picks_from_the_calendar_and_parses_typed_text() {
    let mut h = harness(None::<Date>, |ui, v| {
        ui.mineral(
            DatePicker::new("d", "Date", v)
                .today(day(18))
                .first_weekday(Weekday::Monday),
        );
    });
    h.run();
    h.click("d-date-picker");
    h.event(Event::Text("11/5/2026".to_owned()));
    h.press(Key::Enter);
    assert_eq!(h.state().value, Some(day(5)));
    h.click("d-date-picker");
    h.click("d-date-picker-calendar-2026-11-12-day");
    assert_eq!(h.state().value, Some(day(12)));
    assert!(
        h.tree().nodes.iter().all(|n| n.role != Role::GridCell),
        "picking closes the calendar"
    );
}

#[test]
fn tag_picker_button_variant_opens_the_list_without_an_input() {
    let animals = || vec![("a", ListOption::new("Ant")), ("b", ListOption::new("Bee"))];
    let mut h = harness(
        (Selection::<&str>::multiple(), String::new()),
        move |ui, (sel, text)| {
            ui.mineral(
                TagPicker::new("t", "Pets", sel, text)
                    .items(animals())
                    .control(TagPickerControl::Button),
            );
        },
    );
    h.run();
    assert_eq!(h.node("t-tag-picker").role, Role::ComboBox);
    h.click("t-tag-picker");
    h.click("t-tag-picker-b-option");
    assert!(h.state().value.0.is_selected(&"b"));
    assert_eq!(h.state().value.1, "");
}

#[test]
fn salted_dropdown_option_keeps_its_key_and_picks_when_an_option_is_inserted_before() {
    let mut h = harness((Selection::<&str>::single(), false), |ui, (sel, before)| {
        let mut options = Vec::new();
        if *before {
            options.push(("Extra", ListOption::new("Extra")));
        }
        options.push(("dog", ListOption::new("Dog").salt("dog")));
        ui.mineral(Dropdown::multi("d", "Pet", sel).items(options));
    });
    h.run();
    h.state_mut().value.1 = true;
    h.run();
    h.click("d-dropdown");
    h.run();
    h.click("d-dropdown-dog-option");
    h.run();
    assert!(h.state().value.0.is_selected(&"dog"));
}

#[test]
fn form_blocks_an_empty_required_field_then_submits() {
    use egui_mineral_fluent::{
        egui_mineral::form::{Form, FormState},
        inputs::{FormExt, FormUiExt},
    };

    let mut h = harness(
        (FormState::default(), String::new(), false),
        |ui, (state, name, submitted)| {
            let r = Form::new("signup", state, name).show(ui, |f, name| {
                f.text("name", "Name", name, |s| s.required(true).supporting("Shown to others"));
            });
            *submitted |= r.submitted;
        },
    );
    h.run();
    assert_eq!(h.node("signup-name-field-hint").name, "Shown to others");
    h.click("signup-submit-button");
    assert!(!h.state().value.2);
    assert_eq!(h.node("signup-name-input").role, Role::TextInput);
    assert_eq!(
        h.node("signup-name-field-validation-message").name,
        "This field is required"
    );
    "Ada".clone_into(&mut h.state_mut().value.1);
    h.run();
    h.click("signup-submit-button");
    assert!(h.state().value.2, "the value set from code submits");
}

#[test]
fn form_renders_typed_kinds_and_submits_their_values() {
    use egui_mineral_fluent::{
        egui_mineral::form::{Form, FormState},
        inputs::{FormExt, FormUiExt},
    };

    #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, egui_mineral::semantic::KeySalt)]
    enum Role {
        Dev,
        Pm,
    }

    #[derive(Default)]
    struct Signup {
        role: Option<Role>,
        terms: bool,
    }

    let mut h = harness(
        (FormState::default(), Signup::default(), false),
        |ui, (state, value, submitted)| {
            let r = Form::new("signup", state, value).show(ui, |f, v| {
                f.select(
                    "role",
                    "Role",
                    &mut v.role,
                    [(Role::Dev, "Developer"), (Role::Pm, "PM")],
                    |s| s,
                );
                f.check("terms", "Accept", &mut v.terms, |s| s.required(true));
            });
            *submitted |= r.submitted;
        },
    );
    h.run();
    h.click("signup-submit-button");
    assert!(!h.state().value.2, "the unchecked required box blocks");
    h.click("signup-terms-checkbox");
    h.click("signup-role-dropdown");
    h.run();
    h.click("signup-role-dropdown-dev-option");
    h.run();
    h.click("signup-submit-button");
    let (_, value, submitted) = &h.state().value;
    assert!(*submitted);
    assert_eq!((value.terms, value.role), (true, Some(Role::Dev)));
}

#[test]
fn form_number_field_binds_integers_and_validates_them() {
    use egui_mineral_fluent::{
        egui_mineral::form::{Form, FormState, NumberRange},
        inputs::{FormExt, FormUiExt},
    };

    let mut h = harness((FormState::default(), 0u8, false), |ui, (state, value, submitted)| {
        let r = Form::new("f", state, value).show(ui, |f, v| {
            f.number("years", "Years", v, 0.0..=10.0, |s| {
                s.validator(NumberRange::at_most(1.0))
            });
        });
        *submitted |= r.submitted;
    });
    h.click("f-years-spin-button");
    h.press(Key::ArrowUp);
    h.press(Key::ArrowUp);
    h.click("f-submit-button");
    let (state, value, submitted) = &h.state().value;
    assert_eq!(*value, 2, "whole steps on a u8");
    assert!(!submitted, "above the validator's max");
    assert!(state.field("years").and_then(|f| f.error()).is_some());
}
