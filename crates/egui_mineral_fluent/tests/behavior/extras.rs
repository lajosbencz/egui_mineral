use egui::Key;
use egui_mineral_fluent::actions::PushButton;
use egui_mineral_fluent::extras::{
    BarMenu, CommandPalette, FormGrid, MenuBar, Pagination, PaletteCommand, SplitPane, StatusBar, StatusItem,
    StatusProgress,
};

use crate::support::{FluentHarness, FluentUi, harness};
use egui_mineral::layout::StackBelow;

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
                m.submenu("Recent", |m| {
                    m.item("last", "Last");
                });
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
fn right_on_a_submenu_leaf_moves_to_the_next_bar_menu() {
    let mut h = menu_bar();
    h.click("other-button");
    h.press(Key::F10);
    h.press(Key::ArrowDown);
    h.run();
    assert!(h.focused("bar-popup-0-0-menuitem"), "Down opens on the first row");
    h.press(Key::ArrowDown);
    h.press(Key::ArrowRight);
    h.run();
    h.press(Key::ArrowRight);
    h.run();
    assert!(h.tree().find_key("bar-popup-1-menu").is_some(), "the Edit menu opens");
    assert!(
        h.tree().find_key("bar-popup-0-menu").is_none(),
        "File closes with its submenu"
    );
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

#[test]
fn form_grid_aligns_controls_past_the_widest_label() {
    let mut h = harness((String::new(), String::new()), |ui, (a, b)| {
        FormGrid::new("g").stack_below(StackBelow::Never).show(ui, |g| {
            g.section("conn", "Connection");
            g.field(
                egui_mineral_fluent::inputs::Field::new("host").label("Host"),
                |ui, _| ui.mineral(egui_mineral_fluent::inputs::Input::new("host", "Host", a)),
            );
            g.field(
                egui_mineral_fluent::inputs::Field::new("db")
                    .label("Database name")
                    .required(true),
                |ui, _| ui.mineral(egui_mineral_fluent::inputs::Input::new("db", "Database name", b)),
            );
            g.full(|ui| ui.mineral(PushButton::new("save", "Save")));
        });
    });
    h.run();
    let (host, db) = (h.rect("host-input"), h.rect("db-input"));
    assert_eq!(host.left(), db.left(), "one control column");
    assert!(host.left() > h.rect("db-label").right(), "past the widest label");
    assert!(
        h.rect("save-button").left() < h.rect("host-label").left() + 1.0,
        "full rows start at the edge"
    );
    assert_eq!(h.node("g-conn").role, egui_mineral::semantic::Role::Heading);
}

#[test]
fn form_grid_stacks_fields_vertically_when_narrow() {
    use egui_mineral_fluent::inputs::{Field, Input};
    let mut h = harness(String::new(), |ui, a| {
        FormGrid::new("g").show(ui, |g| {
            g.field(Field::new("host").label("Host"), |ui, _| {
                ui.mineral(Input::new("host", "Host", a))
            });
        });
    });
    h.run();
    let (label, input) = (h.rect("host-label"), h.rect("host-input"));
    assert!(label.bottom() <= input.top(), "label above the control");
    assert!((label.left() - input.left()).abs() < 1.0, "one start edge");
}

#[test]
fn form_grid_places_typed_form_fields_in_its_columns() {
    use egui_mineral::form::{Form, FormState};
    use egui_mineral_fluent::inputs::{FormExt, FormUiExt};
    let mut h = harness(
        (FormState::default(), (String::new(), false)),
        |ui, (state, value): &mut (FormState, (String, bool))| {
            Form::new("f", state, value).show(ui, |f, v| {
                FormGrid::new("g").stack_below(StackBelow::Never).show_form(f, |f| {
                    f.section("acct", "Account");
                    f.text("name", "Name", &mut v.0, |s| s.required(true));
                    f.check("terms", "Accept", &mut v.1, |s| s);
                });
            });
        },
    );
    h.run();
    let (input, check) = (h.rect("f-name-input"), h.rect("f-terms-checkbox"));
    assert!(
        h.rect("f-name-field-label").right() < input.left(),
        "label beside the input"
    );
    assert!(
        h.rect("f-terms-field-label").right() < check.left(),
        "the column carries the checkbox label"
    );
    assert!((input.left() - check.left()).abs() < 1.0, "one control column");
    assert_eq!(h.node("f-acct").role, egui_mineral::semantic::Role::Heading);
}

#[test]
fn split_pane_collapses_either_pane_by_enter_and_expands_on_double_click() {
    use egui_mineral_fluent::egui_mineral::splitter::Pane;
    for pane in [Pane::First, Pane::Second] {
        let mut h = harness((None::<Pane>, 0u32), move |ui, (collapsed, resets)| {
            let r = SplitPane::new("sp", "Panes")
                .collapse(*collapsed)
                .collapsible(Some(pane))
                .show(ui, |ui| ui.label("A"), |ui| ui.label("B"));
            if let Some(c) = r.collapse {
                *collapsed = c.state();
            }
            *resets += u32::from(r.reset);
        });
        h.run();
        let handle = h.widget("sp-splitter");
        h.ctx.memory_mut(|m| m.request_focus(handle));
        h.press(Key::Enter);
        assert_eq!(h.state().value, (Some(pane), 0), "Enter collapses {pane:?}");
        let at = h.rect("sp-splitter").center();
        h.hover_at(at);
        for _ in 0..2 {
            h.drag_at(at);
            h.drop_at(at);
        }
        h.run();
        assert_eq!(h.state().value, (None, 1), "a double click expands and resets");
    }
}

#[test]
fn status_bar_tone_and_grip() {
    use egui_mineral::tone::Tone;
    const TINT: egui::Color32 = egui::Color32::from_rgb(10, 90, 200);
    let bar = |ui: &mut FluentUi<'_>, grip: bool| {
        narrow(ui, 300.0, |ui| {
            StatusBar::new("status", "Status")
                .tone(Tone::Custom {
                    container: TINT,
                    content: egui::Color32::WHITE,
                })
                .grip(grip)
                .trailing("Ln 4")
                .show(ui);
        });
    };
    let mut h = harness((), move |ui, ()| bar(ui, true));
    h.run();
    let (rect, end) = (h.rect("status-status-bar"), h.rect("status-0-status-item"));
    assert!(
        end.right() <= rect.right() - rect.height() + 0.5,
        "the grip keeps its corner"
    );
    let corner = rect.right_bottom() - egui::vec2(4.0, 4.0);
    h.hover_at(corner);
    h.drag_at(corner);
    h.step();
    let commands = &h.output().viewport_output[&egui::ViewportId::ROOT].commands;
    assert!(
        commands.contains(&egui::ViewportCommand::BeginResize(
            egui::viewport::ResizeDirection::SouthEast
        )),
        "{commands:?}"
    );
    h.drop_at(corner);
    h.input_mut()
        .viewports
        .entry(egui::ViewportId::ROOT)
        .or_default()
        .maximized = Some(true);
    h.run();
    let (rect, end) = (h.rect("status-status-bar"), h.rect("status-0-status-item"));
    assert!(
        end.right() > rect.right() - rect.height(),
        "maximized, the grip gives its corner back"
    );
    assert_eq!(
        crate::support::render(|ui| bar(ui, false)).fills(TINT).len(),
        1,
        "the bar takes the tone"
    );
}
