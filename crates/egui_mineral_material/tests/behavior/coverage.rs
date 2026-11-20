//! Interaction contracts of components without a richer suite elsewhere.

use egui::{Key, Rect};
use egui_mineral::{
    expand::Expansion,
    layout::Edge,
    shortcut::{self, ActionId},
};
use egui_mineral_material::{
    actions::{ExtendedFab, Fab},
    communication::{LoadingIndicator, PullToRefresh, Snackbar, SnackbarHost, snackbar_queue},
    containment::{BottomSheet, Card, Divider},
    extras::{Accordion, EmptyPlaceholder, ImageList, Rating},
    icons::{ICON_ADD, ICON_EDIT, ICON_SAVE},
    navigation::{DockedToolbar, FloatingToolbar, SearchSuggestions},
    prelude::*,
    spec::ColorRole,
    text_inputs::TextField,
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
fn card_click_reports_only_when_clickable_and_enabled() {
    for (clickable, disabled, expected) in [(true, false, 1), (false, false, 0), (true, true, 0)] {
        let mut h = still(SCREEN, (Rect::NOTHING, 0), |ui, (rect, clicks)| {
            let r = Card::new("card", "Card")
                .clickable(clickable)
                .disabled(disabled)
                .show(ui, |ui| ui.label("content"));
            *rect = r.response.rect;
            *clicks += u32::from(r.clicked);
        });
        let at = h.state().value.0.center();
        h.click_at(at);
        assert_eq!(
            h.state().value.1,
            expected,
            "clickable {clickable}, disabled {disabled}"
        );
    }
}

#[test]
fn fabs_click_unless_disabled() {
    for disabled in [false, true] {
        let mut h = still(SCREEN, [0u32; 2], |ui, clicks| {
            clicks[0] += u32::from(
                ui.mineral(Fab::new("fab", ICON_ADD, "Add").disabled(disabled))
                    .clicked(),
            );
            clicks[1] += u32::from(
                ui.mineral(ExtendedFab::new("xfab", ICON_EDIT, "Compose").disabled(disabled))
                    .clicked(),
            );
        });
        h.click("fab-fab");
        h.click("xfab-fab");
        let expected = if disabled { [0, 0] } else { [1, 1] };
        assert_eq!(h.state().value, expected, "disabled {disabled}");
        assert_eq!(h.node("fab-fab").state.disabled, disabled);
    }
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
fn docked_toolbar_reports_the_clicked_action() {
    let mut h = still(SCREEN, Vec::new(), |ui, seen| {
        let r = DockedToolbar::new("dock", "Tools")
            .action(IconAction::new(ICON_SAVE, "Save"))
            .action(IconAction::new(ICON_EDIT, "Edit"))
            .show(ui);
        seen.extend(r.clicked.as_ref().map(egui_mineral::action::Clicked::index));
    });
    h.click("dock-1-button");
    h.click("dock-0-button");
    assert_eq!(h.state().value, [1, 0]);
}

#[test]
fn floating_toolbar_reports_its_paired_fab() {
    let mut h = still(SCREEN, (0u32, Vec::new()), |ui, (fab, seen)| {
        let r = FloatingToolbar::new("float", "Tools")
            .action(IconAction::new(ICON_SAVE, "Save"))
            .fab(ICON_ADD, "Add", Edge::End)
            .show(ui);
        *fab += u32::from(r.fab_clicked);
        seen.extend(r.clicked.as_ref().map(egui_mineral::action::Clicked::index));
    });
    h.click("float-fab");
    h.click("float-0-button");
    assert_eq!(h.state().value, (1, vec![0]));
}

#[test]
fn search_suggestions_filter_by_query_and_report_the_pick() {
    let mut h = still(SCREEN, (String::new(), Vec::new()), |ui, (query, picked)| {
        let r = ui.mineral(SearchSuggestions::new(
            "fruit",
            "Search",
            ["apple", "banana", "cherry"],
            query,
        ));
        picked.extend(r.output().selected);
    });
    h.click("fruit-search");
    h.type_text("an");
    let shown: Vec<_> = h
        .tree()
        .nodes
        .iter()
        .filter(|n| n.id.key().ends_with("-suggestion"))
        .map(|n| n.name.clone())
        .collect();
    assert_eq!(shown, ["banana"], "only matches are listed");
    let key = h
        .tree()
        .nodes
        .iter()
        .find(|n| n.id.key().ends_with("-suggestion"))
        .map(|n| n.id.key().to_owned())
        .expect("a suggestion row");
    h.click(&key);
    assert_eq!(h.state().value.1, [1], "the pick indexes the full suggestion list");
    assert_eq!(h.state().value.0, "banana", "the pick fills the query");
}

#[test]
fn pull_to_refresh_indicator_refreshes_from_the_keyboard() {
    let mut h = still(SCREEN, 0u32, |ui, refreshed| {
        let r = PullToRefresh::new("feed").show(ui, |ui| ui.label("items"));
        *refreshed += u32::from(r.refreshed);
    });
    h.press(Key::Tab);
    assert!(
        h.focused("feed-refresh-button"),
        "the hidden indicator takes keyboard focus"
    );
    h.press(Key::Enter);
    assert_eq!(h.state().value, 1);
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
fn display_only_components_paint_and_name_themselves() {
    let pass = render(|ui| {
        ui.mineral(Divider::new("div"));
        ui.mineral(LoadingIndicator::new("load"));
    });
    let line = pass.one(ColorRole::OutlineVariant);
    assert_eq!(line.height(), 1.0, "divider thickness");
    assert_eq!(pass.node("load-loading").name, "Loading", "default accessible name");
}

#[test]
fn text_field_trailing_icon_is_a_named_button() {
    let mut h = still(SCREEN, (String::new(), 0u32), |ui, (text, clicks)| {
        let r = ui.mineral(TextField::new("q", "Query", text).trailing_icon(ICON_EDIT, "Edit query"));
        *clicks += u32::from(r.output().trailing_clicked);
    });
    assert!(h.unnamed_widgets().is_empty(), "every interactive part is named");
    let key = h
        .tree()
        .nodes
        .iter()
        .find(|n| n.name == "Edit query")
        .map(|n| n.id.key().to_owned())
        .expect("a node named by the trailing label");
    h.click(&key);
    assert_eq!(h.state().value.1, 1);
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
fn snackbar_host_reports_action_and_close() {
    const UNDO: ActionId = ActionId::from_static("undo");
    let mut queue = snackbar_queue();
    let first = queue.push(Snackbar::new("Saved").action(TextAction::new("Undo").command(UNDO)));
    let mut h = still(
        SCREEN,
        (queue, Vec::new(), Vec::new(), 0),
        |ui, (queue, actions, dismissed, fired)| {
            let r = SnackbarHost::new(queue).show(ui.rt());
            actions.extend(r.action_clicked);
            dismissed.extend(r.dismissed);
            *fired += usize::from(shortcut::take_action(ui.ctx(), &UNDO));
        },
    );
    h.click("snackbar-action");
    assert_eq!(h.state().value.1, [first]);
    h.run();
    assert_eq!(h.state().value.3, 1, "the command fires");
    let second = h.state_mut().value.0.push(Snackbar::new("Sent").dismissible(true));
    h.run();
    h.click("snackbar-close");
    let dismissed = &h.state().value.2;
    assert_eq!(dismissed, &[second]);
}

#[test]
fn search_suggestions_pick_from_the_keyboard() {
    let mut h = still(SCREEN, (String::new(), Vec::new()), |ui, (query, picked)| {
        let r = ui.mineral(SearchSuggestions::new(
            "fruit",
            "Search",
            ["apple", "banana", "cherry"],
            query,
        ));
        picked.extend(r.output().selected);
    });
    h.click("fruit-search");
    h.type_text("an");
    h.press(Key::ArrowDown);
    h.press(Key::Enter);
    assert_eq!(h.state().value.1, [1], "Down then Enter picks the first match");
    assert_eq!(h.state().value.0, "banana");
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
