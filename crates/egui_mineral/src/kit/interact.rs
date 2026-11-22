//! Interaction plumbing: tap-aware `interact`, press-to-focus, arrow locks, roving group memory, policy cursors,
//! key-down events.

use egui::{Context, Event, EventFilter, Id, Key, Modifiers, Rect, Response, Sense};

use crate::{
    focus::tab_stop_key,
    layout::Axis,
    widget::{Availability, MineralUi},
};

/// `ui.interact` whose response reports touch taps through `clicked()` and takes focus on press when
/// focusable ([`follow_press`]); senses hover only while disabled.
pub fn interact<K>(ui: &MineralUi<'_, K>, rect: Rect, id: Id, sense: Sense, availability: Availability) -> Response {
    let sense = if availability.is_enabled() {
        sense
    } else {
        Sense::hover()
    };
    let mut r = ui.interact(rect, id, sense);
    ui.pass_input().tap_on(&mut r);
    if sense.is_focusable() {
        follow_press(&r);
    }
    r
}

/// Applies the [`Policies::cursors`](crate::policy::Policies::cursors) cursor while `response` is hovered.
pub fn hover_cursor(core: &crate::runtime::Core, response: &Response, availability: Availability) {
    let ctx = core.ctx();
    let cursors = core.policies().cursors;
    let cursor = match availability {
        Availability::Enabled => cursors.enabled,
        Availability::Disabled => cursors.disabled,
    };
    if let Some(cursor) = cursor.filter(|_| response.hovered()) {
        ctx.set_cursor_icon(cursor);
    }
}

/// Any AccessKit request of `action` aimed at `id` among this pass's events.
pub fn accesskit_requested(ctx: &Context, id: Id, action: egui::accesskit::Action) -> bool {
    ctx.input(|i| i.num_accesskit_action_requests(id, action) > 0)
}

/// `r` clicked other than by Enter on its focus: HTML checkboxes, radios and switches toggle on Space only.
pub fn toggle_clicked(r: &egui::Response) -> bool {
    let by_enter =
        r.has_focus() && !r.clicked_by(egui::PointerButton::Primary) && r.ctx.input(|i| i.key_pressed(Key::Enter));
    r.clicked() && !by_enter
}

/// Key-down events of this pass without modifiers.
pub fn plain_key_presses(ctx: &Context) -> Vec<Key> {
    key_presses(ctx)
        .into_iter()
        .filter_map(|(k, m)| m.is_none().then_some(k))
        .collect()
}

/// Key-down events of this pass, with their modifiers.
pub fn key_presses(ctx: &Context) -> Vec<(Key, Modifiers)> {
    ctx.input(|i| {
        i.events
            .iter()
            .filter_map(|e| match e {
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => Some((*key, *modifiers)),
                _ => None,
            })
            .collect()
    })
}

/// Focuses `response` on a pointer press or click this pass, as browsers do for focusable elements (egui
/// itself only focuses text edits on click); `true` when focus moved. Focusable senses get it from
/// [`interact`] and [`crate::widget::MineralUi::mineral`]; roving items built with
/// [`crate::focus::roving_sense`] call it themselves, since [`crate::focus::TabStop::Skip`] is not
/// focusable. Keeps the group's Tab stop on the clicked item: "When a component is clicked/tapped, authors
/// should take the same steps to set the correct tabindex"
/// (<https://www.w3.org/WAI/ARIA/apg/practices/keyboard-interface/>).
pub fn follow_press(response: &Response) -> bool {
    let ctx = &response.ctx;
    // A press released in the same pass clears `is_pointer_button_down_on` but still clicks.
    let press = (response.is_pointer_button_down_on() && ctx.input(|i| i.pointer.any_pressed())) || response.clicked();
    let moved = press && !response.has_focus();
    if moved {
        ctx.memory_mut(|m| m.request_focus(response.id));
    }
    moved
}

/// Arrow keys a focused widget keeps from egui's focus navigation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Arrows {
    /// Arrows along one axis only.
    Axis(Axis),
    /// All four arrows (grids).
    Both,
}

/// Keeps egui from moving focus on the given arrow keys while `id` has focus.
pub fn lock_arrows(ctx: &Context, id: Id, arrows: Arrows) {
    let (horizontal_arrows, vertical_arrows) = match arrows {
        Arrows::Axis(Axis::Horizontal) => (true, false),
        Arrows::Axis(Axis::Vertical) => (false, true),
        Arrows::Both => (true, true),
    };
    ctx.memory_mut(|m| {
        m.set_focus_lock_filter(
            id,
            EventFilter {
                horizontal_arrows,
                vertical_arrows,
                ..EventFilter::default()
            },
        );
    });
}

/// Records the focused one of `items` (item index, widget id) as the group's last focused item for
/// [`crate::focus::tab_stop`].
pub fn remember(ctx: &Context, group: Id, items: impl IntoIterator<Item = (usize, Id)>) {
    if let Some(item) = items.into_iter().find(|(_, id)| ctx.memory(|m| m.has_focus(*id))) {
        ctx.data_mut(|d| d.insert_temp(tab_stop_key(group), item));
    }
}

/// Sense of a frame around an inner text editor: clicks focus the editor, the only Tab stop.
pub fn text_frame_sense() -> Sense {
    Sense::click() - Sense::FOCUSABLE
}
