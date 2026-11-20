use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use egui::{DroppedFile, Key, vec2};
use egui_mineral::widget::SizeExt;
use egui_mineral::{
    overlay::Dismiss,
    platform::{FileFilter, FileType},
    steps::Steps,
};
use egui_mineral_material::{
    actions::{PushButton, SplitButton},
    communication::PlainTooltip,
    extras::{
        Alert, Breadcrumbs, FileDropZone, FileDropZoneOutput, HoverCard, ImageList, ImageListLayout, Severity,
        SplitPane, Stepper, TransferList,
    },
    selection::{Menu, MenuItem},
};

use crate::support::{MineralHarness, SCREEN, WIDE, harness, still};

#[test]
fn split_button_reports_the_leading_click_and_menu_pick() {
    let mut h = still(WIDE, (0, None, false), |ui, (leading, picked, open)| {
        let menu = Menu::new("more", "More")
            .item(0, MenuItem::new("Draft"))
            .item(1, MenuItem::new("Copy"));
        let r = SplitButton::new("send", "Send", menu).show(ui);
        *leading += usize::from(r.leading_clicked);
        *picked = picked.or(r.clicked);
        *open = r.open;
    });
    h.click("send-button");
    assert_eq!(h.state().value, (1, None, false));
    h.click("send-split-menu-button");
    assert!(h.state().value.2, "the trailing button opens the menu");
    h.click("more-1-menu-item");
    assert_eq!(h.state().value, (1, Some(1), false), "a pick closes the menu");
}

#[test]
fn hover_card_and_plain_tooltip_show_while_hovered() {
    let mut h = harness(WIDE, (false, false), |ui, (card, tip)| {
        let trigger = ui.mineral(PushButton::new("trigger", "Profile")).into_response();
        let r = HoverCard::new("card", "Profile").show(ui.rt(), &trigger, |ui| ui.label("Bio"));
        *card = r.response.is_some();
        *tip = PlainTooltip::new("Opens the profile")
            .show(ui.rt(), &trigger)
            .response
            .is_some();
    });
    h.run();
    assert_eq!(h.state().value, (false, false));
    let over = h.rect("trigger-button").center();
    h.hover_at(over);
    h.step_until("both show after the hover delay", |h| h.state().value == (true, true));
    h.hover_at(h.outside());
    h.step_until("both hide once the pointer leaves", |h| {
        h.state().value == (false, false)
    });
}

#[test]
fn stepper_advances_only_to_selectable_steps() {
    let mut h = still(WIDE, Steps::new(3), |ui, steps| {
        Stepper::new("flow", steps)
            .step("Account")
            .step("Plan")
            .step("Pay")
            .show(ui, |ui, i| {
                ui.label(format!("Step {i}"));
            });
    });
    h.click("flow-2-step");
    assert_eq!(h.state().value.current(), 0, "a linear flow skips no step");
    assert!(h.state_mut().value.forward());
    h.run();
    assert_eq!(h.state().value.current(), 1);
    h.click("flow-0-step");
    assert_eq!(h.state().value.current(), 0, "completed steps stay reachable");
}

#[test]
fn transfer_list_moves_checked_items_to_the_target() {
    let mut h = still(WIDE, Vec::new(), |ui, chosen| {
        ui.mineral(TransferList::new("tl", chosen).items(["Red", "Green", "Blue"].into_iter().enumerate()));
    });
    h.click("tl-source-1");
    h.click("tl-to-target-button");
    assert_eq!(h.state().value, [1]);
    h.click("tl-target-1");
    h.click("tl-to-source-button");
    assert!(h.state().value.is_empty(), "moved back");
}

#[test]
fn transfer_list_takes_its_size_from_constraints() {
    let h = still(WIDE, Vec::new(), |ui, chosen| {
        ui.mineral(
            TransferList::new("tl", chosen)
                .item(0, "Red")
                .preferred_size(vec2(480.0, 240.0)),
        );
    });
    assert_eq!(h.rect("tl-transfer").size(), vec2(480.0, 240.0));
}

#[test]
fn transfer_list_drops_repeated_and_unknown_choices() {
    let h = still(WIDE, vec![1, 1, 7, 0], |ui, chosen| {
        ui.mineral(TransferList::new("tl", chosen).items(["Red", "Green", "Blue"].into_iter().enumerate()));
    });
    assert_eq!(h.state().value, [1, 0]);
}

#[test]
fn alert_reports_actions_and_dismissal() {
    let mut h = still(SCREEN, (None, None), |ui, (clicked, dismissed)| {
        let r = Alert::new("a", "Disk almost full")
            .severity(Severity::Warning)
            .action(0, "Clean up")
            .dismissible(true)
            .show(ui);
        *clicked = clicked.or(r.clicked);
        *dismissed = dismissed.or(r.dismissed);
    });
    h.click("a-0-button");
    assert_eq!(h.state().value, (Some(0), None));
    h.click("a-dismiss-button");
    assert_eq!(h.state().value, (Some(0), Some(Dismiss::Close)));
}

#[test]
fn breadcrumbs_report_the_clicked_ancestor() {
    let mut h = still(WIDE, Vec::new(), |ui, seen: &mut Vec<usize>| {
        seen.extend(
            Breadcrumbs::new("bc")
                .items(["Home", "Docs", "Page"].into_iter().enumerate())
                .show(ui)
                .clicked,
        );
    });
    h.click("bc-1-link");
    h.click("bc-2-link");
    assert_eq!(h.state().value, [1], "the current page is no link");
}

#[test]
fn breadcrumbs_with_nothing_to_hide_show_every_crumb_once() {
    for items in [vec!["Home"], vec!["Home", "Page"]] {
        let n = items.len();
        let mut h = still(vec2(30.0, 100.0), items, |ui, items| {
            Breadcrumbs::new("bc").items(items.clone().into_iter().enumerate()).show(ui);
        });
        h.run();
        let keys: Vec<_> = h.tree().nodes.iter().map(|n| n.id.key().to_owned()).collect();
        let count = |key: &str| keys.iter().filter(|k| *k == key).count();
        assert_eq!(count("bc-0-link"), 1, "{n} items: {keys:?}");
        assert_eq!(count("bc-more-button"), 0, "{n} items: {keys:?}");
    }
}

#[test]
fn split_pane_divider_moves_by_key() {
    let mut h = still(WIDE, 0.0, |ui, fraction| {
        *fraction = SplitPane::new("sp", "Panes")
            .show(ui, |ui| ui.label("A"), |ui| ui.label("B"))
            .fraction
            .get();
    });
    let before = h.state().value;
    let handle = h.widget("sp-splitter");
    h.ctx.memory_mut(|m| m.request_focus(handle));
    h.press(Key::ArrowRight);
    assert!(h.state().value > before, "{} > {before}", h.state().value);
}

#[test]
fn image_list_reports_the_activated_tile() {
    let mut h = still(WIDE, None, |ui, activated| {
        let r = ImageList::new("gallery").show(ui, 4, |ui, i| {
            ui.label(format!("{i}"));
        });
        *activated = activated.or(r.activated);
    });
    h.click("gallery-2-tile");
    assert_eq!(h.state().value, Some(2));
}

#[test]
fn image_list_masonry_measures_once_per_version() {
    let calls = std::cell::Cell::new(0_usize);
    let mut h = still(WIDE, 0_u64, |ui, version| {
        ImageList::new("gallery")
            .layout(ImageListLayout::Masonry)
            .aspect(|_| {
                calls.set(calls.get() + 1);
                1.0
            })
            .aspect_version(*version)
            .show(ui, 50, |_, _| {});
    });
    h.run();
    assert_eq!(calls.take(), 50, "measured once");
    h.run();
    assert_eq!(calls.take(), 0, "cached while the version holds");
    h.state_mut().value = 1;
    h.run();
    assert_eq!(calls.take(), 50, "a new version measures again");
}

#[test]
fn image_list_caps_hostile_column_counts() {
    let mut h = still(WIDE, (), |ui, ()| {
        ImageList::new("gallery").columns(usize::MAX).show(ui, 100, |_, _| {});
    });
    h.run();
    assert!(h.rect("gallery-31-tile").top() < h.rect("gallery-32-tile").top());
}

#[derive(Debug)]
struct Fake(PathBuf);

impl DroppedFile for Fake {
    fn path(&self) -> &Path {
        &self.0
    }

    fn bytes(&self) -> Result<Vec<u8>, String> {
        Ok(Vec::new())
    }
}

#[test]
fn file_drop_zone_splits_drops_by_filter() {
    let mut h = still(WIDE, (Vec::new(), Vec::new()), |ui, (dropped, rejected)| {
        let zone = FileDropZone::new("drop").accept(FileFilter::new(FileType::new("Images", ["png"])));
        let r = ui.mineral(zone);
        let FileDropZoneOutput {
            dropped: d,
            rejected: x,
            ..
        } = r.output().clone();
        dropped.extend(d.iter().map(|f| f.path().to_owned()));
        rejected.extend(x.iter().map(|f| f.path().to_owned()));
    });
    h.run();
    let at = h.rect("drop-dropzone").center();
    h.hover_at(at);
    h.input_mut().dropped_files = vec![Arc::new(Fake("a.png".into())), Arc::new(Fake("b.txt".into()))];
    h.step();
    let (dropped, rejected) = &h.state().value;
    assert_eq!(
        (dropped, rejected),
        (&vec![PathBuf::from("a.png")], &vec![PathBuf::from("b.txt")])
    );
}

#[test]
fn sibling_steppers_scope_their_content_apart() {
    let mut h = still(WIDE, Vec::new(), |ui, ids: &mut Vec<egui::Id>| {
        ids.clear();
        for salt in ["a", "b"] {
            let mut steps = Steps::new(2);
            Stepper::new(salt, &mut steps)
                .axis(egui_mineral::layout::Axis::Vertical)
                .step("One")
                .step("Two")
                .show(ui, |ui, _| ids.push(ui.scope_id()));
        }
    });
    h.run();
    let ids = &h.state().value;
    assert_eq!(ids.len(), 2);
    assert_ne!(ids[0], ids[1]);
}

#[test]
fn two_steppers_in_one_parent_keep_separate_headers() {
    let mut h = still(WIDE, (Steps::new(3), Steps::new(3)), |ui, (a, b)| {
        for (salt, steps) in [("a", a), ("b", b)] {
            Stepper::new(salt, steps)
                .step("One")
                .step("Two")
                .step("Three")
                .show(ui, |_, _| {});
        }
    });
    h.state_mut().value.0.forward();
    h.state_mut().value.1.forward();
    h.run();
    h.click("a-0-step");
    assert_eq!((h.state().value.0.current(), h.state().value.1.current()), (0, 1));
}
