use egui::{
    Color32, Event,
    accesskit::{Action, ActionData, ActionRequest, TreeId},
};

use egui_mineral_material::extras::ColorPicker;

use crate::support::{MineralHarness, SCREEN, harness};

#[test]
fn rgb_fields_follow_the_color_both_ways() {
    let mut h = harness(SCREEN, Color32::from_rgb(10, 20, 30), |ui, color| {
        ui.mineral(ColorPicker::new("paint", "Paint", color).rgb_fields(true));
    });
    let green = h.node("paint-green-field");
    assert_eq!(green.state.range.map(|r| r.value), Some(20.0));
    h.event(Event::AccessKitActionRequest(ActionRequest {
        action: Action::SetValue,
        target_tree: TreeId::ROOT,
        target_node: h.widget("paint-green-field").accesskit_id(),
        data: Some(ActionData::NumericValue(200.0)),
    }));
    h.run();
    let color = h.state().value;
    assert_eq!(color, Color32::from_rgb(10, 200, 30), "field writes the color");
    let hex = format!("#{:02x}{:02x}{:02x}", color.r(), color.g(), color.b());
    assert_eq!(h.node("paint-hex-field").value.as_deref(), Some(hex.as_str()));

    h.state_mut().value = Color32::from_rgb(1, 2, 3);
    h.run();
    let values = ["red", "green", "blue"].map(|c| h.node(&format!("paint-{c}-field")).state.range.map(|r| r.value));
    assert_eq!(values, [Some(1.0), Some(2.0), Some(3.0)], "color writes the fields");
}
