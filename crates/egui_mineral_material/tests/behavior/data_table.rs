//! `DataTable`: grid focus, sorting, selection and paging contracts.

use egui::{Key, Modifiers};
use egui_mineral::{
    agent::{AgentAction, AgentValue},
    select::Selection,
};
use egui_mineral_material::extras::{DataTable, TableCell, TableColumn};

use crate::support::{Harness, MineralHarness, WIDE, harness};

const NAMES: [&str; 4] = ["Delta", "alpha", "Charlie", "bravo"];

#[test]
fn single_selection_header_is_a_real_grid_cell() {
    let mut h = harness(WIDE, Selection::single(), |ui, sel| {
        DataTable::new("tbl", "Names", [TableColumn::new("Name")])
            .selection(sel)
            .show(ui, NAMES.len(), |r, _| TableCell::from(NAMES[r]));
    });
    h.click("tbl-0-header");
    h.press(Key::ArrowLeft);
    assert!(
        h.focused("tbl-selection-header"),
        "ArrowLeft reaches the selection column header"
    );
    h.key_press_modifiers(Modifiers::SHIFT, Key::Tab);
    h.run();
    h.press(Key::Tab);
    assert!(h.focused("tbl-selection-header"), "the table keeps its Tab stop");
}

fn shown_rows(h: &impl MineralHarness) -> Vec<usize> {
    let tree = h.tree();
    let mut cells: Vec<_> = tree
        .nodes
        .iter()
        .filter_map(|n| {
            let key = n.id.key().strip_prefix("tbl-")?.strip_suffix("-0-cell")?;
            Some((n.rect.top(), key.parse::<usize>().ok()?))
        })
        .collect();
    cells.sort_by(|a, b| a.0.total_cmp(&b.0));
    cells.into_iter().map(|(_, r)| r).collect()
}

#[test]
fn sorted_order_follows_a_locale_switch() {
    const WORDS: [&str; 2] = ["zebra", "\u{e4}pple"];
    let mut h = harness(WIDE, (), |ui, ()| {
        DataTable::new("tbl", "Words", [TableColumn::new("Word").sortable(true)])
            .show(ui, WORDS.len(), |r, _| TableCell::from(WORDS[r]));
    });
    h.click("tbl-0-header");
    assert_eq!(shown_rows(&h), [1, 0], "en: a-umlaut sorts with a");
    h.rt()
        .i18n()
        .set_locale(&egui_mineral::i18n::locale!("sv"))
        .expect("sv ICU data");
    h.run();
    assert_eq!(shown_rows(&h), [0, 1], "sv: a-umlaut sorts after z");
}

#[test]
fn sort_by_key_receives_the_row_and_the_sorted_column() {
    let mut h = harness(WIDE, (), |ui, ()| {
        let columns = [
            TableColumn::new("Name").sortable(true),
            TableColumn::new("Rank").sortable(true),
        ];
        DataTable::new("tbl", "Names", columns)
            .sort_by_key(|c| match c.column {
                0 => NAMES[c.row].len(),
                _ => NAMES.len() - c.row,
            })
            .show(ui, NAMES.len(), |r, _| TableCell::from(NAMES[r]));
    });
    h.click("tbl-0-header");
    assert_eq!(
        shown_rows(&h),
        [0, 1, 3, 2],
        "column 0: ascending by length, ties in data order"
    );
    h.click("tbl-1-header");
    assert_eq!(shown_rows(&h), [3, 2, 1, 0], "column 1: ascending by reversed row");
}

#[test]
fn changed_page_sizes_replace_a_stale_page_size() {
    let mut h = harness(WIDE, vec![5usize], |ui, sizes| {
        DataTable::new("tbl", "Rows", [TableColumn::new("N")])
            .paginate(sizes.clone())
            .show(ui, 30, |r, _| TableCell::Number(r as f64));
    });
    assert_eq!(shown_rows(&h).len(), 5);
    h.state_mut().value = vec![10, 20];
    h.run();
    assert_eq!(
        shown_rows(&h).len(),
        10,
        "the first offered size replaces one no longer offered"
    );
}

#[derive(Default)]
struct Seen {
    sort_changed: u32,
    page_changed: u32,
    activated: Vec<usize>,
}

fn names_table<'a>(sizes: &'static [usize]) -> Harness<'a, Seen> {
    harness(WIDE, Seen::default(), move |ui, seen| {
        let r = DataTable::new(
            "tbl",
            "Names",
            [TableColumn::new("Name").sortable(true), TableColumn::new("Length")],
        )
        .paginate(sizes.iter().copied())
        .show(ui, NAMES.len(), |r, c| match c {
            0 => TableCell::from(NAMES[r]),
            _ => TableCell::Number(NAMES[r].len() as f64),
        });
        seen.sort_changed += u32::from(r.sort_changed);
        seen.page_changed += u32::from(r.page_changed);
        seen.activated.extend(r.activated);
    })
}

#[test]
fn header_clicks_cycle_ascending_descending_unsorted() {
    let mut h = names_table(&[]);
    assert_eq!(shown_rows(&h), [0, 1, 2, 3]);
    h.click("tbl-0-header");
    assert_eq!(shown_rows(&h), [1, 3, 2, 0], "ascending, case-insensitive");
    h.click("tbl-0-header");
    assert_eq!(shown_rows(&h), [0, 2, 3, 1], "descending");
    h.click("tbl-0-header");
    assert_eq!(shown_rows(&h), [0, 1, 2, 3], "back to data order");
    assert_eq!(h.state().value.sort_changed, 3, "each click reports once");
}

#[test]
fn next_page_shows_the_following_rows_and_reports_once() {
    let mut h = names_table(&[2]);
    assert_eq!(shown_rows(&h), [0, 1]);
    h.click("tbl-next-button");
    assert_eq!(shown_rows(&h), [2, 3]);
    assert_eq!(h.state().value.page_changed, 1);
}

#[test]
fn arrows_move_between_cells_and_a_click_activates_its_row() {
    let mut h = names_table(&[]);
    h.click("tbl-1-0-cell");
    assert_eq!(h.state().value.activated, [1], "a body click activates its data row");
    h.press(Key::ArrowRight);
    assert!(h.focused("tbl-1-1-cell"), "ArrowRight moves to the next column");
    h.press(Key::ArrowDown);
    assert!(h.focused("tbl-2-1-cell"), "ArrowDown moves to the next row");
    h.press(Key::Enter);
    assert_eq!(h.state().value.activated, [1, 2], "Enter activates the focused row");
}

#[test]
fn resize_handle_sets_the_column_width() {
    let mut h = names_table(&[]);
    let before = h.rect("tbl-0-header").width();
    h.drive_agent(|agent| {
        futures_executor::block_on(agent.act("tbl-0-resize", AgentAction::SetValue(AgentValue::Number(240.0))))
    })
    .expect("resize accepted");
    h.run();
    let after = h.rect("tbl-0-header").width();
    assert_ne!(before, 240.0, "the test starts from another width");
    assert_eq!(after, 240.0);
}

#[test]
fn row_style_tints_the_row_container() {
    const TINT: egui::Color32 = egui::Color32::from_rgb(200, 10, 10);
    let pass = crate::support::render(|ui| {
        DataTable::<usize>::new("tbl", "Names", [TableColumn::new("Name")])
            .row_style(|r| (r == 2).then_some(TINT))
            .show(ui, NAMES.len(), |r, _| TableCell::from(NAMES[r]));
    });
    let tinted = pass
        .shapes
        .iter()
        .filter(|s| matches!(&s.shape, egui::Shape::Rect(r) if r.fill == TINT))
        .count();
    assert_eq!(tinted, 1);
}

#[test]
fn rows_report_their_responses_for_context_menus() {
    let mut h = harness(WIDE, None, |ui, hit: &mut Option<usize>| {
        let r = DataTable::<usize>::new("tbl", "Names", [TableColumn::new("Name")])
            .show(ui, NAMES.len(), |r, _| TableCell::from(NAMES[r]));
        assert_eq!(r.rows.len(), NAMES.len());
        *hit = r
            .rows
            .iter()
            .find(|(_, r)| r.secondary_clicked())
            .map(|(k, _)| *k)
            .or(*hit);
    });
    let at = h.rect("tbl-1-0-cell").center();
    h.hover_at(at);
    for pressed in [true, false] {
        h.event(egui::Event::PointerButton {
            pos: at,
            button: egui::PointerButton::Secondary,
            pressed,
            modifiers: Modifiers::NONE,
        });
        h.run();
    }
    assert_eq!(h.state().value, Some(1));
}

#[test]
fn stick_to_end_follows_appended_rows() {
    let mut h = harness(WIDE, 5usize, |ui, n: &mut usize| {
        DataTable::<usize>::new("tbl", "Log", [TableColumn::new("Line")])
            .max_height(120.0)
            .stick_to_end(true)
            .show(ui, *n, |r, _| TableCell::Number(r as f64));
    });
    h.run();
    h.state_mut().value = 200;
    for _ in 0..3 {
        h.run();
    }
    assert!(shown_rows(&h).contains(&199), "{:?}", shown_rows(&h));
}
