use crate::Material;
use crate::MaterialExt;
use egui::{Pos2, Rect, Response, Stroke, vec2};
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::semantic::StableId;
use egui_mineral::widget::Availability;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    layout::Axis,
    splitter::{Splitter, SplitterResponse},
};

use crate::spec::{
    ColorRole,
    tokens::{divider, drag_handle as m},
};
use crate::widget::{
    common::{self, Interaction},
    indication,
};

// No MD3 counterpart; Compose VerticalDragHandle is not focusable, so keyboard focus shows the pressed state

/// Two resizable panes around a draggable [`Splitter`] handle; non-MD3.
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct SplitPane {
    inner: Splitter,
    axis: Axis,
    min_first: f32,
    min_second: f32,
    disabled: bool,
}

impl SplitPane {
    /// Panes side by side; `label` names the divider for assistive technology. Agent key `<id_salt>-splitter`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        Self {
            inner: Splitter::new(id_salt, label).handle_thickness(m::DRAGGED_WIDTH.max(m::PRESSED_WIDTH)),
            axis: Axis::Horizontal,
            min_first: 0.0,
            min_second: 0.0,
            disabled: false,
        }
    }

    /// Split direction; `Vertical` stacks the panes. Default `Horizontal`.
    pub fn axis(mut self, axis: Axis) -> Self {
        self.inner = self.inner.axis(axis);
        self.axis = axis;
        self
    }

    /// Replaces the `<id_salt>-splitter` agent key.
    pub fn agent_key(mut self, key: impl Into<StableId>) -> Self {
        self.inner = self.inner.agent_key(key);
        self
    }

    /// First pane share in 0..=1 before the user drags.
    pub fn default_fraction(mut self, default_fraction: f32) -> Self {
        self.inner = self.inner.default_fraction(default_fraction);
        self
    }

    /// First pane minimum in dp; negative values snap to 0.
    #[track_caller]
    pub fn min_first(mut self, size: f32) -> Self {
        self.min_first = egui_mineral::kit::sanitize::at_least(size, 0.0.., "min_first");
        self
    }

    /// Fixes the divider; drags and keys are ignored.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.inner = self.inner.disabled(disabled);
        self.disabled = disabled;
        self
    }

    /// Second pane minimum in dp; negative values snap to 0.
    #[track_caller]
    pub fn min_second(mut self, size: f32) -> Self {
        self.min_second = egui_mineral::kit::sanitize::at_least(size, 0.0.., "min_second");
        self
    }

    /// Lays out `first` and `second` around a draggable divider.
    pub fn show<A, B>(
        self,
        ui: &mut MineralUi<'_, Material>,
        first: impl FnOnce(&mut MineralUi<'_, Material>) -> A,
        second: impl FnOnce(&mut MineralUi<'_, Material>) -> B,
    ) -> SplitterResponse<A, B> {
        let rt = ui.rt();
        let r = self
            .inner
            .min_first(self.min_first)
            .min_second(self.min_second)
            .show(ui, first, second);
        let t = rt.tokens();
        let side_by_side = self.axis == Axis::Horizontal;
        let line = r.handle_rect;
        let p = ui.painter();
        let stroke = Stroke::new(divider::THICKNESS, t.role(divider::COLOR));
        if side_by_side {
            p.vline(line.center().x, line.y_range(), stroke);
        } else {
            p.hline(line.x_range(), line.center().y, stroke);
        }
        paint_drag_handle(ui, &r.response, line.center(), self.axis, self.disabled);
        r
    }
}

// Compose VerticalDragHandle; `axis` is the resize direction.
pub(crate) fn paint_drag_handle(
    ui: &MineralUi<'_, Material>,
    response: &Response,
    center: Pos2,
    axis: Axis,
    disabled: bool,
) {
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
