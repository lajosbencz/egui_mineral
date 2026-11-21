use egui::{Key, KeyboardShortcut, Modifiers, vec2};
use egui_mineral_fluent::egui_mineral::tone::Tone;
use egui_mineral_fluent::{
    actions::PushButton,
    extras::{
        BarMenu, CommandPalette, MenuBar, Pagination, PaletteCommand, SplitPane, StatusBar, StatusItem, StatusProgress,
    },
    icons::{ICON_ARROW_DOWNLOAD, ICON_SAVE},
};

use super::heading;
use crate::FluentUi;

#[derive(Default)]
pub(crate) struct State {
    page: usize,
    palette_open: bool,
    query: String,
    ran: Option<&'static str>,
}

egui_mineral_fluent::egui_mineral::kit::opaque_debug!(State);

pub(crate) fn show(ui: &mut FluentUi<'_>, s: &mut State) {
    split_pane(ui);
    menu_bar(ui);
    status_bar(ui);
    pagination(ui, s);
    command_palette(ui, s);
}

fn split_pane(ui: &mut FluentUi<'_>) {
    heading(ui, "h-split-pane", "SplitPane");
    let width = ui.available_width();
    ui.allocate_ui(vec2(width, 120.0), |ui| {
        SplitPane::new("split", "Panes").min_first(80.0).min_second(80.0).show(
            ui,
            |ui| ui.label("First pane"),
            |ui| ui.label("Second pane"),
        );
    });
}

fn menu_bar(ui: &mut FluentUi<'_>) {
    heading(ui, "h-menu-bar", "MenuBar");
    MenuBar::new("bar")
        .menu(
            BarMenu::new("File", |m| {
                m.item("new", "New");
                m.item("open", "Open");
                m.divider();
                m.item("exit", "Exit");
            })
            .mnemonic('f'),
        )
        .menu(
            BarMenu::new("Edit", |m| {
                m.item("copy", "Copy");
                m.item("paste", "Paste");
            })
            .mnemonic('e'),
        )
        .show(ui);
}

fn status_bar(ui: &mut FluentUi<'_>) {
    heading(ui, "h-status-bar", "StatusBar");
    StatusBar::new("status", "Status")
        .item(StatusItem::new("Indexing").progress(StatusProgress::Circular(None)))
        .item(StatusItem::new("Upload").progress(StatusProgress::Linear(Some(0.4.into()))))
        .action("errors", StatusItem::new("2 errors").tone(Tone::Error))
        .trailing_action("position", "Ln 4, Col 2")
        .show(ui);
}

fn pagination(ui: &mut FluentUi<'_>, s: &mut State) {
    heading(ui, "h-pagination", "Pagination");
    ui.mineral(Pagination::new("pages", 20, &mut s.page));
}

fn command_palette(ui: &mut FluentUi<'_>, s: &mut State) {
    heading(ui, "h-command-palette", "CommandPalette");
    if ui.mineral(PushButton::new("palette-open", "Open palette")).clicked() {
        s.palette_open = true;
    }
    let commands = [
        ("save", PaletteCommand::new("Save file").group("File").icon(ICON_SAVE)),
        (
            "download",
            PaletteCommand::new("Download").group("File").icon(ICON_ARROW_DOWNLOAD),
        ),
        ("theme", PaletteCommand::new("Toggle theme").group("View")),
    ];
    let toggle = KeyboardShortcut::new(Modifiers::COMMAND, Key::K);
    let rt = ui.rt();
    let out = CommandPalette::new("palette", &mut s.query)
        .actions(commands)
        .shortcut(toggle)
        .show_modal(rt, &mut s.palette_open);
    s.ran = out.clicked.or(s.ran);
}
