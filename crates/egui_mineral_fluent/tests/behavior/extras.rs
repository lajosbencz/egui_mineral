use egui::Key;
use egui_mineral_fluent::actions::PushButton;
use egui_mineral_fluent::extras::{
    BarMenu, CommandPalette, MenuBar, Pagination, PaletteCommand, SplitPane, StatusBar, StatusItem, StatusProgress,
};

use crate::support::{FluentHarness, FluentUi, harness};

fn pane(disabled: bool) -> impl FnMut(&mut FluentUi<'_>, &mut f32) {
    move |ui, fraction| {
        *fraction = SplitPane::new("sp", "Panes")
            .disabled(disabled)
            .show(ui, |ui| ui.label("A"), |ui| ui.label("B"))
            .fraction
            .get();
    }
}

#[test]
fn split_pane_divider_moves_by_key() {
    let mut h = harness(0.0, pane(false));
    h.run();
    let before = h.state().value;
    let handle = h.widget("sp-splitter");
    h.ctx.memory_mut(|m| m.request_focus(handle));
    h.press(Key::ArrowRight);
    assert!(h.state().value > before, "{} > {before}", h.state().value);
}

#[test]
fn disabled_split_pane_ignores_keys() {
    let mut h = harness(0.0, pane(true));
    h.run();
    let before = h.state().value;
    let handle = h.widget("sp-splitter");
    h.ctx.memory_mut(|m| m.request_focus(handle));
    h.press(Key::ArrowRight);
    assert_eq!(h.state().value, before);
}

fn narrow<R>(ui: &mut FluentUi<'_>, width: f32, add: impl FnOnce(&mut FluentUi<'_>) -> R) -> R {
    let rt = ui.rt();
    ui.allocate_ui(egui::vec2(width, 100.0), |ui| add(&mut rt.ui(ui))).inner
}

// Text advances differ with the fonts installed; one-letter labels and the lead indicators keep the widths close.
fn status_bar(ui: &mut FluentUi<'_>, width: f32) -> Option<Status> {
    narrow(ui, width, |ui| {
        StatusBar::new("status", "Status")
            .item(StatusItem::new("x").progress(StatusProgress::Circular(None)))
            .trailing_action(
                Status::Position,
                StatusItem::new("x").progress(StatusProgress::Linear(None)),
            )
            .action(
                Status::Progress,
                StatusItem::new("x").progress(StatusProgress::Linear(Some(0.2.into()))),
            )
            .action(Status::Last, "x")
            .show(ui)
            .clicked
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, egui_mineral::semantic::KeySalt)]
enum Status {
    Position,
    Progress,
    Last,
}

#[test]
fn status_bar_reports_clicks_and_drops_what_does_not_fit() {
    let mut h = harness(None, |ui, clicked| {
        *clicked = clicked.take().or(status_bar(ui, 185.0));
    });
    h.run();
    let bar = h.rect("status-status-bar");
    let (start, end) = (h.rect("status-0-status-item"), h.rect("status-1-status-item"));
    assert!((start.left() - bar.left()).abs() < 0.5 && (end.right() - bar.right()).abs() < 0.5);
    assert!(
        start.contains_rect(h.rect("status-0-progress")),
        "progress inside its item"
    );
    assert_eq!(h.node("status-0-progress").name, "x", "named by the item text");
    assert!(
        h.tree().find_key("status-3-status-item").is_none(),
        "overflow is dropped"
    );
    h.click("status-1-status-item");
    assert_eq!(h.state().value, Some(Status::Position));
}

#[test]
fn status_item_bar_progress_exposes_its_value() {
    let mut h = harness((), |ui, ()| {
        narrow(ui, 300.0, |ui| {
            StatusBar::new("status", "Status")
                .item(StatusItem::new("Upload").progress(StatusProgress::Linear(Some(0.5.into()))))
                .show(ui);
        });
    });
    h.run();
    let progress = h.node("status-0-progress");
    assert_eq!(progress.state.range.map(|r| r.value), Some(0.5));
    assert!(h.rect("status-0-status-item").contains_rect(progress.rect));
}

fn pagination(total: usize, page: usize) -> crate::support::Harness<'static, usize> {
    harness(page, move |ui, page| {
        ui.mineral(Pagination::new("pg", total, page).first_last(true));
    })
}

#[test]
fn pagination_pages_move_the_bound_index_and_mark_the_current_one() {
    let mut h = pagination(10, 4);
    h.run();
    assert_eq!(h.node("pg-5-page-button").state.checked, Some(true.into()));
    h.click("pg-6-page-button");
    assert_eq!(h.state().value, 5);
    h.click("pg-next-button");
    assert_eq!(h.state().value, 6);
    h.click("pg-first-button");
    assert_eq!(h.state().value, 0);
    h.click("pg-last-button");
    assert_eq!(h.state().value, 9);
    assert!(h.node("pg-next-button").state.disabled, "no page after the last");
}

#[test]
fn pagination_moves_a_stale_page_onto_the_last() {
    let mut h = pagination(3, 7);
    h.run();
    assert_eq!(h.state().value, 2);
    assert_eq!(h.node("pg-pagination").role, egui_mineral::semantic::Role::Navigation);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, egui_mineral::semantic::KeySalt)]
enum Cmd {
    Save,
    Print,
    Open,
}

#[test]
fn command_palette_reports_the_clicked_entry() {
    let mut h = harness((true, String::new(), None), |ui, (open, query, clicked)| {
        let r = CommandPalette::new("pal", query)
            .command("Save")
            .action(Cmd::Open, "Open")
            .show_modal(ui.rt(), open);
        *clicked = clicked.take().or(r.clicked);
    });
    h.run();
    h.click("pal-1-command");
    assert_eq!((h.state().value.0, h.state().value.2), (false, Some(Cmd::Open)));
}

#[test]
fn command_palette_keys_skip_disabled_entries() {
    let mut h = harness((true, String::new(), None), |ui, (open, query, clicked)| {
        let r = CommandPalette::new("pal", query)
            .actions([
                (Cmd::Save, PaletteCommand::new("Save")),
                (Cmd::Print, PaletteCommand::new("Print").disabled(true)),
                (Cmd::Open, PaletteCommand::new("Open")),
            ])
            .show_modal(ui.rt(), open);
        *clicked = clicked.take().or(r.clicked);
    });
    h.run();
    h.press(Key::ArrowDown);
    h.press(Key::Enter);
    assert_eq!((h.state().value.0, h.state().value.2), (false, Some(Cmd::Open)));
}

fn menu_bar() -> crate::support::Harness<'static, Vec<&'static str>> {
    harness(Vec::new(), |ui, log| {
        let mut clicked = Vec::new();
        MenuBar::new("bar")
            .menu(BarMenu::new("File", |m| {
                if m.item("open", "Open").clicked {
                    clicked.push("open");
                }
            }))
            .menu(BarMenu::new("Edit", |m| {
                m.item("copy", "Copy");
            }))
            .menu(
                BarMenu::new("Off", |m| {
                    m.item("never", "Never");
                })
                .disabled(true),
            )
            .show(ui);
        log.extend(clicked);
        ui.mineral(PushButton::new("other", "Other"));
    })
}

#[test]
fn menu_bar_click_opens_its_menu_and_a_row_pick_closes_it() {
    let mut h = menu_bar();
    h.run();
    assert_eq!(h.node("bar-menubar").role, egui_mineral::semantic::Role::MenuBar);
    h.click("bar-0-menubar-item");
    h.run();
    assert_eq!(h.node("bar-popup-0-menu").role, egui_mineral::semantic::Role::Menu);
    h.click("bar-popup-0-0-menuitem");
    assert_eq!(h.state().value, ["open"]);
    assert!(
        h.tree().find_key("bar-popup-0-menu").is_none(),
        "picking closes the menu"
    );
}

#[test]
fn menu_bar_keys_open_move_between_menus_and_escape_returns_focus() {
    let mut h = menu_bar();
    h.click("other-button");
    h.press(Key::F10);
    assert!(h.focused("bar-0-menubar-item"), "F10 focuses the first title");
    h.press(Key::ArrowRight);
    assert!(h.focused("bar-1-menubar-item"), "Right roves to the next title");
    h.press(Key::ArrowDown);
    h.run();
    assert!(h.tree().find_key("bar-popup-1-menu").is_some(), "Down opens the menu");
    h.press(Key::ArrowLeft);
    h.run();
    assert!(
        h.tree().find_key("bar-popup-0-menu").is_some(),
        "Left moves to the previous menu"
    );
    h.press(Key::Escape);
    assert!(
        h.tree().find_key("bar-popup-0-menu").is_none(),
        "Escape closes the menu"
    );
    assert!(h.focused("bar-0-menubar-item"), "Escape returns focus to its title");
}

#[test]
fn command_palette_exposes_semantics_and_escape_dismisses() {
    let mut h = harness((true, String::new(), None), |ui, (open, query, dismissed)| {
        let r = CommandPalette::new("pal", query)
            .commands([PaletteCommand::new("Save").group("File"), PaletteCommand::new("Open")])
            .show_modal(ui.rt(), open);
        *dismissed = dismissed.or(r.dismissed);
    });
    h.run();
    assert_eq!(h.node("pal-commands").role, egui_mineral::semantic::Role::ListBox);
    assert_eq!(
        h.node("pal-0-command").role,
        egui_mineral::semantic::Role::ListBoxOption
    );
    assert!(h.tree().find_key("pal-search").is_some());
    h.press(Key::Escape);
    assert!(!h.state().value.0, "Escape closes");
    assert!(h.state().value.2.is_some());
}
