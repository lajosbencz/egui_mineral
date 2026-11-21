//! `TreeView`: selection, keyboard expansion and activation.

use egui::Key;
use egui_mineral::tree::DropPosition;
use egui_mineral::tree::TreeData as _;
use egui_mineral::{expand::Expansion, select::Selection};
use egui_mineral_material::{
    extras::{TreeDrag, TreeView},
    spec::{ColorRole, TypeScale},
};

use crate::support::{Harness, MineralHarness, SCREEN, TreeNode, harness, render};

#[derive(Default)]
struct Tree {
    expansion: Expansion<u32>,
    selection: Selection<u32>,
    expanded: Vec<u32>,
    collapsed: Vec<u32>,
    activated: Vec<u32>,
}

fn tree<'a>() -> Harness<'a, Tree> {
    let roots = vec![
        TreeNode::branch(1, "Docs", vec![TreeNode::leaf(2, "Guide"), TreeNode::leaf(3, "API")]),
        TreeNode::leaf(4, "Notes"),
    ];
    let value = Tree {
        selection: Selection::single(),
        ..Tree::default()
    };
    harness(SCREEN, value, move |ui, t| {
        let r = TreeView::new("tree", "Files", &roots, &mut t.expansion)
            .selection(&mut t.selection)
            .show(ui);
        t.expanded.extend(r.expanded);
        t.collapsed.extend(r.collapsed);
        t.activated.extend(r.activated);
    })
}

#[test]
fn arrows_expand_enter_return_and_collapse() {
    let mut h = tree();
    h.click("tree-1-treeitem");
    assert_eq!(
        h.state().value.selection.selected().copied().collect::<Vec<_>>(),
        [1],
        "a click selects"
    );
    h.press(Key::ArrowRight);
    assert_eq!(h.state().value.expanded, [1], "ArrowRight expands a closed parent");
    h.press(Key::ArrowRight);
    assert!(
        h.focused("tree-2-treeitem"),
        "ArrowRight on an open parent enters its first child"
    );
    h.press(Key::ArrowLeft);
    assert!(
        h.focused("tree-1-treeitem"),
        "ArrowLeft on a leaf returns to its parent"
    );
    h.press(Key::ArrowLeft);
    assert_eq!(h.state().value.collapsed, [1], "ArrowLeft collapses an open parent");
    h.press(Key::End);
    assert!(h.focused("tree-4-treeitem"), "End reaches the last visible row");
}

#[test]
fn double_click_activates_a_row() {
    let mut h = tree();
    let at = h.rect("tree-4-treeitem").center();
    h.hover_at(at);
    for _ in 0..2 {
        h.drag_at(at);
        h.drop_at(at);
    }
    h.run();
    assert_eq!(h.state().value.activated, [4]);
}

#[test]
fn reorderable_row_drops_onto_an_outside_zone() {
    let roots = vec![TreeNode::leaf(1, "Guide"), TreeNode::leaf(2, "Notes")];
    let state = (Expansion::default(), None, egui::Rect::NOTHING);
    let mut h = harness(SCREEN, state, move |ui, (expansion, dropped, zone_rect)| {
        TreeView::new("tree", "Files", &roots, expansion)
            .reorderable(true)
            .show(ui);
        let zone = ui.allocate_response(egui::vec2(200.0, 100.0), egui::Sense::hover());
        *zone_rect = zone.rect;
        if let Some(p) = zone.dnd_release_payload::<TreeDrag<u32>>() {
            *dropped = Some(*p.key());
        }
    });
    let (from, to) = (h.rect("tree-2-treeitem").center(), h.state().value.2.center());
    h.hover_at(from);
    h.drag_at(from);
    h.hover_at(from.lerp(to, 0.5));
    h.hover_at(to);
    h.drop_at(to);
    h.run();
    assert_eq!(h.state().value.1, Some(2));
}

#[test]
fn dense_rows_take_the_set_row_height() {
    let roots = vec![TreeNode::leaf(1, "Guide"), TreeNode::leaf(2, "Notes")];
    let h = harness(SCREEN, Expansion::default(), move |ui, expansion| {
        TreeView::new("tree", "Files", &roots, expansion)
            .row_height(40.0)
            .indent(24.0)
            .text_style(egui_mineral_material::spec::TypeScale::LabelLarge)
            .show(ui);
    });
    let (a, b) = (h.rect("tree-1-treeitem"), h.rect("tree-2-treeitem"));
    assert_eq!((a.height(), b.top() - a.top()), (40.0, 40.0));
}

#[test]
fn culled_rows_register_no_widgets() {
    use egui_mineral::semantic::Action;

    const ROWS: u32 = 1000;
    let roots: Vec<TreeNode> = (0..ROWS).map(|k| TreeNode::leaf(k, "Row")).collect();
    let mut h = harness(SCREEN, Expansion::<u32>::default(), move |ui, open| {
        TreeView::new("tree", "Files", &roots, open).show(ui);
    });
    h.run();
    assert!(h.node("tree-900-treeitem").actions.contains(&Action::ScrollIntoView));
    let widgets: usize = h
        .ctx
        .viewport(|v| v.prev_pass.widgets.layers().map(|(_, w)| w.len()).sum());
    assert!(widgets < 150, "{widgets} widgets for {ROWS} rows");
}

#[test]
fn node_text_style_sets_the_label_role_per_row() {
    let roots = vec![TreeNode::branch(1, "Docs", vec![]), TreeNode::leaf(2, "Notes")];
    let folder = TypeScale::TitleLargeEmphasized;
    let p = render(|ui| {
        let mut expansion = Expansion::default();
        TreeView::new("tree", "Files", &roots, &mut expansion)
            .text_style(TypeScale::BodySmall)
            .node_text_style(|n| {
                if n.label() == "Docs" {
                    folder
                } else {
                    TypeScale::BodySmall
                }
            })
            .show(ui);
    });
    assert_eq!(p.text("Docs").height(), folder.line_height());
    assert_eq!(p.text("Notes").height(), TypeScale::BodySmall.line_height());
}

#[test]
fn disabled_nodes_neither_select_nor_take_focus() {
    let roots = vec![
        TreeNode::leaf(1, "Guide"),
        TreeNode::leaf(2, "Locked"),
        TreeNode::leaf(3, "Notes"),
    ];
    let mut h = harness(SCREEN, Selection::<u32>::single(), move |ui, selection| {
        let mut expansion = Expansion::default();
        TreeView::new("tree", "Files", &roots, &mut expansion)
            .selection(selection)
            .node_disabled(|n| *n.key() == 2)
            .show(ui);
    });
    assert!(h.node("tree-2-treeitem").state.disabled);
    h.click("tree-2-treeitem");
    assert_eq!(h.state().value.selected().count(), 0, "a disabled row does not select");
    h.click("tree-1-treeitem");
    h.press(Key::ArrowDown);
    assert!(h.focused("tree-3-treeitem"), "ArrowDown skips the disabled row");
}

#[test]
fn reveal_expands_the_ancestors_without_taking_focus() {
    let roots = vec![TreeNode::branch(
        1,
        "Docs",
        vec![TreeNode::branch(2, "Guide", vec![TreeNode::leaf(3, "Intro")])],
    )];
    let mut h = harness(
        SCREEN,
        (Expansion::<u32>::default(), true),
        move |ui, (expansion, reveal)| {
            let mut tree = TreeView::new("tree", "Files", &roots, expansion);
            if std::mem::take(reveal) {
                tree = tree.reveal(3);
            }
            tree.show(ui);
        },
    );
    h.run();
    let expansion = &h.state().value.0;
    assert!(expansion.is_expanded(&1) && expansion.is_expanded(&2));
    assert!(
        h.tree().find_key("tree-3-treeitem").is_some(),
        "the revealed row renders"
    );
    assert!(!h.focused("tree-3-treeitem"));
}

#[test]
fn foreign_payload_drops_inside_an_accepting_row() {
    let roots = vec![
        TreeNode::branch(1, "Docs", vec![TreeNode::leaf(3, "Guide")]),
        TreeNode::leaf(2, "Notes"),
    ];
    let state = (None::<(u32, DropPosition, String)>, egui::Rect::NOTHING);
    let mut h = harness(SCREEN, state, move |ui, (dropped, source)| {
        let mut expansion = Expansion::default();
        let r = TreeView::new("tree", "Files", &roots, &mut expansion)
            .accept_drop(|_: &String, n: &TreeNode, _| *n.key() == 1)
            .show(ui);
        if let Some(d) = r.dropped {
            *dropped = d.payload::<String>().map(|p| (*d.target(), d.position(), (*p).clone()));
        }
        *source = ui
            .dnd_drag_source(egui::Id::unique("file"), String::from("report.pdf"), |ui| {
                ui.label("report.pdf")
            })
            .response
            .rect;
    });
    let (from, to) = (h.state().value.1.center(), h.rect("tree-1-treeitem").center());
    h.hover_at(from);
    h.drag_at(from);
    h.hover_at(from.lerp(to, 0.5));
    h.hover_at(to);
    h.drop_at(to);
    h.run();
    assert_eq!(
        h.state().value.0,
        Some((1, DropPosition::Inside, String::from("report.pdf")))
    );
}

#[test]
fn unfocused_selection_takes_its_own_container() {
    let roots = vec![TreeNode::leaf(1, "Guide"), TreeNode::leaf(2, "Notes")];
    let role = ColorRole::SurfaceContainerHighest;
    let p = render(|ui| {
        let mut expansion = Expansion::default();
        let mut selection = Selection::single();
        selection.select(1);
        TreeView::new("tree", "Files", &roots, &mut expansion)
            .selection(&mut selection)
            .unfocused_selection_color(role)
            .show(ui);
    });
    let row = p.rect("tree-1-treeitem");
    assert!(
        p.filled(role).iter().any(|r| r.contains_rect(row.shrink(1.0))),
        "{:?}",
        p.filled(role)
    );
}
