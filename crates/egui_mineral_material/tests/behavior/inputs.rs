use egui::Key;
use egui_mineral_material::{
    extras::{Combobox, ComboboxOutput, InlineEdit, InlineEditOutput, OtpField, TagsField, TagsFieldOutput},
    navigation::SearchBar,
};

use crate::support::{Harness, MineralHarness, SCREEN, harness};

const FRUIT: [&str; 3] = ["Apple", "Apricot", "Banana"];

#[test]
fn combobox_filters_and_reports_the_picked_option() {
    let mut h = harness(SCREEN, (String::new(), Vec::new()), |ui, (text, seen)| {
        let r = ui.mineral(Combobox::new("fruit", "Fruit", FRUIT, text));
        seen.push(r.output().clone());
    });
    h.press(Key::Tab);
    h.type_text("Apr");
    h.press(Key::ArrowDown);
    h.state_mut().value.1.clear();
    h.press(Key::Enter);
    let picks: Vec<_> = h.state().value.1.iter().filter_map(|o| o.selected).collect();
    assert_eq!(picks, [1], "the only match is the option at index 1");
    assert_eq!(h.state().value.0, "Apricot");
}

#[test]
fn free_text_combobox_reports_a_created_entry() {
    let mut h = harness(
        SCREEN,
        (String::new(), ComboboxOutput::default()),
        |ui, (text, seen)| {
            let r = ui.mineral(Combobox::new("fruit", "Fruit", FRUIT, text).free_text(true));
            seen.selected = seen.selected.or(r.output().selected);
            seen.created = seen.created.take().or_else(|| r.output().created.clone());
        },
    );
    h.press(Key::Tab);
    h.type_text("Kiwi");
    h.press(Key::Enter);
    let seen = &h.state().value.1;
    assert_eq!((seen.selected, seen.created.as_deref()), (None, Some("Kiwi")));
}

#[test]
fn otp_field_completes_once_every_cell_is_filled() {
    let mut h = harness(SCREEN, (String::new(), 0), |ui, (code, completed)| {
        let r = ui.mineral(OtpField::new("otp", "Code", 4, code));
        *completed += usize::from(r.output().completed);
    });
    h.press(Key::Tab);
    h.type_text("123");
    assert_eq!(h.state().value, ("123".to_owned(), 0), "three of four cells");
    h.type_text("4");
    assert_eq!(h.state().value, ("1234".to_owned(), 1), "completes once");
}

#[test]
fn tags_field_reports_added_and_removed_tags() {
    let mut h = harness(SCREEN, (Vec::new(), TagsFieldOutput::default()), |ui, (tags, seen)| {
        let r = ui.mineral(TagsField::new("tags", "Tags", tags));
        seen.added.extend(r.output().added.iter().cloned());
        seen.removed.extend(r.output().removed.iter().cloned());
    });
    h.press(Key::Tab);
    h.type_text("red,");
    h.type_text("blue");
    h.press(Key::Enter);
    assert_eq!(h.state().value.0, ["red", "blue"]);
    let field = h.widget("tags-tags");
    h.ctx.memory_mut(|m| m.request_focus(field));
    h.press(Key::Backspace);
    let (tags, seen) = &h.state().value;
    assert_eq!(tags, &["red"]);
    assert_eq!(seen.added, ["red", "blue"]);
    assert_eq!(seen.removed, ["blue"]);
}

#[test]
fn inline_edit_submits_on_enter_and_reverts_on_escape() {
    let mut h = harness(
        SCREEN,
        ("Draft".to_owned(), InlineEditOutput::default()),
        |ui, (value, seen)| {
            let r = ui.mineral(InlineEdit::new("title", "Title", value));
            seen.submitted |= r.output().submitted;
            seen.cancelled |= r.output().cancelled;
        },
    );
    let start = |h: &mut Harness<'_, (String, InlineEditOutput)>| {
        h.state_mut().value.1 = InlineEditOutput::default();
        h.press(Key::Tab);
        h.press(Key::Enter);
        h.key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
        h.type_text("Final");
    };
    start(&mut h);
    h.press(Key::Enter);
    let (value, seen) = &h.state().value;
    assert_eq!((value.as_str(), seen.submitted, seen.cancelled), ("Final", true, false));
    start(&mut h);
    h.press(Key::Escape);
    let (value, seen) = &h.state().value;
    assert_eq!(
        (value.as_str(), seen.submitted, seen.cancelled),
        ("Final", false, true),
        "Escape reverts"
    );
}

#[test]
fn inline_edit_binding_opens_the_editor_with_the_text_selected() {
    let mut h = harness(SCREEN, ("Draft".to_owned(), false), |ui, (value, editing)| {
        ui.mineral(InlineEdit::new("title", "Title", value).editing(editing));
    });
    h.state_mut().value.1 = true;
    h.run();
    h.type_text("Final");
    assert!(h.state().value.1, "still editing");
    h.press(Key::Enter);
    assert_eq!(
        h.state().value,
        ("Final".to_owned(), false),
        "typing replaced the selection"
    );
    h.press(Key::Tab);
    h.press(Key::Enter);
    assert!(h.state().value.1, "the widget reports its own activation");
    h.state_mut().value.1 = false;
    h.run();
    assert!(
        h.tree().find_key("title-field").is_none(),
        "clearing the binding closes the editor"
    );
}

#[test]
fn search_bar_reports_submit_on_enter() {
    let mut h = harness(SCREEN, (String::new(), false), |ui, (query, submitted)| {
        *submitted |= ui.mineral(SearchBar::new("search", "Search", query)).output().submitted;
    });
    h.press(Key::Tab);
    h.type_text("rust");
    assert_eq!(h.state().value, ("rust".to_owned(), false));
    h.press(Key::Enter);
    assert_eq!(h.state().value, ("rust".to_owned(), true));
}

#[test]
fn combobox_toggle_opens_every_option_from_cold() {
    let mut h = harness(SCREEN, (String::new(), None), |ui, (text, picked)| {
        let r = ui.mineral(Combobox::new("fruit", "Fruit", FRUIT, text));
        *picked = picked.or(r.output().selected);
    });
    let key_of = |h: &Harness<'_, (String, Option<usize>)>, name: &str| {
        h.tree()
            .nodes
            .iter()
            .find(|n| n.name == name)
            .map(|n| n.id.key().to_owned())
    };
    let toggle = key_of(&h, "Show options").expect("a named toggle");
    h.click(&toggle);
    h.run();
    for name in FRUIT {
        assert!(key_of(&h, name).is_some(), "{name} is listed");
    }
    let banana = key_of(&h, "Banana").expect("an option row");
    h.click(&banana);
    assert_eq!(h.state().value, ("Banana".to_owned(), Some(2)), "a row click picks");

    h.state_mut().value = (String::new(), None);
    h.run();
    h.click(&toggle);
    h.press(Key::Enter);
    assert_eq!(h.state().value.1, Some(0), "the toggle opens on the first row");
}
