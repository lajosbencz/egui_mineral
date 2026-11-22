//! Overlay key handling: Escape and the default action.

use egui::{Area, Context, Event, Id, Key, LayerId, Memory, Modifiers, Order, Pos2, Response, Ui};

use crate::{overlay::top_modal, runtime::Core};

/// `Area` that skips egui's fade-in under reduced motion and lays out from the reading start; use
/// for every library area.
pub fn area(core: &Core, id: Id) -> Area {
    setup(core, Area::new(id))
}

/// [`area`] on the foreground layer at `pos`, not constrained to the screen.
pub fn floating(core: &Core, id: Id, pos: Pos2) -> Area {
    area(core, id).order(Order::Foreground).fixed_pos(pos).constrain(false)
}

/// [`egui::Modal::default_area`] set up as [`area`]; use for every library modal. Its layer counts as modal for
/// [`crate::shortcut::scope_of`] from this pass on.
pub fn modal_area(core: &Core, id: Id) -> Area {
    let area = setup(core, egui::Modal::default_area(id));
    crate::overlay::declare_modal(core.ctx(), area.layer());
    area
}

/// [`egui::Modal`] on [`modal_area`]; use for every library modal.
pub fn modal(core: &Core, id: Id) -> egui::Modal {
    modal_on(core, id, modal_area(core, id))
}

/// [`egui::Modal`] on `area`, whose layer counts as modal like [`modal_area`]'s; for modals with a custom area.
pub fn modal_on(core: &Core, id: Id, area: Area) -> egui::Modal {
    crate::overlay::declare_modal(core.ctx(), area.layer());
    egui::Modal::new(id).area(area)
}

/// [`crate::overlay::outside_press`] that ignores the frame in which the overlay `id` opened. Same call contract as
/// [`crate::overlay::dismissal`].
pub fn outside_press_after_open(core: &Core, id: Id, inside: &[egui::Rect]) -> bool {
    !crate::overlay::opening_frame(core.ctx(), id) && crate::overlay::outside_press(core, inside)
}

fn setup(core: &Core, area: Area) -> Area {
    area.fade_in(!core.motion().reduced())
        .layout(core.i18n().direction().column())
}

/// Moves `layer` to the top while egui's layer order has it below `parent`. Layers raised in the same pass keep
/// their previous relative order, so a parent re-shown or pressed while `layer` opens can stay on top; call each
/// pass for overlays opened from another layer (a popup in a dialog).
pub fn keep_above(ctx: &Context, layer: LayerId, parent: LayerId) {
    let below = layer != parent && ctx.memory(|m| m.layer_ids().find(|&l| l == layer || l == parent)) == Some(layer);
    if below {
        ctx.move_to_top(layer);
    }
}

/// Consumes an unmodified Escape press.
pub fn consume_escape(ctx: &Context) -> bool {
    ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
}

/// [`consume_escape`] while `id` is focused or was at the start of the pass: egui drops focus on Escape
/// before widgets run unless their event filter keeps it.
pub fn consume_escape_from(ctx: &Context, id: Id) -> bool {
    ctx.memory(|m| m.has_focus(id) || m.had_focus_last_frame(id)) && consume_escape(ctx)
}

/// Unmodified Enter, consumed, when `ui` is the topmost modal and the focused widget does not
/// handle Enter itself: a text edit still focused after the press (multi-line; single-line edits
/// give up focus on Enter) or a clickable widget (egui clicks it on Enter).
pub fn default_action_pressed(ui: &Ui) -> bool {
    default_action_pressed_with(ui, |r| r.sense.senses_click())
}

/// [`default_action_pressed`] with `handles_enter` deciding for the focused widget other than a text edit, given
/// its latest [`Response`]. egui clicks a focused clickable widget on Enter while it is built, so a widget that
/// `handles_enter` rejects (a checkbox, WPF style) misses the press only when this runs before it.
pub fn default_action_pressed_with(ui: &Ui, handles_enter: impl FnOnce(&Response) -> bool) -> bool {
    let ctx = ui.ctx();
    if top_modal(ctx) != Some(ui.layer_id()) {
        return false;
    }
    if let Some(focused) = ctx.memory(Memory::focused)
        && (ctx.text_edit_focused() || ctx.read_response(focused).is_some_and(|r| handles_enter(&r)))
    {
        return false;
    }
    ctx.input_mut(|i| {
        let before = i.events.len();
        i.events.retain(|e| {
            !matches!(
                e,
                Event::Key { key: Key::Enter, pressed: true, repeat: false, modifiers, .. } if modifiers.is_none()
            )
        });
        i.events.len() != before
    })
}
