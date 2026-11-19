use egui_mineral_fluent::{
    actions::{CompoundButton, MenuButton, PushButton, TextLink, ToggleButton},
    data::Text,
    icons::ICON_ADD,
    inputs::FieldLabel,
    surfaces::Divider,
};

use crate::support::{FluentHarness, harness};

#[test]
fn agent_keys_derive_from_the_salt_and_can_be_overridden() {
    let h = harness(false, |ui, on| {
        ui.mineral(PushButton::new("a", "A"));
        ui.mineral(
            PushButton::new("b", "B")
                .icon(ICON_ADD)
                .label_visible(false)
                .agent_key("custom"),
        );
        ui.mineral(ToggleButton::new("c", "C", on));
        ui.mineral(CompoundButton::new("d", "D", "d"));
        ui.mineral(MenuButton::new("e", "E"));
        ui.mineral(TextLink::new("f", "F"));
        ui.mineral(FieldLabel::new("g", "G"));
        ui.mineral(Text::new("h", "H"));
        ui.mineral(Divider::new("i").content("I"));
        ui.mineral(Divider::new("j"));
    });
    for key in [
        "a-button",
        "custom",
        "c-button",
        "d-button",
        "e-button",
        "f-link",
        "g-label",
        "h-text",
        "i-divider",
    ] {
        h.node(key);
    }
    assert_eq!(h.node("custom").name, "B", "icon-only keeps the label as its name");
    assert!(
        h.tree().find_key("j-divider").is_none(),
        "a plain divider is decorative"
    );
}

#[test]
fn persona_is_one_group_named_by_its_lines() {
    use egui_mineral::semantic::Role;
    use egui_mineral_fluent::data::Persona;

    let h = harness((), |ui, ()| {
        ui.mineral(
            Persona::new("p", "Kat Larrson")
                .secondary_text("Designer")
                .agent_key("who"),
        );
        ui.mineral(Persona::new("q", "Ada"));
    });
    assert_eq!(h.node("who").role, Role::Group);
    assert_eq!(h.node("who").name, "Kat Larrson, Designer");
    assert_eq!(h.node("q-persona").name, "Ada");
}

#[test]
fn avatar_name_joins_presence_and_active_state() {
    use egui_mineral_fluent::{
        data::{Avatar, AvatarActive},
        feedback::PresenceStatus,
    };

    let h = harness((), |ui, ()| {
        ui.mineral(
            Avatar::new("a", "Kat Larrson")
                .presence(PresenceStatus::Away)
                .active(AvatarActive::Active),
        );
        ui.mineral(Avatar::new("b", "").initials("KL"));
        ui.mineral(Avatar::new("c", "Kat").agent_key("custom-avatar"));
    });
    assert_eq!(h.node("a-avatar").name, "Kat Larrson, Away, active");
    assert_eq!(h.node("b-avatar").name, "KL");
    h.node("custom-avatar");
}
