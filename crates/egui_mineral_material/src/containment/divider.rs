use crate::Material;
use crate::MaterialExt;
use egui::{IdSalt, Rangef, Sense};
use egui_mineral::i18n::I18n;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    layout::{Axis, Constraints, Measure, MinTarget},
    semantic::{KeyedSalt, SemanticNode},
    widget::{MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::spec::tokens::divider;

/// MD3 divider (Compose `HorizontalDivider`, `VerticalDivider`).
#[derive(Clone, Copy, Debug, PartialEq)]
#[must_use = "show with ui.mineral(..)"]
pub struct Divider {
    id_salt: IdSalt,
    axis: Axis,
    start: f32,
    end: f32,
}

impl Divider {
    /// Horizontal, edge to edge.
    pub fn new(id_salt: impl Into<KeyedSalt>) -> Self {
        Self {
            id_salt: id_salt.into().id_salt(),
            axis: Axis::Horizontal,
            start: 0.0,
            end: 0.0,
        }
    }

    /// Leading inset in dp; not finite or negative becomes 0.
    #[track_caller]
    pub fn inset_start(mut self, inset: f32) -> Self {
        self.start = egui_mineral::kit::sanitize::at_least(inset, 0.0.., "inset_start");
        self
    }

    /// Trailing inset in dp; not finite or negative becomes 0.
    #[track_caller]
    pub fn inset_end(mut self, inset: f32) -> Self {
        self.end = egui_mineral::kit::sanitize::at_least(inset, 0.0.., "inset_end");
        self
    }

    /// Default [`Axis::Horizontal`].
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }
}

impl MineralWidget for Divider {
    type Kit = Material;
    type State = ();
    type Output = ();

    fn id_salt(&self) -> IdSalt {
        self.id_salt
    }

    fn sense(&self) -> Sense {
        Sense::hover()
    }

    fn min_target(&self, _: &MineralUi<'_, Self::Kit>) -> MinTarget {
        MinTarget::None
    }

    /// Fills the main axis.
    fn layout(&self, ui: &MineralUi<'_, Material>, c: Constraints, (): &Self::State) -> Measure {
        let max = self.axis.main(c.max);
        let length = if max.is_finite() {
            max
        } else {
            self.axis.main(ui.available_size())
        };
        c.constrain(self.axis.vec2(length, divider::THICKNESS)).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Material>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let rect = frame.zones.visual;
        let axis = self.axis;
        let main = axis.main_range(rect);
        ui.painter().rect_filled(
            axis.rect(
                Rangef::new(main.min + self.start, main.max - self.end),
                axis.cross_range(rect),
            ),
            0.0,
            t.role(divider::COLOR),
        );
        WidgetResponse::new(frame.response.clone(), ())
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        None
    }
}
