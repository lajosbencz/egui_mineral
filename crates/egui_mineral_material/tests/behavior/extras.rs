use egui::{Event, ImeEvent, Key, Shape};
use egui_mineral::{anchor::Side, date::Date, overlay::Dismiss};
use egui_mineral_material::{
    actions::PushButton,
    communication::RichTooltip,
    extras::{
        DateField, DescriptionItem, DescriptionList, Pagination, Popover, StatusBar, StatusItem, StatusProgress,
        StatusTone, TextLink, Timeline, TimelineItem,
    },
    selection::{Menu, MenuItem},
};

use crate::support::{MineralHarness, SCREEN, dismissals, harness, render, render_directed};
use egui_mineral_material::{spec::ColorRole, text_inputs::Elide};

#[test]
fn focused_date_field_requests_ime_and_takes_soft_keyboard_input() {
    let mut h = harness(SCREEN, None::<Date>, |ui, value| {
        ui.mineral(DateField::new("d", "Date", value));
    });
    assert!(h.output().platform_output.ime.is_none());
    h.key_press(Key::Tab);
    h.run();
    assert!(
        h.output().platform_output.ime.is_some(),
        "focus requests the soft keyboard"
    );
    h.event(Event::Ime(ImeEvent::Commit("03152024".into())));
    h.run();
    assert!(h.state().value.is_some());
    h.event(Event::Ime(ImeEvent::DeleteSurrounding {
        before_chars: 1,
        after_chars: 0,
    }));
    h.run();
    assert!(h.state().value.is_none());
}

#[test]
fn date_field_error_marks_the_node_invalid() {
    let mut h = harness(SCREEN, Some("Required".to_owned()), |ui, error| {
        let mut value = Some(Date::constant(2026, 3, 15));
        ui.mineral(DateField::new("d", "Date", &mut value).maybe_error(error.clone()));
    });
    h.run();
    assert!(h.node("d-date").state.invalid);
    h.state_mut().value = None;
    h.run();
    assert!(!h.node("d-date").state.invalid);
}

#[test]
fn popover_reports_escape_and_outside_dismissal() {
    let mut h = harness(SCREEN, (false, None), |ui, (open, dismissed)| {
        let trigger = ui.mineral(PushButton::new("trigger", "Open")).into_response();
        let r = Popover::new("pop", "Details").show(ui.rt(), &trigger, open, |ui| ui.label("Details"));
        assert_eq!(*open, r.is_open());
        *dismissed = dismissed.or(r.dismissed);
    });
    let outside = h.outside();
    h.click("trigger-button");
    assert!(h.state().value.0, "the trigger opens it");
    let got = dismissals(h, outside, |h| h.click("trigger-button"));
    assert_eq!(got, [Some(Dismiss::Escape), Some(Dismiss::Outside)]);
}

#[test]
fn popover_arrow_stays_on_a_surface_of_any_size() {
    for side in [Side::Bottom, Side::End] {
        for text in ["", "x"] {
            let mut h = harness(SCREEN, false, |ui, open| {
                let trigger = ui.mineral(PushButton::new("trigger", "Open")).into_response();
                Popover::new("pop", "Details")
                    .side(side)
                    .show(ui.rt(), &trigger, open, |ui| {
                        if !text.is_empty() {
                            ui.label(text);
                        }
                    });
            });
            h.click("trigger-button");
            let surface = h.rect("pop-popover");
            let arrows: Vec<_> = h
                .output()
                .shapes
                .iter()
                .filter_map(|s| match &s.shape {
                    Shape::Path(p) if p.closed && p.points.len() == 3 => Some(p.points.clone()),
                    _ => None,
                })
                .collect();
            assert_eq!(arrows.len(), 1, "{side:?} {text:?}: one arrow");
            let along = |p: egui::Pos2| if side.is_vertical() { p.x } else { p.y };
            let span = if side.is_vertical() {
                surface.x_range()
            } else {
                surface.y_range()
            };
            for p in &arrows[0] {
                assert!(
                    span.contains(along(*p)),
                    "{side:?} {text:?}: arrow {p:?} within {surface:?}"
                );
            }
        }
    }
}

#[test]
fn pagination_moves_a_stale_page_onto_the_last() {
    let mut h = harness(SCREEN, (3, 7), |ui, (total, page)| {
        ui.mineral(Pagination::new("p", *total, page));
    });
    h.run();
    assert_eq!(h.state().value.1, 2, "past the last page");
    h.state_mut().value = (0, 4);
    h.run();
    assert_eq!(h.state().value.1, 0, "no pages show one");
}

#[test]
fn pagination_reports_changed_when_a_stale_page_is_clamped() {
    let mut h = harness(SCREEN, (3, 7, false), |ui, (total, page, changed)| {
        *changed |= ui.mineral(Pagination::new("p", *total, page)).changed();
    });
    h.run();
    assert!(h.state().value.2, "clamping writes a different page");
}

#[test]
fn pagination_does_not_report_changed_for_a_valid_page() {
    let mut h = harness(SCREEN, (3, 1, false), |ui, (total, page, changed)| {
        *changed |= ui.mineral(Pagination::new("p", *total, page)).changed();
    });
    h.run();
    assert!(!h.state().value.2);
}

fn click_link(url: Option<&'static str>) -> (bool, bool, Option<String>) {
    let mut h = harness(SCREEN, (false, false), move |ui, (clicked, opened)| {
        let link = TextLink::new("lnk", "Docs");
        let link = match url {
            Some(u) => link.url(u),
            None => link,
        };
        *clicked |= ui.mineral(link).clicked();
        *opened |= ui
            .ctx()
            .output(|o| o.commands.iter().any(|c| matches!(c, egui::OutputCommand::OpenUrl(_))));
    });
    h.click("lnk-link");
    let value = h.node("lnk-link").value.clone();
    let (clicked, opened) = h.state().value;
    (clicked, opened, value)
}

#[test]
fn link_without_url_is_an_in_app_link() {
    assert_eq!(click_link(None), (true, false, None));
    assert_eq!(
        click_link(Some("https://example.com")),
        (true, true, Some("https://example.com".to_owned()))
    );
}

#[test]
fn rich_tooltip_reports_each_action_slot() {
    for (key, expected) in [("tip-0-button", (true, false)), ("tip-1-button", (false, true))] {
        let mut h = harness(SCREEN, (false, false), |ui, (primary, secondary)| {
            let anchor = ui.mineral(PushButton::new("anchor", "Anchor"));
            let r = RichTooltip::new("tip", "Supporting")
                .actions([(0, "Primary"), (1, "Secondary")])
                .show(ui.rt(), anchor.response());
            *primary |= r.clicked == Some(0);
            *secondary |= r.clicked == Some(1);
        });
        let anchor = h.rect("anchor-button").center();
        h.hover_at(anchor);
        h.step_until("tooltip shown", |h| h.tree().find_key(key).is_some());
        h.run();
        h.click(key);
        assert_eq!(h.state().value, expected, "{key}");
    }
}

#[test]
fn agent_key_override_keeps_widget_identity() {
    let ids = |key: Option<&'static str>| {
        let h = harness(SCREEN, (), |ui, ()| {
            let mut list = DescriptionList::new("dl").items([DescriptionItem::term("Term").detail("Detail")]);
            let mut timeline = Timeline::new("tl").item(TimelineItem::new("First"));
            if let Some(k) = key {
                list = list.agent_key(k);
                timeline = timeline.agent_key(k);
            }
            ui.mineral(list);
            ui.mineral(timeline);
        });
        let (list, timeline) = match key {
            Some(k) => (format!("{k}-description-list"), format!("{k}-timeline")),
            None => ("dl-description-list".to_owned(), "tl-timeline".to_owned()),
        };
        (h.widget(&list), h.widget(&timeline))
    };
    assert_eq!(ids(None), ids(Some("x")));
}

#[test]
fn date_field_keeps_the_date_on_unparsable_set_value() {
    use egui::accesskit::{Action, ActionData, ActionRequest, TreeId};
    let mut h = harness(SCREEN, Some(Date::constant(2025, 6, 10)), |ui, value| {
        ui.mineral(DateField::new("d", "Date", value));
    });
    h.run();
    h.event(Event::AccessKitActionRequest(ActionRequest {
        action: Action::SetValue,
        target_tree: TreeId::ROOT,
        target_node: h.widget("d-date").accesskit_id(),
        data: Some(ActionData::Value("not a date".into())),
    }));
    h.run();
    assert_eq!(h.state().value, Some(Date::constant(2025, 6, 10)));
}

#[test]
fn popover_opens_from_code() {
    let mut h = harness(SCREEN, true, |ui, open| {
        let trigger = ui.mineral(PushButton::new("trigger", "Open")).into_response();
        Popover::new("pop", "Details").show(ui.rt(), &trigger, open, |ui| ui.label("Details"));
    });
    h.run();
    assert!(
        h.tree().find_key("pop-popover").is_some(),
        "shown without a trigger click"
    );
    h.press(Key::Escape);
    assert!(!h.state().value, "Escape clears the app's flag");
}

fn status_bar(ui: &mut crate::support::Mui<'_>) -> Option<Status> {
    StatusBar::new("status", "Status")
        .item(StatusItem::new("Indexing").progress(StatusProgress::Circular(Some(0.4.into()))))
        .trailing_action(Status::Position, "Ln 4, Col 2")
        .action(Status::Long, "A very long item that does not fit")
        .show(ui)
        .clicked
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, egui_mineral::semantic::KeySalt)]
enum Status {
    Position,
    Long,
}

#[test]
fn status_bar_reports_clicks_and_drops_what_does_not_fit() {
    let mut h = harness(egui::vec2(340.0, 100.0), None, |ui, clicked| {
        *clicked = clicked.take().or(status_bar(ui));
    });
    let bar = h.rect("status-status-bar");
    let (start, end) = (h.rect("status-0-status-item"), h.rect("status-1-status-item"));
    assert!((start.left() - bar.left()).abs() < 0.5 && (end.right() - bar.right()).abs() < 0.5);
    let progress = h.rect("status-0-progress");
    assert!(start.contains_rect(progress), "progress inside its item");
    assert!(
        h.tree().find_key("status-2-status-item").is_none(),
        "overflow is dropped"
    );
    h.click("status-1-status-item");
    assert_eq!(h.state().value, Some(Status::Position));
}

#[test]
fn status_bar_mirrors_in_rtl() {
    let pass = render_directed(egui_mineral::i18n::Direction::Rtl, egui::vec2(400.0, 100.0), |ui| {
        status_bar(ui);
    });
    let rect = |k: &str| pass.tree.find_key(k).expect(k).rect;
    let bar = rect("status-status-bar");
    assert!(
        (rect("status-0-status-item").right() - bar.right()).abs() < 0.5,
        "start item on the right"
    );
    assert!(
        (rect("status-1-status-item").left() - bar.left()).abs() < 0.5,
        "end item on the left"
    );
}

fn long_item_bar(ui: &mut crate::support::Mui<'_>, elide: bool) {
    let item = StatusItem::new("A long log line that cannot possibly fit the narrow bar").tone(StatusTone::Error);
    StatusBar::new("status", "Status")
        .item(StatusItem::new("main"))
        .item(if elide { item.elide(Elide::End) } else { item })
        .trailing(StatusItem::new("UTF-8"))
        .show(ui);
}

#[test]
fn status_item_elides_into_the_room_left_instead_of_dropping() {
    let mut h = harness(egui::vec2(260.0, 100.0), true, |ui, elide| long_item_bar(ui, *elide));
    let (bar, item, end) = (
        h.rect("status-status-bar"),
        h.rect("status-1-status-item"),
        h.rect("status-2-status-item"),
    );
    assert!(item.right() <= end.left() + 0.5 && item.width() > 0.0 && bar.contains_rect(item));
    h.state_mut().value = false;
    h.run();
    assert!(
        h.tree().find_key("status-1-status-item").is_none(),
        "dropped without elide"
    );
}

#[test]
fn status_item_custom_tone_paints_its_strip() {
    let amber = egui::Color32::from_rgb(250, 200, 20);
    let warning = StatusTone::Custom {
        container: amber,
        content: egui::Color32::BLACK,
    };
    let pass = render(|ui| {
        StatusBar::new("status", "Status")
            .item(StatusItem::new("Low disk").tone(warning))
            .item(StatusItem::new("Failed").tone(StatusTone::Error))
            .item(StatusItem::new("Plain").tone(StatusTone::Error).tone(warning))
            .show(ui);
    });
    let strip = |c: egui::Color32| {
        pass.shapes
            .iter()
            .filter(|s| matches!(&s.shape, egui::Shape::Rect(r) if r.fill == c))
            .count()
    };
    assert_eq!(strip(amber), 2, "custom tones paint their strips");
    assert_eq!(
        pass.filled(ColorRole::ErrorContainer).len(),
        1,
        "the error tone paints its own"
    );
}

#[test]
fn popover_show_at_follows_the_app_flag_and_ignores_anchor_clicks() {
    let mut h = harness(SCREEN, (false, None), |ui, (open, dismissed)| {
        let anchor = ui.mineral(PushButton::new("trigger", "Open")).response().rect;
        let r = Popover::new("pop", "Details").show_at(ui.rt(), anchor, open, |ui| ui.label("Details"));
        *dismissed = dismissed.or(r.dismissed);
    });
    h.run();
    assert!(
        h.tree().find_key("pop-popover").is_none(),
        "closed until the app opens it"
    );
    h.state_mut().value.0 = true;
    h.run();
    assert!(h.tree().find_key("pop-popover").is_some(), "opened from code");
    h.click("trigger-button");
    assert!(h.state().value.0, "an anchor click neither toggles nor dismisses");
    let outside = h.outside();
    assert_eq!(
        crate::support::flag_dismissals(h, outside),
        [Some(Dismiss::Escape), Some(Dismiss::Outside)]
    );
}

#[test]
fn popover_opened_by_a_menu_pick_survives_the_picking_press() {
    let mut h = harness(SCREEN, (false, false), |ui, (menu, pop)| {
        let more = ui.mineral(PushButton::new("more", "More")).into_response();
        if more.clicked() {
            *menu = !*menu;
        }
        let picked = Menu::new("m", "More")
            .item(0, MenuItem::new("Rename"))
            .show_at(ui.rt(), more.rect, menu)
            .clicked;
        *pop |= picked.is_some();
        ui.add_space(200.0);
        let target = ui.mineral(PushButton::new("target", "Target")).response().rect;
        Popover::new("pop", "Details").show_at(ui.rt(), target, pop, |ui| ui.label("Details"));
    });
    h.click("more-button");
    h.click("m-0-menu-item");
    assert!(h.state().value.1, "still open after the picking pass");
    h.run();
    assert!(h.tree().find_key("pop-popover").is_some());
}

#[test]
fn popover_show_at_focuses_the_first_focusable_of_its_content() {
    let mut h = harness(SCREEN, false, |ui, open| {
        let anchor = ui.mineral(PushButton::new("trigger", "Open")).response().rect;
        Popover::new("pop", "Details").show_at(ui.rt(), anchor, open, |ui| {
            ui.mineral(PushButton::new("first", "First"));
            ui.mineral(PushButton::new("second", "Second"));
        });
    });
    h.run();
    h.state_mut().value = true;
    h.run();
    h.run();
    assert!(h.focused("first-button"));
}

#[test]
fn menu_opened_by_a_menu_pick_survives_the_picking_press() {
    let mut h = harness(SCREEN, (false, false), |ui, (first, second)| {
        let more = ui.mineral(PushButton::new("more", "More")).into_response();
        if more.clicked() {
            *first = !*first;
        }
        let picked = Menu::new("a", "More")
            .item(0, MenuItem::new("Rename"))
            .show_at(ui.rt(), more.rect, first)
            .clicked;
        *second |= picked.is_some();
        ui.add_space(200.0);
        let target = ui.mineral(PushButton::new("target", "Target")).response().rect;
        Menu::new("b", "Target")
            .item(0, MenuItem::new("Copy"))
            .show_at(ui.rt(), target, second);
    });
    h.click("more-button");
    h.click("a-0-menu-item");
    assert!(h.state().value.1, "still open after the picking pass");
}

#[test]
fn status_item_linear_progress_paints_a_status_line_bar_inside_its_item() {
    let mut h = harness(SCREEN, (), |ui, ()| {
        StatusBar::new("s", "Status")
            .item(StatusItem::new("Upload").progress(StatusProgress::Linear(Some(0.5.into()))))
            .item(StatusItem::new("Sync").progress(StatusProgress::Linear(None)))
            .show(ui);
    });
    h.step();
    h.step();
    for i in 0..2 {
        let (item, bar) = (
            h.rect(&format!("s-{i}-status-item")),
            h.rect(&format!("s-{i}-progress")),
        );
        assert!(item.contains_rect(bar), "bar {i} inside its item");
        assert!(bar.width() > 4.0 * bar.height(), "bar {i} is linear: {bar:?}");
        assert!(bar.height() < item.height(), "bar {i} fits the status line");
    }
}

#[test]
fn status_item_with_tabular_figures_keeps_its_text_and_place() {
    let bar = |tabular| {
        render(|ui| {
            StatusBar::new("status", "Status")
                .item(StatusItem::new("Frame 120").tabular_figures(tabular))
                .show(ui);
        })
        .text("Frame 120")
    };
    let (plain, tabular) = (bar(false), bar(true));
    assert_eq!(plain.min, tabular.min);
    assert!(tabular.width() >= plain.width());
}
