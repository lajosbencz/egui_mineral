use egui::Key;
use egui_mineral::{
    expand::Expansion,
    layout::Axis,
    select::{Check, Selection, SelectionMode},
    semantic::{Action, Role},
};
use egui_mineral_fluent::actions::PushButton;
use egui_mineral_fluent::icons::{ICON_MORE_HORIZONTAL, ICON_PIN, ICON_SHARE};
use egui_mineral_fluent::navigation::{
    Breadcrumb, BreadcrumbButton, BreadcrumbFocus, BreadcrumbItem, Carousel, MenuGrid, MenuGridAction, MenuGridItem,
    MenuGridRow, Nav, NavCategory, NavDivider, NavItem, NavSubItem, Overflow, OverflowDirection, Tab, TabList, Toolbar,
    ToolbarButton, ToolbarRadioGroup, ToolbarToggleButton, Tree, TreeItem,
};

use crate::support::{FluentHarness, Harness, harness};

fn tabs(vertical: bool, select_on_focus: bool) -> Harness<'static, usize> {
    harness(0_usize, move |ui, selected| {
        TabList::new("t", "Tabs", selected)
            .tab(0, "One")
            .tab(1, Tab::new("Two").disabled(true))
            .tab(2, "Three")
            .axis(if vertical { Axis::Vertical } else { Axis::Horizontal })
            .select_on_focus(select_on_focus)
            .show(ui);
    })
}

#[test]
fn tabs_expose_roles_and_selection() {
    let mut h = tabs(false, false);
    h.run();
    assert_eq!(h.node("t-tablist").role, Role::TabList);
    assert_eq!(h.node("t-0-tab").role, Role::Tab);
    assert_eq!(h.node("t-0-tab").state.selected, Some(true));
    assert_eq!(h.node("t-2-tab").state.selected, Some(false));
    assert!(h.node("t-1-tab").state.disabled);
}

#[test]
fn clicking_selects_and_disabled_tabs_ignore_clicks() {
    let mut h = tabs(false, false);
    h.run();
    h.click("t-1-tab");
    assert_eq!(h.state().value, 0);
    h.click("t-2-tab");
    assert_eq!(h.state().value, 2);
    assert_eq!(h.node("t-2-tab").state.selected, Some(true));
}

#[test]
fn arrows_move_focus_and_enter_selects() {
    let mut h = tabs(false, false);
    h.run();
    h.click("t-0-tab");
    h.press(Key::ArrowRight);
    assert_eq!(h.state().value, 0);
    h.press(Key::Enter);
    assert_eq!(h.state().value, 2);
}

#[test]
fn select_on_focus_selects_with_the_arrow() {
    let mut h = tabs(false, true);
    h.run();
    h.click("t-0-tab");
    h.press(Key::ArrowRight);
    assert_eq!(h.state().value, 2);
    h.press(Key::ArrowRight);
    assert_eq!(h.state().value, 0);
}

#[test]
fn vertical_lists_use_the_down_arrow() {
    let mut h = tabs(true, true);
    h.run();
    h.click("t-0-tab");
    h.press(Key::ArrowRight);
    assert_eq!(h.state().value, 0);
    h.press(Key::ArrowDown);
    assert_eq!(h.state().value, 2);
}

#[derive(Default)]
struct Bar {
    bold: bool,
    align: usize,
    activated: Option<usize>,
}

fn toolbar(vertical: bool) -> Harness<'static, Bar> {
    harness(Bar::default(), move |ui, s| {
        Toolbar::new("tb", "Tools")
            .axis(if vertical { Axis::Vertical } else { Axis::Horizontal })
            .action("Cut")
            .action(ToolbarButton::new("Copy").disabled(true))
            .divider()
            .toggle(ToolbarToggleButton::new("Bold", &mut s.bold))
            .radio_group(
                ToolbarRadioGroup::new(&mut s.align)
                    .option(0, "Left")
                    .option(1, "Right"),
            )
            .show(ui);
    })
}

#[test]
fn toolbar_exposes_roles_and_states() {
    let mut h = toolbar(false);
    h.run();
    assert_eq!(h.node("tb-toolbar").role, Role::Toolbar);
    assert_eq!(h.node("tb-0-button").role, Role::Button);
    assert!(h.node("tb-1-button").state.disabled);
    assert_eq!(h.node("tb-2-button").state.checked, Some(Check::Unchecked));
    assert_eq!(h.node("tb-3-button").role, Role::RadioButton);
    assert_eq!(h.node("tb-3-button").state.checked, Some(Check::Checked));
    assert_eq!(h.node("tb-4-button").state.checked, Some(Check::Unchecked));
}

#[test]
fn toolbar_toggle_and_radio_write_their_values() {
    let mut h = toolbar(false);
    h.run();
    h.click("tb-2-button");
    assert!(h.state().value.bold);
    h.click("tb-4-button");
    assert_eq!(h.state().value.align, 1);
    assert_eq!(h.node("tb-4-button").state.checked, Some(Check::Checked));
    assert_eq!(h.node("tb-3-button").state.checked, Some(Check::Unchecked));
}

#[test]
fn toolbar_arrows_skip_disabled_items_and_wrap() {
    let mut h = toolbar(false);
    h.run();
    h.click("tb-0-button");
    h.press(Key::ArrowRight);
    h.press(Key::Enter);
    assert!(h.state().value.bold, "skipped the disabled Copy");
    h.press(Key::ArrowLeft);
    h.press(Key::ArrowLeft);
    h.press(Key::Enter);
    assert_eq!(h.state().value.align, 1, "wrapped to the last radio button");
}

#[test]
fn vertical_toolbar_uses_the_down_arrow() {
    let mut h = toolbar(true);
    h.run();
    h.click("tb-0-button");
    h.press(Key::ArrowRight);
    h.press(Key::Enter);
    assert!(!h.state().value.bold);
    h.press(Key::ArrowDown);
    h.press(Key::Enter);
    assert!(h.state().value.bold);
}

#[derive(Default)]
struct Trail {
    clicked: Option<usize>,
}

fn breadcrumb(focus: BreadcrumbFocus) -> Harness<'static, Trail> {
    harness(Trail::default(), move |ui, s| {
        let r = Breadcrumb::new("bc", "Path")
            .focus(focus)
            .item("Home")
            .item(BreadcrumbItem::new(BreadcrumbButton::new("Docs").disabled(true)))
            .item("Guide")
            .item(BreadcrumbButton::new("Page").current(true))
            .show(ui);
        if let Some(c) = r.clicked {
            s.clicked = Some(c.index());
        }
    })
}

#[test]
fn breadcrumb_exposes_roles_and_states() {
    let mut h = breadcrumb(BreadcrumbFocus::Tab);
    h.run();
    assert_eq!(h.node("bc-breadcrumb").role, Role::Navigation);
    assert_eq!(h.node("bc-0-button").role, Role::Button);
    assert!(h.node("bc-1-button").state.disabled);
    assert!(h.node("bc-3-button").state.disabled, "current is aria-disabled");
    assert!(!h.node("bc-2-button").state.disabled);
}

#[test]
fn breadcrumb_clicks_report_but_current_and_disabled_do_not() {
    let mut h = breadcrumb(BreadcrumbFocus::Tab);
    h.run();
    h.click("bc-2-button");
    assert_eq!(h.state().value.clicked, Some(2));
    h.state_mut().value.clicked = None;
    h.click("bc-3-button");
    h.click("bc-1-button");
    assert_eq!(h.state().value.clicked, None);
}

#[test]
fn breadcrumb_tab_mode_makes_every_button_a_tab_stop() {
    let mut h = breadcrumb(BreadcrumbFocus::Tab);
    h.run();
    h.press(Key::Tab);
    h.press(Key::Enter);
    assert_eq!(h.state().value.clicked, Some(0));
    h.press(Key::Tab);
    h.press(Key::Enter);
    assert_eq!(h.state().value.clicked, Some(2), "Tab skips the disabled item");
}

#[test]
fn breadcrumb_arrow_mode_roves_and_wraps() {
    let mut h = breadcrumb(BreadcrumbFocus::Arrow);
    h.run();
    h.click("bc-0-button");
    h.press(Key::ArrowRight);
    h.press(Key::Enter);
    assert_eq!(h.state().value.clicked, Some(2), "skips the disabled item");
    h.press(Key::ArrowLeft);
    h.press(Key::Enter);
    assert_eq!(h.state().value.clicked, Some(0));
    h.press(Key::ArrowLeft);
    h.press(Key::Enter);
    assert_eq!(
        h.state().value.clicked,
        Some(0),
        "wrapped onto the current item, which ignores Enter"
    );
    h.press(Key::ArrowLeft);
    h.press(Key::Enter);
    assert_eq!(h.state().value.clicked, Some(2));
    h.press(Key::Home);
    h.press(Key::Enter);
    assert_eq!(h.state().value.clicked, Some(0));
}

const LABELS: [&str; 6] = ["Cut", "Copy", "Paste", "Undo", "Redo", "Find"];

fn narrow_toolbar(width: Option<f32>) -> Harness<'static, Bar> {
    harness(Bar::default(), move |ui, s| {
        if let Some(w) = width {
            ui.set_max_width(w);
        }
        let mut bar = Toolbar::new("tb", "Tools").overflow(Overflow::new("More"));
        for label in LABELS {
            bar = bar.action(label);
        }
        let r = bar.show(ui);
        if let Some(c) = r.clicked {
            s.activated = Some(c.index());
        }
    })
}

fn fitting_width() -> f32 {
    let mut wide = narrow_toolbar(None);
    wide.run();
    let w = wide.rect("tb-0-button").width();
    16.0 + 3.0 * w + 32.0 + 1.0
}

#[test]
fn toolbar_without_overflow_keeps_every_item() {
    let mut h = harness(Bar::default(), |ui, _| {
        ui.set_max_width(60.0);
        let mut bar = Toolbar::new("tb", "Tools");
        for label in LABELS {
            bar = bar.action(label);
        }
        bar.show(ui);
    });
    h.run();
    for i in 0..LABELS.len() {
        h.node(&format!("tb-{i}-button"));
    }
    assert!(h.tree().find_key("tb-overflow-button").is_none());
}

#[test]
fn toolbar_overflow_hides_trailing_items_behind_a_menu_button() {
    let mut h = narrow_toolbar(Some(fitting_width()));
    h.run();
    for i in 0..3 {
        h.node(&format!("tb-{i}-button"));
    }
    assert!(h.tree().find_key("tb-3-button").is_none());
    let more = h.node("tb-overflow-button");
    assert!((more.rect.width() - 32.0).abs() < 0.5, "{:?}", more.rect);
    assert!(more.rect.left() >= h.rect("tb-2-button").right() - 0.5);
}

#[test]
fn toolbar_overflow_menu_rows_activate_hidden_items() {
    let mut h = narrow_toolbar(Some(fitting_width()));
    h.run();
    h.click("tb-overflow-button");
    h.click("tb-overflow-menu-0-menuitem");
    assert_eq!(h.state().value.activated, Some(3));
}

#[test]
fn toolbar_overflow_button_joins_the_roving_group() {
    let mut h = narrow_toolbar(Some(fitting_width()));
    h.run();
    h.click("tb-2-button");
    h.press(Key::ArrowRight);
    h.press(Key::Enter);
    assert!(h.tree().find_key("tb-overflow-menu-0-menuitem").is_some());
}

fn narrow_trail(width: Option<f32>, direction: OverflowDirection) -> Harness<'static, Trail> {
    harness(Trail::default(), move |ui, s| {
        if let Some(w) = width {
            ui.set_max_width(w);
        }
        let r = Breadcrumb::new("bc", "Path")
            .overflow(Overflow::new("More").direction(direction))
            .items(["Home", "Docs", "Wiki", "Page", "Info"])
            .show(ui);
        s.clicked = r.clicked.map(|c| c.index()).or(s.clicked);
    })
}

// Room for the two items that stay visible, their dividers and the menu button.
fn trail_width(direction: OverflowDirection) -> f32 {
    let mut wide = narrow_trail(None, direction);
    wide.run();
    let width = |i: usize| wide.rect(&format!("bc-{i}-button")).width();
    let items = if direction == OverflowDirection::Start {
        width(3) + width(4) + 2.0 * 16.0
    } else {
        width(0) + width(1) + 16.0
    };
    items + 32.0 + 6.0
}

#[test]
fn breadcrumb_overflow_start_hides_leading_items_behind_a_leading_button() {
    let mut h = narrow_trail(Some(trail_width(OverflowDirection::Start)), OverflowDirection::Start);
    h.run();
    for i in 0..3 {
        assert!(h.tree().find_key(&format!("bc-{i}-button")).is_none(), "{i}");
    }
    let more = h.rect("bc-overflow-button");
    assert!(more.right() <= h.rect("bc-3-button").left() + 0.5);
    h.node("bc-4-button");
}

#[test]
fn breadcrumb_overflow_end_puts_the_button_after_the_trail() {
    let mut h = narrow_trail(Some(trail_width(OverflowDirection::End)), OverflowDirection::End);
    h.run();
    for i in 2..5 {
        assert!(h.tree().find_key(&format!("bc-{i}-button")).is_none(), "{i}");
    }
    let more = h.rect("bc-overflow-button");
    assert!(more.left() >= h.rect("bc-1-button").right() - 0.5);
}

#[test]
fn breadcrumb_overflow_menu_rows_report_the_hidden_item() {
    let mut h = narrow_trail(Some(trail_width(OverflowDirection::Start)), OverflowDirection::Start);
    h.run();
    h.click("bc-overflow-button");
    h.click("bc-overflow-menu-1-menuitem");
    assert_eq!(h.state().value.clicked, Some(1));
}

#[derive(Default)]
struct Side {
    selected: Option<usize>,
}

fn side(multiple: bool) -> Harness<'static, Side> {
    harness(Side::default(), move |ui, s| {
        Nav::new("n", "Main", &mut s.selected)
            .multiple(multiple)
            .default_open([10])
            .show(ui, |nav| {
                nav.section_header("Pages");
                nav.item(NavItem::new(1, "Home"));
                nav.category(NavCategory::new(10, "Docs"), |nav| {
                    nav.sub_item(NavSubItem::new(11, "Guide"));
                    nav.sub_item(NavSubItem::new(12, "API"));
                });
                nav.divider(NavDivider::new("d"));
                nav.category(NavCategory::new(20, "Admin"), |nav| {
                    nav.sub_item(NavSubItem::new(21, "Users"));
                });
            });
    })
}

#[test]
fn nav_exposes_roles_and_state() {
    let mut h = side(true);
    h.run();
    assert_eq!(h.node("n-nav").role, Role::Navigation);
    assert_eq!(h.node("n-1-navitem").role, Role::Link);
    assert_eq!(h.node("n-1-navitem").state.selected, Some(false));
    assert_eq!(h.node("n-10-navcategory").state.expanded, Some(true));
    assert_eq!(h.node("n-20-navcategory").state.expanded, Some(false));
    assert_eq!(h.node("n-11-navsubitem").role, Role::Link);
    assert!(
        h.tree().find_key("n-21-navsubitem").is_none(),
        "closed categories hide their sub items"
    );
}

#[test]
fn nav_click_selects_one_value() {
    let mut h = side(true);
    h.run();
    h.click("n-1-navitem");
    assert_eq!(h.state().value.selected, Some(1));
    h.click("n-12-navsubitem");
    assert_eq!(h.state().value.selected, Some(12));
    assert_eq!(h.node("n-12-navsubitem").state.selected, Some(true));
    assert_eq!(h.node("n-1-navitem").state.selected, Some(false));
}

#[test]
fn nav_category_toggles_and_keeps_the_selection_indicator_while_closed() {
    let mut h = side(true);
    h.run();
    h.click("n-11-navsubitem");
    h.click("n-10-navcategory");
    assert_eq!(h.node("n-10-navcategory").state.expanded, Some(false));
    assert!(h.tree().find_key("n-11-navsubitem").is_none());
    assert_eq!(h.node("n-10-navcategory").state.selected, Some(true));
    h.click("n-10-navcategory");
    assert_eq!(h.node("n-10-navcategory").state.expanded, Some(true));
    assert_eq!(h.node("n-10-navcategory").state.selected, Some(false));
}

#[test]
fn nav_single_mode_closes_the_other_category() {
    let mut h = side(false);
    h.run();
    h.click("n-20-navcategory");
    assert_eq!(h.node("n-20-navcategory").state.expanded, Some(true));
    assert_eq!(h.node("n-10-navcategory").state.expanded, Some(false));
}

#[test]
fn nav_rows_are_tab_stops_and_enter_activates() {
    let mut h = side(true);
    h.run();
    h.press(Key::Tab);
    h.press(Key::Enter);
    assert_eq!(h.state().value.selected, Some(1));
    h.press(Key::Tab);
    h.press(Key::Enter);
    assert_eq!(h.node("n-10-navcategory").state.expanded, Some(false));
}

struct Files {
    open: Expansion<String>,
    checked: Selection<String>,
    activated: Vec<String>,
}

fn files() -> Vec<TreeItem> {
    vec![
        TreeItem::new("a", "Alpha").nested([
            TreeItem::new("a1", "Apple"),
            TreeItem::new("a2", "Apricot").nested([TreeItem::new("a2x", "Axe")]),
        ]),
        TreeItem::new("b", "Beta"),
    ]
}

fn tree(mode: Option<SelectionMode>) -> Harness<'static, Files> {
    let roots = files();
    let mut open = Expansion::default();
    open.expand("a".to_owned());
    harness(
        Files {
            open,
            checked: match mode {
                Some(SelectionMode::Multiple) => Selection::multiple(),
                _ => Selection::single(),
            },
            activated: Vec::new(),
        },
        move |ui, s| {
            let mut tree = Tree::items("t", "Files", &roots, &mut s.open);
            if mode.is_some() {
                tree = tree.selection(&mut s.checked);
            }
            let r = tree.show(ui);
            s.activated.extend(r.activated);
        },
    )
}

fn focused(h: &Harness<'_, Files>) -> Option<&'static str> {
    ["a", "a1", "a2", "b"]
        .into_iter()
        .find(|k| h.node(&format!("t-{k}-treeitem")).state.focused)
}

#[test]
fn tree_exposes_roles_levels_and_expansion() {
    let mut h = tree(None);
    h.run();
    assert_eq!(h.node("t-tree").role, Role::Tree);
    let a = h.node("t-a-treeitem");
    assert_eq!((a.role, a.state.expanded, a.depth), (Role::TreeItem, Some(true), 1));
    assert_eq!(h.node("t-a2-treeitem").state.expanded, Some(false));
    assert_eq!(h.node("t-a1-treeitem").state.expanded, None);
    assert_eq!(h.node("t-a1-treeitem").depth, 2);
    assert!(
        h.tree().find_key("t-a2x-treeitem").is_none(),
        "closed branches hide their children"
    );
    assert!(h.node("t-a-treeitem").actions.contains(&Action::Collapse));
    assert!(h.node("t-a2-treeitem").actions.contains(&Action::Expand));
}

#[test]
fn clicking_a_branch_toggles_it_and_a_leaf_activates() {
    let mut h = tree(None);
    h.run();
    h.click("t-a2-treeitem");
    assert!(h.state().value.open.is_expanded(&"a2".to_owned()));
    h.node("t-a2x-treeitem");
    h.click("t-b-treeitem");
    assert_eq!(h.state().value.activated, ["a2", "b"]);
}

#[test]
fn arrows_follow_the_tree_navigation_rules() {
    let mut h = tree(None);
    h.run();
    h.click("t-a1-treeitem");
    h.press(Key::ArrowLeft);
    assert_eq!(focused(&h), Some("a"), "Left on a leaf goes to its parent");
    h.press(Key::ArrowLeft);
    assert!(
        !h.state().value.open.is_expanded(&"a".to_owned()),
        "Left on an open branch collapses it"
    );
    assert_eq!(focused(&h), Some("a"));
    h.press(Key::ArrowRight);
    assert!(h.state().value.open.is_expanded(&"a".to_owned()));
    h.press(Key::ArrowRight);
    assert_eq!(focused(&h), Some("a1"), "Right on an open branch enters it");
    h.press(Key::ArrowDown);
    assert_eq!(focused(&h), Some("a2"));
    h.press(Key::End);
    assert_eq!(focused(&h), Some("b"));
    h.press(Key::ArrowUp);
    assert_eq!(focused(&h), Some("a2"));
    h.press(Key::Home);
    assert_eq!(focused(&h), Some("a"));
}

#[test]
fn enter_activates_and_toggles_a_branch() {
    let mut h = tree(None);
    h.run();
    h.click("t-a1-treeitem");
    h.press(Key::ArrowDown);
    h.press(Key::Enter);
    assert!(h.state().value.open.is_expanded(&"a2".to_owned()));
    assert_eq!(h.state().value.activated.last().map(String::as_str), Some("a2"));
}

#[test]
fn typing_jumps_to_a_matching_label() {
    let mut h = tree(None);
    h.run();
    h.click("t-a1-treeitem");
    h.event(egui::Event::Text("b".into()));
    h.run();
    assert_eq!(focused(&h), Some("b"));
}

fn selector(h: &Harness<'_, Files>, key: &str) -> egui::Pos2 {
    let r = h.rect(key);
    let depth = h.node(key).depth as f32;
    egui::pos2(r.left() + 24.0 * (depth - 1.0) + 24.0 + 16.0, r.center().y)
}

#[test]
fn multiselect_checks_cascade_and_show_the_mixed_state() {
    let mut h = tree(Some(SelectionMode::Multiple));
    h.run();
    let at = selector(&h, "t-a1-treeitem");
    h.click_at(at);
    assert_eq!(h.state().value.checked.selected().collect::<Vec<_>>(), ["a1"]);
    assert_eq!(h.node("t-a-treeitem").state.checked, Some(Check::Mixed));
    assert!(h.state().value.activated.is_empty(), "the selector does not activate");
    let at = selector(&h, "t-a-treeitem");
    h.click_at(at);
    assert!(
        h.state().value.checked.is_selected(&"a2x".to_owned()) && h.state().value.checked.is_selected(&"a".to_owned())
    );
    assert_eq!(h.node("t-a-treeitem").state.checked, Some(Check::Checked));
}

#[test]
fn single_selection_keeps_one_checked_item_and_space_toggles_it() {
    let mut h = tree(Some(SelectionMode::Single));
    h.run();
    let at = selector(&h, "t-a1-treeitem");
    h.click_at(at);
    let at = selector(&h, "t-b-treeitem");
    h.click_at(at);
    assert_eq!(h.state().value.checked.selected().collect::<Vec<_>>(), ["b"]);
    assert_eq!(h.node("t-b-treeitem").state.checked, Some(Check::Checked));
    h.click("t-a1-treeitem");
    h.press(Key::Space);
    assert_eq!(h.state().value.checked.selected().collect::<Vec<_>>(), ["a1"]);
}

#[test]
fn flat_levels_nest_into_branches() {
    let items = TreeItem::from_levels([
        (0, TreeItem::new("x", "X")),
        (1, TreeItem::new("y", "Y")),
        (3, TreeItem::new("z", "Z")),
        (0, TreeItem::new("w", "W")),
    ]);
    let mut open = Expansion::default();
    open.expand("x".to_owned());
    open.expand("y".to_owned());
    let mut h = harness(open, move |ui, open| {
        Tree::items("t", "Flat", &items, open).show(ui);
    });
    h.run();
    let depths = ["x", "y", "z", "w"].map(|k| h.node(&format!("t-{k}-treeitem")).depth);
    assert_eq!(depths, [1, 2, 3, 1]);
}

struct Grid {
    open: bool,
    activated: Vec<(usize, usize)>,
}

fn grid() -> Harness<'static, Grid> {
    let state = Grid {
        open: true,
        activated: Vec::new(),
    };
    harness(state, |ui, s| {
        let trigger = ui.mineral(PushButton::new("trigger", "Open")).into_response();
        let r = MenuGrid::new("g", "Actions").show(ui.rt(), &trigger, &mut s.open, |g| {
            g.row(
                MenuGridRow::new()
                    .cell(MenuGridItem::new("Cut").secondary_text("Ctrl+X"))
                    .cell(MenuGridAction::new("More cut", ICON_MORE_HORIZONTAL)),
            );
            g.row(
                MenuGridRow::new()
                    .cell("Copy")
                    .cell(MenuGridAction::new("Share", ICON_SHARE).disabled(true))
                    .cell(MenuGridAction::new("Pin", ICON_PIN)),
            );
            g.row(MenuGridRow::new().cell("Delete"));
        });
        s.activated.extend(r.activated.map(|c| (c.row, c.column)));
    })
}

fn at(h: &Harness<'_, Grid>) -> Option<(usize, usize)> {
    [(0, 0), (0, 1), (1, 0), (1, 1), (1, 2), (2, 0)]
        .into_iter()
        .find(|(r, c)| h.node(&format!("g-{r}-{c}-gridcell")).state.focused)
}

#[test]
fn menu_grid_is_a_popover_of_its_trigger() {
    let mut h = grid();
    h.run();
    assert!(
        h.node("g-0-0-gridcell").state.focused,
        "the first cell takes focus on opening"
    );
    h.press(Key::Escape);
    assert!(!h.state().value.open);
    assert!(h.tree().find_key("g-menugrid").is_none());
    h.click("trigger-button");
    assert!(h.state().value.open, "the trigger toggles it");
    h.click("g-0-1-gridcell");
    assert!(h.state().value.open, "activating a cell keeps it open");
    h.click_at(crate::support::SCREEN.to_pos2() - egui::vec2(4.0, 4.0));
    assert!(!h.state().value.open, "an outside press closes it");
}

#[test]
fn menu_grid_exposes_grid_rows_and_cells() {
    let mut h = grid();
    h.run();
    assert_eq!(h.node("g-menugrid").role, Role::Grid);
    assert_eq!(h.node("g-0-gridrow").role, Role::Row);
    assert_eq!(h.node("g-0-gridrow").name, "Cut");
    let cell = h.node("g-0-1-gridcell");
    assert_eq!((cell.role, cell.name.as_str()), (Role::GridCell, "More cut"));
    assert!(h.node("g-1-1-gridcell").state.disabled);
}

#[test]
fn menu_grid_rows_size_from_content_and_share_the_widest_row() {
    let mut h = grid();
    h.run();
    h.run();
    let widths = [0, 1, 2].map(|r| h.rect(&format!("g-{r}-gridrow")).width());
    assert!(widths.iter().all(|w| (w - widths[0]).abs() < 0.5), "{widths:?}");
    assert!(widths[0] < 290.0, "short rows do not fill the 290 maximum: {widths:?}");
}

// Headless text has no width, so the rows are sized by action cells (24 wide, 4 apart, 6 padding each side).
fn action_row(actions: usize) -> MenuGridRow {
    (0..actions).fold(MenuGridRow::new(), |row, i| {
        row.cell(MenuGridAction::new(format!("Action {i}"), ICON_PIN))
    })
}

fn action_grid(widest: usize) -> Harness<'static, ()> {
    let mut open = true;
    harness((), move |ui, ()| {
        let trigger = ui.mineral(PushButton::new("trigger", "Open")).into_response();
        MenuGrid::new("g", "Actions").show(ui.rt(), &trigger, &mut open, |g| {
            g.row(action_row(1));
            g.row(action_row(widest));
        });
    })
}

#[test]
fn menu_grid_rows_stop_growing_at_the_maximum_width() {
    let mut h = action_grid(11);
    h.run();
    h.run();
    assert_eq!(h.rect("g-1-gridrow").width(), 290.0);
    assert_eq!(h.rect("g-0-gridrow").width(), 290.0, "rows share the widest");
}

#[test]
fn menu_grid_rows_grow_to_the_widest_row_only() {
    let mut h = action_grid(6);
    h.run();
    h.run();
    let expected = 6.0 * 24.0 + 5.0 * 4.0 + 12.0;
    assert_eq!(h.rect("g-1-gridrow").width(), expected);
    assert_eq!(h.rect("g-0-gridrow").width(), expected);
}

#[test]
fn menu_grid_clicks_activate_the_cell_and_disabled_cells_ignore_them() {
    let mut h = grid();
    h.run();
    h.click("g-0-1-gridcell");
    h.click("g-1-1-gridcell");
    h.click("g-1-2-gridcell");
    assert_eq!(h.state().value.activated, [(0, 1), (1, 2)]);
}

#[test]
fn menu_grid_arrows_move_by_cell_and_row_and_skip_disabled_cells() {
    let mut h = grid();
    h.run();
    h.click("g-0-0-gridcell");
    h.press(Key::ArrowRight);
    assert_eq!(at(&h), Some((0, 1)));
    h.press(Key::ArrowDown);
    assert_eq!(at(&h), Some((2, 0)), "the disabled cell is skipped, the column clamps");
    h.press(Key::ArrowUp);
    assert_eq!(at(&h), Some((1, 0)));
    h.press(Key::ArrowRight);
    assert_eq!(at(&h), Some((1, 2)), "Right skips the disabled cell");
    h.press(Key::ArrowLeft);
    assert_eq!(at(&h), Some((1, 0)), "Left skips it too");
}

#[test]
fn menu_grid_home_end_and_ctrl_variants() {
    let mut h = grid();
    h.run();
    h.click("g-0-0-gridcell");
    h.press(Key::End);
    assert_eq!(at(&h), Some((0, 1)));
    h.press(Key::Home);
    assert_eq!(at(&h), Some((0, 0)));
    h.key_press_modifiers(egui::Modifiers::COMMAND, Key::End);
    h.run();
    assert_eq!(at(&h), Some((2, 0)));
    h.key_press_modifiers(egui::Modifiers::COMMAND, Key::Home);
    h.run();
    assert_eq!(at(&h), Some((0, 0)));
}

#[test]
fn menu_grid_enter_activates_the_focused_cell() {
    let mut h = grid();
    h.run();
    h.click("g-0-0-gridcell");
    h.press(Key::ArrowRight);
    h.press(Key::Enter);
    assert_eq!(h.state().value.activated.last(), Some(&(0, 1)));
}

fn carousel(circular: bool, autoplay: bool) -> Harness<'static, usize> {
    harness(0_usize, move |ui, active| {
        let mut c = Carousel::new("c", "Gallery", active).circular(circular);
        if autoplay {
            c = c.autoplay(std::time::Duration::from_millis(50));
        }
        c.show(ui, |c| {
            for name in ["One", "Two", "Three"] {
                c.card(name, |ui| {
                    ui.label(name);
                });
            }
        });
    })
}

#[test]
fn carousel_shows_the_active_card_with_tab_like_dots() {
    let mut h = carousel(false, false);
    h.run();
    assert_eq!(h.node("c-carousel").role, Role::Group);
    assert_eq!(
        h.node("c-previous-button").name,
        "Previous slide",
        "named by the catalog"
    );
    h.node("c-0-carouselcard");
    assert!(h.tree().find_key("c-1-carouselcard").is_none());
    assert_eq!(h.node("c-0-carouselnav").state.selected, Some(true));
    assert_eq!(h.node("c-1-carouselnav").role, Role::Tab);
    assert_eq!(h.node("c-1-carouselnav").name, "Two");
    assert!(h.node("c-previous-button").state.disabled, "no wrap at the first card");
    assert!(h.tree().find_key("c-autoplay-button").is_none(), "autoplay is opt-in");
}

#[test]
fn carousel_buttons_and_dots_change_the_card() {
    let mut h = carousel(false, false);
    h.run();
    h.click("c-next-button");
    assert_eq!(h.state().value, 1);
    h.node("c-1-carouselcard");
    h.click("c-2-carouselnav");
    assert_eq!(h.state().value, 2);
    assert!(h.node("c-next-button").state.disabled);
    h.click("c-previous-button");
    assert_eq!(h.state().value, 1);
}

#[test]
fn carousel_arrows_move_between_dots_and_circular_wraps() {
    let mut h = carousel(true, false);
    h.run();
    h.click("c-previous-button");
    assert_eq!(h.state().value, 2, "previous on the first card wraps to the last");
    h.click("c-0-carouselnav");
    h.press(Key::ArrowRight);
    assert_eq!(h.state().value, 1);
    h.press(Key::ArrowLeft);
    h.press(Key::ArrowLeft);
    assert_eq!(h.state().value, 2);
}

#[test]
fn carousel_autoplay_starts_paused_and_stops_under_reduced_motion() {
    let mut h = carousel(true, true);
    h.rt().core().motion().set_reduced(false);
    h.run();
    for _ in 0..12 {
        h.step();
    }
    assert_eq!(h.state().value, 0, "paused until the button is pressed");
    h.click("c-autoplay-button");
    for _ in 0..12 {
        h.step();
    }
    assert_ne!(h.state().value, 0, "playing advances the card");
    h.rt().core().motion().set_reduced(true);
    h.run();
    assert!(h.node("c-autoplay-button").state.disabled);
    let at = h.state().value;
    for _ in 0..12 {
        h.step();
    }
    assert_eq!(h.state().value, at, "reduced motion stops autoplay");
}

mod big_tree {
    use super::*;

    const PARENTS: usize = 20;
    const KIDS: usize = 24;

    fn big() -> Harness<'static, Expansion<String>> {
        let roots: Vec<TreeItem> = (0..PARENTS)
            .map(|p| {
                TreeItem::new(format!("p{p}"), format!("Parent {p}"))
                    .nested((0..KIDS).map(|k| TreeItem::new(format!("p{p}k{k}"), format!("Kid {k}"))))
            })
            .collect();
        let open = (0..PARENTS).map(|p| format!("p{p}")).collect();
        harness(open, move |ui, open| {
            let rt = ui.rt();
            egui::ScrollArea::vertical().show(ui, |ui| {
                Tree::items("t", "Big", &roots, open).show(&mut rt.ui(ui));
            });
        })
    }

    #[test]
    fn culled_rows_register_no_widgets() {
        let mut h = big();
        h.run();
        let last = format!("t-p{}k{}-treeitem", PARENTS - 1, KIDS - 1);
        assert!(h.node(&last).actions.contains(&Action::ScrollIntoView));
        let widgets: usize = h
            .ctx
            .viewport(|v| v.prev_pass.widgets.layers().map(|(_, w)| w.len()).sum());
        assert!(widgets < 150, "{widgets} widgets for {} rows", PARENTS * (KIDS + 1));
    }

    #[test]
    fn end_focuses_the_culled_last_row() {
        let mut h = big();
        h.run();
        h.click("t-p0-treeitem");
        h.key_press(Key::End);
        let last = format!("t-p{}k{}-treeitem", PARENTS - 1, KIDS - 1);
        // Step until focused: the scroll to the target animates past kittest's `max_steps`.
        let focused = (0..30).any(|_| {
            h.step();
            h.node(&last).state.focused
        });
        assert!(focused, "End did not focus {last}");
    }
}

#[test]
fn nav_children_state_does_not_outlive_removed_categories() {
    let mut h = harness((3_usize, 0_usize), |ui, (categories, stored)| {
        let mut selected = None;
        Nav::new("n", "Main", &mut selected)
            .multiple(true)
            .default_open([0, 1, 2])
            .show(ui, |nav| {
                for v in 0..*categories {
                    nav.category(NavCategory::new(v, "Cat"), |nav| {
                        nav.sub_item(NavSubItem::new(100 + v, "Sub"));
                    });
                }
            });
        *stored = ui.ctx().data(egui::util::IdTypeMap::count::<Vec<usize>>);
    });
    h.run();
    h.state_mut().value.0 = 1;
    h.run();
    // The open set is the only Vec<usize> kept per nav.
    assert_eq!(h.state().value.1, 1);
}
