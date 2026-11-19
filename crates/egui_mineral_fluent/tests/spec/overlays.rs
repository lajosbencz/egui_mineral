use std::time::Duration;

use egui::{Align, Rect, Sense, ViewportId, vec2};
use egui_mineral::{anchor::Side, layout::Flow};
use egui_mineral_fluent::{
    actions::PushButton,
    spec::defaults::{button, dialog, drawer, popover, teaching_popover, tooltip},
    surfaces::{
        Dialog, Drawer, DrawerPosition, DrawerSize, Popover, PopoverSize, TeachingPopover, TeachingStep, Tooltip,
    },
};

use crate::support::{FluentHarness, Pass, SCREEN, harness, render};

fn rect(p: &Pass, key: &str) -> Rect {
    let tree = p.rt.tree(ViewportId::ROOT).expect("semantics collected");
    tree.find_key(key).expect("node").rect
}

#[test]
fn tooltip_is_padded_and_sits_the_offset_above_its_trigger() {
    let mut h = harness((), |ui, ()| {
        let rt = ui.rt();
        ui.add_space(200.0);
        let trigger = ui.mineral(PushButton::new("trg", "Trigger")).response().clone();
        Tooltip::new("tip", "Short")
            .show_delay(Duration::ZERO)
            .show(rt, &trigger);
    });
    let trigger = h.rect("trg-button");
    h.hover_at(trigger.center());
    for _ in 0..3 {
        h.run();
    }
    let tip = h.rect("tip-tooltip");
    assert!(tip.width() <= tooltip::MAX_WIDTH, "{tip:?}");
    assert_eq!(
        tip.height(),
        tooltip::PADDING_TOP + tooltip::PADDING_BOTTOM + 2.0 + 16.0
    );
    assert_eq!(tip.bottom() + tooltip::OFFSET, trigger.top());
}

#[test]
fn tooltip_arrow_adds_its_height_to_the_gap() {
    let mut h = harness((), |ui, ()| {
        let rt = ui.rt();
        ui.add_space(200.0);
        let trigger = ui.mineral(PushButton::new("trg", "Trigger")).response().clone();
        Tooltip::new("tip", "Short")
            .arrow(true)
            .show_delay(Duration::ZERO)
            .show(rt, &trigger);
    });
    let trigger = h.rect("trg-button");
    h.hover_at(trigger.center());
    for _ in 0..3 {
        h.run();
    }
    let tip = h.rect("tip-tooltip");
    assert_eq!(tip.bottom() + tooltip::OFFSET + tooltip::ARROW_HEIGHT, trigger.top());
}

#[test]
fn popover_pads_its_content_by_size() {
    for (size, pad) in [
        (PopoverSize::Small, popover::PADDING_SMALL),
        (PopoverSize::Medium, popover::PADDING_MEDIUM),
        (PopoverSize::Large, popover::PADDING_LARGE),
    ] {
        let p = render(|ui| {
            let rt = ui.rt();
            let trigger = ui.mineral(PushButton::new("trg", "Open")).response().clone();
            let mut open = true;
            Popover::new("pop", "Details")
                .size(size)
                .side(Side::Bottom)
                .show(rt, &trigger, &mut open, |ui| {
                    ui.allocate_exact_size(vec2(100.0, 20.0), Sense::hover());
                });
        });
        assert_eq!(
            rect(&p, "pop-popover").size(),
            vec2(100.0 + 2.0 * pad, 20.0 + 2.0 * pad)
        );
    }
}

#[test]
fn teaching_popover_is_at_least_the_min_width() {
    let p = render(|ui| {
        let rt = ui.rt();
        let trigger = ui.mineral(PushButton::new("trg", "Open")).response().clone();
        let mut open = true;
        TeachingPopover::new("tp", "Tour")
            .step(TeachingStep::new("Body").title("Title"))
            .show(rt, &trigger, &mut open);
    });
    assert_eq!(
        rect(&p, "tp-teaching-popover").width(),
        teaching_popover::BODY_WIDTH + 2.0 * teaching_popover::PADDING
    );
}

#[test]
fn dialog_is_the_max_width_with_the_surface_padding_around_the_title() {
    let p = render(|ui| {
        let rt = ui.rt();
        let mut open = true;
        Dialog::new("dlg", "Title").action("Ok").show(rt, &mut open, |_| {});
    });
    let surface = rect(&p, "dlg-dialog");
    assert_eq!(surface.width(), dialog::MAX_WIDTH.min(SCREEN.x));
    let title = p.text("Title");
    assert_eq!(title.left() - surface.left(), dialog::PADDING + 1.0);
}

#[test]
fn dialog_actions_keep_their_width_and_end_align_inside_a_flow() {
    let mut h = harness(true, |ui, open| {
        let rt = ui.rt();
        Flow::new(vec2(8.0, 8.0), Align::Center).show(ui, |row| {
            row.add(|_| {});
            Dialog::new("dlg", "Delete file")
                .action("Cancel")
                .action("Delete")
                .show(rt, open, |_| {});
        });
    });
    h.run();
    let (a, b) = (h.rect("dlg-action-0-button"), h.rect("dlg-action-1-button"));
    assert!(
        a.width() >= button::MIN_WIDTH && b.width() >= button::MIN_WIDTH,
        "{a:?} {b:?}"
    );
    assert!(a.right() <= b.left(), "{a:?} {b:?}");
    assert_eq!(b.right(), h.rect("dlg-dialog").right() - dialog::PADDING - 1.0);
}

#[test]
fn overlay_drawer_width_follows_the_size() {
    for (size, width) in [
        (DrawerSize::Small, drawer::SIZE_SMALL),
        (DrawerSize::Medium, drawer::SIZE_MEDIUM),
        (DrawerSize::Large, SCREEN.x),
        (DrawerSize::Full, SCREEN.x),
    ] {
        let p = render(|ui| {
            let rt = ui.rt();
            let mut open = true;
            Drawer::new("dr", "Menu")
                .size(size)
                .position(DrawerPosition::End)
                .show_overlay(rt, &mut open, |_| {});
        });
        let r = rect(&p, "dr-drawer");
        assert_eq!(
            (r.width(), r.height(), r.right()),
            (width, SCREEN.y, SCREEN.x),
            "{size:?}"
        );
    }
}

#[test]
fn bottom_drawer_height_follows_the_size() {
    let p = render(|ui| {
        let rt = ui.rt();
        let mut open = true;
        Drawer::new("dr", "Menu")
            .size(DrawerSize::Small)
            .position(DrawerPosition::Bottom)
            .show_overlay(rt, &mut open, |_| {});
    });
    let r = rect(&p, "dr-drawer");
    assert_eq!(
        (r.height(), r.width(), r.bottom()),
        (drawer::SIZE_SMALL, SCREEN.x, SCREEN.y)
    );
}
