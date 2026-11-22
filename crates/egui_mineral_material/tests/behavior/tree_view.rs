//! `TreeView`: selection, keyboard expansion and activation.

use egui::{Key, Modifiers};
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
fn foreign_payloads_of_each_accepted_type_drop_on_their_rows() {
    let roots = vec![
        TreeNode::branch(1, "Docs", vec![TreeNode::leaf(3, "Guide")]),
        TreeNode::leaf(2, "Notes"),
    ];
    // (drag a file or a tag, last drop, drag source)
    let state = (true, None::<(u32, DropPosition, String)>, egui::Rect::NOTHING);
    let mut h = harness(SCREEN, state, move |ui, (file, dropped, source)| {
        let mut expansion = Expansion::default();
        let r = TreeView::new("tree", "Files", &roots, &mut expansion)
            .accept_drop(|_: &String, n: &TreeNode, _| *n.key() == 1)
            .accept_drop(|_: &u32, n: &TreeNode, _| *n.key() == 2)
            .show(ui);
        if let Some(d) = r.dropped {
            let what = d
                .payload::<String>()
                .map(|p| (*p).clone())
                .or_else(|| d.payload::<u32>().map(|t| format!("tag {t}")));
            *dropped = what.map(|w| (*d.target(), d.position(), w));
        }
        let id = egui::Id::unique("source");
        *source = if *file {
            ui.dnd_drag_source(id, String::from("report.pdf"), |ui| ui.label("report.pdf"))
        } else {
            ui.dnd_drag_source(id, 7u32, |ui| ui.label("tag"))
        }
        .response
        .rect;
    });
    for (file, row, expected) in [(true, 1, "report.pdf"), (false, 2, "tag 7")] {
        h.state_mut().value.0 = file;
        h.run();
        let (from, to) = (
            h.state().value.2.center(),
            h.rect(&format!("tree-{row}-treeitem")).center(),
        );
        h.hover_at(from);
        h.drag_at(from);
        h.hover_at(from.lerp(to, 0.5));
        h.hover_at(to);
        h.drop_at(to);
        h.run();
        let dropped = h.state().value.1.clone();
        assert_eq!(dropped.map(|d| (d.0, d.2)), Some((row, String::from(expected))));
    }
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

fn multi_tree<'a>() -> Harness<'a, Selection<u32>> {
    let roots = vec![
        TreeNode::leaf(1, "Guide"),
        TreeNode::leaf(2, "API"),
        TreeNode::leaf(3, "Notes"),
        TreeNode::leaf(4, "Todo"),
    ];
    let mut expansion = Expansion::default();
    harness(SCREEN, Selection::multiple(), move |ui, sel| {
        TreeView::new("tree", "Files", &roots, &mut expansion)
            .selection(sel)
            .reorderable(true)
            .show(ui);
    })
}

fn selected(h: &Harness<'_, Selection<u32>>) -> Vec<u32> {
    h.state().value.selected().copied().collect()
}

#[test]
fn shift_arrows_extend_ctrl_arrows_move_focus_and_ctrl_a_selects_all() {
    let mut h = multi_tree();
    h.click("tree-1-treeitem");
    h.key_press_modifiers(Modifiers::SHIFT, Key::ArrowDown);
    h.run();
    h.key_press_modifiers(Modifiers::SHIFT, Key::ArrowDown);
    h.run();
    assert_eq!(selected(&h), [1, 2, 3]);
    h.key_press_modifiers(Modifiers::COMMAND, Key::ArrowDown);
    h.run();
    assert!(h.focused("tree-4-treeitem"));
    assert_eq!(h.state().value.current(), Some(&4));
    assert_eq!(selected(&h), [1, 2, 3], "Ctrl+arrow keeps the selection");
    h.key_press_modifiers(Modifiers::COMMAND, Key::A);
    h.run();
    assert_eq!(selected(&h), [1, 2, 3, 4]);
}

#[test]
fn dragging_a_selected_row_keeps_the_selection_and_a_plain_click_narrows_it() {
    let mut h = multi_tree();
    h.click("tree-1-treeitem");
    for _ in 0..2 {
        h.key_press_modifiers(Modifiers::SHIFT, Key::ArrowDown);
        h.run();
    }
    assert_eq!(selected(&h), [1, 2, 3]);
    let (from, to) = (h.rect("tree-2-treeitem").center(), h.rect("tree-4-treeitem").center());
    h.hover_at(from);
    h.drag_at(from);
    h.hover_at(from.lerp(to, 0.5));
    h.hover_at(to);
    h.step();
    assert_eq!(
        selected(&h),
        [1, 2, 3],
        "pressing a selected row keeps the set while it drags"
    );
    h.drop_at(to);
    h.run();
    assert_eq!(selected(&h), [1, 2, 3], "a drag never narrows it");
    h.click("tree-2-treeitem");
    assert_eq!(selected(&h), [2], "a plain click without a drag does");
    let (from, to) = (h.rect("tree-4-treeitem").center(), h.rect("tree-1-treeitem").center());
    h.hover_at(from);
    h.drag_at(from);
    h.hover_at(from.lerp(to, 0.5));
    h.step();
    assert_eq!(
        selected(&h),
        [4],
        "pressing an unselected row selects it before it drags"
    );
    h.drop_at(to);
    h.run();
}

#[test]
fn plain_arrows_select_and_ctrl_a_wins_over_an_app_binding() {
    use egui_mineral::shortcut::{self, Scope};
    let roots = vec![TreeNode::leaf(1, "Guide"), TreeNode::leaf(2, "API")];
    let mut expansion = Expansion::default();
    let mut h = harness(SCREEN, (Selection::multiple(), false), move |ui, (sel, app)| {
        let keys = egui::KeyboardShortcut::new(Modifiers::COMMAND, Key::A);
        shortcut::bind(ui.ctx(), Scope::Global, keys, "app-select-all");
        *app |= shortcut::take_action(ui.ctx(), &"app-select-all".into());
        TreeView::new("tree", "Files", &roots, &mut expansion)
            .selection(sel)
            .show(ui);
    });
    h.click("tree-1-treeitem");
    h.press(Key::ArrowDown);
    assert_eq!(
        h.state().value.0.selected().copied().collect::<Vec<_>>(),
        [2],
        "Replace: the move selects"
    );
    h.key_press_modifiers(Modifiers::COMMAND, Key::A);
    h.run();
    assert_eq!(h.state().value.0.len(), 2);
    assert!(!h.state().value.1, "the tree consumed Ctrl+A");
}

#[test]
fn a_touch_tap_activates_until_a_long_press_starts_selecting() {
    use egui::{Event, PointerButton, TouchPhase};
    use egui_mineral::input::synth;
    let mut h = tree();
    let tap = |h: &mut Harness<'_, Tree>, key: &str, hold: u32| {
        let at = h.rect(key).center();
        h.input_mut().events.extend([
            synth::touch(at, TouchPhase::Start),
            Event::PointerMoved(at),
            synth::pointer_down(at, PointerButton::Primary),
        ]);
        h.step();
        for _ in 0..hold {
            h.step();
        }
        h.input_mut().events.extend([
            synth::touch(at, TouchPhase::End),
            synth::pointer_up(at, PointerButton::Primary),
            Event::PointerGone,
        ]);
        h.run();
    };
    tap(&mut h, "tree-4-treeitem", 0);
    assert_eq!(
        h.state().value.activated,
        [4],
        "a tap on an empty selection is the row action"
    );
    assert!(h.state().value.selection.is_empty(), "a touch press never selects");
    tap(&mut h, "tree-4-treeitem", 40);
    assert!(h.state().value.selection.is_selected(&4), "a long press selects");
    tap(&mut h, "tree-1-treeitem", 0);
    assert!(h.state().value.selection.is_selected(&1), "then taps select");
    assert_eq!(h.state().value.activated, [4]);
}

fn request(h: &mut Harness<'_, impl Sized>, key: &str, action: egui::accesskit::Action) {
    use egui::accesskit::{ActionRequest, TreeId};
    let target_node = h.widget(key).accesskit_id();
    h.event(egui::Event::AccessKitActionRequest(ActionRequest {
        action,
        target_tree: TreeId::ROOT,
        target_node,
        data: None,
    }));
    h.run();
}

#[test]
fn collapsing_over_the_focused_row_moves_focus_and_current_to_the_node() {
    let mut h = tree();
    h.click("tree-1-treeitem");
    h.press(Key::ArrowRight);
    h.press(Key::ArrowRight);
    assert!(h.focused("tree-2-treeitem"));
    request(&mut h, "tree-1-treeitem", egui::accesskit::Action::Collapse);
    assert_eq!(h.state().value.collapsed, [1]);
    assert!(h.focused("tree-1-treeitem"), "focus leaves the hidden row");
    let sel = &h.state().value.selection;
    assert_eq!(sel.current(), Some(&1));
    assert!(sel.is_selected(&2), "hidden rows stay selected");
}

#[test]
fn agents_click_rows_and_add_with_ctrl_space() {
    let mut h = multi_tree();
    h.run();
    assert!(h.node("tree-tree").state.multiselectable);
    request(&mut h, "tree-1-treeitem", egui::accesskit::Action::Click);
    request(&mut h, "tree-3-treeitem", egui::accesskit::Action::Click);
    assert_eq!(selected(&h), [3], "a click selects only the row");
    // Only the Tab stop row, the selected one, takes focus.
    request(&mut h, "tree-3-treeitem", egui::accesskit::Action::Focus);
    for _ in 0..2 {
        h.key_press_modifiers(Modifiers::COMMAND, Key::ArrowUp);
        h.run();
    }
    assert!(h.focused("tree-1-treeitem"));
    h.key_press_modifiers(Modifiers::COMMAND, Key::Space);
    h.run();
    assert_eq!(selected(&h), [1, 3], "Ctrl+Space adds");
}
