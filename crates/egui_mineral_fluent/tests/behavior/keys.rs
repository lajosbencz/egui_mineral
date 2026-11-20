use egui_mineral_fluent::{
    actions::PushButton,
    inputs::{Select, Swatch, SwatchPicker},
    navigation::{
        Breadcrumb, BreadcrumbButton, Menu, MenuGrid, MenuGridAction, MenuGridItem, MenuGridRow, MenuItem, TabList,
        Toolbar, ToolbarRadioGroup,
    },
    surfaces::{Dialog, DialogAction},
};

use crate::support::{FluentHarness, Harness, harness};

#[test]
fn dialog_reports_the_value_of_the_clicked_action() {
    let mut h = harness((true, None), |ui, (open, clicked)| {
        let r = Dialog::new("dlg", "Delete file")
            .action("cancel", "Cancel")
            .action("delete", DialogAction::new("Delete"))
            .show_modal(ui.rt(), open, |_| {});
        *clicked = r.clicked.or(clicked.take());
    });
    h.run();
    h.click("dlg-action-delete-button");
    h.run();
    assert_eq!(h.state().value.1, Some("delete"));
}

fn menu() -> Harness<'static, (bool, Option<&'static str>)> {
    harness((false, None), |ui, (open, clicked)| {
        let rt = ui.rt();
        let trigger = ui.mineral(PushButton::new("trig", "Open")).response().clone();
        let r = Menu::new("m", "Actions").show(rt, &trigger, open, |m| {
            m.item("cut", MenuItem::new("Cut"));
            m.checkbox_item("Wrap", &mut false);
            m.submenu("More", |m| {
                m.item("deep", MenuItem::new("Deep"));
            });
        });
        *clicked = r.clicked.or(clicked.take());
    })
}

#[test]
fn menu_reports_the_value_of_the_clicked_row() {
    let mut h = menu();
    h.run();
    h.click("trig-button");
    h.run();
    h.click("m-1-menuitem");
    assert_eq!(h.state().value.1, None, "a checkbox row is not an action");
    h.click("m-0-menuitem");
    assert_eq!(h.state().value.1, Some("cut"));
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
    assert_eq!(h.state().value.1, Some("deep"));
}

#[test]
fn menu_grid_reports_the_value_of_the_cell() {
    let mut h = harness((false, None), |ui, (open, clicked)| {
        let rt = ui.rt();
        let trigger = ui.mineral(PushButton::new("trig", "Open")).response().clone();
        let r = MenuGrid::new("g", "Actions").show(rt, &trigger, open, |g| {
            g.row(MenuGridRow::new().cell("cut", MenuGridItem::new("Cut")));
            g.row(
                MenuGridRow::new()
                    .cell("copy", MenuGridItem::new("Copy"))
                    .cell("pin", MenuGridAction::new("Pin", egui_mineral_fluent::icons::ICON_PIN)),
            );
        });
        *clicked = r.clicked.or(clicked.take());
    });
    h.run();
    h.click("trig-button");
    h.run();
    h.click("g-1-1-gridcell");
    assert_eq!(h.state().value.1, Some("pin"));
}

#[test]
fn toolbar_and_breadcrumb_report_values() {
    let mut h = harness((None, None), |ui, (tool, crumb)| {
        let r = Toolbar::new("tb", "Tools")
            .action("cut", "Cut")
            .action("copy", "Copy")
            .show(ui);
        *tool = r.clicked.or(tool.take());
        let r = Breadcrumb::new("bc", "Path")
            .item("home", BreadcrumbButton::new("Home"))
            .item("page", BreadcrumbButton::new("Page").current(true))
            .show(ui);
        *crumb = r.clicked.or(crumb.take());
    });
    h.run();
    h.click("tb-copy-button");
    h.click("bc-0-button");
    assert_eq!(h.state().value, (Some("copy"), Some("home")));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, egui_mineral::semantic::KeySalt)]
enum View {
    List,
    Grid,
    Board,
}

const VIEWS: [(View, &str); 3] = [(View::List, "List"), (View::Grid, "Grid"), (View::Board, "Board")];

#[test]
fn tab_list_binds_the_value_of_the_selected_tab() {
    let mut h = harness(View::Grid, |ui, view| {
        TabList::new("t", "Views", view).items(VIEWS).show(ui);
    });
    h.run();
    assert_eq!(h.node("t-grid-tab").state.selected, Some(true));
    h.click("t-board-tab");
    assert_eq!(h.state().value, View::Board);
}

#[test]
fn a_value_matching_no_item_selects_none() {
    let mut h = harness(Some(View::Board), |ui, view| {
        TabList::new("t", "Views", view)
            .items([(Some(View::List), "List"), (Some(View::Grid), "Grid")])
            .show(ui);
    });
    h.run();
    assert_eq!(h.node("t-list-tab").state.selected, Some(false));
    assert_eq!(h.node("t-grid-tab").state.selected, Some(false));
    h.click("t-list-tab");
    assert_eq!(h.state().value, Some(View::List));
}

#[test]
fn toolbar_radio_group_binds_a_value() {
    let mut h = harness(View::List, |ui, view| {
        Toolbar::new("tb", "Tools")
            .radio_group(ToolbarRadioGroup::new(view).items(VIEWS))
            .show(ui);
    });
    h.run();
    h.click("tb-1-button");
    assert_eq!(h.state().value, View::Grid);
}

#[test]
fn select_binds_a_value() {
    let mut h = harness(View::List, |ui, view| {
        ui.mineral(Select::new("s", "View", view).items(VIEWS));
    });
    h.run();
    assert_eq!(h.node("s-select").value.as_deref(), Some("List"));
    h.click("s-select");
    h.click("s-select-board-option");
    assert_eq!(h.state().value, View::Board);
}

#[test]
fn swatch_picker_binds_a_value() {
    let mut h = harness(View::List, |ui, view| {
        let swatch = |(v, label): (View, &str)| (v, Swatch::color(label, egui::Color32::RED));
        SwatchPicker::new("s", "Color", view).items(VIEWS.map(swatch)).show(ui);
    });
    h.run();
    h.click("s-grid-swatch");
    assert_eq!(h.state().value, View::Grid);
}
