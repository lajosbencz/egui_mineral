use egui::FontFamily;
use egui_mineral::{icon::Icon, select::Selection};
use egui_mineral_fluent::{
    Fluent, IconSet,
    fonts::ICON_FAMILY,
    icons::{ICON_CHEVRON_DOWN, ICON_STAR},
    inputs::{Dropdown, ListOption},
};
use egui_mineral_material::icons as material;

use crate::support::render_with;

const MATERIAL_OUTLINED: &str = "material-icons-outlined";

fn dropdown(kit: Fluent) -> crate::support::Pass {
    let mut sel = Selection::<String>::single();
    render_with(egui::Theme::Light, kit, |ui| {
        ui.mineral(Dropdown::new("d", "Pet", [ListOption::new("cat", "Cat")], &mut sel));
    })
}

#[test]
fn fluent_icon_converts_to_both_forms_of_its_family() {
    let icon = Icon::from(ICON_STAR);
    assert_eq!(icon.regular().family(), ICON_FAMILY);
    assert_eq!(icon.filled().family(), ICON_FAMILY);
    assert_eq!(
        (icon.regular().ch(), icon.filled().ch()),
        (ICON_STAR.regular(), ICON_STAR.filled())
    );
}

#[test]
fn dropdown_chevron_defaults_to_the_fluent_font() {
    let pass = dropdown(Fluent::default());
    let chevron = ICON_CHEVRON_DOWN.regular().to_string();
    assert_eq!(pass.text_family(&chevron), Some(ICON_FAMILY.font_family()));
}

#[test]
fn dropdown_chevron_comes_from_a_material_icon_set_without_the_fluent_font() {
    let mut set = IconSet::fluent();
    set.chevron_down = material::ICON_KEYBOARD_ARROW_DOWN.into();
    let pass = dropdown(Fluent::with_icons(&set, &[]));
    let material_chevron = set.chevron_down.regular().ch().to_string();
    assert_eq!(
        pass.text_family(&material_chevron),
        Some(FontFamily::Name(MATERIAL_OUTLINED.into()))
    );
    assert_eq!(pass.text_family(&ICON_CHEVRON_DOWN.regular().to_string()), None);
}
