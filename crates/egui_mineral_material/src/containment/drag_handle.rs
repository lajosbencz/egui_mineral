use crate::Material;
use crate::MaterialExt as _;
use crate::spec::{ColorRole, tokens::drag_handle as m};
use crate::widget::{
    common::{self, Interaction},
    indication,
};
use egui::{Pos2, Rect, Response, vec2};
use egui_mineral::layout::Axis;
use egui_mineral::widget::{Availability, MineralUi};

// Compose VerticalDragHandle; `axis` is the resize direction.
pub(crate) fn paint(ui: &MineralUi<'_, Material>, response: &Response, center: Pos2, axis: Axis, disabled: bool) {
    let t = ui.rt().tokens();
    let i = Interaction::of(response, Availability::disabled_if(disabled));
    let pressed = !i.disabled && (i.pressed || i.focused);
    let (w, h, shape, color) = if i.dragged {
        (m::DRAGGED_WIDTH, m::DRAGGED_HEIGHT, m::DRAGGED_SHAPE, m::DRAGGED_COLOR)
    } else if pressed {
        (m::PRESSED_WIDTH, m::PRESSED_HEIGHT, m::PRESSED_SHAPE, m::PRESSED_COLOR)
    } else {
        (m::WIDTH, m::HEIGHT, m::SHAPE, m::COLOR)
    };
    let size = if axis == Axis::Horizontal {
        vec2(w, h)
    } else {
        vec2(h, w)
    };
    let handle = Rect::from_center_size(center, size);
    let radius = common::radius(handle, shape.scale().radius(&t.shape));
    let fill = if i.disabled {
        common::dim(t.role(color), &t)
    } else {
        t.role(color)
    };
    ui.painter().rect_filled(handle, radius, fill);
    indication::paint(ui, response, handle, radius, t.role(ColorRole::OnSurface), i, &t);
}
