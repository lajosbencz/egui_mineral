//! Interaction contracts of components without a richer suite elsewhere.

use egui::{Key, Rect};
use egui_mineral::{
    layout::Edge,
    shortcut::{self, ActionId},
};
use egui_mineral_material::{
    actions::{ExtendedFab, Fab},
    communication::{LoadingIndicator, PullToRefresh, Snackbar, SnackbarAction, SnackbarHost, snackbar_queue},
    containment::{Card, Divider},
    icons::{ICON_ADD, ICON_EDIT, ICON_SAVE},
    navigation::{SearchSuggestions, Toolbar},
    prelude::*,
    spec::ColorRole,
    text_inputs::TextField,
};

use crate::support::{MineralHarness, SCREEN, render, still};

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
fn docked_toolbar_reports_the_clicked_action() {
    let mut h = still(SCREEN, Vec::new(), |ui, seen| {
        let r = Toolbar::docked("dock", "Tools")
            .action(IconAction::new(ICON_SAVE, "Save"))
            .action(IconAction::new(ICON_EDIT, "Edit"))
            .show(ui);
        seen.extend(r.clicked);
    });
    h.click("dock-1-button");
    h.click("dock-0-button");
    assert_eq!(h.state().value, [1, 0]);
}

#[test]
fn floating_toolbar_reports_its_paired_fab() {
    let mut h = still(SCREEN, (0u32, Vec::new()), |ui, (fab, seen)| {
        let r = Toolbar::floating("float", "Tools")
            .action(IconAction::new(ICON_SAVE, "Save"))
            .fab(ICON_ADD, "Add", Edge::End)
            .show(ui);
        *fab += u32::from(r.fab_clicked);
        seen.extend(r.clicked);
    });
    h.click("float-fab");
    h.click("float-0-button");
    assert_eq!(h.state().value, (1, vec![0]));
}

#[test]
fn search_suggestions_filter_by_query_and_report_the_pick() {
    let mut h = still(SCREEN, (String::new(), Vec::new()), |ui, (query, picked)| {
        let r = ui.mineral(SearchSuggestions::new("fruit", ["apple", "banana", "cherry"], query));
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
fn snackbar_host_reports_action_and_close() {
    const UNDO: ActionId = ActionId::from_static("undo");
    let mut queue = snackbar_queue();
    let first = queue.push(Snackbar::new("Saved").action(SnackbarAction::new("Undo").command(UNDO)));
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
        let r = ui.mineral(SearchSuggestions::new("fruit", ["apple", "banana", "cherry"], query));
        picked.extend(r.output().selected);
    });
    h.click("fruit-search");
    h.type_text("an");
    h.press(Key::ArrowDown);
    h.press(Key::Enter);
    assert_eq!(h.state().value.1, [1], "Down then Enter picks the first match");
    assert_eq!(h.state().value.0, "banana");
}
