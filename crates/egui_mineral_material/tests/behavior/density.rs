use egui_mineral::layout::Zones;
use egui_mineral_material::{
    MaterialUiExt as _,
    actions::{Fab, IconButton, PushButton, SplitButton},
    adaptive::Density,
    containment::ListItem,
    icons::ICON_MAIL,
    navigation::{NavDestination, NavigationBar, TopAppBar},
    selection::{CheckButton, Chip, Menu, Radio, Switch, ValueSlider},
    text_inputs::TextField,
};

use crate::support::{Mui, SCREEN, min_interactive, render_on};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Visual {
    Dense,
    // The box is the glyph; the state layer and target shrink.
    Glyph,
    // Sized by the spec.
    Fixed,
}

// (visual height, hit height)
type Add = fn(&mut Mui<'_>) -> (f32, f32);

fn heights(zones: Zones) -> (f32, f32) {
    (zones.visual.height(), zones.hit.height())
}

fn scope(ui: &mut Mui<'_>, add: impl FnOnce(&mut Mui<'_>)) -> (f32, f32) {
    let rt = ui.rt();
    let h = ui.scope(|ui| add(&mut rt.ui(ui))).response.rect.height();
    (h, h)
}

// name, widget, content minimum of the visual in dp, what its visual does
fn cases() -> [(&'static str, Add, f32, Visual); 13] {
    [
        (
            "button",
            |ui| heights(ui.mineral(PushButton::new("b", "Go")).zones()),
            20.0,
            Visual::Dense,
        ),
        (
            "icon button",
            |ui| heights(ui.mineral(IconButton::new("i", ICON_MAIL, "Mail")).zones()),
            24.0,
            Visual::Dense,
        ),
        (
            "chip",
            |ui| heights(ui.mineral(Chip::assist("c", "Chip")).zones()),
            20.0,
            Visual::Dense,
        ),
        (
            "checkbox",
            |ui| heights(ui.mineral(CheckButton::new("c", "Label", &mut false)).zones()),
            18.0,
            Visual::Glyph,
        ),
        (
            "radio",
            |ui| heights(ui.mineral(Radio::new("r", "Label")).zones()),
            20.0,
            Visual::Glyph,
        ),
        (
            "switch",
            |ui| heights(ui.mineral(Switch::new("s", "Label", &mut false)).zones()),
            32.0,
            Visual::Glyph,
        ),
        (
            "slider",
            |ui| heights(ui.mineral(ValueSlider::new("v", "Value", 0.0..=1.0, &mut 0.5)).zones()),
            16.0,
            Visual::Dense,
        ),
        (
            "text field",
            |ui| heights(ui.mineral(TextField::new("t", "Label", &mut String::new())).zones()),
            24.0,
            Visual::Dense,
        ),
        (
            "list item",
            |ui| heights(ui.mineral(ListItem::new("l", "Label")).zones()),
            24.0,
            Visual::Dense,
        ),
        (
            "split button",
            |ui| {
                scope(ui, |ui| {
                    SplitButton::new("sp", "Go", Menu::new("m", "More").item(0, "A")).show(ui);
                })
            },
            24.0,
            Visual::Dense,
        ),
        (
            "top app bar",
            |ui| {
                scope(ui, |ui| {
                    TopAppBar::new("bar", "Title").show(ui);
                })
            },
            24.0,
            Visual::Dense,
        ),
        (
            "navigation bar",
            |ui| {
                scope(ui, |ui| {
                    let mut selected = 0;
                    NavigationBar::new("nav", &mut selected)
                        .items(
                            ([NavDestination::new(ICON_MAIL, "A"), NavDestination::new(ICON_MAIL, "B")])
                                .into_iter()
                                .enumerate(),
                        )
                        .show(ui);
                })
            },
            32.0,
            Visual::Dense,
        ),
        (
            "fab",
            |ui| heights(ui.mineral(Fab::new("f", ICON_MAIL, "Mail")).zones()),
            56.0,
            Visual::Fixed,
        ),
    ]
}

#[test]
fn every_component_follows_density_down_to_its_content() {
    for (name, add, content, visual) in cases() {
        let at = |density: Density| {
            let mut out = (f32::NAN, f32::NAN);
            render_on(SCREEN, |ui| out = ui.with_density(density, add));
            out
        };
        let [full, mid, min] = [Density::DEFAULT, Density::new(-3), Density::MIN].map(at);
        for (box_h, hit) in [full, mid, min] {
            assert!(hit >= box_h, "{name}: hit {hit} inside visual {box_h}");
        }
        assert!(
            min.0 <= mid.0 && mid.0 <= full.0,
            "{name} visual: {} {} {}",
            full.0,
            mid.0,
            min.0
        );
        assert!(
            min.1 <= mid.1 && mid.1 <= full.1,
            "{name} hit: {} {} {}",
            full.1,
            mid.1,
            min.1
        );
        assert!(min.0 >= content, "{name}: {} below its content {content}", min.0);
        match visual {
            Visual::Fixed => assert_eq!(min, full, "{name} keeps its spec size"),
            Visual::Glyph => assert!(min.1 < full.1, "{name}: hit target {} does not shrink", full.1),
            Visual::Dense => {
                assert!(min.1 < full.1, "{name}: hit target {} does not shrink", full.1);
                assert!(min.0 < full.0, "{name}: visual {} does not shrink", full.0);
            }
        }
    }
}

#[test]
fn minimum_target_shrinks_4dp_per_step() {
    let hit = |step: i8| {
        let mut out = f32::NAN;
        render_on(SCREEN, |ui| {
            out = ui
                .with_density(Density::new(step), |ui| ui.mineral(Radio::new("r", "Label")).zones())
                .hit
                .height();
        });
        out
    };
    assert_eq!(hit(0), min_interactive());
    assert_eq!(hit(-1), min_interactive() - 4.0);
    assert_eq!(hit(-2), min_interactive() - 8.0);
}
