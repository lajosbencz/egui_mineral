use egui_mineral::action::Clicked;
use egui_mineral_fluent::{
    actions::PushButton,
    inputs::{Select, Swatch, SwatchPicker},
    navigation::{
        Breadcrumb, BreadcrumbButton, Menu, MenuGrid, MenuGridAction, MenuGridItem, MenuGridRow, MenuItem, TabList,
        Toolbar, ToolbarButton, ToolbarRadioGroup,
    },
    surfaces::{Dialog, DialogAction},
};

use crate::support::{FluentHarness, Harness, harness};

fn key(clicked: Option<&Clicked>) -> Option<&str> {
    clicked.and_then(Clicked::key::<&str>).copied()
}

#[test]
fn dialog_reports_the_key_of_the_clicked_action() {
    let mut h = harness((true, None), |ui, (open, clicked)| {
        let r = Dialog::new("dlg", "Delete file")
            .action("Cancel")
            .action(DialogAction::new("Delete").key("delete"))
            .show_modal(ui.rt(), open, |_| {});
        *clicked = r.clicked.or(clicked.take());
    });
    h.run();
    h.click("dlg-action-1-button");
    h.run();
    let clicked = h.state().value.1.as_ref();
    assert_eq!(clicked.map(Clicked::index), Some(1));
    assert_eq!(key(clicked), Some("delete"));
}

#[test]
fn an_item_without_a_key_reports_its_index_only() {
    let mut h = harness((true, None), |ui, (open, clicked)| {
        let r = Dialog::new("dlg", "Delete file")
            .action("Cancel")
            .show_modal(ui.rt(), open, |_| {});
        *clicked = r.clicked.or(clicked.take());
    });
    h.run();
    h.click("dlg-action-0-button");
    h.run();
    let clicked = h.state().value.1.as_ref();
    assert_eq!(clicked.map(Clicked::index), Some(0));
    assert_eq!(key(clicked), None);
}

fn menu() -> Harness<'static, (bool, Option<Clicked>)> {
    harness((false, None), |ui, (open, clicked)| {
        let rt = ui.rt();
        let trigger = ui.mineral(PushButton::new("trig", "Open")).response().clone();
        let r = Menu::new("m", "Actions").show(rt, &trigger, open, |m| {
            m.item(MenuItem::new("Cut").key("cut"));
            m.checkbox_item("Wrap", &mut false);
            m.submenu("More", |m| {
                m.item(MenuItem::new("Deep").key("deep"));
            });
        });
        *clicked = r.clicked.or(clicked.take());
    })
}

#[test]
fn menu_reports_the_key_of_the_clicked_row() {
    let mut h = menu();
    h.run();
    h.click("trig-button");
    h.run();
    h.click("m-1-menuitem");
    assert_eq!(h.state().value.1, None, "a checkbox row is not an action");
    h.click("m-0-menuitem");
    assert_eq!(key(h.state().value.1.as_ref()), Some("cut"));
}

#[test]
fn menu_reports_submenu_rows() {
    let mut h = menu();
    h.run();
    h.click("trig-button");
    h.run();
    h.click("m-2-menuitem");
    h.run();
    h.click("m-sub-2-0-menuitem");
    let clicked = h.state().value.1.as_ref();
    assert_eq!(key(clicked), Some("deep"));
    assert_eq!(clicked.map(Clicked::index), Some(0), "index within the submenu");
}

#[test]
fn menu_grid_reports_the_key_and_position_of_the_cell() {
    let mut h = harness((false, None, None), |ui, (open, clicked, cell)| {
        let rt = ui.rt();
        let trigger = ui.mineral(PushButton::new("trig", "Open")).response().clone();
        let r = MenuGrid::new("g", "Actions").show(rt, &trigger, open, |g| {
            g.row(MenuGridRow::new().cell(MenuGridItem::new("Cut").key("cut")));
            g.row(
                MenuGridRow::new()
                    .cell(MenuGridItem::new("Copy").key("copy"))
                    .cell(MenuGridAction::new("Pin", egui_mineral_fluent::icons::ICON_PIN).key("pin")),
            );
        });
        *clicked = r.clicked.or(clicked.take());
        *cell = r.activated.map(|c| (c.row, c.column)).or(*cell);
    });
    h.run();
    h.click("trig-button");
    h.run();
    h.click("g-1-1-gridcell");
    let (_, clicked, cell) = &h.state().value;
    assert_eq!(key(clicked.as_ref()), Some("pin"));
    assert_eq!(*cell, Some((1, 1)));
}

#[test]
fn toolbar_and_breadcrumb_report_keys() {
    let mut h = harness((None, None), |ui, (tool, crumb)| {
        let r = Toolbar::new("tb", "Tools")
            .action("Cut")
            .action(ToolbarButton::new("Copy").key("copy"))
            .show(ui);
        *tool = r.clicked.or(tool.take());
        let r = Breadcrumb::new("bc", "Path")
            .item(BreadcrumbButton::new("Home").key("home"))
            .item(BreadcrumbButton::new("Page").current(true))
            .show(ui);
        *crumb = r.clicked.or(crumb.take());
    });
    h.run();
    h.click("tb-1-button");
    h.click("bc-0-button");
    let (tool, crumb) = &h.state().value;
    assert_eq!(key(tool.as_ref()), Some("copy"));
    assert_eq!(key(crumb.as_ref()), Some("home"));
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum View {
    List,
    Grid,
    Board,
}

const VIEWS: [(View, &str); 3] = [(View::List, "List"), (View::Grid, "Grid"), (View::Board, "Board")];

#[test]
fn tab_list_binds_the_value_of_the_selected_tab() {
    let mut h = harness(View::Grid, |ui, view| {
        TabList::new("t", "Views", view).tabs(VIEWS).show(ui);
    });
    h.run();
    assert_eq!(h.node("t-1-tab").state.selected, Some(true));
    h.click("t-2-tab");
    assert_eq!(h.state().value, View::Board);
}

#[test]
fn a_value_matching_no_item_selects_none() {
    let mut h = harness(Some(View::Board), |ui, view| {
        TabList::new("t", "Views", view)
            .tabs([(Some(View::List), "List"), (Some(View::Grid), "Grid")])
            .show(ui);
    });
    h.run();
    assert_eq!(h.node("t-0-tab").state.selected, Some(false));
    assert_eq!(h.node("t-1-tab").state.selected, Some(false));
    h.click("t-0-tab");
    assert_eq!(h.state().value, Some(View::List));
}

#[test]
fn toolbar_radio_group_binds_a_value() {
    let mut h = harness(View::List, |ui, view| {
        Toolbar::new("tb", "Tools")
            .radio_group(ToolbarRadioGroup::new(view).options(VIEWS))
            .show(ui);
    });
    h.run();
    h.click("tb-1-button");
    assert_eq!(h.state().value, View::Grid);
}

#[test]
fn select_binds_a_value() {
    let mut h = harness(View::List, |ui, view| {
        ui.mineral(Select::bound("s", "View", VIEWS, view));
    });
    h.run();
    assert_eq!(h.node("s-select").value.as_deref(), Some("List"));
    h.click("s-select");
    h.click("s-select-2-option");
    assert_eq!(h.state().value, View::Board);
}

#[test]
fn swatch_picker_binds_a_value() {
    let mut h = harness(View::List, |ui, view| {
        let swatch = |(v, label): (View, &str)| (v, Swatch::color(label, egui::Color32::RED));
        SwatchPicker::bound("s", "Color", view)
            .swatches(VIEWS.map(swatch))
            .show(ui);
    });
    h.run();
    h.click("s-1-swatch");
    assert_eq!(h.state().value, View::Grid);
}
