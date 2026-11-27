use egui::Key;
use egui_mineral_material::{
    extras::{Combobox, ComboboxOutput, InlineEdit, InlineEditOutput, OtpField, TagsField, TagsFieldOutput},
    navigation::SearchBar,
};

use crate::support::{Harness, MineralHarness, SCREEN, harness};

const FRUIT: [&str; 3] = ["Apple", "Apricot", "Banana"];

#[test]
fn combobox_filters_and_reports_the_picked_option() {
    let mut h = harness(SCREEN, (String::new(), Vec::new(), None), |ui, (text, seen, bound)| {
        let r = ui.mineral(Combobox::optional("fruit", "Fruit", bound, text).items(FRUIT.into_iter().enumerate()));
        seen.push(r.output().clone());
    });
    h.press(Key::Tab);
    h.type_text("Apr");
    h.press(Key::ArrowDown);
    h.state_mut().value.1.clear();
    h.press(Key::Enter);
    let picks: Vec<_> = h.state().value.1.iter().filter_map(|o| o.selected).collect();
    assert_eq!(picks, [1], "the only match is the option at index 1");
    assert_eq!((h.state().value.0.as_str(), h.state().value.2), ("Apricot", Some(1)));
}

#[test]
fn free_text_combobox_reports_a_created_entry() {
    let mut h = harness(
        SCREEN,
        (String::new(), ComboboxOutput::default(), None),
        |ui, (text, seen, bound)| {
            let r = ui.mineral(
                Combobox::optional("fruit", "Fruit", bound, text)
                    .items(FRUIT.into_iter().enumerate())
                    .free_text(true),
            );
            seen.selected = seen.selected.or(r.output().selected);
            seen.created = seen.created.take().or_else(|| r.output().created.clone());
        },
    );
    h.press(Key::Tab);
    h.type_text("Kiwi");
    h.press(Key::Enter);
    let (_, seen, bound) = &h.state().value;
    assert_eq!(
        (seen.selected, seen.created.as_deref(), *bound),
        (None, Some("Kiwi"), None)
    );
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
    let mut h = harness(SCREEN, (String::new(), None, None), |ui, (text, picked, bound)| {
        let r = ui.mineral(Combobox::optional("fruit", "Fruit", bound, text).items(FRUIT.into_iter().enumerate()));
        *picked = picked.or(r.output().selected);
    });
    let key_of = |h: &Harness<'_, (String, Option<usize>, Option<usize>)>, name: &str| {
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
    assert_eq!(
        h.state().value,
        ("Banana".to_owned(), Some(2), Some(2)),
        "a row click picks"
    );

    h.state_mut().value = (String::new(), None, None);
    h.run();
    h.click(&toggle);
    h.press(Key::Enter);
    assert_eq!(h.state().value.1, Some(0), "the toggle opens on the first row");
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, egui_mineral::semantic::KeySalt)]
enum Role {
    Dev,
    Pm,
}

const ROLES: [(Role, &str); 2] = [(Role::Dev, "Developer"), (Role::Pm, "PM")];

#[test]
fn form_renders_declared_fields_and_submits_their_values() {
    use egui_mineral_material::{
        egui_mineral::form::{Form, FormState},
        text_inputs::FormExt,
    };

    #[derive(Default, Form)]
    struct Signup {
        #[form(choices = ROLES)]
        role: Option<Role>,
        #[form(label = "Accept", required)]
        terms: bool,
    }

    let mut h = harness(
        SCREEN,
        (FormState::default(), Signup::default(), false),
        |ui, (state, value, submitted)| {
            *submitted |= Form::new("signup", state, value).show_all(ui).submitted;
        },
    );
    h.click("signup-submit-button");
    assert!(!h.state().value.2, "the unchecked required box blocks");
    h.click("signup-terms-checkbox");
    h.click("signup-role-select");
    h.run();
    h.click("signup-role-select-dev-option");
    h.run();
    h.click("signup-submit-button");
    let (_, value, submitted) = &h.state().value;
    assert!(*submitted);
    assert_eq!((value.terms, value.role), (true, Some(Role::Dev)));
}

#[test]
fn form_renders_every_core_kind() {
    use egui_mineral::{date::Date, form::Form, select::Selection};
    use egui_mineral_material::{egui_mineral::form::FormState, text_inputs::FormExt};

    #[derive(Default, Form)]
    struct Kinds {
        #[form(label = "Name", supporting = "Full name", placeholder = "Ann")]
        name: String,
        #[form(label = "Accept")]
        terms: bool,
        #[form(label = "Notify", kind = Switch, supporting = "By mail")]
        notify: bool,
        #[form(label = "Role", choices = ROLES)]
        role: Option<Role>,
        #[form(label = "Roles", choices = ROLES)]
        roles: Selection<Role>,
        #[form(label = "Start", kind = Date)]
        start: Option<Date>,
        #[form(label = "Level", kind = Slider, range = 0.0..=10.0)]
        level: f32,
    }

    let (mut state, mut value) = (FormState::default(), Kinds::default());
    let p = crate::support::render_on(SCREEN, |ui| {
        Form::new("k", &mut state, &mut value).show_all(ui);
    });
    for key in [
        "k-name-field",
        "k-terms-checkbox",
        "k-notify-switch",
        "k-role-select",
        "k-roles-select",
        "k-level-slider",
    ] {
        assert!(p.rect(key).width() > 0.0, "{key} shown");
    }
    for text in ["Start", "Full name", "By mail"] {
        assert!(p.text(text).width() > 0.0, "{text} painted");
    }
}

#[test]
fn form_switch_and_slider_show_errors_below() {
    use egui_mineral::form::{Form, FormState, NumberRange};
    use egui_mineral_material::text_inputs::FormExt;

    #[derive(Default, Form)]
    struct Prefs {
        #[form(label = "Notify", kind = Switch, required)]
        notify: bool,
        #[form(label = "Level", kind = Slider, range = 0.0..=10.0, check = NumberRange::at_least(5.0))]
        level: f32,
    }

    let mut h = harness(
        SCREEN,
        (FormState::default(), Prefs::default()),
        |ui, (state, value)| {
            Form::new("p", state, value).show_all(ui);
        },
    );
    h.click("p-submit-button");
    h.run();
    let (mut state, mut value) = std::mem::take(&mut h.state_mut().value);
    assert!(state.field(Prefs::NOTIFY).and_then(|f| f.error()).is_some());
    assert!(state.field(Prefs::LEVEL).and_then(|f| f.error()).is_some());
    let p = crate::support::render_on(SCREEN, |ui| {
        Form::new("p", &mut state, &mut value).show_all(ui);
    });
    let below = p.text("This field is required");
    assert!(
        below.top() >= p.rect("p-notify-switch").bottom(),
        "the error goes under the switch"
    );
    assert!(p.text("Enter a number of at least 5").top() >= p.rect("p-level-slider").bottom());
}

#[cfg(feature = "extras")]
#[test]
fn form_number_field_binds_integers_and_validates_them() {
    use egui_mineral_material::{
        egui_mineral::form::{Form, FormState, NumberRange},
        text_inputs::FormExt,
    };

    #[derive(Default, Form)]
    struct Exp {
        #[form(range = 0.0..=10.0, check = NumberRange::at_most(1.0))]
        years: u8,
    }

    let mut h = harness(
        SCREEN,
        (FormState::default(), Exp::default(), false),
        |ui, (state, value, submitted)| {
            *submitted |= Form::new("f", state, value).show_all(ui).submitted;
        },
    );
    h.click("f-years-increment");
    h.click("f-years-increment");
    h.click("f-submit-button");
    let (state, value, submitted) = &h.state().value;
    assert_eq!(value.years, 2, "whole steps on a u8");
    assert!(!submitted, "above the validator's max");
    assert!(state.field(Exp::YEARS).and_then(|f| f.error()).is_some());
}
