//! Apps glob `egui::*` next to the stylekit prelude; names shared with egui would be ambiguous here.

use egui::*;
use egui_mineral_material::prelude::*;

use crate::support::{MineralHarness as _, SCREEN, harness};

#[derive(Default)]
struct Bound {
    checked: bool,
    value: f64,
    selection: Selection<usize>,
}

#[test]
fn preludes_glob_with_egui() {
    let h = harness(
        SCREEN,
        Bound::default(),
        |ui: &mut MineralUi<'_, Material>, b: &mut Bound| {
            ui.add(Label::new("egui"));
            MenuStrip::new("strip")
                .menu(Menu::new("file", "File").item(0, MenuItem::new("Open")))
                .show(ui);
            ui.mineral(
                PushButton::new("push", "Push")
                    .variant(ButtonVariant::Filled)
                    .size(Size::Small),
            );
            ui.mineral(CheckButton::new("check", "Check", &mut b.checked));
            ui.mineral(Radio::new("radio", "Radio").selected(b.selection.is_selected(&0)));
            ui.mineral(ValueSlider::new("slider", "Slider", 0.0..=1.0, &mut b.value).preferred_width(200.0));
            ui.mineral(TextLink::new("link", "Link"));
        },
    );
    // Agent key kinds follow the semantic role, not the type name.
    for key in [
        "strip-menubar",
        "push-button",
        "check-checkbox",
        "radio-radio",
        "slider-slider",
        "link-link",
    ] {
        assert!(h.tree().find_key(key).is_some(), "{key}");
    }
}
