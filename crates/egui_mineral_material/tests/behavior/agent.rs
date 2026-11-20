//! Agent parity: agents act through the same raw input path as users and screen readers.

use std::collections::BTreeSet;

use crate::support::{MineralHarness, SCREEN, TreeNode, WIDE, harness};
use egui::accesskit::{Role, Toggled};
use egui_kittest::kittest::{NodeT as _, Queryable as _};
use egui_mineral::{
    agent::{AgentAction, AgentError},
    expand::Expansion,
    range::RangeModel,
    select::Selection,
    tree,
};
use egui_mineral_material::{
    actions::PushButton,
    containment::ListSlot,
    extras::{DataTable, TableCell, TreeView},
    selection::{CheckButton, ValueSlider},
};

#[test]
fn agent_click_and_tap_activate_button() {
    let mut h = harness(SCREEN, 0, |ui, clicks| {
        if ui.mineral(PushButton::new("ok", "OK")).clicked() {
            *clicks += 1;
        }
    });
    let md = h
        .drive_agent(|agent| {
            futures_executor::block_on(async {
                let md = agent.snapshot().await?;
                agent.act("ok-button", AgentAction::Click).await?;
                agent.act("ok-button", AgentAction::Tap).await?;
                let missing = agent.act("nope", AgentAction::Click).await;
                assert!(matches!(missing, Err(AgentError::NotFound(_))));
                let wrong = agent.act("ok-button", AgentAction::Increment).await;
                assert!(matches!(wrong, Err(AgentError::ActionNotAvailable { .. })));
                Ok::<_, AgentError>(md)
            })
        })
        .expect("agent session");
    assert!(md.contains("- [ok-button] Button: OK\n  Actions: Click, Focus"), "{md}");
    assert_eq!(h.state().value, 2);
}

#[test]
fn agent_drives_value_widgets() {
    let mut h = harness(SCREEN, (0.0, false), |ui, (volume, agree)| {
        let range = RangeModel::new(0.0, 100.0).step(1.0);
        ui.mineral(ValueSlider::new("volume", "Volume", range, volume));
        ui.mineral(CheckButton::new("agree", "Agree", agree));
    });
    let md = h
        .drive_agent(|agent| {
            futures_executor::block_on(async {
                agent.act("volume-slider", AgentAction::SetValue(42.4.into())).await?;
                agent.act("volume-slider", AgentAction::Increment).await?;
                agent.act("agree-checkbox", AgentAction::Click).await?;
                agent.snapshot().await
            })
        })
        .expect("agent session");
    let (volume, agree) = h.state().value;
    assert_eq!(volume, 43.0, "snapped to the step, then one step up");
    assert!(agree);
    assert!(
        md.contains("(range: 43 in 0..=100)") && md.contains("(checked)"),
        "{md}"
    );
}

#[test]
fn agent_reaches_culled_widget_and_rejects_non_finite() {
    let mut h = harness(SCREEN, 0, |ui, clicks| {
        ui.add_space(SCREEN.y * 2.0);
        if ui.mineral(PushButton::new("far", "Far")).clicked() {
            *clicks += 1;
        }
    });
    let md = h
        .drive_agent(|agent| {
            futures_executor::block_on(async {
                let md = agent.snapshot().await?;
                agent.act("far-button", AgentAction::Click).await?;
                let nan = agent.act("far-button", AgentAction::SetValue(f64::NAN.into())).await;
                assert!(matches!(nan, Err(AgentError::NonFinite(_))));
                Ok::<_, AgentError>(md)
            })
        })
        .expect("agent session");
    assert!(md.contains("[far-button]"), "{md}");
    assert_eq!(h.state().value, 1);
}

#[test]
fn tap_rejects_nodes_off_screen() {
    let mut h = harness(SCREEN, 0, |ui, clicks| {
        ui.add_space(SCREEN.y * 2.0);
        if ui.mineral(PushButton::new("far", "Far")).clicked() {
            *clicks += 1;
        }
    });
    h.drive_agent(|agent| {
        futures_executor::block_on(async {
            agent.snapshot().await?;
            let far = agent.act("far-button", AgentAction::Tap).await;
            assert!(matches!(far, Err(AgentError::NotOnScreen(_))), "{far:?}");
            Ok::<_, AgentError>(())
        })
    })
    .expect("agent session");
    assert_eq!(h.state().value, 0);
}

#[test]
fn checked_rows_survive_paging() {
    const PAGE: usize = 5;
    let names: Vec<String> = (0..12).map(|i| format!("Row {i}")).collect();
    let mut h = harness(WIDE, Selection::multiple(), move |ui, sel| {
        DataTable::new("tbl", "Rows", ["Name"])
            .selection(sel)
            .paginate([PAGE])
            .show(ui, names.len(), |r, _| TableCell::from(names[r].as_str()));
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

#[test]
fn inserting_a_toolbar_action_keeps_the_keys_of_identified_ones() {
    use egui_mineral_material::{IconAction, icons, navigation::FloatingToolbar};
    let mut h = harness(WIDE, (false, 0), |ui, (insert, pastes)| {
        let actions = insert
            .then(|| IconAction::new(icons::ICON_CONTENT_CUT, "Cut").command("edit.cut"))
            .into_iter()
            .chain([
                IconAction::new(icons::ICON_CONTENT_COPY, "Copy").command("edit.copy"),
                IconAction::new(icons::ICON_CONTENT_PASTE, "Paste").salt("paste"),
            ]);
        let out = FloatingToolbar::new("tb", "Edit").actions(actions).show(ui);
        *pastes += usize::from(
            out.clicked.as_ref().map(egui_mineral::action::Clicked::index) == Some(usize::from(*insert) + 1),
        );
    });
    let keys = ["tb-edit-copy-button", "tb-paste-button"];
    let rects = keys.map(|k| h.rect(k));
    h.state_mut().value.0 = true;
    h.run();
    assert!(
        h.rect(keys[0]).min.x > rects[0].min.x,
        "the action moved behind the insert"
    );
    h.drive_agent(move |agent| futures_executor::block_on(agent.act(keys[1], AgentAction::Click)))
        .expect("agent click");
    assert_eq!(h.state().value.1, 1, "the moved action keeps its key");
}
