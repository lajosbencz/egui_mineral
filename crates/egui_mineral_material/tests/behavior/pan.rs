//! Drag widgets inside a scroll area: touch pans off a single axis, or not held first on a free pannable, scroll the
//! page; mouse drags always reach them.

use egui::{Color32, Event, PointerButton, Pos2, TouchPhase, vec2};
use egui_mineral::{date::Time, input::synth, range::RangeModel};
use egui_mineral_material::{
    extras::ColorPicker,
    selection::{Switch, TimePicker, ValueSlider},
};

use crate::support::{Harness, MineralHarness, still};

#[derive(Default)]
struct Page {
    offset: f32,
    value: f64,
    on: bool,
}

fn page<'a>() -> Harness<'a, Page> {
    still(vec2(400.0, 300.0), Page::default(), |ui, p| {
        let rt = ui.rt();
        let out = egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
            let mut mui = rt.ui(ui);
            mui.mineral(ValueSlider::new(
                "s",
                "Volume",
                RangeModel::new(0.0, 100.0),
                &mut p.value,
            ));
            mui.mineral(Switch::new("sw", "Enabled", &mut p.on));
            ui.allocate_space(vec2(1.0, 2000.0));
        });
        p.offset = out.state.clamped_offset().y;
    })
}

// eframe web order: pointerdown runs a pass before touchstart arrives.
fn touch_drag<S>(h: &mut Harness<'_, S>, from: Pos2, step: egui::Vec2, n: u32) {
    held_touch_drag(h, from, step, n, 0);
}

// Rests `hold` passes at 1/60 s before moving.
fn held_touch_drag<S>(h: &mut Harness<'_, S>, from: Pos2, step: egui::Vec2, n: u32, hold: u32) {
    h.input_mut()
        .events
        .push(synth::pointer_down(from, PointerButton::Primary));
    h.step();
    h.input_mut()
        .events
        .extend([synth::touch(from, TouchPhase::Start), Event::PointerMoved(from)]);
    h.step();
    for _ in 0..hold {
        h.step();
    }
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

fn mouse_drag(h: &mut Harness<'_, Page>, from: Pos2, step: egui::Vec2, n: u32) {
    h.input_mut().events.push(Event::PointerMoved(from));
    h.step();
    h.input_mut()
        .events
        .push(synth::pointer_down(from, PointerButton::Primary));
    h.step();
    for i in 1..=n {
        h.input_mut().events.push(Event::PointerMoved(from + step * i as f32));
        h.step();
    }
    h.input_mut()
        .events
        .push(synth::pointer_up(from + step * n as f32, PointerButton::Primary));
    h.step();
}

#[test]
fn slider_vertical_touch_flick_scrolls_the_page_and_keeps_the_value() {
    let mut h = page();
    h.step();
    let r = h.rect("s-slider");
    touch_drag(&mut h, r.center(), vec2(0.0, -20.0), 6);
    h.run();
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
    h.step();
    let r = h.rect("s-slider");
    touch_drag(&mut h, r.left_center() + vec2(60.0, 0.0), vec2(25.0, 0.0), 6);
    h.run();
    assert!(
        h.state().value.value > 40.0,
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
fn slider_mouse_drag_is_immediate_on_either_axis() {
    let mut h = page();
    h.step();
    let r = h.rect("s-slider");
    mouse_drag(&mut h, r.left_center() + vec2(60.0, 0.0), vec2(25.0, 0.0), 4);
    let v = h.state().value.value;
    assert!(v > 30.0, "value followed the mouse: {v}");
    mouse_drag(&mut h, r.left_center() + vec2(60.0, 0.0), vec2(0.0, 25.0), 4);
    assert!(h.state().value.offset.abs() < f32::EPSILON, "page stayed");
}

#[test]
fn switch_vertical_touch_flick_scrolls_the_page_and_keeps_the_state() {
    let mut h = page();
    h.step();
    let r = h.rect("sw-switch");
    touch_drag(&mut h, r.center(), vec2(0.0, -20.0), 6);
    h.run();
    assert!(
        h.state().value.offset > 50.0,
        "page scrolled: {}",
        h.state().value.offset
    );
    assert!(!h.state().value.on, "switch kept");
}

#[test]
fn switch_horizontal_touch_drag_flips_it() {
    let mut h = page();
    h.step();
    let r = h.rect("sw-switch");
    touch_drag(&mut h, r.center(), vec2(25.0, 0.0), 4);
    h.run();
    assert!(h.state().value.on, "switch flipped");
    assert!(h.state().value.offset.abs() < f32::EPSILON, "page stayed");
}

#[test]
fn switch_mouse_drag_flips_it() {
    let mut h = page();
    h.step();
    let r = h.rect("sw-switch");
    mouse_drag(&mut h, r.center(), vec2(25.0, 0.0), 4);
    assert!(h.state().value.on, "switch flipped");
}

#[derive(Default)]
struct Free {
    offset: f32,
    color: Color32,
    time: Time,
}

// A 2D pannable at the top of a page: the color picker's area, or the time picker's dial.
fn free_page<'a>(dial: bool) -> Harness<'a, Free> {
    still(vec2(400.0, 600.0), Free::default(), move |ui, p| {
        let rt = ui.rt();
        let out = egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
            let mut mui = rt.ui(ui);
            if dial {
                TimePicker::new("tp", "Time", &mut p.time).show(&mut mui);
            } else {
                mui.mineral(ColorPicker::new("cp", "Color", &mut p.color));
            }
            ui.allocate_space(vec2(1.0, 2000.0));
        });
        p.offset = out.state.clamped_offset().y;
    })
}

#[test]
fn free_pannables_let_a_quick_touch_flick_scroll_the_page() {
    for (dial, key) in [(false, "cp-area"), (true, "tp-hour-dial")] {
        let mut h = free_page(dial);
        h.step();
        let before = (h.state().value.color, h.state().value.time);
        let r = h.rect(key);
        touch_drag(&mut h, r.center(), vec2(0.0, -20.0), 6);
        h.run();
        let s = &h.state().value;
        assert!(s.offset > 50.0, "{key}: page scrolled: {}", s.offset);
        assert_eq!((s.color, s.time), before, "{key}: value kept");
    }
}

#[test]
fn free_pannables_take_a_touch_drag_held_first() {
    for (dial, key) in [(false, "cp-area"), (true, "tp-hour-dial")] {
        let mut h = free_page(dial);
        h.step();
        let before = (h.state().value.color, h.state().value.time);
        let r = h.rect(key);
        // 12 passes rest past the 150 ms hold.
        held_touch_drag(&mut h, r.center(), vec2(15.0, 10.0), 4, 12);
        h.run();
        let s = &h.state().value;
        assert!(s.offset.abs() < f32::EPSILON, "{key}: page stayed: {}", s.offset);
        assert_ne!((s.color, s.time), before, "{key}: value followed");
    }
}
