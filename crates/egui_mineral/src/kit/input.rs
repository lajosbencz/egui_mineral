//! Widget input plumbing: tap-aware `interact`, policy cursors, key-down events.

use egui::{Context, Event, Id, Key, Modifiers, Rect, Response, Sense};

use crate::widget::{Availability, MineralUi};

/// `ui.interact` whose response reports touch taps through `clicked()` and takes focus on press when
/// focusable ([`super::focus::follow_press`]); senses hover only while disabled.
pub fn interact<K>(ui: &MineralUi<'_, K>, rect: Rect, id: Id, sense: Sense, availability: Availability) -> Response {
    let sense = if availability.is_enabled() {
        sense
    } else {
        Sense::hover()
    };
    let mut r = ui.interact(rect, id, sense);
    ui.pass_input().tap_on(&mut r);
    if sense.is_focusable() {
        super::focus::follow_press(&r);
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
