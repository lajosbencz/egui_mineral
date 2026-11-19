use std::collections::BTreeSet;

use egui_mineral_fluent::{
    ColorToken, FluentExt, Size,
    actions::PushButton,
    data::{Avatar, AvatarColor},
    egui_mineral::{expand::Expansion, focus::Cell},
    icons::{
        ICON_COPY, ICON_CUT, ICON_DOCUMENT, ICON_FOLDER, ICON_HOME, ICON_MORE_HORIZONTAL, ICON_PERSON, ICON_PIN,
        ICON_SETTINGS, ICON_SHARE,
    },
    navigation::{
        Breadcrumb, BreadcrumbButton, BreadcrumbFocus, Carousel, CarouselAppearance, Menu, MenuGrid, MenuGridAction,
        MenuGridItem, MenuGridRow, MenuItem, Nav, NavCategory, NavDensity, NavDivider, NavItem, NavSubItem, Overflow,
        OverflowDirection, Tab, TabAppearance, TabList, Toolbar, ToolbarButton, ToolbarRadioButton, ToolbarRadioGroup,
        ToolbarToggleButton, Tree, TreeAppearance, TreeItem, TreeItemLayout, TreeItemPersonaLayout, TreeSelection,
    },
};

use super::heading;
use crate::Fui;

#[derive(Debug, Default)]
pub(crate) struct State {
    appearances: [usize; 4],
    sizes: [usize; 3],
    vertical: usize,
    menu_open: bool,
    menu_grid_open: bool,
    wrap: bool,
    size: u8,
    last: &'static str,
    bold: [bool; 4],
    align: usize,
    crumb: Option<usize>,
    overflow_last: Option<usize>,
    nav: Option<usize>,
    nav_small: Option<usize>,
    tree_open: [Expansion<String>; 3],
    tree_checked: [BTreeSet<String>; 3],
    tree_activated: Option<String>,
    grid_last: Option<Cell>,
    carousel: [usize; 2],
}

pub(crate) fn show(ui: &mut Fui<'_>, s: &mut State) {
    tabs(ui, s);
    toolbars(ui, s);
    breadcrumbs(ui, s);
    overflow(ui, s);
    navs(ui, s);
    menus(ui, s);
    menu_grids(ui, s);
    carousels(ui, s);
    trees(ui, s);
}

fn items() -> [Tab; 3] {
    [
        Tab::new("Home").icon(ICON_HOME),
        Tab::new("People").icon(ICON_PERSON),
        Tab::new("Settings").icon(ICON_SETTINGS).disabled(true),
    ]
}

fn tabs(ui: &mut Fui<'_>, s: &mut State) {
    heading(ui, "h-tabs", "Tab list");
    for ((i, appearance), selected) in TabAppearance::ALL.into_iter().enumerate().zip(&mut s.appearances) {
        TabList::new(("tabs-appearance", i), format!("{appearance:?}"), selected)
            .tabs(items())
            .appearance(appearance)
            .show(ui);
        ui.add_space(4.0);
    }
    for ((i, size), selected) in Size::ALL.into_iter().enumerate().zip(&mut s.sizes) {
        TabList::new(("tabs-size", i), format!("{size:?}"), selected)
            .tabs(["One", "Two", "Three"])
            .size(size)
            .show(ui);
    }
    ui.add_space(4.0);
    TabList::new("tabs-vertical", "Vertical", &mut s.vertical)
        .tabs(items())
        .vertical(true)
        .show(ui);
}

fn toolbars(ui: &mut Fui<'_>, s: &mut State) {
    heading(ui, "h-toolbar", "Toolbar");
    for (i, size) in Size::ALL.into_iter().enumerate() {
        Toolbar::new(("toolbar", i), format!("{size:?}"))
            .size(size)
            .action("Cut")
            .action(ToolbarButton::new("Paste").disabled(true))
            .toggle(ToolbarToggleButton::new("Bold", &mut s.bold[i]))
            .show(ui);
    }
    let icon = |label: &'static str, icon| ToolbarButton::new(label).icon(icon).label_visible(false);
    let radio = |label: &'static str, icon| ToolbarRadioButton::new(label).icon(icon).label_visible(false);
    Toolbar::new("toolbar-icons", "Icons")
        .action(icon("Cut", ICON_CUT))
        .action(icon("Settings", ICON_SETTINGS).disabled(true))
        .divider()
        .toggle(
            ToolbarToggleButton::new("Home", &mut s.bold[3])
                .icon(ICON_HOME)
                .label_visible(false),
        )
        .radio_group(
            ToolbarRadioGroup::new(&mut s.align)
                .option(radio("Person", ICON_PERSON))
                .option(radio("Home", ICON_HOME))
                .option(radio("Cut", ICON_CUT)),
        )
        .show(ui);
    Toolbar::new("toolbar-vertical", "Vertical")
        .vertical(true)
        .action(ToolbarButton::new("Home").icon(ICON_HOME))
        .divider()
        .action(ToolbarButton::new("People").icon(ICON_PERSON))
        .show(ui);
}

fn breadcrumbs(ui: &mut Fui<'_>, s: &mut State) {
    heading(ui, "h-breadcrumb", "Breadcrumb");
    for (i, size) in Size::ALL.into_iter().enumerate() {
        let r = Breadcrumb::new(("breadcrumb", i), format!("{size:?}"))
            .size(size)
            .item(BreadcrumbButton::new("Home").icon(ICON_HOME))
            .item("Docs")
            .item(BreadcrumbButton::new("Page").current(true))
            .show(ui);
        s.crumb = r.clicked.or(s.crumb);
    }
    Breadcrumb::new("breadcrumb-arrow", "Arrow keys")
        .focus(BreadcrumbFocus::Arrow)
        .item("Home")
        .item(BreadcrumbButton::new("Docs").disabled(true))
        .item(BreadcrumbButton::new("Page").current(true))
        .show(ui);
    ui.label(format!("Clicked: {:?}", s.crumb));
}

fn overflow(ui: &mut Fui<'_>, s: &mut State) {
    heading(ui, "h-overflow", "Overflow");
    let rt = ui.rt();
    ui.scope(|ui| {
        ui.set_max_width(280.0);
        let ui = &mut rt.ui(ui);
        let mut bar = Toolbar::new("toolbar-overflow", "Edit").overflow(Overflow::new("More items"));
        for label in ["Cut", "Copy", "Paste", "Undo", "Redo"] {
            bar = bar.action(label);
        }
        let r = bar.show(ui);
        s.overflow_last = r.activated.or(s.overflow_last);
        let r = Breadcrumb::new("breadcrumb-overflow", "Path")
            .overflow(Overflow::new("More items").direction(OverflowDirection::Start))
            .items(["Home", "Docs", "Guide", "Topic", "Page"])
            .show(ui);
        s.crumb = r.clicked.or(s.crumb);
    });
    ui.label(format!("Activated: {:?}", s.overflow_last));
}

fn navs(ui: &mut Fui<'_>, s: &mut State) {
    heading(ui, "h-nav", "Nav");
    let rt = ui.rt();
    let t = rt.tokens();
    for (i, density) in [NavDensity::Medium, NavDensity::Small].into_iter().enumerate() {
        let selected = if i == 0 { &mut s.nav } else { &mut s.nav_small };
        ui.scope(|ui| {
            ui.set_max_width(260.0);
            egui::Frame::new()
                .fill(t.color(ColorToken::ColorNeutralBackground4))
                .inner_margin(egui::Margin::symmetric(10, 4))
                .show(ui, |ui| {
                    let ui = &mut rt.ui(ui);
                    Nav::new(("nav", i), format!("{density:?}"), selected)
                        .density(density)
                        .default_open([10])
                        .show(ui, |nav| {
                            nav.section_header("Pages");
                            nav.item(NavItem::new(1, "Home").icon(ICON_HOME));
                            nav.item(NavItem::new(2, "People").icon(ICON_PERSON));
                            nav.divider(NavDivider::new("divider"));
                            nav.category(NavCategory::new(10, "Settings").icon(ICON_SETTINGS), |nav| {
                                nav.sub_item(NavSubItem::new(11, "Account"));
                                nav.sub_item(NavSubItem::new(12, "Privacy"));
                            });
                        });
                });
        });
        ui.add_space(4.0);
    }
}

fn menus(ui: &mut Fui<'_>, s: &mut State) {
    heading(ui, "h-menu", "Menu");
    let rt = ui.rt();
    let trigger = ui.mineral(PushButton::new("menu-trigger", "Edit")).response().clone();
    let State {
        menu_open,
        wrap,
        size,
        last,
        ..
    } = s;
    Menu::new("menu", "Edit").show(rt, &trigger, menu_open, |m| {
        if m.item(MenuItem::new("Cut").icon(ICON_CUT).secondary_content("Ctrl+X"))
            .clicked
        {
            *last = "Cut";
        }
        m.item(MenuItem::new("Paste").disabled(true));
        m.divider();
        m.checkbox_item("Word wrap", wrap);
        m.group("Size", |m| {
            m.radio_item("Small", size, 0);
            m.radio_item("Large", size, 1);
        });
        if m.split_item("Paste", |m| {
            if m.item("Paste as plain text").clicked {
                *last = "Paste as plain text";
            }
        })
        .clicked
        {
            *last = "Paste";
        }
        m.submenu("More", |m| {
            if m.item("Select all").clicked {
                *last = "Select all";
            }
        });
    });
    ui.label(format!("Last: {}", s.last));
}

fn tree_items(persona: bool) -> Vec<TreeItem> {
    let layout = |icon| TreeItemLayout::new().icon_before(icon);
    if persona {
        let person = |description: &str| {
            TreeItemPersonaLayout::new()
                .media(Avatar::new("persona", description).color(AvatarColor::Colorful))
                .description(description)
        };
        return vec![
            TreeItem::new("team", "Design team").layout(person("4 people")).nested([
                TreeItem::new("ada", "Ada").layout(person("Lead")),
                TreeItem::new("lin", "Lin").layout(person("Researcher")),
            ]),
            TreeItem::new("ops", "Operations").layout(person("2 people")),
        ];
    }
    vec![
        TreeItem::new("src", "src").layout(layout(ICON_FOLDER)).nested([
            TreeItem::new("lib", "lib.rs").layout(layout(ICON_DOCUMENT).aside("12 KB")),
            TreeItem::new("ui", "ui").layout(layout(ICON_FOLDER)).nested([
                TreeItem::new("tree", "tree.rs").layout(layout(ICON_DOCUMENT)),
                TreeItem::new("menu", "menu.rs").layout(layout(ICON_DOCUMENT)),
            ]),
        ]),
        TreeItem::new("readme", "README.md").layout(layout(ICON_DOCUMENT)),
    ]
}

fn trees(ui: &mut Fui<'_>, s: &mut State) {
    heading(ui, "h-tree", "Tree");
    let rt = ui.rt();
    let cases = [
        ("none", TreeSelection::None, TreeAppearance::Subtle, false),
        ("single", TreeSelection::Single, TreeAppearance::SubtleAlpha, false),
        (
            "multiselect",
            TreeSelection::Multiple,
            TreeAppearance::Transparent,
            true,
        ),
    ];
    let State {
        tree_open,
        tree_checked,
        tree_activated,
        ..
    } = s;
    for (i, ((name, mode, appearance, persona), (open, checked))) in cases
        .into_iter()
        .zip(tree_open.iter_mut().zip(tree_checked.iter_mut()))
        .enumerate()
    {
        let items = tree_items(persona);
        ui.scope(|ui| {
            ui.set_max_width(ui.available_width().min(360.0));
            let r = Tree::items(("tree", i), name, &items, open)
                .selection(mode, checked)
                .appearance(appearance)
                .show(&mut rt.ui(ui));
            *tree_activated = r.activated.or(tree_activated.take());
        });
        ui.add_space(4.0);
    }
    ui.label(format!("Activated: {tree_activated:?}"));
}

fn menu_grids(ui: &mut Fui<'_>, s: &mut State) {
    heading(ui, "h-menu-grid", "MenuGrid");
    let trigger = ui
        .mineral(PushButton::new("menu-grid-trigger", "Open grid"))
        .into_response();
    let r = MenuGrid::new("menu-grid", "Actions").show(ui.rt(), &trigger, &mut s.menu_grid_open, |g| {
        g.row(
            MenuGridRow::new()
                .cell(MenuGridItem::new("Cut").icon(ICON_CUT).sub_text("Ctrl+X"))
                .cell(MenuGridAction::new("More", ICON_MORE_HORIZONTAL)),
        );
        g.row(
            MenuGridRow::new()
                .cell(MenuGridItem::new("Copy").icon(ICON_COPY))
                .cell(MenuGridAction::new("Share", ICON_SHARE).disabled(true))
                .cell(MenuGridAction::new("Pin", ICON_PIN)),
        );
        g.row(MenuGridRow::new().cell(MenuGridItem::new("Paste").disabled(true)));
    });
    s.grid_last = r.activated.or(s.grid_last);
    ui.label(format!("Activated: {:?}", s.grid_last.map(|c| (c.row, c.column))));
}

fn carousels(ui: &mut Fui<'_>, s: &mut State) {
    heading(ui, "h-carousel", "Carousel");
    let rt = ui.rt();
    let cases = [
        ("default", CarouselAppearance::Default, false),
        ("elevated", CarouselAppearance::Elevated, true),
    ];
    for (i, ((name, appearance, autoplay), active)) in cases.into_iter().zip(s.carousel.iter_mut()).enumerate() {
        ui.scope(|ui| {
            ui.set_max_width(ui.available_width().min(360.0));
            let mut carousel = Carousel::new(("carousel", i), name, active)
                .appearance(appearance)
                .brand(autoplay);
            if autoplay {
                carousel = carousel.autoplay(std::time::Duration::from_secs(4)).circular(true);
            }
            carousel.show(&mut rt.ui(ui), |c| {
                for n in 1..=3 {
                    c.card(format!("Slide {n}"), |ui| {
                        ui.set_height(96.0);
                        ui.label(format!("Slide {n}"));
                    });
                }
            });
        });
        ui.add_space(8.0);
    }
}
