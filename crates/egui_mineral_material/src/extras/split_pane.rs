use crate::Material;
use crate::MaterialExt;
use egui::Stroke;
use egui_mineral::layout::Fraction;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::semantic::StableId;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    layout::Axis,
    splitter::{Splitter, SplitterResponse},
};

use crate::spec::tokens::{divider, drag_handle as m};

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

    /// First pane share before the user drags.
    pub fn default_fraction(mut self, default_fraction: impl Into<Fraction>) -> Self {
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
        crate::containment::drag_handle::paint(ui, &r.response, line.center(), self.axis, self.disabled);
        r
    }
}
