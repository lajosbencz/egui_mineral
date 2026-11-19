use egui::{Key, KeyboardShortcut, Modifiers, vec2};
use egui_mineral_fluent::{
    actions::PushButton,
    extras::{
        BarMenu, CommandPalette, MenuBar, Pagination, PaletteCommand, SplitPane, StatusBar, StatusItem, StatusProgress,
        StatusTone,
    },
    icons::{ICON_ARROW_DOWNLOAD, ICON_SAVE},
};

use super::heading;
use crate::Fui;

#[derive(Default)]
pub(crate) struct State {
    page: usize,
    palette_open: bool,
    query: String,
    ran: Option<usize>,
}

egui_mineral_fluent::egui_mineral::kit::opaque_debug!(State);

pub(crate) fn show(ui: &mut Fui<'_>, s: &mut State) {
    split_pane(ui);
    menu_bar(ui);
    status_bar(ui);
    pagination(ui, s);
    command_palette(ui, s);
}

fn split_pane(ui: &mut Fui<'_>) {
    heading(ui, "h-split-pane", "SplitPane");
    let rt = ui.rt();
    let width = ui.available_width();
    ui.allocate_ui(vec2(width, 120.0), |ui| {
        let ui = &mut rt.ui(ui);
        SplitPane::new("split", "Panes").min_first(80.0).min_second(80.0).show(
            ui,
            |ui| ui.label("First pane"),
            |ui| ui.label("Second pane"),
        );
    });
}

fn menu_bar(ui: &mut Fui<'_>) {
    heading(ui, "h-menu-bar", "MenuBar");
    MenuBar::new("bar")
        .menu(
            BarMenu::new("File", |m| {
                m.item("New");
                m.item("Open");
                m.divider();
                m.item("Exit");
            })
            .mnemonic('f'),
        )
        .menu(
            BarMenu::new("Edit", |m| {
                m.item("Copy");
                m.item("Paste");
            })
            .mnemonic('e'),
        )
        .show(ui);
}

fn status_bar(ui: &mut Fui<'_>) {
    heading(ui, "h-status-bar", "StatusBar");
    StatusBar::new("status", "Status")
        .item(StatusItem::new("Indexing").progress(StatusProgress::Circular(None)))
        .item(StatusItem::new("Upload").progress(StatusProgress::Linear(Some(0.4))))
        .item(StatusItem::new("2 errors").tone(StatusTone::Error).clickable(true))
        .trailing(StatusItem::new("Ln 4, Col 2").clickable(true))
        .show(ui);
}

fn pagination(ui: &mut Fui<'_>, s: &mut State) {
    heading(ui, "h-pagination", "Pagination");
    ui.mineral(Pagination::new("pages", 20, &mut s.page));
}

fn command_palette(ui: &mut Fui<'_>, s: &mut State) {
    heading(ui, "h-command-palette", "CommandPalette");
    if ui.mineral(PushButton::new("palette-open", "Open palette")).clicked() {
        s.palette_open = true;
    }
    let commands = [
        PaletteCommand::new("Save file").group("File").icon(ICON_SAVE),
        PaletteCommand::new("Download").group("File").icon(ICON_ARROW_DOWNLOAD),
        PaletteCommand::new("Toggle theme").group("View"),
    ];
    let toggle = KeyboardShortcut::new(Modifiers::COMMAND, Key::K);
    let rt = ui.rt();
    let out = CommandPalette::new("palette", commands, &mut s.query)
        .shortcut(toggle)
        .show_modal(rt, &mut s.palette_open);
    s.ran = out.clicked.or(s.ran);
}
