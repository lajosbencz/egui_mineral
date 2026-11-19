use egui::{Key, Modifiers, Rect, vec2};
use egui_mineral::{
    edit::{self, EditEvent},
    number::NumberSource,
    range::{RangeModel, Scale},
};
use egui_mineral_material::{extras::NumberField, text_inputs::TextField};

use crate::support::{Harness, MineralHarness, Mui, SCREEN, harness};

struct Field<T> {
    value: T,
    rect: Rect,
    events: Vec<EditEvent>,
}

fn field<'a, T: 'a>(
    value: T,
    mut show: impl FnMut(&mut Mui<'_>, &mut T) -> (Rect, Option<EditEvent>) + 'a,
) -> Harness<'a, Field<T>> {
    harness(
        SCREEN,
        Field {
            value,
            rect: Rect::NOTHING,
            events: Vec::new(),
        },
        move |ui, f| {
            let (rect, event) = show(ui, &mut f.value);
            f.rect = rect;
            f.events.extend(event);
        },
    )
}

fn take<T>(h: &mut Harness<'_, Field<T>>) -> Vec<EditEvent> {
    std::mem::take(&mut h.state_mut().value.events)
}

fn focus<T>(h: &mut Harness<'_, Field<T>>) {
    let at = h.state().value.rect.center();
    h.click_at(at);
    take(h);
}

fn retype<T>(h: &mut Harness<'_, Field<T>>, text: &str) {
    h.key_press_modifiers(Modifiers::COMMAND, Key::A);
    h.type_text(text);
}

fn press<T>(h: &mut Harness<'_, Field<T>>, key: Key) -> Vec<EditEvent> {
    h.press(key);
    take(h)
}

fn text(ui: &mut Mui<'_>, value: &mut String) -> (Rect, Option<EditEvent>) {
    let r = ui.mineral(TextField::new("t", "Name", value).commit_on_submit(true));
    (r.rect, r.output().edit)
}

fn number(ui: &mut Mui<'_>, value: &mut f64) -> (Rect, Option<EditEvent>) {
    let r =
        ui.mineral(NumberField::new("n", "Gain", RangeModel::new(0.0, 10.0).step(1.0), value).commit_on_submit(true));
    (r.rect, r.output().edit)
}

#[test]
fn text_field_commits_on_enter_flush_and_blur_and_reverts_on_escape() {
    let mut h = field(String::from("a"), text);
    focus(&mut h);
    retype(&mut h, "b");
    assert_eq!(take(&mut h), [], "typing stays in the draft");
    assert_eq!(h.state().value.value, "a");
    assert_eq!(press(&mut h, Key::Enter), [EditEvent::Committed]);
    assert_eq!(h.state().value.value, "b");

    focus(&mut h);
    retype(&mut h, "c");
    edit::commit_pending(&h.ctx);
    h.run();
    assert_eq!(take(&mut h), [EditEvent::Committed]);
    assert_eq!(h.state().value.value, "c", "flushed while focused");
    retype(&mut h, "d");
    assert_eq!(press(&mut h, Key::Escape), [EditEvent::Cancelled]);
    assert_eq!(h.state().value.value, "c", "escape restores the committed value");

    focus(&mut h);
    retype(&mut h, "e");
    let below = h.state().value.rect.center_bottom() + vec2(0.0, h.state().value.rect.height());
    h.click_at(below);
    assert_eq!(take(&mut h), [EditEvent::Committed], "blur commits");
    assert_eq!(h.state().value.value, "e");
}

#[test]
fn number_field_buffers_typing_until_enter_or_flush() {
    let mut h = field(5.0, number);
    focus(&mut h);
    retype(&mut h, "7");
    assert_eq!(h.state().value.value, 5.0, "typing stays in the draft");
    assert_eq!(press(&mut h, Key::Enter), [EditEvent::Committed]);
    assert_eq!(h.state().value.value, 7.0);

    focus(&mut h);
    retype(&mut h, "8");
    edit::commit_pending(&h.ctx);
    h.run();
    assert_eq!(take(&mut h), [EditEvent::Committed]);
    assert_eq!(h.state().value.value, 8.0);
}

#[test]
fn number_field_reports_what_wrote_each_commit() {
    let mut h = harness(
        SCREEN,
        (5.0, Vec::new()),
        |ui, (v, seen): &mut (f64, Vec<(Option<EditEvent>, NumberSource)>)| {
            let range = RangeModel::new(0.0, 10.0).step(1.0);
            let r = ui.mineral(NumberField::new("n", "Gain", range, v).commit_on_submit(true));
            seen.extend(r.output().source.map(|s| (r.output().edit, s)));
        },
    );
    let at = h.rect("n-field").center();
    h.click_at(at);
    h.key_press_modifiers(Modifiers::COMMAND, Key::A);
    h.type_text("7");
    assert!(h.state().value.1.is_empty(), "buffered typing reports nothing");
    h.press(Key::Enter);
    h.click_at(at);
    h.press(Key::ArrowUp);
    h.hover_at(at);
    h.event(egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Line,
        delta: vec2(0.0, 1.0),
        phase: egui::TouchPhase::Move,
        source: egui::MouseWheelSource::Unknown,
        modifiers: Modifiers::NONE,
    });
    h.run();
    let committed = Some(EditEvent::Committed);
    assert_eq!(
        h.state().value.1,
        [
            (committed, NumberSource::Typed),
            (committed, NumberSource::Step),
            (committed, NumberSource::Wheel)
        ]
    );
    assert_eq!(h.state().value.0, 9.0);
}

#[test]
fn number_field_escape_restores_pre_edit_value() {
    let mut h = field(5.0, |ui, v: &mut f64| {
        let r = ui.mineral(NumberField::new("n", "Count", RangeModel::new(0.0, 10.0).step(1.0), v));
        (r.rect, None)
    });
    focus(&mut h);
    h.press(Key::ArrowUp);
    assert_eq!(h.state().value.value, 6.0, "arrow keeps focus and steps");
    h.press(Key::Escape);
    assert_eq!(h.state().value.value, 5.0, "escape restores");
}

#[test]
fn number_field_log_steps_land_on_significant_digits_and_typing_keeps_its_own() {
    let mut h = field(1.2, |ui, v: &mut f64| {
        let range = RangeModel::new(0.1, 100.0).scale(Scale::Log).significant(3);
        let r = ui.mineral(NumberField::new("n", "Gain", range, v).snap_typed(false));
        (r.rect, None)
    });
    focus(&mut h);
    h.press(Key::ArrowUp);
    assert_eq!(h.state().value.value, 1.29, "a log step lands on 3 digits");
    assert_eq!(
        h.node("n-field").value.as_deref(),
        Some("1.29"),
        "the text shows the rounded value"
    );
    h.press(Key::ArrowUp);
    assert_eq!(
        h.state().value.value,
        1.38,
        "the next step starts from the rounded draft"
    );
    retype(&mut h, "1.2345");
    assert_eq!(h.state().value.value, 1.2345, "typed digits stay off the grid");
}

#[test]
fn number_field_reports_changed_when_nan_is_rewritten_into_range() {
    let mut h = harness(SCREEN, (f64::NAN, false), |ui, (v, changed)| {
        *changed |= ui
            .mineral(NumberField::new("n", "Gain", RangeModel::new(0.0, 10.0).step(1.0), v))
            .changed();
    });
    h.run();
    assert!(h.state().value.0.is_finite() && h.state().value.1);
}

#[test]
fn number_field_focuses_by_handle_and_steps_on_the_wheel() {
    let mut h = harness(
        SCREEN,
        (5.0, None, false),
        |ui, (v, field, focus): &mut (f64, Option<egui::Id>, bool)| {
            let r = ui.mineral(NumberField::new("n", "Count", RangeModel::new(0.0, 10.0).step(1.0), v));
            *field = r.output().field;
            if std::mem::take(focus) {
                ui.memory_mut(|m| r.output().field.into_iter().for_each(|id| m.request_focus(id)));
            }
        },
    );
    h.state_mut().value.2 = true;
    h.run();
    let id = h.state().value.1.expect("shown");
    assert!(h.ctx.memory(|m| m.has_focus(id)), "the handle focuses the text");
    let over = h.rect("n-field").center();
    h.hover_at(over);
    for (unit, dy, want) in [
        (egui::MouseWheelUnit::Line, 1.0, 6.0),
        (egui::MouseWheelUnit::Line, -3.0, 5.0),
        (egui::MouseWheelUnit::Point, 10.0, 5.0),
        (egui::MouseWheelUnit::Point, 100.0, 6.0),
    ] {
        h.event(egui::Event::MouseWheel {
            unit,
            delta: vec2(0.0, dy),
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
            modifiers: Modifiers::NONE,
        });
        h.run();
        assert_eq!(h.state().value.0, want, "{unit:?} {dy}");
    }
}

#[test]
fn number_field_scrub_reports_its_start_and_one_edit() {
    let state = (5.0, 0, Vec::new());
    let mut h = harness(
        SCREEN,
        state,
        |ui, (v, starts, edits): &mut (f64, u32, Vec<EditEvent>)| {
            let r = ui.mineral(NumberField::new("n", "Count", RangeModel::new(0.0, 10.0).step(1.0), v).scrub(true));
            *starts += u32::from(r.output().scrub_started);
            edits.extend(r.output().edit);
        },
    );
    let from = h.rect("n-field").center();
    h.hover_at(from);
    h.drag_at(from);
    for dx in [10.0, 40.0, 80.0] {
        h.hover_at(from + vec2(dx, 0.0));
    }
    h.drop_at(from + vec2(80.0, 0.0));
    h.run();
    let (v, starts, edits) = &h.state().value;
    assert!(*v > 5.0, "scrubbed up: {v}");
    assert_eq!(*starts, 1, "one start per drag");
    assert_eq!(edits, &[EditEvent::Committed], "one edit at release");
}
