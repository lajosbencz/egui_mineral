use crate::Mui;
use crate::Rt;
use egui::{Key, KeyboardShortcut, Modifiers, Sense, UiBuilder, vec2};
use egui_mineral::shortcut;
use egui_mineral_material::MaterialExt;
use egui_mineral_material::{
    catalog::Component,
    extras::MenuPath,
    icons::{
        ICON_ADD, ICON_ARROW_BACK, ICON_BOOKMARK, ICON_CALL_SPLIT, ICON_CONTENT_COPY, ICON_CONTENT_CUT,
        ICON_CONTENT_PASTE, ICON_DELETE, ICON_EDIT, ICON_EXPLORE, ICON_FAVORITE, ICON_FORMAT_BOLD, ICON_FORMAT_ITALIC,
        ICON_FORMAT_UNDERLINED, ICON_HOME, ICON_INBOX, ICON_MAIL, ICON_MENU, ICON_MIC, ICON_PERSON, ICON_SEARCH,
        ICON_SEND, ICON_SETTINGS, ICON_SHARE, ICON_STAR,
    },
    navigation::{ToolbarResponse, TopAppBarResponse},
    prelude::*,
};

use super::{Shared, caption};

pub(crate) const TOPICS: &[&str] = &[
    "Tabs",
    "Navigation bar",
    "Navigation rail",
    "Navigation drawer",
    "Top app bar",
    "Toolbar",
    "Menu",
    "Menu bar",
    "Breadcrumbs",
    "Pagination",
    "Stepper",
    "Search bar",
    "Search view",
    "Search with suggestions",
    "Status bar",
];

const WORDS: [&str; 12] = [
    "Alpine lake",
    "Alpaca farm",
    "Amber rock",
    "Birch forest",
    "Blue lagoon",
    "Canyon trail",
    "Cedar grove",
    "Desert dune",
    "Glacier peak",
    "Granite ridge",
    "Mossy creek",
    "Volcano rim",
];

pub(crate) struct State {
    tab_primary: usize,
    tab_secondary: usize,
    tab_primary_icons: usize,
    tab_secondary_icons: usize,
    tab_scroll: usize,
    bar: usize,
    bar_badges: usize,
    bar_flex: usize,
    bar_horizontal: usize,
    bar_selected_only: usize,
    rail: usize,
    rail_expr: usize,
    rail_toggle: usize,
    rail_expanded: bool,
    drawer: usize,
    drawer_sections: usize,
    appbar_scroll: f64,
    appbar_query: String,
    toolbar_bold: bool,
    toolbar_italic: bool,
    toolbar_expanded: bool,
    menu_selected: usize,
    menu_checked: bool,
    pub(crate) last_action: String,
    pub(crate) zoom: u32,
    page: usize,
    page_wide: usize,
    crumbs: usize,
    steps_h: Steps,
    steps_v: Steps,
    steps_free: Steps,
    bar_query: String,
    back_query: String,
    view_query: String,
    view_open: bool,
    view_docked: bool,
    suggest_query: String,
    suggest_chosen: Option<usize>,
    suggestions: Vec<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            tab_primary: 0,
            tab_secondary: 1,
            tab_primary_icons: 0,
            tab_secondary_icons: 1,
            tab_scroll: 0,
            bar: 0,
            bar_badges: 0,
            bar_flex: 0,
            bar_horizontal: 1,
            bar_selected_only: 0,
            rail: 0,
            rail_expr: 0,
            rail_toggle: 0,
            rail_expanded: false,
            drawer: 0,
            drawer_sections: 1,
            appbar_scroll: 0.0,
            appbar_query: String::new(),
            toolbar_bold: true,
            toolbar_italic: false,
            toolbar_expanded: true,
            menu_selected: 1,
            menu_checked: true,
            last_action: "none".to_owned(),
            zoom: 100,
            page: 2,
            page_wide: 9,
            crumbs: 0,
            steps_h: Steps::new(4).optional_step(2),
            steps_v: Steps::new(3),
            steps_free: Steps::new(4).non_linear(true).disable_step(3),
            bar_query: String::new(),
            back_query: String::new(),
            view_query: String::new(),
            view_open: false,
            view_docked: true,
            suggest_query: String::new(),
            suggest_chosen: None,
            suggestions: WORDS.iter().map(|s| (*s).to_owned()).collect(),
        }
    }
}

// Pagination rows of touch targets are wider than compact windows.
fn scroll_x(ui: &mut Mui<'_>, id_salt: &str, add: impl FnOnce(&mut Mui<'_>)) {
    let rt = ui.rt();
    egui::ScrollArea::horizontal()
        .id_salt(id_salt)
        .show(ui, |ui| add(&mut rt.ui(ui)));
}

fn boxed(ui: &mut Mui<'_>, size: egui::Vec2, add: impl FnOnce(&mut Mui<'_>)) {
    let rt = ui.rt();
    let t = rt.tokens();
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter().rect_filled(rect, 12, t.color.surface_container_lowest);
    let mut child = ui.new_child(UiBuilder::new().max_rect(rect));
    child.set_clip_rect(rect.intersect(ui.clip_rect()));
    add(&mut rt.ui(&mut child));
    ui.painter().rect_stroke(
        rect,
        12,
        egui::Stroke::new(1.0, t.color.outline_variant),
        egui::StrokeKind::Inside,
    );
}

pub(crate) fn show(ui: &mut Mui<'_>, s: &mut State, shared: &mut Shared) {
    tabs(ui, s);
    bars(ui, s);
    rail_drawer(ui, s, shared);
    app_bars(ui, s, shared);
    toolbars(ui, s, shared);
    menus(ui, s, shared);
    paging(ui, s);
    stepper(ui, s);
    search(ui, s, shared);
    status_bar(ui, s);
}

fn status_bar(ui: &mut Mui<'_>, s: &mut State) {
    super::section(ui, TOPICS[14], Some(StatusBar::META), |ui| {
        let r = StatusBar::new("nav-status", "Editor status")
            .item(StatusItem::new("main").icon(ICON_CALL_SPLIT).clickable(true))
            .item(StatusItem::new("Indexing").progress(None).tooltip("Background task"))
            .trailing(StatusItem::new(format!("Zoom {}%", s.zoom)).clickable(true))
            .item(
                StatusItem::new("Build failed: a long message that elides when the window narrows")
                    .elide(true)
                    .tone(StatusTone::Error),
            )
            .trailing(StatusItem::new("UTF-8"))
            .show(ui);
        match r.clicked {
            Some(0) => "Status: branch".clone_into(&mut s.last_action),
            Some(2) => s.zoom = 100,
            _ => {}
        }
    });
}

fn tabs(ui: &mut Mui<'_>, s: &mut State) {
    super::section(ui, TOPICS[0], Some(<Tabs<'static>>::META), |ui| {
        caption(ui, TypeScale::LabelMedium, "Primary, text only");
        Tabs::new("nav-tabs-primary", "Primary tabs", &mut s.tab_primary)
            .item(Tab::new("Flights"))
            .item(Tab::new("Trips"))
            .item(Tab::new("Explore"))
            .show(ui);
        caption(ui, TypeScale::LabelMedium, "Primary with icons");
        Tabs::new(
            "nav-tabs-primary-icons",
            "Primary tabs with icons",
            &mut s.tab_primary_icons,
        )
        .item(Tab::new("Flights").icon(ICON_SEND))
        .item(Tab::new("Trips").icon(ICON_BOOKMARK))
        .item(Tab::new("Explore").icon(ICON_EXPLORE))
        .show(ui);
        caption(ui, TypeScale::LabelMedium, "Secondary");
        Tabs::new("nav-tabs-secondary", "Secondary tabs", &mut s.tab_secondary)
            .variant(TabsVariant::Secondary)
            .item(Tab::new("Video"))
            .item(Tab::new("Photos"))
            .item(Tab::new("Audio"))
            .show(ui);
        caption(ui, TypeScale::LabelMedium, "Secondary with icons");
        Tabs::new(
            "nav-tabs-secondary-icons",
            "Secondary tabs with icons",
            &mut s.tab_secondary_icons,
        )
        .variant(TabsVariant::Secondary)
        .item(Tab::new("Video").icon(ICON_MAIL))
        .item(Tab::new("Photos").icon(ICON_STAR))
        .item(Tab::new("Audio").icon(ICON_MIC))
        .show(ui);
        caption(ui, TypeScale::LabelMedium, "Scrollable");
        let mut t = Tabs::new("nav-tabs-scroll", "Scrollable tabs", &mut s.tab_scroll).scrollable(true);
        for name in [
            "Overview",
            "Specifications",
            "Reviews",
            "Accessories",
            "Warranty",
            "Shipping",
            "Returns",
            "Support",
            "Community",
            "Downloads",
        ] {
            t = t.item(Tab::new(name));
        }
        t.show(ui);
    });
}

fn bars(ui: &mut Mui<'_>, s: &mut State) {
    super::section(ui, TOPICS[1], Some(<NavigationBar<'static>>::META), |ui| {
        caption(ui, TypeScale::LabelMedium, "Baseline");
        NavigationBar::new("nav-bar", &mut s.bar)
            .item(NavDestination::new(ICON_HOME, "Home"))
            .item(NavDestination::new(ICON_SEARCH, "Search"))
            .item(NavDestination::new(ICON_FAVORITE, "Saved"))
            .item(NavDestination::new(ICON_PERSON, "Profile"))
            .show(ui);
        caption(ui, TypeScale::LabelMedium, "Badges (dot, count)");
        NavigationBar::new("nav-bar-badges", &mut s.bar_badges)
            .item(NavDestination::new(ICON_HOME, "Home"))
            .item(NavDestination::new(ICON_MAIL, "Mail").badge(Badge::count(7)))
            .item(NavDestination::new(ICON_INBOX, "Inbox").badge(Badge::dot()))
            .show(ui);
        caption(ui, TypeScale::LabelMedium, "Flexible (Expressive), vertical items");
        NavigationBar::new("nav-bar-flex", &mut s.bar_flex)
            .flexible(true)
            .item_axis(Axis::Vertical)
            .item(NavDestination::new(ICON_HOME, "Home"))
            .item(NavDestination::new(ICON_SEARCH, "Search"))
            .item(NavDestination::new(ICON_SETTINGS, "Settings"))
            .show(ui);
        caption(ui, TypeScale::LabelMedium, "Flexible, horizontal items");
        NavigationBar::new("nav-bar-flex-h", &mut s.bar_horizontal)
            .flexible(true)
            .item_axis(Axis::Horizontal)
            .item(NavDestination::new(ICON_HOME, "Home"))
            .item(NavDestination::new(ICON_SEARCH, "Search"))
            .item(NavDestination::new(ICON_SETTINGS, "Settings"))
            .show(ui);
        caption(ui, TypeScale::LabelMedium, "Labels: selected only");
        NavigationBar::new("nav-bar-sel", &mut s.bar_selected_only)
            .labels(NavLabels::SelectedOnly)
            .item(NavDestination::new(ICON_HOME, "Home"))
            .item(NavDestination::new(ICON_SEARCH, "Search"))
            .item(NavDestination::new(ICON_FAVORITE, "Saved"))
            .show(ui);
    });
}

fn rail_drawer(ui: &mut Mui<'_>, s: &mut State, shared: &mut Shared) {
    let rt = ui.rt();
    super::section(ui, TOPICS[2], Some(<NavigationRail<'static>>::META), |ui| {
        super::top_row(ui, |row| {
            row.add(|ui| {
                caption(&mut rt.ui(ui), TypeScale::LabelMedium, "Baseline: menu, FAB, badges");
                boxed(&mut rt.ui(ui), vec2(120.0, 420.0), |ui| {
                    let r = NavigationRail::new("nav-rail", &mut s.rail)
                        .menu_button(true)
                        .fab(|ui| {
                            if ui.mineral(Fab::new("nav-rail-fab", ICON_EDIT, "Compose")).clicked() {
                                shared.toast("Compose");
                            }
                        })
                        .item(NavDestination::new(ICON_HOME, "Home"))
                        .item(NavDestination::new(ICON_MAIL, "Mail").badge(Badge::count(3)))
                        .item(NavDestination::new(ICON_STAR, "Starred").badge(Badge::dot()))
                        .item(NavDestination::new(ICON_SETTINGS, "Settings"))
                        .show(ui);
                    if r.leading_clicked {
                        shared.toast("Rail menu");
                    }
                });
            });
            row.add(|ui| {
                caption(&mut rt.ui(ui), TypeScale::LabelMedium, "Expressive collapsed");
                boxed(&mut rt.ui(ui), vec2(120.0, 420.0), |ui| {
                    NavigationRail::new("nav-rail-expr", &mut s.rail_expr)
                        .expanded(false)
                        .item(NavDestination::new(ICON_HOME, "Home"))
                        .item(NavDestination::new(ICON_MAIL, "Mail"))
                        .item(NavDestination::new(ICON_STAR, "Starred"))
                        .show(ui);
                });
            });
            row.add(|ui| {
                caption(
                    &mut rt.ui(ui),
                    TypeScale::LabelMedium,
                    "Expressive, toggled by the menu button",
                );
                boxed(&mut rt.ui(ui), vec2(340.0, 420.0), |ui| {
                    let expanded = s.rail_expanded;
                    let r = NavigationRail::new("nav-rail-toggle", &mut s.rail_toggle)
                        .expanded(expanded)
                        .menu_button(true)
                        .labels(NavLabels::SelectedOnly)
                        .item(NavDestination::new(ICON_HOME, "Home"))
                        .item(NavDestination::new(ICON_MAIL, "Mail"))
                        .item(NavDestination::new(ICON_STAR, "Starred"))
                        .show(ui);
                    if r.leading_clicked {
                        s.rail_expanded = !s.rail_expanded;
                    }
                });
            });
        });
    });
    super::section(ui, TOPICS[3], Some(NavigationDrawer::META), |ui| {
        super::top_row(ui, |row| {
            row.add(|ui| {
                caption(&mut rt.ui(ui), TypeScale::LabelMedium, "Standard");
                boxed(&mut rt.ui(ui), vec2(300.0, 300.0), |ui| {
                    NavigationDrawer::new("nav-drawer", &mut s.drawer)
                        .item(NavDestination::new(ICON_INBOX, "Inbox").badge(Badge::count(24)))
                        .item(NavDestination::new(ICON_SEND, "Sent"))
                        .item(NavDestination::new(ICON_DELETE, "Trash"))
                        .show(ui);
                });
            });
            row.add(|ui| {
                caption(&mut rt.ui(ui), TypeScale::LabelMedium, "Sections and dividers");
                boxed(&mut rt.ui(ui), vec2(300.0, 380.0), |ui| {
                    NavigationDrawer::new("nav-drawer-sections", &mut s.drawer_sections)
                        .section("Mail")
                        .item(NavDestination::new(ICON_INBOX, "Inbox").badge(Badge::count(24)))
                        .item(NavDestination::new(ICON_SEND, "Sent"))
                        .divider()
                        .section("Labels")
                        .item(NavDestination::new(ICON_BOOKMARK, "Work"))
                        .item(NavDestination::new(ICON_FAVORITE, "Family").badge(Badge::count(100).max(99)))
                        .show(ui);
                });
            });
        });
        caption(
            ui,
            TypeScale::LabelMedium,
            "The modal drawer is the shell's menu button (top bar).",
        );
    });
}

fn report_bar(r: &TopAppBarResponse, shared: &mut Shared) {
    if r.leading_clicked {
        shared.toast("Navigation icon");
    }
    if let Some(a) = r.clicked {
        shared.toast(format!("Action {a}"));
    }
}

fn app_bar_overflow(ui: &mut Mui<'_>, shared: &mut Shared) {
    let rt = ui.rt();
    caption(
        ui,
        TypeScale::LabelMedium,
        "Overflow: at most 3 inline actions on compact windows, 5 elsewhere; the rest move to the \
         More menu with their index and agent key. Share anchors a menu to its own button.",
    );
    let r = TopAppBar::new("nav-appbar-overflow", "Document")
        .navigation(ICON_ARROW_BACK, "Back")
        .actions([
            IconAction::new(ICON_SEARCH, "Search"),
            IconAction::new(ICON_SHARE, "Share"),
            IconAction::new(ICON_EDIT, "Edit").disabled(true),
            IconAction::new(ICON_BOOKMARK, "Bookmark"),
            IconAction::new(ICON_DELETE, "Delete"),
            IconAction::new(ICON_SETTINGS, "Settings"),
            IconAction::new(ICON_CONTENT_COPY, "Copy").command(&COMMANDS[COPY].0),
        ])
        .show(ui);
    if let Some(share) = r.actions.get(&1) {
        let menu = Menu::new("nav-appbar-share-menu", "Share")
            .item(MenuItem::new("Mail").leading(ICON_MAIL))
            .item(MenuItem::new("Copy link").leading(ICON_CONTENT_COPY));
        if let Some(i) = menu.show(rt, share).clicked {
            shared.toast(format!("Share via item {i}"));
        }
    }
    if r.clicked != Some(1) {
        report_bar(&r, shared);
    }
}

fn app_bars(ui: &mut Mui<'_>, s: &mut State, shared: &mut Shared) {
    super::section(ui, TOPICS[4], Some(<TopAppBar<'static>>::META), |ui| {
        super::row(ui, |row| {
            row.mineral(
                ValueSlider::new(
                    "nav-appbar-scroll",
                    "Scroll offset (medium/large collapse)",
                    0.0..=120.0,
                    &mut s.appbar_scroll,
                )
                .preferred_width(320.0),
            );
        });
        let variants = [
            ("Small", TopAppBarVariant::Small, false),
            ("Center aligned", TopAppBarVariant::CenterAligned, false),
            ("Medium", TopAppBarVariant::Medium, false),
            ("Large", TopAppBarVariant::Large, false),
            ("Medium, flexible", TopAppBarVariant::Medium, true),
            ("Large, flexible", TopAppBarVariant::Large, true),
        ];
        for (i, (name, variant, flexible)) in variants.into_iter().enumerate() {
            caption(ui, TypeScale::LabelMedium, name);
            let r = TopAppBar::new(("nav-appbar", i), name)
                .variant(variant)
                .flexible(flexible)
                .scroll_offset(s.appbar_scroll as f32)
                .navigation(ICON_MENU, "Menu")
                .action(IconAction::new(ICON_SEARCH, "Search"))
                .action(IconAction::new(ICON_STAR, "Favorite"))
                .show(ui);
            report_bar(&r, shared);
        }
        app_bar_overflow(ui, shared);
        caption(ui, TypeScale::LabelMedium, "Centered title with subtitle");
        let r = TopAppBar::new("nav-appbar-sub", "Inbox")
            .centered(true)
            .supporting("24 unread")
            .navigation(ICON_ARROW_BACK, "Back")
            .action(IconAction::new(ICON_SHARE, "Share"))
            .show(ui);
        report_bar(&r, shared);
        caption(ui, TypeScale::LabelMedium, "Medium with subtitle");
        let r = TopAppBar::new("nav-appbar-med-sub", "Trip to Lisbon")
            .variant(TopAppBarVariant::Medium)
            .supporting("5 days")
            .flexible(true)
            .navigation(ICON_ARROW_BACK, "Back")
            .show(ui);
        report_bar(&r, shared);
        caption(ui, TypeScale::LabelMedium, "Scrolled (container shifts)");
        let r = TopAppBar::new("nav-appbar-scrolled", "Scrolled")
            .scrolled(true)
            .navigation(ICON_MENU, "Menu")
            .show(ui);
        report_bar(&r, shared);
        caption(ui, TypeScale::LabelMedium, "Search app bar");
        let r = TopAppBar::new("nav-appbar-search", "Search contacts")
            .navigation(ICON_ARROW_BACK, "Back")
            .search(&mut s.appbar_query)
            .show(ui);
        if r.submitted {
            shared.toast(format!("Search: {}", s.appbar_query));
        }
    });
}

fn report_action(r: &ToolbarResponse, shared: &mut Shared) {
    if let Some(i) = r.clicked {
        shared.toast(format!("Toolbar action {i}"));
    }
}

fn toolbars(ui: &mut Mui<'_>, s: &mut State, shared: &mut Shared) {
    let rt = ui.rt();
    super::section(ui, TOPICS[5], Some(Toolbar::META), |ui| {
        caption(ui, TypeScale::LabelMedium, "Docked, toggle actions");
        let r = Toolbar::docked("nav-toolbar-docked", "Formatting")
            .action(IconAction::new(ICON_FORMAT_BOLD, "Bold").selected(s.toolbar_bold))
            .action(IconAction::new(ICON_FORMAT_ITALIC, "Italic").selected(s.toolbar_italic))
            .action(IconAction::new(ICON_FORMAT_UNDERLINED, "Underline").disabled(true))
            .action(IconAction::new(ICON_CONTENT_CUT, "Cut"))
            .show(ui);
        match r.clicked {
            Some(0) => s.toolbar_bold = !s.toolbar_bold,
            Some(1) => s.toolbar_italic = !s.toolbar_italic,
            _ => report_action(&r, shared),
        }
        ui.add_space(8.0);
        super::top_row(ui, |row| {
            row.add(|ui| {
                caption(&mut rt.ui(ui), TypeScale::LabelMedium, "Floating");
                let r = Toolbar::floating("nav-toolbar-float", "Edit")
                    .actions([
                        IconAction::new(ICON_EDIT, "Edit"),
                        IconAction::new(ICON_SHARE, "Share"),
                        IconAction::new(ICON_DELETE, "Delete"),
                    ])
                    .show(&mut rt.ui(ui));
                report_action(&r, shared);
            });
            row.add(|ui| {
                caption(&mut rt.ui(ui), TypeScale::LabelMedium, "Floating, vibrant");
                let r = Toolbar::floating("nav-toolbar-vibrant", "Edit")
                    .colors(ToolbarColors::Vibrant)
                    .actions([
                        IconAction::new(ICON_EDIT, "Edit"),
                        IconAction::new(ICON_SHARE, "Share"),
                        IconAction::new(ICON_DELETE, "Delete"),
                    ])
                    .show(&mut rt.ui(ui));
                report_action(&r, shared);
            });
            row.add(|ui| {
                caption(&mut rt.ui(ui), TypeScale::LabelMedium, "Floating, vertical");
                let r = Toolbar::floating("nav-toolbar-vert", "Edit")
                    .axis(Axis::Vertical)
                    .actions([
                        IconAction::new(ICON_EDIT, "Edit"),
                        IconAction::new(ICON_SHARE, "Share"),
                        IconAction::new(ICON_DELETE, "Delete"),
                    ])
                    .show(&mut rt.ui(ui));
                report_action(&r, shared);
            });
        });
        ui.add_space(8.0);
        caption(
            ui,
            TypeScale::LabelMedium,
            "Floating with a FAB; collapsed shows only the FAB",
        );
        rt.ui(ui).mineral(CheckButton::new(
            "nav-toolbar-expanded",
            "Expanded",
            &mut s.toolbar_expanded,
        ));
        let r = Toolbar::floating("nav-toolbar-fab", "Edit")
            .colors(ToolbarColors::Vibrant)
            .actions([
                IconAction::new(ICON_EDIT, "Edit"),
                IconAction::new(ICON_SHARE, "Share"),
                IconAction::new(ICON_DELETE, "Delete"),
            ])
            .fab(ICON_ADD, "Add", Edge::End)
            .expanded(s.toolbar_expanded)
            .show(ui);
        report_action(&r, shared);
        if r.fab_clicked {
            s.toolbar_expanded = !s.toolbar_expanded;
        }
    });
}

pub(crate) const COMMANDS: [(ActionId, KeyboardShortcut, &str); 8] = [
    (
        ActionId::from_static("file.new"),
        KeyboardShortcut::new(Modifiers::COMMAND, Key::N),
        "New",
    ),
    (
        ActionId::from_static("file.open"),
        KeyboardShortcut::new(Modifiers::COMMAND, Key::O),
        "Open",
    ),
    (
        ActionId::from_static("file.quit"),
        KeyboardShortcut::new(Modifiers::COMMAND, Key::Q),
        "Quit",
    ),
    (
        ActionId::from_static("view.zoom-in"),
        KeyboardShortcut::new(Modifiers::COMMAND, Key::Plus),
        "Zoom in",
    ),
    (
        ActionId::from_static("view.zoom-out"),
        KeyboardShortcut::new(Modifiers::COMMAND, Key::Minus),
        "Zoom out",
    ),
    (
        ActionId::from_static("edit.cut"),
        KeyboardShortcut::new(Modifiers::COMMAND, Key::X),
        "Cut",
    ),
    (
        ActionId::from_static("edit.copy"),
        KeyboardShortcut::new(Modifiers::COMMAND, Key::C),
        "Copy",
    ),
    (
        ActionId::from_static("edit.paste"),
        KeyboardShortcut::new(Modifiers::COMMAND, Key::V),
        "Paste",
    ),
];
const NEW: usize = 0;
const OPEN: usize = 1;
const QUIT: usize = 2;
pub(crate) const ZOOM_IN: usize = 3;
pub(crate) const ZOOM_OUT: usize = 4;
const CUT: usize = 5;
const COPY: usize = 6;
const PASTE: usize = 7;

fn command_item(i: usize) -> MenuItem {
    let (id, key, label) = &COMMANDS[i];
    MenuItem::new(*label).command(id).trailing(*key)
}

fn edit_menu(id: impl Into<KeyedSalt>, s: &State) -> Menu {
    Menu::new(id, "Edit")
        .mnemonic('e')
        .item(command_item(CUT).leading(ICON_CONTENT_CUT).mnemonic('t'))
        .item(command_item(COPY).leading(ICON_CONTENT_COPY).mnemonic('c'))
        .item(command_item(PASTE).leading(ICON_CONTENT_PASTE).mnemonic('p'))
        .divider()
        .item(MenuItem::new("Checked").mnemonic('k').selected(s.menu_checked))
        .item(MenuItem::new("Share").trailing(ICON_SHARE).mnemonic('s').submenu(|m| {
            m.item(MenuItem::new("Mail").leading(ICON_MAIL).mnemonic('m')).item(
                MenuItem::new("Copy link")
                    .leading(ICON_CONTENT_COPY)
                    .mnemonic('l')
                    .disabled(true),
            )
        }))
}

const EDIT_ITEMS: [&str; 7] = [
    "Cut",
    "Copy",
    "Paste",
    "Checked",
    "Share",
    "Share > Mail",
    "Share > Copy link",
];
const FILE_ITEMS: [&str; 6] = ["New", "Open", "Recent", "report.md", "notes.txt", "Quit"];
const SWATCHES: [(&str, [u8; 3]); 3] = [
    ("Red", [0xd3, 0x2f, 0x2f]),
    ("Green", [0x38, 0x8e, 0x3c]),
    ("Blue", [0x19, 0x76, 0xd2]),
];

fn edit_picked(s: &mut State, origin: &str, item: usize) {
    if item == 3 {
        s.menu_checked = !s.menu_checked;
    }
    s.last_action = format!("{origin}: {}", EDIT_ITEMS.get(item).unwrap_or(&"?"));
}

pub(crate) fn run_commands(rt: &Rt, s: &mut State, shared: &mut Shared) {
    let ctx = rt.ctx();
    for (i, (id, _, label)) in COMMANDS.iter().enumerate() {
        if !shortcut::take_action(ctx, id) {
            continue;
        }
        match i {
            ZOOM_IN => {
                s.zoom = (s.zoom + 10).min(300);
                egui::gui_zoom::zoom_in(ctx);
            }
            ZOOM_OUT => {
                s.zoom = s.zoom.saturating_sub(10).max(10);
                egui::gui_zoom::zoom_out(ctx);
            }
            QUIT => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            PASTE => {
                if let Some(text) = shortcut::take_paste(ctx, id) {
                    s.last_action = format!("Command: {label} ({} chars from the paste event)", text.chars().count());
                    shared.toast(s.last_action.clone());
                    continue;
                }
            }
            _ => {}
        }
        s.last_action = format!("Command: {label}");
        shared.toast(s.last_action.clone());
    }
}

fn menubar_picked(s: &mut State, shared: &mut Shared, menu: usize, item: usize) {
    s.last_action = match menu {
        1 => {
            edit_picked(s, "Menu bar", item);
            s.last_action.clone()
        }
        _ => format!("Menu bar: {}", FILE_ITEMS.get(item).unwrap_or(&"?")),
    };
    shared.toast(s.last_action.clone());
}

fn menus(ui: &mut Mui<'_>, s: &mut State, shared: &mut Shared) {
    let rt = ui.rt();
    super::section(ui, TOPICS[6], Some(Menu::META), |ui| {
        super::row(ui, |row| {
            for (i, (label, style)) in [
                ("Baseline", MenuColors::Baseline),
                ("Standard", MenuColors::Standard),
                ("Vibrant", MenuColors::Vibrant),
            ]
            .into_iter()
            .enumerate()
            {
                let anchor = row
                    .mineral(PushButton::new(("nav-menu-anchor", i), label).variant(ButtonVariant::Tonal))
                    .into_response();
                if let Some(item) = edit_menu(("nav-menu", i), s).colors(style).show(rt, &anchor).clicked {
                    edit_picked(s, label, item);
                }
            }
            let anchor = row
                .mineral(PushButton::new("nav-menu-group-anchor", "Groups").variant(ButtonVariant::Outlined))
                .into_response();
            let mut grouped = Menu::new("nav-menu-groups", "Groups")
                .colors(MenuColors::Vibrant)
                .group_label("Sort by")
                .item(MenuItem::new("Name").selected(s.menu_selected == 0))
                .item(MenuItem::new("Date").selected(s.menu_selected == 1))
                .group_label("View")
                .item(MenuItem::new("List"))
                .item(MenuItem::new("Grid"))
                .group_label("Accent");
            for (name, [r, g, b]) in SWATCHES {
                let swatch = LeadingPaint::new(move |p, slot, _| {
                    p.circle_filled(
                        slot.center(),
                        slot.width() / 2.0 - 2.0,
                        egui::Color32::from_rgb(r, g, b),
                    );
                });
                grouped = grouped.item(MenuItem::new(name).leading(swatch));
            }
            if let Some(item) = grouped.show(rt, &anchor).clicked {
                if item < 2 {
                    s.menu_selected = item;
                }
                s.last_action = format!("Groups item {item}");
            }
        });
        ui.add_space(8.0);
        let width = ui.available_width().min(480.0);
        let (rect, target) = ui.allocate_exact_size(vec2(width, 80.0), Sense::click());
        let t = rt.tokens();
        ui.painter().rect_filled(rect, 12, t.color.surface_container);
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Right-click or long-press for a context menu",
            egui_mineral_material::spec::TypeScale::BodyMedium.font(ui.ctx()),
            t.color.on_surface,
        );
        if let Some(item) = edit_menu("nav-menu-context", s).show_context(rt, &target).clicked {
            edit_picked(s, "Context", item);
        }
        ui.label(format!("Last activated: {}", s.last_action));
    });
    super::section(ui, TOPICS[7], Some(MenuStrip::META), |ui| {
        let bar = MenuStrip::new("nav-menubar")
            .menu(
                Menu::new("nav-mb-file", "File")
                    .mnemonic('f')
                    .item(command_item(NEW).mnemonic('n'))
                    .item(command_item(OPEN).mnemonic('o'))
                    .item(
                        MenuItem::new("Recent")
                            .mnemonic('r')
                            .submenu(|m| m.item(MenuItem::new("report.md")).item(MenuItem::new("notes.txt"))),
                    )
                    .divider()
                    .item(command_item(QUIT).mnemonic('q')),
            )
            .menu(edit_menu("nav-mb-edit", s))
            .menu(
                Menu::new("nav-mb-view", "View")
                    .mnemonic('v')
                    .item(command_item(ZOOM_IN).mnemonic('i'))
                    .item(command_item(ZOOM_OUT).mnemonic('o'))
                    .item(MenuItem::new("Full screen").disabled(true)),
            );
        if let Some(MenuPath { menu, item }) = bar.show(ui).clicked {
            menubar_picked(s, shared, menu, item);
        }
        ui.label(format!("Zoom: {}%", s.zoom));
    });
}

fn paging(ui: &mut Mui<'_>, s: &mut State) {
    super::section(ui, TOPICS[8], Some(Breadcrumbs::META), |ui| {
        let path = ["Home", "Library", "Data", "2026", "Reports", "Q3", "Summary"];
        caption(ui, TypeScale::LabelMedium, "Full path");
        let mut b = Breadcrumbs::new("nav-crumbs");
        for p in path {
            b = b.item(p);
        }
        if let Some(i) = b.show(ui).clicked {
            s.crumbs = i;
        }
        caption(ui, TypeScale::LabelMedium, "Collapsed with max_items = 4");
        let mut b = Breadcrumbs::new("nav-crumbs-max").max_items(4);
        for p in path {
            b = b.item(p);
        }
        if let Some(i) = b.show(ui).clicked {
            s.crumbs = i;
        }
        ui.label(format!("Last crumb: {}", path.get(s.crumbs).unwrap_or(&"?")));
    });
    super::section(ui, TOPICS[9], Some(<Pagination<'static>>::META), |ui| {
        caption(ui, TypeScale::LabelMedium, "Default");
        scroll_x(ui, "nav-page-scroll", |ui| {
            ui.mineral(Pagination::new("nav-page", 12, &mut s.page));
        });
        caption(ui, TypeScale::LabelMedium, "First/last, 2 siblings, 2 boundaries");
        scroll_x(ui, "nav-page-wide-scroll", |ui| {
            ui.mineral(
                Pagination::new("nav-page-wide", 50, &mut s.page_wide)
                    .first_last(true)
                    .siblings(2)
                    .boundaries(2),
            );
        });
        ui.label(format!("Pages: {} and {}", s.page + 1, s.page_wide + 1));
    });
}

fn step_buttons(ui: &mut Mui<'_>, steps: &mut Steps, id: &'static str) {
    super::row(ui, |row| {
        if row
            .mineral(PushButton::new((id, "back"), "Back").variant(ButtonVariant::Text))
            .clicked()
        {
            steps.back();
        }
        if row.mineral(PushButton::new((id, "next"), "Next")).clicked() {
            steps.forward();
        }
        if row
            .mineral(
                PushButton::new((id, "skip"), "Skip")
                    .variant(ButtonVariant::Tonal)
                    .disabled(!steps.is_optional(steps.current())),
            )
            .clicked()
        {
            steps.skip();
        }
        if row
            .mineral(PushButton::new((id, "reset"), "Reset").variant(ButtonVariant::Outlined))
            .clicked()
        {
            steps.reset();
        }
    });
}

fn stepper(ui: &mut Mui<'_>, s: &mut State) {
    let rt = ui.rt();
    super::section(ui, TOPICS[10], Some(<Stepper<'static>>::META), |ui| {
        caption(ui, TypeScale::LabelMedium, "Horizontal, linear, step 3 optional");
        Stepper::new("nav-step-h", &mut s.steps_h)
            .step("Account")
            .step(StepLabel::new("Profile").supporting("Name and photo"))
            .step("Preferences")
            .step("Review")
            .show(ui, |ui, i| {
                ui.label(format!("Content of step {}", i + 1));
            });
        step_buttons(ui, &mut s.steps_h, "nav-step-h-btn");
        let mut err = s.steps_h.status(1) == egui_mineral::steps::StepStatus::Error;
        if rt
            .ui(ui)
            .mineral(CheckButton::new("nav-step-error", "Profile step in error", &mut err))
            .changed()
        {
            s.steps_h.set_error(1, err);
        }
        ui.add_space(8.0);
        caption(ui, TypeScale::LabelMedium, "Vertical");
        Stepper::new("nav-step-v", &mut s.steps_v)
            .axis(Axis::Vertical)
            .step(StepLabel::new("Select campaign").supporting("Choose a target"))
            .step("Create ad group")
            .step("Create ad")
            .show(ui, |ui, i| {
                ui.label(format!("Vertical content {}", i + 1));
            });
        step_buttons(ui, &mut s.steps_v, "nav-step-v-btn");
        ui.add_space(8.0);
        caption(ui, TypeScale::LabelMedium, "Non-linear, last step disabled");
        Stepper::new("nav-step-free", &mut s.steps_free)
            .step("One")
            .step("Two")
            .step("Three")
            .step("Four")
            .show(ui, |ui, i| {
                ui.label(format!("Free content {}", i + 1));
            });
    });
}

fn search(ui: &mut Mui<'_>, s: &mut State, shared: &mut Shared) {
    let rt = ui.rt();
    super::section(ui, TOPICS[11], Some(<SearchBar<'static>>::META), |ui| {
        caption(ui, TypeScale::LabelMedium, "Default with trailing action");
        let bar = SearchBar::new("nav-sbar", &mut s.bar_query)
            .placeholder("Search the catalog")
            .trailing_icon(ICON_MIC, "Voice search");
        let out = rt.ui(ui).mineral(bar).into_output();
        if out.trailing_clicked {
            shared.toast("Voice search");
        }
        if out.submitted {
            shared.toast("Search submitted");
        }
        ui.add_space(8.0);
        caption(ui, TypeScale::LabelMedium, "Back button, narrow");
        let bar = SearchBar::new("nav-sbar-back", &mut s.back_query)
            .back_button(true)
            .preferred_width(360.0);
        if rt.ui(ui).mineral(bar).output().leading_clicked {
            s.back_query.clear();
        }
        ui.add_space(8.0);
        caption(ui, TypeScale::LabelMedium, "Disabled");
        let mut empty = String::new();
        rt.ui(ui)
            .mineral(SearchBar::new("nav-sbar-off", &mut empty).disabled(true));
    });
    super::section(ui, TOPICS[12], Some(<SearchView<'static>>::META), |ui| {
        rt.ui(ui).mineral(CheckButton::new(
            "nav-view-docked",
            "Docked (off: full screen covers the window)",
            &mut s.view_docked,
        ));
        let mode = if s.view_docked {
            SearchViewMode::Docked
        } else {
            SearchViewMode::FullScreen
        };
        let width = ui.available_width().min(720.0);
        boxed(ui, vec2(width, 420.0), |ui| {
            let r = SearchView::new("nav-sview", &mut s.view_query)
                .suggestions(&s.suggestions)
                .mode(mode)
                .max_items(5)
                .show(ui, &mut s.view_open, |ui, query| {
                    ui.label(format!("Results for \"{query}\""));
                });
            if let Some(i) = r.selected {
                shared.toast(format!("Chose {}", s.suggestions[i]));
            }
            if r.submitted {
                shared.toast("View submitted");
            }
        });
    });
    super::section(ui, TOPICS[13], Some(<SearchSuggestions<'static>>::META), |ui| {
        caption(ui, TypeScale::LabelMedium, "SearchSuggestions");
        if let Some(i) = rt
            .ui(ui)
            .mineral(SearchSuggestions::new(
                "nav-suggest",
                &s.suggestions,
                &mut s.suggest_query,
            ))
            .into_output()
            .selected
        {
            s.suggest_chosen = Some(i);
        }
        ui.add_space(260.0);
        if let Some(i) = s.suggest_chosen {
            ui.label(format!("Chosen: {}", s.suggestions[i]));
        }
    });
}
