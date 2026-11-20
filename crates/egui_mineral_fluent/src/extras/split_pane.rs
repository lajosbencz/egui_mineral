use egui::{CornerRadius, Stroke};
use egui_mineral::layout::Fraction;
use egui_mineral::{
    layout::Axis,
    semantic::{KeyedSalt, StableId},
    splitter::{Splitter, SplitterResponse},
    widget::{Availability, Interaction, MineralUi},
};

use crate::ColorToken::{ColorCompoundBrandStrokePressed, ColorNeutralStroke1Hover, ColorNeutralStroke2};
use crate::{Fluent, FluentExt, spec::tokens::geometry as g, widget::focus as ring};

// Not in Fluent UI React v9; the divider is the Table column resize line (`colorNeutralStroke1` on hover).
const HANDLE: f32 = g::SPACING_HORIZONTAL_S;

/// Two resizable panes around a draggable [`Splitter`] handle; not in Fluent UI React v9.
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
            inner: Splitter::new(id_salt, label).handle_thickness(HANDLE),
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
        ui: &mut MineralUi<'_, Fluent>,
        first: impl FnOnce(&mut MineralUi<'_, Fluent>) -> A,
        second: impl FnOnce(&mut MineralUi<'_, Fluent>) -> B,
    ) -> SplitterResponse<A, B> {
        let r = self
            .inner
            .min_first(self.min_first)
            .min_second(self.min_second)
            .show(ui, first, second);
        let t = ui.tokens();
        let i = Interaction::of(&r.response, Availability::disabled_if(self.disabled));
        let (color, width) = if i.disabled {
            (ColorNeutralStroke2, g::STROKE_WIDTH_THIN)
        } else if i.dragged || i.pressed {
            (ColorCompoundBrandStrokePressed, g::STROKE_WIDTH_THICK)
        } else if i.hovered || i.focused {
            (ColorNeutralStroke1Hover, g::STROKE_WIDTH_THICK)
        } else {
            (ColorNeutralStroke2, g::STROKE_WIDTH_THIN)
        };
        let line = r.handle_rect;
        let stroke = Stroke::new(width, t.color(color));
        let p = ui.painter();
        if self.axis == Axis::Horizontal {
            p.vline(line.center().x, line.y_range(), stroke);
        } else {
            p.hline(line.x_range(), line.center().y, stroke);
        }
        if i.focused {
            ring::outline(p, line, CornerRadius::ZERO, &t);
        }
        r
    }
}
