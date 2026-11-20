use egui::{
    Color32, Event, Key,
    accesskit::{Action, ActionData, ActionRequest, TreeId},
};

use egui_mineral::{select::Selection, widget::SizeExt};
use egui_mineral_material::{
    extras::ColorPicker,
    selection::{Select, SelectOption},
    text_inputs::TextField,
};

use crate::support::{Harness, MineralHarness, SCREEN, harness};

#[test]
fn select_hides_label_keeps_name_and_skips_disabled_options() {
    let mut h = harness(SCREEN, Some(0), |ui, selected| {
        let options = [
            SelectOption::new("Wav"),
            SelectOption::new("Mp3").disabled(true),
            SelectOption::new("Flac"),
        ];
        ui.mineral(Select::new("format", "Format", options, selected).label_visible(false));
    });
    assert_eq!(
        h.node("format-select").name,
        "Format",
        "hidden label stays the accessible name"
    );
    h.click("format-select");
    assert!(h.node("format-select-1-option").state.disabled);
    h.press(Key::ArrowDown);
    h.press(Key::Enter);
    assert_eq!(h.state().value, Some(2), "arrow skips the disabled option");
}

#[test]
fn select_typeahead_skips_disabled_options() {
    let mut h = harness(SCREEN, None, |ui, selected| {
        let options = [
            SelectOption::new("Apple"),
            SelectOption::new("Avocado").disabled(true),
            SelectOption::new("Banana"),
            SelectOption::new("Apricot"),
        ];
        ui.mineral(Select::new("fruit", "Fruit", options, selected));
    });
    h.click("fruit-select");
    h.type_text("a");
    h.press(Key::Enter);
    assert_eq!(h.state().value, Some(3), "from Apple, `a` skips the disabled Avocado");
}

#[test]
fn growing_field_wraps_beside_its_affixes() {
    let height = |prefix: Option<&'static str>| {
        let h = harness(SCREEN, String::from("wrap me"), |ui, text| {
            let field = TextField::new("f", "Label", text).grow_rows(2);
            let field = match prefix {
                Some(p) => field.prefix(p),
                None => field,
            };
            ui.mineral(field.preferred_width(200.0));
        });
        h.rect("f-field").height()
    };
    assert!(
        height(Some("a prefix about as wide as the field")) > height(None),
        "the prefix narrows the span the text wraps in"
    );
}

#[test]
fn select_reveals_an_offscreen_active_option() {
    let mut h = harness(SCREEN, Some(60), |ui, selected| {
        let options = (0..100).map(|i| SelectOption::new(format!("Option {i}")));
        ui.mineral(Select::new("pick", "Pick", options, selected));
    });
    h.click("pick-select");
    assert!(
        h.tree().find_key("pick-select-60-option").is_some(),
        "opening shows the selected option"
    );
    h.press(Key::End);
    assert!(
        h.tree().find_key("pick-select-99-option").is_some(),
        "End reveals the last option"
    );
    h.press(Key::Home);
    assert!(
        h.tree().find_key("pick-select-0-option").is_some(),
        "Home reveals the first option"
    );
}

#[test]
fn rgb_fields_follow_the_color_both_ways() {
    let mut h = harness(SCREEN, Color32::from_rgb(10, 20, 30), |ui, color| {
        ui.mineral(ColorPicker::new("paint", "Paint", color).rgb_fields(true));
    });
    let green = h.node("paint-green-field");
    assert_eq!(green.state.range.map(|r| r.value), Some(20.0));
    h.event(Event::AccessKitActionRequest(ActionRequest {
        action: Action::SetValue,
        target_tree: TreeId::ROOT,
        target_node: h.widget("paint-green-field").accesskit_id(),
        data: Some(ActionData::NumericValue(200.0)),
    }));
    h.run();
    let color = h.state().value;
    assert_eq!(color, Color32::from_rgb(10, 200, 30), "field writes the color");
    let hex = format!("#{:02x}{:02x}{:02x}", color.r(), color.g(), color.b());
    assert_eq!(h.node("paint-hex-field").value.as_deref(), Some(hex.as_str()));

    h.state_mut().value = Color32::from_rgb(1, 2, 3);
    h.run();
    let values = ["red", "green", "blue"].map(|c| h.node(&format!("paint-{c}-field")).state.range.map(|r| r.value));
    assert_eq!(values, [Some(1.0), Some(2.0), Some(3.0)], "color writes the fields");
}

#[test]
fn multi_select_toggles_options_and_stays_open() {
    let mut h = harness(SCREEN, Selection::multiple(), |ui, sel| {
        ui.mineral(Select::multi("fruit", "Fruit", ["Apple", "Banana", "Cherry"], sel));
    });
    let picked = |h: &Harness<'_, Selection<usize>>| h.state().value.selected().copied().collect::<Vec<_>>();
    h.click("fruit-select");
    h.press(Key::Enter);
    assert_eq!(picked(&h), [0], "Enter toggles the active option on");
    assert!(
        h.tree().find_key("fruit-select-2-option").is_some(),
        "the list stays open"
    );
    h.press(Key::ArrowDown);
    h.press(Key::ArrowDown);
    h.press(Key::Enter);
    assert_eq!(picked(&h), [0, 2]);
    h.click("fruit-select-0-option");
    assert_eq!(picked(&h), [2], "a row click toggles its option off");
}

#[test]
fn multiline_fields_in_one_ui_scroll_independently() {
    let long = (0..12).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n");
    let rows = 2;
    let mut h = harness(SCREEN, (long, String::from("short")), |ui, (a, b)| {
        ui.mineral(TextField::new("a", "A", a).rows(rows).preferred_width(200.0));
        ui.mineral(TextField::new("b", "B", b).rows(rows).preferred_width(200.0));
    });
    // `click` settles: a shared scroll state flips scrollbar visibility every pass.
    h.click("a-field");
    h.press(Key::End);
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Fruit {
    Apple,
    Banana,
    Cherry,
}

#[test]
fn select_value_binds_the_option_value() {
    let mut h = harness(SCREEN, (Fruit::Banana, 0), |ui, (fruit, changes)| {
        let options = [
            (Fruit::Apple, "Apple"),
            (Fruit::Banana, "Banana"),
            (Fruit::Cherry, "Cherry"),
        ];
        *changes += u32::from(ui.mineral(Select::bound("fruit", "Fruit", options, fruit)).changed());
    });
    h.click("fruit-select");
    h.press(Key::ArrowDown);
    h.press(Key::Enter);
    assert_eq!(
        h.state().value,
        (Fruit::Cherry, 1),
        "down from the bound Banana picks Cherry"
    );
}
