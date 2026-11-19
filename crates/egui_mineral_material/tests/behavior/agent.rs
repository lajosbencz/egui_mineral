//! Agent parity: agents act through the same raw input path as users and screen readers.

use crate::support::{MineralHarness, SCREEN, WIDE, harness};
use egui_mineral::{
    agent::{AgentAction, AgentError},
    range::RangeModel,
};
use egui_mineral_material::{
    actions::PushButton,
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
fn inserting_a_toolbar_action_keeps_the_keys_of_identified_ones() {
    use egui_mineral_material::{IconAction, icons, navigation::Toolbar};
    let mut h = harness(WIDE, (false, 0), |ui, (insert, pastes)| {
        let actions = insert
            .then(|| IconAction::new(icons::ICON_CONTENT_CUT, "Cut").command("edit.cut"))
            .into_iter()
            .chain([
                IconAction::new(icons::ICON_CONTENT_COPY, "Copy").command("edit.copy"),
                IconAction::new(icons::ICON_CONTENT_PASTE, "Paste").salt("paste"),
            ]);
        let out = Toolbar::floating("tb", "Edit").actions(actions).show(ui);
        *pastes += usize::from(out.clicked == Some(usize::from(*insert) + 1));
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
