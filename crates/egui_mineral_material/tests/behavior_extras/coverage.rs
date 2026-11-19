//! Interaction contracts of components without a richer suite elsewhere.

use egui::Key;
use egui_mineral::expand::Expansion;
use egui_mineral_material::{
    containment::BottomSheet,
    extras::{Accordion, EmptyPlaceholder, ImageList, Rating},
};

use crate::support::{MineralHarness, SCREEN, render, still};

#[test]
fn accordion_header_click_toggles_its_panel_and_collapses_the_other() {
    let mut h = still(SCREEN, (Expansion::single(), Vec::new()), |ui, (ex, toggled)| {
        let r = Accordion::new("acc", ex).show(ui, |a| {
            a.panel("One", |ui| ui.label("first"));
            a.panel("Two", |ui| ui.label("second"));
        });
        assert_eq!(r.changed, r.toggled.is_some());
        toggled.extend(r.toggled);
    });
    h.click("acc-0-header");
    assert!(h.state().value.0.is_expanded(&0), "click expands");
    h.click("acc-1-header");
    let ex = &h.state().value.0;
    assert!(
        ex.is_expanded(&1) && !ex.is_expanded(&0),
        "single mode collapses the other panel"
    );
    h.click("acc-1-header");
    assert_eq!(h.state().value.0.expanded().count(), 0, "second click collapses");
    assert_eq!(h.state().value.1, [0, 1, 1]);
}

#[test]
fn rating_sets_by_pointer_and_steps_by_arrow_keys() {
    let mut h = still(SCREEN, 0.0, |ui, value| {
        ui.mineral(Rating::new("rate", "Quality", value));
    });
    let rect = h.rect("rate-rating");
    h.click_at(egui::pos2(rect.right() - 1.0, rect.center().y));
    assert_eq!(h.state().value, 5.0, "the last star sets the maximum");
    h.press(Key::ArrowLeft);
    assert_eq!(h.state().value, 4.0, "ArrowLeft steps down");
    h.press(Key::Home);
    assert_eq!(h.state().value, 0.0, "Home clears to the minimum");
}

#[test]
fn rating_read_only_ignores_input() {
    let mut h = still(SCREEN, 3.0, |ui, value| {
        ui.mineral(Rating::new("rate", "Quality", value).read_only(true));
    });
    let rect = h.rect("rate-rating");
    h.click_at(egui::pos2(rect.right() - 1.0, rect.center().y));
    h.press(Key::ArrowLeft);
    assert_eq!(h.state().value, 3.0);
}

#[test]
fn empty_placeholder_reports_its_action() {
    let mut h = still(SCREEN, 0u32, |ui, clicks| {
        let r = EmptyPlaceholder::new("empty", "No mail").action("Refresh").show(ui);
        *clicks += u32::from(r.action_clicked);
    });
    let key = h
        .tree()
        .nodes
        .iter()
        .find(|n| n.name == "Refresh")
        .map(|n| n.id.key().to_owned())
        .expect("an action node");
    h.click(&key);
    assert_eq!(h.state().value, 1);
}

#[test]
fn image_list_and_bottom_sheet_are_named() {
    let pass = render(|ui| {
        ImageList::new("gallery", 2).show(ui, |ui, i| {
            ui.label(i.to_string());
        });
        BottomSheet::new("sheet")
            .label("Filters")
            .show(ui.rt(), &mut true, |ui| ui.label("Body"));
    });
    assert_eq!(pass.node("gallery-image-list").name, "Images");
    assert_eq!(pass.node("gallery-1-tile").name, "Image 2 of 2");
    assert_eq!(pass.node("sheet-bottom-sheet").name, "Filters");
}

#[test]
fn sibling_accordions_scope_their_panels_apart() {
    let mut h = still(SCREEN, Vec::new(), |ui, ids: &mut Vec<egui::Id>| {
        ids.clear();
        for salt in ["a", "b"] {
            let mut ex = Expansion::single();
            ex.expand(0);
            Accordion::new(salt, &mut ex).show(ui, |a| {
                a.panel("One", |ui| ids.push(ui.scope_id()));
            });
        }
    });
    h.run();
    let ids = &h.state().value;
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1]);
}

#[test]
fn accordion_drops_measured_heights_of_removed_panels() {
    let mut h = still(SCREEN, (3_usize, usize::MAX), |ui, (panels, stored)| {
        let mut ex = Expansion::default();
        for i in 0..*panels {
            ex.expand(i);
        }
        Accordion::new("acc", &mut ex).show(ui, |a| {
            for i in 0..*panels {
                a.panel(format!("P{i}"), |ui| ui.label("body"));
            }
        });
        let id = ui.make_persistent_id(egui_mineral::semantic::KeyedSalt::from("acc").id_salt());
        *stored = ui
            .ctx()
            .data(|d| d.get_temp::<Vec<f32>>(id.with("heights")))
            .map_or(0, |v| v.len());
    });
    h.run();
    assert_eq!(h.state().value.1, 3);
    h.state_mut().value.0 = 1;
    h.run();
    assert_eq!(h.state().value.1, 1);
}
