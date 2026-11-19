use std::time::Duration;

use egui::Key;
use egui_kittest::kittest::{NodeT, Queryable};
use egui_mineral::{anchor::Side, overlay::Dismiss, semantic::Role};
use egui_mineral_fluent::{
    actions::PushButton,
    surfaces::{
        Dialog, DialogKind, Drawer, DrawerPosition, DrawerSize, Popover, TeachingPopover, TeachingStep, Tooltip,
        TooltipRelationship,
    },
};

use crate::support::{FluentHarness, Harness, harness};

fn tooltip_harness(relationship: TooltipRelationship) -> Harness<'static, ()> {
    harness((), move |ui, ()| {
        let rt = ui.rt();
        let trigger = ui.mineral(PushButton::new("trg", "Trigger")).response().clone();
        Tooltip::new("tip", "Helpful words")
            .relationship(relationship)
            .show_delay(Duration::ZERO)
            .hide_delay(Duration::ZERO)
            .show(rt, &trigger);
    })
}

#[test]
fn tooltip_shows_on_hover_and_escape_hides_it() {
    let mut h = tooltip_harness(TooltipRelationship::Description);
    assert!(h.tree().find_key("tip-tooltip").is_none());
    let at = h.rect("trg-button").center();
    h.hover_at(at);
    h.run();
    h.run();
    let node = h.node("tip-tooltip");
    assert_eq!((node.role, node.name.as_str()), (Role::Tooltip, "Helpful words"));
    h.press(Key::Escape);
    h.run();
    assert!(h.tree().find_key("tip-tooltip").is_none());
}

#[test]
fn tooltip_description_relationship_describes_the_trigger() {
    let mut h = tooltip_harness(TooltipRelationship::Description);
    let at = h.rect("trg-button").center();
    h.hover_at(at);
    h.run();
    let trigger = h.get_by_label("Trigger");
    assert_eq!(trigger.accesskit_node().description().as_deref(), Some("Helpful words"));
}

#[test]
fn inaccessible_tooltip_has_no_node() {
    let mut h = tooltip_harness(TooltipRelationship::Inaccessible);
    let at = h.rect("trg-button").center();
    h.hover_at(at);
    h.run();
    h.run();
    assert!(h.tree().find_key("tip-tooltip").is_none());
}

fn popover_harness(trap: bool) -> Harness<'static, bool> {
    harness(false, move |ui, open| {
        let rt = ui.rt();
        let trigger = ui.mineral(PushButton::new("trg", "Open")).response().clone();
        Popover::new("pop", "Details")
            .arrow(true)
            .side(Side::Bottom)
            .trap_focus(trap)
            .show(rt, &trigger, open, |ui| {
                ui.mineral(PushButton::new("first", "First"));
                ui.mineral(PushButton::new("second", "Second"));
            });
    })
}

#[test]
fn popover_toggles_on_trigger_and_closes_on_escape_or_outside_press() {
    let mut h = popover_harness(false);
    assert!(h.tree().find_key("pop-popover").is_none());
    h.click("trg-button");
    h.run();
    assert_eq!(h.node("pop-popover").role, Role::Dialog);
    h.press(Key::Escape);
    h.run();
    assert!(!h.state().value);
    h.click("trg-button");
    h.run();
    assert!(h.state().value);
    h.click_at(egui::pos2(590.0, 590.0));
    h.run();
    assert!(!h.state().value);
}

#[test]
fn trapped_focus_cycles_inside_the_popover() {
    let mut h = popover_harness(true);
    h.click("trg-button");
    h.run();
    let mut seen = Vec::new();
    for _ in 0..8 {
        h.press(Key::Tab);
        let focused = h.ctx.memory(egui::Memory::focused);
        let tree = h.tree();
        seen.push(
            tree.nodes
                .iter()
                .find(|n| focused.is_some() && n.widget_id == focused)
                .map(|n| n.id.key().to_owned()),
        );
    }
    assert!(
        seen.iter()
            .flatten()
            .all(|k| k == "first-button" || k == "second-button"),
        "{seen:?}"
    );
    assert!(seen.iter().flatten().any(|k| k == "first-button"), "{seen:?}");
}

#[test]
fn teaching_carousel_steps_and_finishes() {
    let mut h = harness((false, 0usize, false), |ui, (open, step, done)| {
        let rt = ui.rt();
        let trigger = ui.mineral(PushButton::new("trg", "Open")).response().clone();
        let r = TeachingPopover::new("tp", "Tour")
            .step(TeachingStep::new("One").title("First"))
            .step(TeachingStep::new("Two").title("Second"))
            .brand(true)
            .primary("Got it")
            .show(rt, &trigger, open);
        *step = r.step;
        *done |= r.primary_clicked;
    });
    h.click("trg-button");
    h.run();
    assert_eq!(h.state().value.1, 0);
    h.click("tp-primary-button");
    h.run();
    assert_eq!(h.state().value.1, 1);
    h.click("tp-primary-button");
    h.run();
    let (open, _, done) = h.state().value;
    assert!(done && !open);
}

type DialogState = (bool, Option<usize>, Option<Dismiss>);

fn dialog_harness(kind: DialogKind) -> Harness<'static, DialogState> {
    harness((true, None, None), move |ui, (open, clicked, dismissed)| {
        let rt = ui.rt();
        let r = Dialog::new("dlg", "Delete file")
            .kind(kind)
            .action("Cancel")
            .action("Delete")
            .show(rt, open, |ui| {
                ui.label("Sure?");
            });
        *clicked = clicked.or(r.clicked);
        *dismissed = dismissed.or(r.dismissed);
    })
}

#[test]
fn dialog_exposes_role_and_reports_the_clicked_action() {
    let mut h = dialog_harness(DialogKind::Modal);
    h.run();
    assert_eq!(h.node("dlg-dialog").role, Role::Dialog);
    h.click("dlg-action-1-button");
    h.run();
    let (open, clicked, _) = h.state().value;
    assert!(!open);
    assert_eq!(clicked, Some(1));
}

#[test]
fn modal_dialog_escape_dismisses_and_alert_ignores_it() {
    let mut h = dialog_harness(DialogKind::Modal);
    h.run();
    h.press(Key::Escape);
    h.run();
    assert_eq!(h.state().value.2, Some(Dismiss::Escape));
    let mut h = dialog_harness(DialogKind::Alert);
    h.run();
    assert_eq!(h.node("dlg-dialog").role, Role::AlertDialog);
    h.press(Key::Escape);
    h.run();
    assert!(h.state().value.0);
}

#[test]
fn drawer_overlay_closes_on_escape() {
    let mut h = harness(true, |ui, open| {
        let rt = ui.rt();
        Drawer::new("dr", "Menu")
            .title("Menu")
            .close_button(true)
            .footer_action("Save")
            .show_overlay(rt, open, |ui| {
                ui.label("Body");
            });
    });
    h.run();
    assert_eq!(h.node("dr-drawer").role, Role::Dialog);
    h.press(Key::Escape);
    h.run();
    assert!(!h.state().value);
}

#[test]
fn inline_drawer_close_button_closes_it() {
    let mut h = harness(true, |ui, open| {
        Drawer::new("dr", "Menu")
            .title("Menu")
            .close_button(true)
            .position(DrawerPosition::Start)
            .size(DrawerSize::Small)
            .show_inline(ui, open, |ui| {
                ui.label("Body");
            });
    });
    h.run();
    assert_eq!(h.node("dr-drawer").role, Role::Complementary);
    h.click("dr-close-button");
    h.run();
    assert!(!h.state().value);
}

#[test]
fn dialog_opens_with_focus_on_the_first_action() {
    let mut h = dialog_harness(DialogKind::Modal);
    for _ in 0..3 {
        h.run();
    }
    let focused = h.ctx.memory(egui::Memory::focused);
    assert_eq!(focused, h.node("dlg-action-0-button").widget_id);
}

#[test]
fn page_scroll_nests_floating_bars_and_fills_the_viewport() {
    let mut h = harness((false, 0.0), |ui, (floating, height)| {
        let out = egui_mineral_fluent::scroll::page(ui, "pg", |ui| {
            *floating = ui.spacing().scroll.floating;
            ui.allocate_space(egui::vec2(10.0, 2000.0));
        });
        *height = out.inner_rect.height();
    });
    h.run();
    let (floating, height) = h.state().value;
    assert!(floating);
    assert!(height > 500.0, "page fills the viewport: {height}");
}
