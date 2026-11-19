use egui::{Key, Modifiers, Rect, vec2};
use egui_mineral::edit::{self, EditEvent};
use egui_mineral_material::text_inputs::TextField;

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
