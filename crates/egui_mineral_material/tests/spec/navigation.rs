use egui::vec2;
use egui_mineral::{
    i18n::Direction,
    layout::{Axis, Edge},
};
use egui_mineral_material::{
    IconAction,
    icons::{ICON_ADD, ICON_EDIT, ICON_HOME, ICON_SHARE},
    navigation::{NavDestination, NavigationBar, Tab, Tabs, Toolbar, TopAppBar, TopAppBarVariant},
    spec::{
        ColorRole,
        defaults::tab,
        tokens::{
            app_bar, app_bar_large, app_bar_medium, app_bar_small, docked_toolbar, fab_baseline, fab_medium,
            floating_toolbar, navigation_bar, navigation_bar_vertical_item, primary_navigation_tab,
        },
    },
};

use crate::support::{SCREEN, min_interactive, render, render_directed};

#[test]
fn top_app_bars_take_their_container_heights() {
    for (variant, height) in [
        (TopAppBarVariant::Small, app_bar_small::CONTAINER_HEIGHT),
        (TopAppBarVariant::Medium, app_bar_medium::CONTAINER_HEIGHT),
        (TopAppBarVariant::Large, app_bar_large::CONTAINER_HEIGHT),
    ] {
        let p = render(|ui| {
            TopAppBar::new("bar", "Title").variant(variant).show(ui);
        });
        assert_eq!(p.one(app_bar::CONTAINER_COLOR).height(), height, "{variant:?}");
    }
}

#[test]
fn tab_rows_follow_compose_heights() {
    let mut sel = 0;
    let p = render(|ui| {
        Tabs::new("t", "Tabs", &mut sel)
            .item(Tab::new("A"))
            .item(Tab::new("B"))
            .show(ui);
    });
    assert_eq!(p.outer.height(), primary_navigation_tab::CONTAINER_HEIGHT);
    let indicator = p.filled(primary_navigation_tab::ACTIVE_INDICATOR_COLOR);
    assert!(
        indicator
            .iter()
            .any(|r| r.height() == primary_navigation_tab::ACTIVE_INDICATOR_HEIGHT
                && r.width() == tab::INDICATOR_MIN_WIDTH),
        "indicator floored at the minimum width: {indicator:?}"
    );
    let p = render(|ui| {
        Tabs::new("ti", "Tabs", &mut sel)
            .item(Tab::new("A").icon(ICON_HOME))
            .show(ui);
    });
    assert_eq!(p.outer.height(), tab::LARGE_HEIGHT);
}

#[test]
fn navigation_bars_take_their_container_heights() {
    let mut sel = 0;
    for (flexible, height) in [
        (false, navigation_bar::TALL_CONTAINER_HEIGHT),
        (true, navigation_bar::CONTAINER_HEIGHT),
    ] {
        let p = render(|ui| {
            NavigationBar::new(("nb", flexible), &mut sel)
                .item(NavDestination::new(ICON_HOME, "Home"))
                .item(NavDestination::new(ICON_HOME, "Away"))
                .flexible(flexible)
                .item_axis(Axis::Vertical)
                .show(ui);
        });
        assert_eq!(p.outer.height(), height, "flexible {flexible}");
        let pill = p.filled(navigation_bar::ITEM_ACTIVE_INDICATOR_COLOR);
        let size = vec2(
            navigation_bar_vertical_item::ACTIVE_INDICATOR_WIDTH,
            navigation_bar_vertical_item::ACTIVE_INDICATOR_HEIGHT,
        );
        assert!(pill.iter().any(|r| r.size() == size), "indicator in {pill:?}");
    }
}

fn actions() -> [IconAction; 2] {
    [IconAction::new(ICON_EDIT, "Edit"), IconAction::new(ICON_SHARE, "Share")]
}

#[test]
fn floating_toolbar_pads_its_actions() {
    let p = render(|ui| {
        Toolbar::floating("t", "Edit").actions(actions()).show(ui);
    });
    let pad = floating_toolbar::CONTAINER_LEADING_SPACE + floating_toolbar::CONTAINER_TRAILING_SPACE;
    assert_eq!(
        p.one(ColorRole::SurfaceContainer).size(),
        vec2(
            pad + actions().len() as f32 * min_interactive(),
            floating_toolbar::CONTAINER_HEIGHT
        ),
        "padding around the action targets"
    );
    for direction in [Direction::Ltr, Direction::Rtl] {
        let p = render_directed(direction, SCREEN, |ui| {
            Toolbar::floating("t", "Edit").actions(actions()).show(ui);
        });
        let bar = p.one(ColorRole::SurfaceContainer);
        let (a, b) = (p.rect("t-0-button"), p.rect("t-1-button"));
        let (left, right) = (a.left().min(b.left()), a.right().max(b.right()));
        assert_eq!(
            left - bar.left(),
            floating_toolbar::CONTAINER_LEADING_SPACE,
            "{direction:?} start"
        );
        assert_eq!(
            bar.right() - right,
            floating_toolbar::CONTAINER_TRAILING_SPACE,
            "{direction:?} end"
        );
    }
}

#[test]
fn docked_toolbar_spans_the_width() {
    let p = render(|ui| {
        Toolbar::docked("t", "Edit").actions(actions()).show(ui);
    });
    let bar = p.one(ColorRole::SurfaceContainer);
    assert_eq!(bar.height(), docked_toolbar::CONTAINER_HEIGHT);
    assert_eq!(bar.width(), p.outer.width());
}

#[test]
fn paired_fab_grows_as_the_toolbar_collapses() {
    for (expanded, side) in [
        (true, fab_baseline::CONTAINER_WIDTH),
        (false, fab_medium::CONTAINER_WIDTH),
    ] {
        let p = render(|ui| {
            Toolbar::floating("t", "Edit")
                .actions(actions())
                .fab(ICON_ADD, "Add", Edge::End)
                .expanded(expanded)
                .show(ui);
        });
        let fab = p.one(ColorRole::PrimaryContainer);
        assert_eq!(fab.size(), vec2(side, side), "expanded {expanded}");
        assert_eq!(p.outer.height(), fab_medium::CONTAINER_WIDTH, "FabSizeRange end");
        assert_eq!(fab.right(), p.outer.right(), "FAB at the end");
        let bars = p.filled(ColorRole::SurfaceContainer);
        assert_eq!(bars.len(), usize::from(expanded), "a collapsed toolbar is hidden");
    }
}
