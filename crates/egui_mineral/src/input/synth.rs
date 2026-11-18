//! Raw egui events as a backend would deliver them, for agents and tests.

use egui::{Event, Key, Modifiers, PointerButton, Pos2, TouchDeviceId, TouchId, TouchPhase};

fn key(key: Key, modifiers: Modifiers, pressed: bool) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed,
        repeat: false,
        modifiers,
    }
}

/// Key press; `repeat` is never set.
#[must_use]
pub fn key_down(k: Key, modifiers: Modifiers) -> Event {
    key(k, modifiers, true)
}

/// Key release.
#[must_use]
pub fn key_up(k: Key, modifiers: Modifiers) -> Event {
    key(k, modifiers, false)
}

/// Key press followed by its release.
#[must_use]
pub fn key_tap(k: Key, modifiers: Modifiers) -> impl IntoIterator<Item = Event> {
    [key_down(k, modifiers), key_up(k, modifiers)]
}

fn pointer_button(pos: Pos2, button: PointerButton, pressed: bool) -> Event {
    Event::PointerButton {
        pos,
        button,
        pressed,
        modifiers: Modifiers::NONE,
    }
}

/// Unmodified press of a pointer button at `pos`.
#[must_use]
pub fn pointer_down(pos: Pos2, button: PointerButton) -> Event {
    pointer_button(pos, button, true)
}

/// Unmodified release of a pointer button at `pos`.
#[must_use]
pub fn pointer_up(pos: Pos2, button: PointerButton) -> Event {
    pointer_button(pos, button, false)
}

/// Primary-button press and release at `pos`, preceded by the pointer move.
#[must_use]
pub fn click(pos: Pos2) -> impl IntoIterator<Item = Event> {
    [
        Event::PointerMoved(pos),
        pointer_down(pos, PointerButton::Primary),
        pointer_up(pos, PointerButton::Primary),
    ]
}

/// Single-finger touch event at `pos` (device 0, id 0, no force).
#[must_use]
pub fn touch(pos: Pos2, phase: TouchPhase) -> Event {
    Event::Touch {
        device_id: TouchDeviceId(0),
        id: TouchId(0),
        phase,
        pos,
        force: None,
    }
}
