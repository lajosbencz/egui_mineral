//! Touch pans off a slider's axis scroll the enclosing page, along it they drag the value.

use egui::{Event, PointerButton, Pos2, TouchPhase, vec2};
use egui_mineral::input::synth;
use egui_mineral_fluent::{egui_mineral::range::RangeModel, inputs::Slider};

use crate::support::{FluentHarness, Harness, harness};

#[derive(Default)]
struct Page {
    offset: f32,
    value: f64,
}

fn page<'a>() -> Harness<'a, Page> {
    harness(Page::default(), |ui, p| {
        let rt = ui.rt();
        let out = egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
            let mut mui = rt.ui(ui);
            mui.mineral(Slider::new("s", "Volume", RangeModel::new(0.0, 100.0), &mut p.value));
            ui.allocate_space(vec2(1.0, 2000.0));
        });
        p.offset = out.state.clamped_offset().y;
    })
}

// eframe web order: pointerdown runs a pass before touchstart arrives.
fn touch_drag(h: &mut Harness<'_, Page>, from: Pos2, step: egui::Vec2, n: u32) {
    h.input_mut()
        .events
        .push(synth::pointer_down(from, PointerButton::Primary));
    h.step();
    h.input_mut()
        .events
        .extend([synth::touch(from, TouchPhase::Start), Event::PointerMoved(from)]);
    h.step();
    for i in 1..=n {
        let at = from + step * i as f32;
        h.input_mut()
            .events
            .extend([synth::touch(at, TouchPhase::Move), Event::PointerMoved(at)]);
        h.step();
    }
    let end = from + step * n as f32;
    h.input_mut().events.extend([
        synth::pointer_up(end, PointerButton::Primary),
        Event::PointerGone,
        synth::touch(end, TouchPhase::End),
    ]);
    h.step();
}

#[test]
fn slider_vertical_touch_flick_scrolls_the_page_and_keeps_the_value() {
    let mut h = page();
    h.run();
    let r = h.rect("s-slider");
    touch_drag(&mut h, r.center(), vec2(0.0, -20.0), 6);
    h.run_steps(30);
    assert!(
        h.state().value.offset > 50.0,
        "page scrolled: {}",
        h.state().value.offset
    );
    assert!(
        h.state().value.value.abs() < f64::EPSILON,
        "value kept: {}",
        h.state().value.value
    );
}

#[test]
fn slider_horizontal_touch_drag_changes_the_value_not_the_page() {
    let mut h = page();
    h.run();
    let r = h.rect("s-slider");
    touch_drag(&mut h, r.left_center() + vec2(60.0, 0.0), vec2(25.0, 0.0), 6);
    h.run();
    assert!(
        h.state().value.value > 20.0,
        "value followed: {}",
        h.state().value.value
    );
    assert!(
        h.state().value.offset.abs() < f32::EPSILON,
        "page stayed: {}",
        h.state().value.offset
    );
}

#[test]
fn slider_mouse_drag_is_immediate() {
    let mut h = page();
    h.run();
    let from = h.rect("s-slider").left_center() + vec2(60.0, 0.0);
    h.input_mut().events.push(Event::PointerMoved(from));
    h.step();
    h.input_mut()
        .events
        .push(synth::pointer_down(from, PointerButton::Primary));
    h.step();
    h.input_mut().events.push(Event::PointerMoved(from + vec2(25.0, 0.0)));
    h.step();
    assert!(
        h.state().value.value > 0.0,
        "value followed the mouse: {}",
        h.state().value.value
    );
}
