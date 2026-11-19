use egui_mineral::range::RangeModel;

use crate::support::render;

#[test]
fn slider_value_field_shares_the_value_and_the_name() {
    use egui_mineral_material::selection::ValueSlider;
    let mut v = 42.0;
    let p = render(|ui| {
        ui.mineral(ValueSlider::new("v", "Opacity", RangeModel::new(0.0, 100.0).step(1.0), &mut v).value_field(true));
    });
    assert_eq!(p.node("v-value-field").name, "Opacity");
    p.text("42");
    let (track, field) = (p.rect("v-slider"), p.rect("v-value-field"));
    assert!(field.left() >= track.center().x, "{field:?} beside {track:?}");
    assert!(
        p.shapes
            .iter()
            .all(|s| !matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "Opacity"))
    );
}
