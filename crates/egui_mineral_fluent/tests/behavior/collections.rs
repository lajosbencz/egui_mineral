use egui::Key;
use egui_mineral::{expand::Expansion, select::Selection, semantic::Role};
use egui_mineral_fluent::{
    data::{List, ListItem, ListNavigation},
    surfaces::{Accordion, AccordionItem, AccordionNavigation, Card},
};

use crate::support::{FluentHarness, Harness, harness, render};

#[test]
fn selectable_card_toggles_on_click() {
    let mut h = harness(false, |ui, sel| {
        Card::new("c").selectable(sel).show(ui, |c| {
            c.body(|ui| {
                ui.label("Body");
            });
        });
    });
    h.run();
    assert_eq!(h.node("c-card").role, Role::Group);
    assert_eq!(h.node("c-card").state.selected, Some(false));
    h.click("c-card");
    assert!(h.state().value);
    assert_eq!(h.node("c-card").state.selected, Some(true));
}

fn accordion_harness(multiple: bool, collapsible: bool) -> Harness<'static, Expansion<usize>> {
    let open = if multiple {
        Expansion::default()
    } else {
        Expansion::single()
    };
    harness(open, move |ui, open| {
        Accordion::new("a", open)
            .collapsible(collapsible)
            .navigation(Some(AccordionNavigation::Linear))
            .show(ui, |a| {
                for i in 0..3 {
                    a.item(i, AccordionItem::new(format!("Item {i}")), |ui| {
                        ui.label("Panel");
                    });
                }
            });
    })
}

#[test]
fn single_accordion_keeps_one_item_open() {
    let mut h = accordion_harness(false, false);
    h.run();
    h.click("a-item-0-header");
    assert!(h.state().value.is_expanded(&0));
    h.click("a-item-1-header");
    assert!(!h.state().value.is_expanded(&0));
    assert!(h.state().value.is_expanded(&1));
    h.click("a-item-1-header");
    assert!(
        h.state().value.is_expanded(&1),
        "not collapsible keeps the last open item"
    );
    assert_eq!(h.node("a-item-1-header").state.expanded, Some(true));
}

#[test]
fn collapsible_multiple_accordion_opens_and_closes_freely() {
    let mut h = accordion_harness(true, true);
    h.run();
    h.click("a-item-0-header");
    h.click("a-item-1-header");
    assert_eq!(h.state().value.expanded().count(), 2);
    h.click("a-item-0-header");
    h.click("a-item-1-header");
    assert_eq!(h.state().value.expanded().count(), 0);
}

#[test]
fn arrow_keys_move_between_headers() {
    let mut h = accordion_harness(true, true);
    h.run();
    h.click("a-item-0-header");
    let before = h.ctx.memory(egui::Memory::focused);
    h.press(Key::ArrowDown);
    assert_ne!(h.ctx.memory(egui::Memory::focused), before);
}

#[test]
fn multiple_list_toggles_and_extends_by_shift_arrows() {
    let mut h = list_harness(true);
    h.run();
    let selected = |h: &ListHarness| h.state().value.0.selected().copied().collect::<Vec<_>>();
    for key in ["l-item-0-item", "l-item-2-item"] {
        let at = item_body(&h, key);
        h.click_at(at);
    }
    assert_eq!(selected(&h), [0, 2], "clicks toggle");
    h.press(Key::Tab);
    h.press(Key::ArrowDown);
    h.press(Key::ArrowDown);
    assert!(h.focused("l-item-2-item"), "arrows move focus");
    assert_eq!(selected(&h), [0, 2], "and only focus");
    h.press(Key::Enter);
    assert_eq!(selected(&h), [0], "the item action toggles by default");
    assert_eq!(h.state().value.1, Some(2));
    h.press(Key::Space);
    h.key_press_modifiers(egui::Modifiers::SHIFT, Key::ArrowDown);
    h.run();
    assert_eq!(selected(&h), [0, 2, 3]);
    h.press(Key::Space);
    assert_eq!(selected(&h), [0, 2], "Space toggles");
    h.key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
    h.run();
    assert_eq!(selected(&h), [0, 1, 2, 3]);
}

#[test]
fn an_own_item_action_leaves_selection_to_space() {
    let mut h = list_harness(false);
    h.run();
    let at = item_body(&h, "l-item-1-item");
    h.click_at(at);
    assert!(h.state().value.0.is_empty(), "the click runs only the action");
    assert_eq!(h.state().value.1, Some(1));
    h.press(Key::Tab);
    h.press(Key::Enter);
    assert!(h.state().value.0.is_empty());
    h.press(Key::Space);
    assert_eq!(h.state().value.0.len(), 1);
}

type ListHarness = Harness<'static, (Selection<usize>, Option<usize>)>;

// Past the checkmark, which takes focus of its own.
fn item_body(h: &ListHarness, key: &str) -> egui::Pos2 {
    h.rect(key).right_center() - egui::vec2(8.0, 0.0)
}

fn list_harness(action_selects: bool) -> ListHarness {
    harness((Selection::multiple(), None), move |ui, (sel, act)| {
        let out = List::new("l")
            .navigation(Some(ListNavigation::Items))
            .selection(sel)
            .action_selects(action_selects)
            .show(ui, |l| {
                for i in 0..4 {
                    l.item(i, format!("Row {i}"), |ui| {
                        ui.add(egui::Label::new(format!("Row {i}")).selectable(false));
                    });
                }
            });
        *act = out.activated.or(*act);
    })
}

#[test]
fn single_list_selects_one_item_by_click() {
    let mut h = harness(Selection::single(), |ui, sel: &mut Selection<usize>| {
        List::new("l")
            .navigation(Some(ListNavigation::Items))
            .selection(sel)
            .show(ui, |l| {
                for i in 0..3 {
                    l.item(i, format!("Row {i}"), |ui| {
                        ui.label(format!("Row {i}"));
                    });
                }
            });
    });
    h.run();
    assert_eq!(h.node("l-list").role, Role::ListBox);
    h.click("l-item-1-item");
    assert!(h.state().value.is_selected(&1));
    h.click("l-item-2-item");
    assert!(h.state().value.is_selected(&2));
    assert!(!h.state().value.is_selected(&1));
    assert_eq!(h.node("l-item-2-item").state.selected, Some(true));
}

#[test]
fn image_takes_its_natural_size_or_the_explicit_box() {
    use egui::{TextureId, load::SizedTexture, vec2};
    use egui_mineral_fluent::data::{Image, ImageFit};

    let source = || SizedTexture::new(TextureId::Managed(1), vec2(40.0, 20.0));
    let mut h = harness((), move |ui, ()| {
        ui.mineral(Image::new("natural", source()).alt("Natural"));
        ui.mineral(
            Image::new("boxed", source())
                .alt("Boxed")
                .fit(ImageFit::Cover)
                .size(vec2(30.0, 30.0)),
        );
    });
    h.run();
    assert_eq!(h.node("natural-image").role, Role::Image);
    assert_eq!(h.rect("natural-image").size(), vec2(40.0, 20.0));
    assert_eq!(h.rect("boxed-image").size(), vec2(30.0, 30.0));
}

#[test]
fn salted_items_keep_their_agent_key_when_an_item_is_inserted_before() {
    use egui_mineral_fluent::inputs::{RadioGroup, RadioOption};

    let mut h = harness(false, |ui, before| {
        List::new("l").show(ui, |l| {
            if *before {
                l.item(0, "Extra", |_| ());
            }
            l.item(1, ListItem::new("Keep").salt("keep"), |_| ());
        });
        Accordion::new("a", &mut Expansion::single()).show(ui, |a| {
            if *before {
                a.item(0, AccordionItem::new("Extra"), |_| ());
            }
            a.item(1, AccordionItem::new("Keep").salt("keep"), |_| ());
        });
        let mut picked = 0_u8;
        let mut options = Vec::new();
        if *before {
            options.push((0_u8, RadioOption::new("Extra")));
        }
        options.push((1_u8, RadioOption::new("Keep").salt("keep")));
        RadioGroup::new("g", "Group", &mut picked).items(options).show(ui);
    });
    h.run();
    let keys = ["keep-item", "keep-header", "keep-radio"];
    let before: Vec<_> = keys.iter().map(|k| h.node(k).id).collect();
    h.state_mut().value = true;
    h.run();
    h.run();
    let after: Vec<_> = keys.iter().map(|k| h.node(k).id).collect();
    assert_eq!(before, after);
}

mod avatar_group {
    use egui::Key;
    use egui_mineral::semantic::Role;
    use egui_mineral_fluent::data::{Avatar, AvatarGroup, AvatarGroupLayout};

    use crate::support::{FluentHarness, Harness, harness};

    fn group(layout: AvatarGroupLayout, count: usize) -> Harness<'static, ()> {
        harness((), move |ui, ()| {
            let mut g = AvatarGroup::new("g", "Team").layout(layout).max_inline_items(3);
            for i in 0..count {
                g = g.item(Avatar::new(("m", i), format!("Member {i}")));
            }
            g.show(ui);
        })
    }

    #[test]
    fn overflow_button_shows_the_rest_in_a_popover() {
        let mut h = group(AvatarGroupLayout::Spread, 5);
        h.run();
        let trigger = h.node("g-overflow");
        assert_eq!(trigger.role, Role::Button);
        assert_eq!(h.node("g-avatar-group").role, Role::Group);
        assert!(h.tree().find_key("m-0-avatar").is_none(), "the first items overflow");
        h.click("g-overflow");
        assert_eq!(h.node("g-overflow").state.expanded, Some(true));
        h.node("g-popover");
        h.node("g-overflow-item-0-avatar");
        h.node("g-overflow-item-2-avatar");
        h.press(Key::Escape);
        assert_eq!(h.node("g-overflow").state.expanded, Some(false));
    }

    #[test]
    fn keyboard_opens_the_overflow_popover() {
        let mut h = group(AvatarGroupLayout::Stack, 5);
        h.run();
        h.press(Key::Tab);
        h.press(Key::Enter);
        assert_eq!(h.node("g-overflow").state.expanded, Some(true));
    }

    #[test]
    fn a_group_that_fits_has_no_overflow_button() {
        let mut h = group(AvatarGroupLayout::Spread, 3);
        h.run();
        assert!(h.tree().find_key("g-overflow").is_none());
        h.node("m-0-avatar");
    }

    #[test]
    fn pie_lists_every_item_in_the_popover() {
        let mut h = group(AvatarGroupLayout::Pie, 5);
        h.run();
        h.click("g-overflow");
        for i in 0..5 {
            h.node(&format!("g-overflow-item-{i}-avatar"));
        }
    }
}

mod tag_group {
    use egui::Key;
    use egui_mineral::{select::Selection, semantic::Role};
    use egui_mineral_fluent::data::{InteractionTag, Tag, TagGroup};

    use crate::support::{FluentHarness, Harness, harness};

    struct State {
        values: Vec<String>,
        selection: Selection<String>,
    }

    fn group() -> Harness<'static, State> {
        let state = State {
            values: ["a", "b", "c"].map(str::to_owned).to_vec(),
            selection: Selection::multiple(),
        };
        harness(state, |ui, s| {
            let State { values, selection } = s;
            let mut g = TagGroup::new("g", "Tags").dismissible(true).selection(selection);
            for v in values.iter() {
                g = g.interaction_tag(v.clone(), InteractionTag::new(v.clone(), v.to_uppercase()));
            }
            g = g.tag("t".to_owned(), Tag::new("t", "Static").dismissible(false));
            let out = g.show(ui);
            if let Some(gone) = out.removed {
                values.retain(|v| *v != gone);
            }
        })
    }

    #[test]
    fn agent_keys_and_roles() {
        let mut h = group();
        h.run();
        assert_eq!(h.node("g-tag-group").role, Role::Toolbar);
        assert_eq!(h.node("a-interaction-tag").role, Role::Button);
        assert_eq!(h.node("a-dismiss").name, "Dismiss A");
        assert_eq!(h.node("t-tag").role, Role::Label, "a static tag has no action");
    }

    #[test]
    fn primary_click_toggles_the_selection() {
        let mut h = group();
        h.run();
        h.click("b-interaction-tag");
        assert!(h.state().value.selection.is_selected(&"b".to_owned()));
        assert_eq!(
            h.node("b-interaction-tag").state.checked,
            Some(egui_mineral::select::Check::Checked)
        );
        h.click("b-interaction-tag");
        assert!(!h.state().value.selection.is_selected(&"b".to_owned()));
    }

    #[test]
    fn secondary_click_dismisses() {
        let mut h = group();
        h.run();
        h.click("b-dismiss");
        assert_eq!(h.state().value.values, ["a", "c"]);
        assert!(h.tree().find_key("b-interaction-tag").is_none());
    }

    #[test]
    fn arrows_rove_wrap_and_dismissing_moves_focus_to_a_neighbor() {
        let selected = |h: &Harness<'_, State>, v: &str| h.state().value.selection.is_selected(&v.to_owned());
        let mut h = group();
        h.run();
        h.click("a-interaction-tag");
        h.press(Key::ArrowRight);
        h.press(Key::Enter);
        assert_eq!(
            h.state().value.values,
            ["b", "c"],
            "Right reached the secondary button, Enter dismissed"
        );
        h.press(Key::Enter);
        assert!(selected(&h, "b"), "focus moved to the next tag");
        h.press(Key::ArrowLeft);
        h.press(Key::Enter);
        assert_eq!(
            h.state().value.values,
            ["b"],
            "Left wraps to the last stop: the secondary button of c"
        );
        h.press(Key::Delete);
        assert!(
            h.state().value.values.is_empty(),
            "focus went to the last stop of the previous tag"
        );
    }

    #[test]
    fn enter_activates_the_primary() {
        let mut h = group();
        h.run();
        h.click("a-interaction-tag");
        h.press(Key::Enter);
        assert!(
            !h.state().value.selection.is_selected(&"a".to_owned()),
            "the click selected, Enter toggled it back"
        );
    }
}

mod table {
    use egui::Key;
    use egui_mineral::{
        select::{Selection, SelectionBehavior},
        semantic::Role,
        table::{SortOrder, SortState},
    };
    use egui_mineral_fluent::{
        actions::PushButton,
        data::{DataGrid, Table, TableColumn, Text},
    };

    use crate::support::{FluentHarness, Harness, SCREEN, harness};

    const NAMES: [&str; 3] = ["Cara", "Abe", "Bea"];

    struct State {
        sort: SortState,
        selection: Selection<usize>,
        activated: Option<usize>,
    }

    fn state() -> State {
        State {
            sort: SortState::default(),
            selection: Selection::multiple(),
            activated: None,
        }
    }

    fn columns() -> [TableColumn; 2] {
        [TableColumn::new("Name").sortable(true), TableColumn::new("Rank")]
    }

    fn table(grid: bool) -> Harness<'static, State> {
        table_with(grid, SelectionBehavior::Toggle)
    }

    fn table_with(grid: bool, behavior: SelectionBehavior) -> Harness<'static, State> {
        harness(state(), move |ui, s| {
            let State {
                sort,
                selection,
                activated,
            } = s;
            let cell =
                |ui: &mut egui_mineral_fluent::egui_mineral::widget::MineralUi<'_, _>, row: usize, col: usize| {
                    let text = if col == 0 {
                        NAMES[row].to_owned()
                    } else {
                        row.to_string()
                    };
                    ui.mineral(Text::new((("cell", row), col), text));
                };
            let out = if grid {
                DataGrid::new("t", "People", columns())
                    .sort(sort)
                    .selection(selection)
                    .selection_behavior(behavior)
                    .sort_by_key(|c| NAMES[c.row])
                    .show(ui, NAMES.len(), cell)
            } else {
                Table::new("t", "People", columns())
                    .sort(sort)
                    .selection(selection)
                    .selection_behavior(behavior)
                    .sort_by_key(|c| NAMES[c.row])
                    .show(ui, NAMES.len(), cell)
            };
            if out.activated.is_some() {
                *activated = out.activated;
            }
        })
    }

    #[test]
    fn roles_and_agent_keys() {
        let mut h = table(false);
        h.run();
        assert_eq!(h.node("t-table").role, Role::Grid);
        assert_eq!(h.node("t-header-0").role, Role::ColumnHeader);
        assert_eq!(h.node("t-row-0").role, Role::Row);
        assert_eq!(h.node("t-row-0").state.selected, Some(false));
    }

    #[test]
    fn resize_grip_clicks_do_not_sort_and_drags_track_the_pointer() {
        use egui_mineral::table::{ColumnSize, ColumnWidths};
        let size = |width| ColumnSize { width, min: 40.0 };
        let mut h = harness(
            (SortState::default(), ColumnWidths::new([size(120.0), size(80.0)])),
            |ui, (sort, widths)| {
                let both = [
                    TableColumn::new("Name").sortable(true),
                    TableColumn::new("Rank").sortable(true),
                ];
                Table::new("t", "People", both)
                    .sort(sort)
                    .resizable(widths)
                    .show(ui, NAMES.len(), |ui, row, col| {
                        ui.mineral(Text::new((("cell", row), col), NAMES[row]));
                    });
            },
        );
        h.run();
        let at = h.rect("t-header-0").right_center();
        h.click_at(at);
        let unsorted = |h: &Harness<'_, (SortState, ColumnWidths)>| {
            (h.state().value.0.order(0), h.state().value.0.order(1)) == (None, None)
        };
        assert!(unsorted(&h), "the grip owns its click, over either header");
        h.hover_at(at);
        h.drag_at(at);
        h.hover_at(at + egui::vec2(30.0, 0.0));
        h.hover_at(at + egui::vec2(64.0, 0.0));
        h.drop_at(at + egui::vec2(64.0, 0.0));
        h.run();
        assert_eq!(
            h.state().value.1.width(0),
            184.0,
            "the travel before the drag decision counts"
        );
        assert!(unsorted(&h));
    }

    #[test]
    fn sortable_header_cycles_and_orders_the_rows() {
        let mut h = table(false);
        h.run();
        let top = |h: &Harness<'_, State>| h.rect("t-row-1").min.y < h.rect("t-row-0").min.y;
        assert!(!top(&h), "data order: Cara first");
        h.click("t-header-0");
        assert_eq!(h.state().value.sort.order(0), Some(SortOrder::Ascending));
        assert!(top(&h), "Abe (row 1) now above Cara (row 0)");
        assert_eq!(h.node("t-header-0").name, "Name, sorted ascending");
        h.click("t-header-0");
        assert_eq!(h.state().value.sort.order(0), Some(SortOrder::Descending));
        h.click("t-header-1");
        assert_eq!(
            h.state().value.sort.order(0),
            Some(SortOrder::Descending),
            "an unsortable header ignores clicks"
        );
    }

    #[test]
    fn row_click_selects_and_the_checkbox_toggles() {
        let mut h = table(false);
        h.run();
        h.click("t-row-1");
        assert!(h.state().value.selection.is_selected(&1));
        assert_eq!(h.state().value.activated, Some(1));
        assert_eq!(h.node("t-row-1").state.selected, Some(true));
        h.click("t-select-2-checkbox");
        assert!(h.state().value.selection.is_selected(&2));
        assert!(h.state().value.selection.is_selected(&1));
        h.click("t-select-all-checkbox");
        assert_eq!(h.state().value.selection.len(), 3);
        h.click("t-select-all-checkbox");
        assert!(h.state().value.selection.is_empty());
    }

    #[test]
    fn data_grid_moves_between_cells_and_space_selects_the_row() {
        let mut h = table(true);
        h.run();
        h.press(Key::Tab);
        h.press(Key::ArrowRight);
        h.press(Key::Enter);
        assert_eq!(
            h.state().value.sort.order(0),
            Some(SortOrder::Ascending),
            "Enter on a header sorts"
        );
        h.press(Key::ArrowDown);
        h.press(Key::Space);
        let first = *h.state().value.selection.first().expect("a row is selected");
        assert_eq!(first, 1, "after sorting the first display row is Abe");
        h.press(Key::ArrowDown);
        h.press(Key::ArrowRight);
        h.press(Key::Space);
        assert!(
            h.state().value.selection.is_selected(&2),
            "the second display row is Bea"
        );
    }

    #[test]
    fn data_grid_shift_arrows_extend_the_selection_and_ctrl_a_selects_all() {
        let mut h = table(true);
        h.run();
        h.press(Key::Tab);
        h.press(Key::ArrowDown);
        assert!(h.state().value.selection.is_empty(), "a plain move selects nothing");
        assert_eq!(h.state().value.selection.current(), Some(&0));
        h.press(Key::Enter);
        assert!(h.state().value.selection.is_empty(), "Enter is the row action");
        h.press(Key::Space);
        let selected = |h: &Harness<'static, State>| h.state().value.selection.selected().copied().collect::<Vec<_>>();
        for (key, want) in [
            (Key::ArrowDown, vec![0, 1]),
            (Key::ArrowDown, vec![0, 1, 2]),
            (Key::ArrowUp, vec![0, 1]),
        ] {
            h.key_press_modifiers(egui::Modifiers::SHIFT, key);
            h.run();
            assert_eq!(selected(&h), want, "Shift+{key:?}");
        }
        h.key_press_modifiers(egui::Modifiers::COMMAND, Key::A);
        h.run();
        assert_eq!(selected(&h), [0, 1, 2]);
    }

    #[test]
    fn rows_toggle_by_default_and_replace_selects_on_press() {
        let mut h = table(false);
        h.run();
        h.click("t-row-0");
        h.click("t-row-2");
        let selected = |h: &Harness<'static, State>| h.state().value.selection.selected().copied().collect::<Vec<_>>();
        assert_eq!(selected(&h), [0, 2], "clicks toggle");
        h.click("t-row-0");
        assert_eq!(selected(&h), [2]);

        let mut h = table_with(false, SelectionBehavior::Replace);
        h.run();
        h.click("t-row-0");
        h.click("t-row-2");
        assert_eq!(selected(&h), [2], "a click replaces");
        let at = h.rect("t-row-1").center();
        h.hover_at(at);
        h.drag_at(at);
        h.step();
        assert!(h.state().value.selection.is_selected(&1), "selected before the release");
        h.drop_at(at);
        h.run();
        assert_eq!(h.state().value.selection.len(), 1);
    }

    #[test]
    fn data_grid_is_one_tab_stop() {
        let mut h = harness(Selection::<usize>::multiple(), |ui, selection| {
            DataGrid::new("t", "People", columns())
                .selection(selection)
                .show(ui, NAMES.len(), |ui, row, col| {
                    ui.mineral(Text::new((("cell", row), col), NAMES[row]));
                });
            ui.mineral(PushButton::new("after", "After"));
        });
        h.run();
        h.press(Key::Tab);
        assert!(h.ctx.memory(egui::Memory::focused).is_some(), "Tab enters the grid");
        h.press(Key::Tab);
        let focused = h.ctx.memory(egui::Memory::focused);
        assert_eq!(
            h.node("after-button").widget_id,
            focused,
            "selection checkboxes are no Tab stops of their own"
        );
    }

    #[test]
    fn keyed_rows_keep_their_selection_when_a_row_is_inserted() {
        let state = (vec!["ada", "bob"], Selection::multiple(), None);
        let mut h = harness(state, |ui, (ids, selection, activated): &mut (Vec<&str>, _, _)| {
            let out = Table::new("t", "People", columns())
                .selection(selection)
                .show(ui, &*ids, |ui, row, col| {
                    ui.mineral(Text::new((("cell", row), col), "x"));
                });
            *activated = out.activated.or(*activated);
        });
        h.run();
        h.click("t-row-bob");
        assert_eq!(h.state().value.2, Some("bob"));
        h.state_mut().value.0.insert(0, "cy");
        h.run();
        assert_eq!(h.node("t-row-bob").state.selected, Some(true));
        assert_eq!(h.node("t-row-cy").state.selected, Some(false));
    }

    #[test]
    fn row_style_tones_the_row_and_its_cell_text() {
        use egui_mineral::{table::RowStyle, tone::Tone};
        const TINT: egui::Color32 = egui::Color32::from_rgb(200, 10, 10);
        const INK: egui::Color32 = egui::Color32::from_rgb(1, 2, 3);
        let pass = crate::support::render(|ui| {
            Table::<usize>::new("t", "People", columns())
                .row_style(|r| match r {
                    1 => Tone::Custom {
                        container: TINT,
                        content: INK,
                    }
                    .into(),
                    _ => RowStyle::default(),
                })
                .show(ui, 3, |ui, row, col| {
                    ui.mineral(Text::new((("cell", row), col), format!("r{row}c{col}")));
                });
        });
        assert_eq!(pass.fills(TINT).len(), 1);
        assert_eq!(pass.text_color("r1c0"), INK, "the tone reaches the cell text");
        assert_ne!(pass.text_color("r0c0"), INK);
    }

    #[test]
    fn rows_report_secondary_clicks() {
        let mut h = harness(None, |ui, hit: &mut Option<usize>| {
            let out = Table::<usize>::new("t", "People", columns()).show(ui, 3, |ui, row, col| {
                ui.mineral(Text::new((("cell", row), col), "x"));
            });
            *hit = out
                .rows
                .iter()
                .find(|(_, r)| r.secondary_clicked())
                .map(|(k, _)| *k)
                .or(*hit);
        });
        h.run();
        let at = h.rect("t-row-2").right_center() - egui::vec2(4.0, 0.0);
        h.hover_at(at);
        for pressed in [true, false] {
            h.event(egui::Event::PointerButton {
                pos: at,
                button: egui::PointerButton::Secondary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            });
            h.run();
        }
        assert_eq!(h.state().value, Some(2));
    }

    #[test]
    fn only_visible_rows_are_drawn() {
        const ROWS: usize = 1000;
        let mut h = harness(Vec::new(), |ui, drawn: &mut Vec<usize>| {
            drawn.clear();
            Table::new("t", "People", columns()).show(ui, ROWS, |ui, row, col| {
                if col == 0 {
                    drawn.push(row);
                }
                ui.mineral(Text::new((("cell", row), col), "x"));
            });
        });
        h.run();
        let drawn = h.state().value.len();
        let fit = (SCREEN.y / h.rect("t-row-0").height()).ceil();
        assert!(drawn > 0 && drawn as f32 <= fit, "{drawn} of {ROWS} rows drawn");
    }

    #[test]
    fn culled_rows_register_no_widgets() {
        use egui_mineral::semantic::Action;

        const ROWS: usize = 1000;
        let mut h = harness((), |ui, (): &mut ()| {
            let rt = ui.rt();
            egui::ScrollArea::vertical().show(ui, |ui| {
                Table::new("t", "People", columns()).show(&mut rt.ui(ui), ROWS, |ui, row, col| {
                    ui.mineral(Text::new((("cell", row), col), "x"));
                });
            });
        });
        h.run();
        assert!(h.node("t-row-900").actions.contains(&Action::ScrollIntoView));
        let widgets: usize = h
            .ctx
            .viewport(|v| v.prev_pass.widgets.layers().map(|(_, w)| w.len()).sum());
        assert!(widgets < 150, "{widgets} widgets for {ROWS} rows");
    }

    #[test]
    fn agents_scroll_culled_rows_into_view() {
        use egui_mineral::{agent::AgentAction, semantic::Action};

        const ROWS: usize = 1000;
        const FAR: usize = 700;
        let mut h = harness(Vec::new(), |ui, drawn: &mut Vec<usize>| {
            drawn.clear();
            let rt = ui.rt();
            egui::ScrollArea::vertical().show(ui, |ui| {
                Table::new("t", "People", columns()).show(&mut rt.ui(ui), ROWS, |ui, row, col| {
                    if col == 0 {
                        drawn.push(row);
                    }
                    ui.mineral(Text::new((("cell", row), col), "x"));
                });
            });
        });
        h.run();
        let far = format!("t-row-{FAR}");
        assert!(h.node(&far).actions.contains(&Action::ScrollIntoView));
        assert!(h.rect(&far).min.y > SCREEN.y, "row is below the viewport");
        assert!(!h.state().value.contains(&FAR));
        let agent = h.rt().agent(egui::ViewportId::ROOT);
        #[expect(clippy::disallowed_methods, reason = "agent client thread, joined below")]
        let client =
            std::thread::spawn(move || futures_executor::block_on(agent.act(&far, AgentAction::ScrollIntoView)));
        for _ in 0..30 {
            h.step();
        }
        client.join().expect("agent thread").expect("scroll into view");
        assert!(
            h.state().value.contains(&FAR),
            "row {FAR} drawn after the agent scrolled it in"
        );
    }
}

#[test]
fn image_with_an_unloadable_uri_shows_the_failed_state() {
    use egui_mineral_fluent::data::Image;

    let pass = render(|ui| {
        ui.mineral(
            Image::new("bad", "file:///nonexistent/missing.png")
                .alt("Bad")
                .size(egui::vec2(40.0, 40.0)),
        );
    });
    let icon = egui_mineral_fluent::icons::ICON_IMAGE_OFF.regular().to_string();
    pass.text(&icon);
}
