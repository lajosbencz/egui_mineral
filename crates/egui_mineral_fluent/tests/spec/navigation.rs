use egui_mineral::{
    expand::Expansion,
    layout::Axis,
    select::{Selection, SelectionMode},
};
use egui_mineral_fluent::{
    FluentExt, Size,
    actions::PushButton,
    data::Avatar,
    icons::{ICON_CUT, ICON_HOME, ICON_MORE_HORIZONTAL, ICON_PERSON, ICON_PIN},
    navigation::{
        Breadcrumb, BreadcrumbButton, Carousel, Menu, MenuGrid, MenuGridAction, MenuGridItem, MenuGridRow, Nav,
        NavCategory, NavDensity, NavDrawer, NavItem, NavSubItem, Tab, TabAppearance, TabList, Toolbar,
        ToolbarToggleButton, Tree, TreeItem, TreeItemLayout, TreeItemPersonaLayout,
    },
    spec::ColorToken,
    spec::defaults::{carousel as cdef, menu, menu_grid as mgdef, nav as ndef, tab, toolbar, tree as tdef},
    typography::TypeRamp,
};

use crate::support::{FluentUi, Pass, render};

fn list(size: Size, appearance: TabAppearance, vertical: bool) -> impl FnMut(&mut FluentUi<'_>) {
    let mut selected = 0;
    move |ui| {
        TabList::new("t", "Tabs", &mut selected)
            .items(["One", "Two"].into_iter().enumerate())
            .size(size)
            .appearance(appearance)
            .axis(if vertical { Axis::Vertical } else { Axis::Horizontal })
            .show(ui);
    }
}

fn tab_rect(p: &Pass, key: &str) -> egui::Rect {
    p.rt.tree(egui::ViewportId::ROOT)
        .expect("semantics collected")
        .find_key(key)
        .expect("tab node")
        .rect
}

#[test]
fn tab_height_follows_the_size_padding() {
    for (size, height) in [(Size::Small, 32.0), (Size::Medium, 44.0), (Size::Large, 54.0)] {
        let p = render(list(size, TabAppearance::Transparent, false));
        let r = tab_rect(&p, "t-0-tab");
        assert!((r.height() - height).abs() < 0.5, "{size:?}: {r:?}");
    }
}

#[test]
fn selection_indicator_is_a_brand_bar_under_the_selected_tab() {
    for (size, thickness) in [(Size::Small, 2.0), (Size::Medium, 3.0), (Size::Large, 3.0)] {
        let p = render(list(size, TabAppearance::Transparent, false));
        let t = p.rt.tokens();
        let bar = p.one(t.color(ColorToken::ColorCompoundBrandStroke));
        assert!((bar.height() - thickness).abs() < f32::EPSILON, "{size:?}: {bar:?}");
    }
}

#[test]
fn vertical_indicator_is_a_bar_on_the_start_edge() {
    let p = render(list(Size::Medium, TabAppearance::Transparent, true));
    let t = p.rt.tokens();
    let bar = p.one(t.color(ColorToken::ColorCompoundBrandStroke));
    assert!((bar.width() - 3.0).abs() < f32::EPSILON, "{bar:?}");
}

#[test]
fn circular_appearances_have_no_indicator_and_fill_the_selected_tab() {
    for (appearance, token) in [
        (TabAppearance::SubtleCircular, ColorToken::ColorBrandBackground2),
        (TabAppearance::FilledCircular, ColorToken::ColorBrandBackground),
    ] {
        let p = render(list(Size::Medium, appearance, false));
        let t = p.rt.tokens();
        assert!(
            p.fills(t.color(ColorToken::ColorCompoundBrandStroke))
                .iter()
                .all(|r| r.height() > 3.0)
        );
        assert_eq!(p.fills(t.color(token)).len(), 1, "{appearance:?}");
    }
}

#[test]
fn icon_slot_adds_its_size_and_gap() {
    let with_icon = render(|ui| {
        let mut s = 0;
        TabList::new("t", "Tabs", &mut s)
            .item(0, Tab::new("One").icon(ICON_PERSON))
            .show(ui);
    });
    let plain = render(|ui| {
        let mut s = 0;
        TabList::new("t", "Tabs", &mut s).item(0, "One").show(ui);
    });
    let gap = egui_mineral_fluent::spec::tokens::geometry::SPACING_HORIZONTAL_S_NUDGE;
    let extra = tab_rect(&with_icon, "t-0-tab").width() - tab_rect(&plain, "t-0-tab").width();
    assert!((extra - tab::ICON_SIZE - gap).abs() < 0.5, "{extra}");
    let t = with_icon.rt.tokens();
    assert_eq!(
        with_icon.text_color("One"),
        t.color(ColorToken::ColorNeutralForeground1)
    );
}

#[test]
fn menu_rows_are_the_item_height_inside_the_minimum_width() {
    let mut open = true;
    let p = render(|ui| {
        let rt = ui.rt();
        let trigger = ui.mineral(PushButton::new("trig", "Open")).response().clone();
        Menu::new("m", "Actions").show(rt, &trigger, &mut open, |m| {
            m.item("Cut");
        });
    });
    let tree = p.rt.tree(egui::ViewportId::ROOT).expect("semantics collected");
    let row = tree.find_key("m-0-menuitem").expect("row").rect;
    let menu = tree.find_key("m-menu").expect("menu").rect;
    assert!((row.height() - menu::ITEM_MIN_HEIGHT).abs() < 0.5, "{row:?}");
    assert!(menu.width() >= menu::POPOVER_MIN_WIDTH - 0.5, "{menu:?}");
}

#[test]
fn toolbar_padding_follows_the_size() {
    for (size, block, inline) in [
        (Size::Small, 0.0, 4.0),
        (Size::Medium, 4.0, 8.0),
        (Size::Large, 4.0, 20.0),
    ] {
        let p = render(|ui| {
            Toolbar::new("tb", "Tools").size(size).action("Cut").show(ui);
        });
        let button = tab_rect(&p, "tb-0-button");
        let bar = tab_rect(&p, "tb-toolbar");
        assert!(
            (bar.height() - button.height() - 2.0 * block).abs() < 0.5,
            "{size:?}: {bar:?} {button:?}"
        );
        assert!(
            (bar.width() - button.width() - 2.0 * inline).abs() < 0.5,
            "{size:?}: {bar:?} {button:?}"
        );
    }
}

#[test]
fn toolbar_checked_toggle_uses_the_selected_subtle_fill() {
    let mut on = true;
    let p = render(|ui| {
        Toolbar::new("tb", "Tools")
            .action("Cut")
            .toggle(ToolbarToggleButton::new("Bold", &mut on))
            .show(ui);
    });
    let t = p.rt.tokens();
    assert_eq!(p.fills(t.color(ColorToken::ColorSubtleBackgroundSelected)).len(), 1);
    assert_eq!(p.fills(t.color(ColorToken::ColorNeutralBackground1)).len(), 0);
}

#[test]
fn toolbar_divider_adds_its_padding_on_both_sides() {
    let plain = render(|ui| {
        Toolbar::new("tb", "Tools").action("Cut").action("Copy").show(ui);
    });
    let divided = render(|ui| {
        Toolbar::new("tb", "Tools")
            .action("Cut")
            .divider()
            .action("Copy")
            .show(ui);
    });
    let extra = tab_rect(&divided, "tb-toolbar").width() - tab_rect(&plain, "tb-toolbar").width();
    let line = egui_mineral_fluent::spec::tokens::geometry::STROKE_WIDTH_THIN;
    assert!((extra - 2.0 * toolbar::DIVIDER_PADDING - line).abs() < 0.5, "{extra}");
}

fn trail(size: Size) -> impl FnMut(&mut FluentUi<'_>) {
    move |ui| {
        Breadcrumb::new("bc", "Path")
            .size(size)
            .item(BreadcrumbButton::new("Home").icon(ICON_HOME))
            .item("Docs")
            .item(BreadcrumbButton::new("Page").current(true))
            .show(ui);
    }
}

#[test]
fn breadcrumb_buttons_have_the_size_height() {
    for (size, height) in [(Size::Small, 24.0), (Size::Medium, 32.0), (Size::Large, 40.0)] {
        let p = render(trail(size));
        for key in ["bc-0-button", "bc-1-button", "bc-2-button"] {
            let r = tab_rect(&p, key);
            assert!((r.height() - height).abs() < 0.5, "{size:?} {key}: {r:?}");
        }
    }
}

#[test]
fn breadcrumb_dividers_take_the_icon_size_between_items() {
    for (size, icon) in [(Size::Small, 12.0), (Size::Medium, 16.0), (Size::Large, 20.0)] {
        let p = render(trail(size));
        let (a, b) = (tab_rect(&p, "bc-0-button"), tab_rect(&p, "bc-1-button"));
        assert!((b.left() - a.right() - icon).abs() < 0.5, "{size:?}: {a:?} {b:?}");
    }
}

#[test]
fn breadcrumb_icon_slot_adds_its_size_and_gap() {
    let p = render(|ui| {
        Breadcrumb::new("bc", "Path")
            .item(BreadcrumbButton::new("Home").icon(ICON_HOME))
            .show(ui);
    });
    // Padding and border, icon and its gap, then the label.
    let expected = 2.0 * 6.0 + 2.0 + 16.0 + 4.0 + p.text("Home").width();
    let width = tab_rect(&p, "bc-0-button").width();
    assert!((width - expected).abs() < 0.5, "{width} {expected}");
}

#[test]
fn breadcrumb_current_item_is_strong_and_ignores_hover_colors() {
    let p = render(trail(Size::Medium));
    let t = p.rt.tokens();
    assert_eq!(p.text_color("Docs"), t.color(ColorToken::ColorNeutralForeground2));
    assert_eq!(p.text_color("Page"), t.color(ColorToken::ColorNeutralForeground2));
    let strong = p.text("Page").height();
    assert!((strong - 20.0).abs() < 1.5, "{strong}");
}

fn nav(density: NavDensity, open: bool) -> impl FnMut(&mut FluentUi<'_>) {
    let mut selected = Some(11);
    move |ui| {
        let mut n = Nav::optional("n", "Main", &mut selected).density(density);
        if open {
            n = n.default_open([10]);
        }
        n.show(ui, |nav| {
            nav.item(NavItem::new(1, "Home").icon(ICON_HOME));
            nav.category(NavCategory::new(10, "Docs"), |nav| {
                nav.sub_item(NavSubItem::new(11, "Guide"));
            });
        });
    }
}

#[test]
fn nav_rows_follow_the_density_height() {
    for (density, height) in [(NavDensity::Medium, 40.0), (NavDensity::Small, 28.0)] {
        let p = render(nav(density, true));
        for key in ["n-1-navitem", "n-10-navcategory", "n-11-navsubitem"] {
            let r = tab_rect(&p, key);
            assert!((r.height() - height).abs() < 0.5, "{density:?} {key}: {r:?}");
        }
    }
}

#[test]
fn nav_sub_items_indent_and_icons_reserve_their_slot() {
    let p = render(nav(NavDensity::Medium, true));
    let sub = tab_rect(&p, "n-11-navsubitem");
    assert!((p.text("Guide").left() - sub.left() - ndef::SUB_ITEM_INDENT).abs() < 0.5);
    let home = tab_rect(&p, "n-1-navitem");
    let icon_and_gap = ndef::ICON_SIZE + egui_mineral_fluent::spec::tokens::geometry::SPACING_VERTICAL_L;
    assert!((p.text("Home").left() - home.left() - 10.0 - icon_and_gap).abs() < 0.5);
    let docs = tab_rect(&p, "n-10-navcategory");
    assert!((p.text("Docs").left() - docs.left() - 10.0).abs() < 0.5);
}

#[test]
fn nav_selected_sub_item_carries_the_indicator_bar() {
    let p = render(nav(NavDensity::Medium, true));
    let t = p.rt.tokens();
    let bar = p.one(t.color(ColorToken::ColorCompoundBrandForeground1));
    assert!((bar.width() - ndef::INDICATOR_WIDTH).abs() < f32::EPSILON, "{bar:?}");
    assert!((bar.height() - ndef::INDICATOR_HEIGHT).abs() < f32::EPSILON, "{bar:?}");
    let sub = tab_rect(&p, "n-11-navsubitem");
    assert!((bar.left() - (sub.left() + ndef::SUB_ITEM_INDENT - ndef::INDICATOR_OFFSET)).abs() < 0.5);
}

#[test]
fn nav_rows_fill_with_the_nav_background() {
    let p = render(nav(NavDensity::Medium, true));
    let t = p.rt.tokens();
    assert_eq!(p.fills(t.color(ColorToken::ColorNeutralBackground4)).len(), 3);
}

#[test]
fn nav_drawer_is_260_wide_with_body_padding() {
    let p = render(|ui| {
        let mut selected = None;
        NavDrawer::optional("n", "Main", &mut selected).show(ui, |nav| {
            nav.item(NavItem::new(1, "Home"));
        });
    });
    let row = tab_rect(&p, "n-1-navitem");
    let (end, start) = ndef::BODY_PADDING;
    assert!(
        (row.width() - (ndef::DRAWER_WIDTH - start - end)).abs() < 0.5,
        "{row:?}"
    );
    assert!((row.left() - start).abs() < 0.5, "{row:?}");
}

fn tree_items() -> Vec<TreeItem> {
    vec![
        TreeItem::new("a", "Alpha").nested([TreeItem::new("a1", "Apple")]),
        TreeItem::new("b", "Beta").layout(TreeItemLayout::new().icon_before(ICON_HOME).aside("3")),
        TreeItem::new("c", "Gamma").layout(
            TreeItemPersonaLayout::new()
                .media(Avatar::new("p", "Pat Lee"))
                .secondary_text("Desc"),
        ),
        TreeItem::new("d", "Delta").layout(TreeItemPersonaLayout::new()),
    ]
}

fn tree_pass(size: Size, mode: Option<SelectionMode>) -> Pass {
    let items = tree_items();
    let mut open = Expansion::default();
    open.expand("a".to_owned());
    let mut checked = match mode {
        Some(SelectionMode::Multiple) => Selection::multiple(),
        _ => Selection::single(),
    }
    .with(["b".to_owned()]);
    render(move |ui| {
        let mut tree = Tree::items("t", "Files", &items, &mut open).size(size);
        if mode.is_some() {
            tree = tree.selection(&mut checked);
        }
        tree.show(ui);
    })
}

#[test]
fn tree_rows_are_the_size_height_with_the_row_gap() {
    for (size, height) in [(Size::Small, tdef::MIN_HEIGHT_SMALL), (Size::Medium, tdef::MIN_HEIGHT)] {
        let p = tree_pass(size, None);
        let (a, a1) = (tab_rect(&p, "t-a-treeitem"), tab_rect(&p, "t-a1-treeitem"));
        assert!((a.height() - height).abs() < 0.5, "{size:?}: {a:?}");
        assert!(
            (a1.top() - a.bottom() - tdef::ROW_GAP).abs() < 0.5,
            "{size:?}: {a:?} {a1:?}"
        );
    }
}

#[test]
fn tree_persona_rows_grow_with_the_description() {
    let p = tree_pass(Size::Medium, None);
    let described = tab_rect(&p, "t-c-treeitem").height();
    let plain = tab_rect(&p, "t-d-treeitem").height();
    let (block, ..) = tdef::PERSONA_MAIN_PADDING;
    let body = TypeRamp::Body1.line_height();
    assert!((plain - (2.0 * block + body)).abs() < 0.5, "{plain}");
    assert!(
        (described - (2.0 * block + body + TypeRamp::Caption1.line_height())).abs() < 0.5,
        "{described}"
    );
}

#[test]
fn tree_labels_indent_one_level_per_depth_and_leaves_skip_the_chevron_slot() {
    let p = tree_pass(Size::Medium, None);
    let branch = p.text("Alpha").left() - tab_rect(&p, "t-a-treeitem").left();
    let child = p.text("Apple").left() - tab_rect(&p, "t-a1-treeitem").left();
    let leaf = p.text("Beta").left() - tab_rect(&p, "t-b-treeitem").left();
    assert!(
        (branch - (tdef::EXPAND_ICON_WIDTH + tdef::MAIN_PADDING)).abs() < 0.5,
        "{branch}"
    );
    assert!((child - branch - tdef::LEVEL_INDENT).abs() < 0.5, "{child}");
    assert!(
        (leaf - (tdef::LEVEL_INDENT + tdef::MAIN_PADDING + tdef::ICON_SIZE + tdef::ICON_GAP)).abs() < 0.5,
        "{leaf}"
    );
}

#[test]
fn tree_selectors_take_the_checkbox_slot_and_fill_when_checked() {
    let off = tree_pass(Size::Medium, None);
    let on = tree_pass(Size::Medium, Some(SelectionMode::Multiple));
    let slot = tdef::SELECTOR_INDICATOR + 2.0 * tdef::SELECTOR_MARGIN;
    let shift = on.text("Alpha").left() - off.text("Alpha").left();
    assert!((shift - slot).abs() < 0.5, "{shift}");
    let t = on.rt.tokens();
    let fill = on.one(t.color(ColorToken::ColorCompoundBrandBackground));
    assert!((fill.width() - tdef::SELECTOR_INDICATOR).abs() < 0.5, "{fill:?}");
}

#[test]
fn tree_aside_sits_at_the_end_of_the_row() {
    let p = tree_pass(Size::Medium, None);
    let row = tab_rect(&p, "t-b-treeitem");
    let aside = p.text("3");
    assert!(
        (row.right() - tdef::ASIDE_PADDING - aside.right()).abs() < 0.5,
        "{row:?} {aside:?}"
    );
}

fn grid_pass() -> Pass {
    render(|ui| {
        let trigger = ui.mineral(PushButton::new("trig", "Open")).into_response();
        MenuGrid::new("g", "Actions").show(ui.rt(), &trigger, &mut true, |g| {
            g.row(
                MenuGridRow::new()
                    .cell(MenuGridItem::new("Cut").icon(ICON_CUT).secondary_text("Ctrl+X"))
                    .cell(MenuGridAction::new("More", ICON_MORE_HORIZONTAL)),
            );
            g.row(
                MenuGridRow::new()
                    .cell("Copy")
                    .cell(MenuGridAction::new("Pin", ICON_PIN)),
            );
        });
    })
}

#[test]
fn menu_grid_rows_hold_the_cells_with_padding_and_the_row_gap() {
    let p = grid_pass();
    let (r0, r1) = (tab_rect(&p, "g-0-gridrow"), tab_rect(&p, "g-1-gridrow"));
    let (block, inline) = mgdef::ROW_PADDING;
    let tall = 2.0 * block + TypeRamp::Body1.line_height() + TypeRamp::Caption2.line_height();
    assert!((r0.height() - tall).abs() < 0.5, "{r0:?}");
    assert!((r1.height() - mgdef::ROW_MIN_HEIGHT).abs() < 0.5, "{r1:?}");
    assert!((r1.top() - r0.bottom() - mgdef::GRID_GAP).abs() < 0.5, "{r0:?} {r1:?}");
    let min = egui_mineral_fluent::spec::defaults::menu::POPOVER_MIN_WIDTH
        - 2.0 * egui_mineral_fluent::spec::defaults::menu::POPOVER_PADDING;
    assert!(
        r0.width() >= min - 0.5 && r0.width() <= mgdef::ROW_MAX_WIDTH + 0.5,
        "sized from content: {r0:?}"
    );
    let (item, action) = (tab_rect(&p, "g-0-0-gridcell"), tab_rect(&p, "g-0-1-gridcell"));
    assert!((item.left() - r0.left() - inline).abs() < 0.5, "{item:?}");
    assert!((action.width() - mgdef::ACTION_WIDTH).abs() < 0.5, "{action:?}");
    assert!(
        (action.left() - item.right() - mgdef::ROW_GAP).abs() < 0.5,
        "{item:?} {action:?}"
    );
    assert!((r0.right() - inline - action.right()).abs() < 0.5, "{r0:?} {action:?}");
}

#[test]
fn menu_grid_rows_fill_with_the_menu_surface_color_inside_the_popover_padding() {
    let p = grid_pass();
    let t = p.rt.tokens();
    let fills = p.fills(t.color(ColorToken::ColorNeutralBackground1));
    let (r0, r1) = (tab_rect(&p, "g-0-gridrow"), tab_rect(&p, "g-1-gridrow"));
    assert!(fills.contains(&r0) && fills.contains(&r1), "{fills:?}");
    let surface = r0
        .union(r1)
        .expand(egui_mineral_fluent::spec::defaults::menu::POPOVER_PADDING);
    assert!(
        fills
            .iter()
            .any(|f| (f.min - surface.min).length() < 0.5 && (f.max - surface.max).length() < 0.5),
        "{surface:?} in {fills:?}"
    );
}

fn carousel_pass(active: usize) -> Pass {
    let mut active = active;
    render(move |ui| {
        Carousel::new("c", "Gallery", &mut active).show(ui, |c| {
            for (i, name) in ["One", "Two", "Three"].into_iter().enumerate() {
                c.card(i, name, |ui| {
                    ui.set_height(100.0);
                });
            }
        });
    })
}

#[test]
fn carousel_dots_are_square_tap_areas_and_the_selected_one_is_a_wide_pill() {
    let p = carousel_pass(1);
    let side = cdef::DOT + 2.0 * cdef::DOT_PADDING;
    for key in ["c-0-carouselnav", "c-1-carouselnav", "c-2-carouselnav"] {
        let r = tab_rect(&p, key);
        assert!(
            (r.width() - side).abs() < 0.5 && (r.height() - side).abs() < 0.5,
            "{key}: {r:?}"
        );
    }
    let t = p.rt.tokens();
    let selected = p.one(t.color(ColorToken::ColorNeutralForeground1));
    assert!(
        (selected.width() - cdef::SELECTED_DOT_WIDTH).abs() < 0.5,
        "{selected:?}"
    );
    assert!((selected.height() - cdef::DOT).abs() < 0.5, "{selected:?}");
    let dimmed = t
        .color(ColorToken::ColorNeutralForeground1)
        .gamma_multiply(cdef::UNSELECTED_OPACITY[0]);
    let others = p.fills(dimmed);
    assert_eq!(others.len(), 2);
    assert!(others.iter().all(|r| (r.width() - cdef::DOT).abs() < 0.5), "{others:?}");
}

#[test]
fn carousel_navigation_sits_below_the_card_by_the_margin() {
    let p = carousel_pass(0);
    let card = tab_rect(&p, "c-0-carouselcard");
    let dot = tab_rect(&p, "c-previous-button");
    assert!((card.height() - 100.0).abs() < 0.5, "{card:?}");
    assert!(
        (dot.top() - card.bottom() - cdef::NAV_MARGIN_TOP).abs() < 0.5,
        "{card:?} {dot:?}"
    );
}

#[test]
fn menu_split_row_is_the_main_part_plus_the_trigger_with_a_separator() {
    let mut open = true;
    let p = render(|ui| {
        let rt = ui.rt();
        let trigger = ui.mineral(PushButton::new("trig", "Open")).response().clone();
        Menu::new("m", "Actions").show(rt, &trigger, &mut open, |m| {
            m.item("Cut");
            m.split_item("Paste", |m| {
                m.item("Plain");
            });
        });
    });
    let (cut, main, split) = (
        tab_rect(&p, "m-0-menuitem"),
        tab_rect(&p, "m-1-menuitem"),
        tab_rect(&p, "m-split-1-menusplit"),
    );
    assert!((split.width() - menu::SPLIT_TRIGGER_WIDTH).abs() < 0.5, "{split:?}");
    assert!((main.right() - split.left()).abs() < 0.5, "{main:?} {split:?}");
    assert!(
        (main.width() + split.width() - cut.width()).abs() < 0.5,
        "{main:?} {split:?} {cut:?}"
    );
    assert!(
        (main.height() - menu::ITEM_MIN_HEIGHT).abs() < 0.5 && (split.height() - menu::ITEM_MIN_HEIGHT).abs() < 0.5
    );
    let t = p.rt.tokens();
    let line = p.one(t.color(ColorToken::ColorNeutralStroke1));
    assert!(
        (line.height() - menu::SPLIT_SEPARATOR_HEIGHT).abs() < 0.5 && (line.width() - 1.0).abs() < 0.5,
        "{line:?}"
    );
}
