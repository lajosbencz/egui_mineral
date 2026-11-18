use egui::{Key, Modifiers};
use egui_mineral::{i18n::Direction, layout::Axis, overlay::Dismiss};
use egui_mineral_material::{
    actions::PushButton,
    icons,
    navigation::{NavDestination, NavigationBar, NavigationDrawer, NavigationRail, SearchBar, SearchView, Tab, Tabs},
    selection::{RadioGroup, RadioOption},
};

use crate::support::{Harness, MineralHarness, SCREEN, WIDE, render_directed, still};

fn tabs<'a>() -> Harness<'a, usize> {
    still(SCREEN, 0, |ui, selected| {
        Tabs::new("tabs", "Letters", selected)
            .item(Tab::new("A"))
            .item(Tab::new("B").disabled(true))
            .item(Tab::new("C").mnemonic('c'))
            .show(ui);
    })
}

fn nav_set<'a>(n: usize, selected: usize) -> Harness<'a, [usize; 3]> {
    still(WIDE, [selected; 3], move |ui, [tabs, bar, rail]| {
        let items = (0..n).map(|i| NavDestination::new(icons::ICON_HOME, format!("D{i}")));
        (0..n)
            .fold(Tabs::new("tabs", "Letters", tabs), |t, i| {
                t.item(Tab::new(format!("T{i}")))
            })
            .show(ui);
        items
            .clone()
            .fold(NavigationBar::new("bar", bar), NavigationBar::item)
            .show(ui);
        items
            .fold(NavigationRail::new("rail", rail), NavigationRail::item)
            .show(ui);
    })
}

#[test]
fn empty_navigation_renders_nothing_interactive() {
    let mut h = nav_set(0, 0);
    h.run();
    for key in ["tabs-0-tab", "bar-0-nav-item", "rail-0-nav-item"] {
        assert!(h.tree().find_key(key).is_none(), "{key}");
    }
    assert_eq!(h.state().value, [0; 3]);
}

#[test]
fn stale_selection_moves_onto_the_last_item() {
    let mut h = nav_set(2, 5);
    h.run();
    assert_eq!(h.state().value, [1; 3]);
    assert_eq!(h.node("tabs-1-tab").state.selected, Some(true));
    assert_eq!(h.node("bar-1-nav-item").state.selected, Some(true));
    assert_eq!(h.node("rail-1-nav-item").state.selected, Some(true));
}

#[test]
fn sibling_groups_keep_separate_item_ids() {
    let mut h = still(SCREEN, [0usize; 4], |ui, [t0, t1, b0, b1]| {
        for (salt, sel) in [("t0", t0), ("t1", t1)] {
            Tabs::new(salt, "Letters", sel)
                .items([Tab::new("A"), Tab::new("B")])
                .show(ui);
        }
        for (salt, sel) in [("b0", b0), ("b1", b1)] {
            NavigationBar::new(salt, sel)
                .item(NavDestination::new(icons::ICON_HOME, "A"))
                .item(NavDestination::new(icons::ICON_STAR, "B"))
                .show(ui);
        }
    });
    h.click("t0-1-tab");
    h.click("b0-1-nav-item");
    assert_eq!(h.state().value, [1, 0, 1, 0]);
}

#[test]
fn scrollable_tabs_past_the_viewport_keep_their_width() {
    let mut h = still(egui::vec2(240.0, 200.0), 0, |ui, sel| {
        Tabs::new("s", "Letters", sel)
            .scrollable(true)
            .items((0..8).map(|i| Tab::new(format!("Tab {i}"))))
            .show(ui);
    });
    h.run();
    let first = h.rect("s-0-tab").width();
    assert!(first > 0.0);
    let i = 7;
    assert_eq!(h.rect(&format!("s-{i}-tab")).width(), first);
}

#[test]
fn disabled_tab_ignores_click() {
    let mut h = tabs();
    h.click("tabs-1-tab");
    assert_eq!(h.state().value, 0);
    h.click("tabs-2-tab");
    assert_eq!(h.state().value, 2);
}

#[test]
fn alt_mnemonic_selects_tab() {
    // kittest reports Nix, where Material enables mnemonics.
    let mut h = tabs();
    h.run();
    h.key_press_modifiers(Modifiers::ALT, Key::C);
    h.run();
    assert_eq!(h.state().value, 2);
}

#[test]
fn arrow_keys_skip_disabled_tab() {
    let mut h = tabs();
    h.press(Key::Tab);
    h.press(Key::ArrowRight);
    assert_eq!(h.state().value, 2);
}

#[test]
fn clicked_tab_takes_roving_focus() {
    let mut h = tabs();
    h.click("tabs-2-tab");
    assert!(h.focused("tabs-2-tab"), "a click moves the Tab stop to the clicked tab");
    h.press(Key::ArrowLeft);
    assert_eq!(
        h.state().value,
        0,
        "arrows continue from the clicked tab, past the disabled one"
    );
}

#[test]
fn modal_rail_moves_and_returns_focus() {
    let mut h = still(WIDE, (false, 1), |ui, (open, selected)| {
        NavigationRail::new("rail", selected)
            .menu_button(true)
            .item(NavDestination::new(icons::ICON_HOME, "Home"))
            .item(NavDestination::new(icons::ICON_SEARCH, "Search"))
            .show_modal(ui, open);
    });
    let focused = |h: &Harness<'_, (bool, usize)>| h.ctx.memory(egui::Memory::focused);
    h.press(Key::Tab);
    let trigger = focused(&h);
    assert!(trigger.is_some(), "Tab focuses the menu button");
    h.press(Key::Enter);
    assert!(h.state().value.0, "Enter opens the modal rail");
    let inside = focused(&h);
    assert!(inside.is_some() && inside != trigger, "focus moves into the rail");
    h.press(Key::Escape);
    assert!(!h.state().value.0, "Escape closes it");
    assert_eq!(focused(&h), trigger, "focus returns to the menu button");
}

#[test]
fn modal_rail_reports_its_surface() {
    let mut h = still(WIDE, (false, None), |ui, (open, popup)| {
        let mut selected = 0;
        *popup = NavigationRail::new("rail", &mut selected)
            .item(NavDestination::new(icons::ICON_HOME, "Home"))
            .show_modal(ui, open)
            .popup
            .map(|r| r.rect);
    });
    h.run();
    assert_eq!(h.state().value.1, None, "no surface while collapsed");
    h.state_mut().value.0 = true;
    h.run();
    assert_eq!(h.state().value.1, Some(h.rect("rail-dialog")));
}

fn tab_group<'a>(group: &'static str) -> Harness<'a, usize> {
    still(SCREEN, 0, move |ui, selected| {
        if group == "bar" {
            NavigationBar::new("bar", selected)
                .item(NavDestination::new(icons::ICON_HOME, "A"))
                .item(NavDestination::new(icons::ICON_SEARCH, "B").disabled(true))
                .item(NavDestination::new(icons::ICON_STAR, "C"))
                .show(ui);
        } else {
            RadioGroup::new("radio", "Letters", selected)
                .options([
                    RadioOption::new(0, "A"),
                    RadioOption::new(1, "B").disabled(true),
                    RadioOption::new(2, "C"),
                ])
                .axis(Axis::Horizontal)
                .show(ui);
        }
        ui.mineral(PushButton::new("after", "After"));
    })
}

#[test]
fn composite_groups_have_one_tab_stop_and_arrow_between_items() {
    for (group, item) in [("bar", "bar-{}-nav-item"), ("radio", "radio-{}-radio")] {
        let item = |i: usize| item.replace("{}", &i.to_string());
        let mut h = tab_group(group);
        h.press(Key::Tab);
        assert!(h.focused(&item(0)), "{group}: Tab enters at the selected item");
        h.press(Key::Tab);
        assert!(h.focused("after-button"), "{group}: the next Tab leaves the group");

        let mut h = tab_group(group);
        h.press(Key::Tab);
        h.press(Key::ArrowRight);
        assert!(h.focused(&item(2)), "{group}: arrow skips the disabled item");
        h.press(Key::Tab);
        assert!(h.focused("after-button"), "{group}: Tab leaves from the roving item");
    }
}

#[test]
fn open_search_view_input_is_named() {
    let mut h = still(WIDE, (true, String::new()), |ui, (open, query)| {
        SearchView::new("view", query)
            .placeholder("Find")
            .show(ui, open, |_, _| ());
    });
    h.run();
    assert!(h.state().value.0, "the view stays open");
    assert!(h.unnamed_widgets().is_empty(), "every input has an accessible name");
    assert_eq!(
        h.node("view-search").name,
        "Find",
        "the view input is named by its placeholder"
    );
}

#[test]
fn disabled_group_blocks_items_and_reports_it() {
    let mut h = still(WIDE, [0usize; 2], |ui, [tabs, bar]| {
        Tabs::new("tabs", "Letters", tabs)
            .items(["A", "B"])
            .disabled(true)
            .show(ui);
        NavigationBar::new("bar", bar)
            .item(NavDestination::new(icons::ICON_HOME, "Home"))
            .item(NavDestination::new(icons::ICON_SEARCH, "Search"))
            .disabled(true)
            .show(ui);
    });
    h.click("tabs-1-tab");
    h.click("bar-1-nav-item");
    assert_eq!(h.state().value, [0, 0], "disabled groups keep their selection");
    for key in ["tabs-tablist", "bar-navigation", "tabs-1-tab", "bar-1-nav-item"] {
        assert!(h.node(key).state.disabled, "{key} reports disabled");
    }
}

#[test]
fn items_equals_chained_item() {
    let nodes = |chained: bool| {
        let h = still(SCREEN, 0, move |ui, selected| {
            let tabs = Tabs::new("tabs", "Letters", selected);
            let tabs = if chained {
                tabs.item("A").item(Tab::new("B").disabled(true)).item("C")
            } else {
                tabs.items([Tab::new("A"), Tab::new("B").disabled(true), Tab::new("C")])
            };
            tabs.show(ui);
        });
        (0..3)
            .map(|i| {
                let n = h.node(&format!("tabs-{i}-tab"));
                (n.name.clone(), n.state.disabled)
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(nodes(false), nodes(true));
}

#[test]
fn navigation_bar_label_names_the_landmark() {
    let h = still(WIDE, [0usize; 2], |ui, [plain, named]| {
        NavigationBar::new("plain", plain)
            .item(NavDestination::new(icons::ICON_HOME, "Home"))
            .show(ui);
        NavigationBar::new("named", named)
            .label("Sections")
            .item(NavDestination::new(icons::ICON_HOME, "Home"))
            .show(ui);
    });
    assert_eq!(h.node("plain-navigation").name, "Navigation");
    assert_eq!(h.node("named-navigation").name, "Sections");
}

// (open, query, submitted, picked, dismissed)
type Search = (bool, String, u32, Vec<usize>, Vec<Dismiss>);

fn search_view<'a>() -> Harness<'a, Search> {
    let mut h = still(
        WIDE,
        (true, String::new(), 0, Vec::new(), Vec::new()),
        |ui, (open, query, submitted, picked, dismissed)| {
            let r = SearchView::new("view", query)
                .suggestions(["Apple", "Banana", "Cherry"])
                .show(ui, open, |_, _| ());
            *submitted += u32::from(r.submitted);
            picked.extend(r.selected);
            dismissed.extend(r.dismissed);
        },
    );
    h.click("view-search");
    assert!(h.state().value.0, "the view stays open");
    h
}

#[test]
fn search_view_enter_submits_the_typed_query() {
    let mut h = search_view();
    h.type_text("ban");
    h.press(Key::Enter);
    let (open, query, submitted, picked, _) = &h.state().value;
    assert_eq!((query.as_str(), *submitted, picked.len()), ("ban", 1, 0));
    assert!(*open, "submitting keeps the view open");
}

#[test]
fn search_view_pick_fills_the_query_and_closes() {
    let mut h = search_view();
    h.type_text("ch");
    h.click("view-2-suggestion");
    let (open, query, submitted, picked, dismissed) = &h.state().value;
    assert_eq!((query.as_str(), *submitted), ("Cherry", 0));
    assert_eq!(picked, &[2], "the suggestion index, not the row");
    assert!(!*open && dismissed.is_empty(), "a pick closes without a dismissal");
}

#[test]
fn search_view_arrow_enter_picks_the_active_row() {
    let mut h = search_view();
    h.type_text("an");
    h.press(Key::ArrowDown);
    h.press(Key::Enter);
    let (open, query, submitted, picked, _) = &h.state().value;
    assert_eq!((query.as_str(), *submitted, picked.as_slice()), ("Banana", 0, &[1][..]));
    assert!(!*open);
}

#[test]
fn search_view_escape_and_outside_press_dismiss() {
    let mut h = search_view();
    h.press(Key::Escape);
    assert!(!h.state().value.0, "Escape closes");
    assert_eq!(h.state().value.4, [Dismiss::Escape]);
    h.state_mut().value.0 = true;
    h.state_mut().value.4.clear();
    h.run();
    h.click_at(h.outside());
    assert!(!h.state().value.0, "an outside press closes");
    assert_eq!(h.state().value.4, [Dismiss::Outside]);
}

#[test]
fn search_view_opened_from_the_bar_takes_keys_in_the_view() {
    let mut h = still(WIDE, (false, String::new(), Vec::new()), |ui, (open, query, picked)| {
        let r = SearchView::new("view", query)
            .suggestions(["Apple", "Banana", "Cherry"])
            .show(ui, open, |_, _| ());
        picked.extend(r.selected);
    });
    h.click("view-search");
    assert!(h.state().value.0, "the bar opens the view");
    h.type_text("an");
    h.press(Key::ArrowDown);
    h.press(Key::Enter);
    assert_eq!(
        h.state().value.2,
        [1],
        "the release does not pull focus back to the bar"
    );
}

#[test]
fn drawer_section_titles_start_at_the_reading_start() {
    for (direction, start) in [(Direction::Ltr, true), (Direction::Rtl, false)] {
        let pass = render_directed(direction, WIDE, |ui| {
            NavigationDrawer::new("nav", &mut 0)
                .section("Mail")
                .item(NavDestination::new(icons::ICON_HOME, "Inbox"))
                .show(ui);
        });
        let drawer = pass.rect("nav-navigation");
        assert_eq!(
            pass.text("Mail").center().x < drawer.center().x,
            start,
            "{direction:?}: the title sits on the reading-start side"
        );
    }
}

#[test]
fn search_back_arrows_point_along_the_reading_direction() {
    for (direction, back) in [
        (Direction::Ltr, icons::ICON_ARROW_BACK),
        (Direction::Rtl, icons::ICON_ARROW_FORWARD),
    ] {
        let bar = render_directed(direction, WIDE, |ui| {
            ui.mineral(SearchBar::new("bar", &mut String::new()).back_button(true));
        });
        bar.text(back.codepoint);
        let view = render_directed(direction, WIDE, |ui| {
            SearchView::new("view", &mut String::new()).show(ui, &mut true, |_, _| ());
        });
        view.text(back.codepoint);
    }
}
