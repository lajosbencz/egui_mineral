//! Agent parity: agents act through the same raw input path as users and screen readers.

use std::collections::BTreeSet;

use crate::support::{MineralHarness, SCREEN, TreeNode, WIDE, harness};
use egui::accesskit::{Role, Toggled};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_mineral::{agent::AgentAction, expand::Expansion, select::Selection, tree};
use egui_mineral_material::{
    containment::ListSlot,
    extras::{DataTable, TableCell, TreeView},
};

#[test]
fn checked_rows_survive_paging() {
    const PAGE: usize = 5;
    let names: Vec<String> = (0..12).map(|i| format!("Row {i}")).collect();
    let mut h = harness(WIDE, Selection::multiple(), move |ui, sel| {
        DataTable::new("tbl", "Rows", ["Name"], names.len())
            .selection(sel)
            .paginate([PAGE])
            .show(ui, |r, _| TableCell::from(names[r].as_str()));
    });
    let md = h
        .drive_agent(|agent| {
            futures_executor::block_on(async {
                agent.snapshot().await?;
                agent.act("tbl-1-checkbox", AgentAction::Click).await?;
                agent.act("tbl-3-checkbox", AgentAction::Click).await?;
                agent.act("tbl-next-button", AgentAction::Click).await?;
                agent.act("tbl-6-checkbox", AgentAction::Click).await?;
                // Plain row click on another page must not clear page 1.
                agent.act("tbl-8-0-cell", AgentAction::Click).await?;
                agent.act("tbl-prev-button", AgentAction::Click).await?;
                agent.snapshot().await
            })
        })
        .expect("agent session");
    assert_eq!(h.state().value.selected().copied().collect::<Vec<_>>(), [1, 3, 6, 8]);
    for key in ["tbl-1-checkbox", "tbl-3-checkbox"] {
        let line = md
            .lines()
            .find(|l| l.contains(key))
            .unwrap_or_else(|| panic!("{key} missing:\n{md}"));
        assert!(line.contains("(checked)"), "{line}");
    }
    assert!(!md.contains("tbl-6-checkbox"), "back on page 1:\n{md}");
}

#[test]
fn partial_tree_check_reaches_both_serializers() {
    let roots = [TreeNode::branch(
        1,
        "root",
        vec![TreeNode::leaf(2, "on"), TreeNode::leaf(3, "off")],
    )];
    let checked = BTreeSet::from([2]);
    let mut expansion = Expansion::default();
    expansion.expand(1);
    let mut h = harness(
        SCREEN,
        (expansion, Selection::single()),
        move |ui, (expansion, selection)| {
            TreeView::new("tree", "Tree", &roots, expansion)
                .selection(selection)
                .leading(|n| ListSlot::new().checkbox(tree::check_state(n, &checked)))
                .show(ui);
        },
    );
    let md = h
        .drive_agent(|agent| futures_executor::block_on(agent.snapshot()))
        .expect("agent session");
    let root = md
        .lines()
        .find(|l| l.contains("CheckBox: root"))
        .unwrap_or_else(|| panic!("root checkbox missing:\n{md}"));
    assert!(root.contains("(mixed)"), "{root}");
    let node = h.get_by_role_and_label(Role::CheckBox, "root");
    assert_eq!(node.accesskit_node().toggled(), Some(Toggled::Mixed));
}
