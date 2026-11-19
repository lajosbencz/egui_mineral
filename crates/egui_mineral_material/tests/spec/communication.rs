use egui::{Rect, Vec2, vec2};
use egui_mineral_material::{
    communication::{CircularProgressIndicator, LinearProgressIndicator, ProgressVariant, PullToRefresh},
    spec::{
        defaults::{progress_indicator, pull_to_refresh as ptr},
        tokens::{circular_progress_indicator, linear_progress_indicator},
    },
};

use crate::support::render;

#[test]
fn progress_indicators_take_their_compose_default_sizes() {
    let p = render(|ui| {
        ui.mineral(LinearProgressIndicator::determinate("lp", 0.5));
    });
    assert_eq!(
        p.outer.size(),
        vec2(progress_indicator::LINEAR_WIDTH, linear_progress_indicator::HEIGHT)
    );
    let p = render(|ui| {
        ui.mineral(CircularProgressIndicator::determinate("cp", 0.5));
    });
    assert_eq!(p.outer.size(), Vec2::splat(circular_progress_indicator::SIZE));
    let p = render(|ui| {
        ui.mineral(CircularProgressIndicator::determinate("cpw", 0.5).variant(ProgressVariant::Wavy));
    });
    assert_eq!(p.outer.size(), Vec2::splat(circular_progress_indicator::WAVE_SIZE));
}

#[test]
fn refreshing_indicator_rests_at_max_distance() {
    let p = render(|ui| {
        PullToRefresh::new("ptr")
            .refreshing(true)
            .show(ui, |ui| ui.label("content"));
    });
    let indicator = *p
        .filled(ptr::INDICATOR_CONTAINER_COLOR)
        .first()
        .expect("indicator container painted");
    assert_eq!(indicator.size(), Vec2::splat(ptr::CONTAINER_SIZE));
    assert_eq!(indicator.bottom(), p.outer.top() + ptr::INDICATOR_MAX_DISTANCE);
    assert_eq!(indicator.center().x, p.outer.center().x);
}

#[test]
fn indicator_hidden_at_rest() {
    let p = render(|ui| {
        PullToRefresh::new("ptr").show(ui, |ui| ui.label("content"));
    });
    assert_eq!(p.filled(ptr::INDICATOR_CONTAINER_COLOR), Vec::<Rect>::new());
}
