use egui::{Sense, emath::Numeric};
use egui_mineral::{
    i18n::I18n,
    input::PanMode,
    kit,
    layout::{Axis, Constraints, Measure, MinTarget},
    range::{self, RangeModel},
    semantic::{AgentKey, KeyedSalt, SemanticNode},
    widget::{MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use super::bar::Bar;
use crate::{Fluent, Size};

/// Fluent `Slider`: one thumb on a rail bound to `value`, any [`Numeric`].
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Slider<'a, N = f64> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    value: &'a mut N,
    bar: Bar,
}

impl<'a, N: Numeric> Slider<'a, N> {
    /// `range` sets the bounds and step, whole for integer `N`; `label` is the accessible name. Agent key `<id_salt>-slider`.
    pub fn new(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        range: impl Into<RangeModel>,
        value: &'a mut N,
    ) -> Self {
        Self {
            salt: id_salt.into(),
            key: egui_mineral::agent_key!("slider"),
            label: label.into(),
            value,
            bar: Bar {
                range: range.into().for_numeric::<N>(),
                size: Size::Medium,
                axis: Axis::Horizontal,
                disabled: false,
            },
        }
    }

    /// [`Size::Small`] or [`Size::Medium`] (default); [`Size::Large`] is medium, as in React.
    pub fn size(mut self, size: Size) -> Self {
        self.bar.size = size;
        self
    }

    /// `vertical` is [`Axis::Vertical`]; default [`Axis::Horizontal`].
    pub fn axis(mut self, axis: Axis) -> Self {
        self.bar.axis = axis;
        self
    }

    /// Greys out the slider and ignores input.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.bar.disabled = disabled;
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-slider` agent key.
        agent_key(key)
    }
}

impl<N: Numeric> MineralWidget for Slider<'_, N> {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt);

    fn enabled(&self) -> bool {
        !self.bar.disabled
    }

    fn sense(&self) -> Sense {
        self.bar.sense(1)
    }

    fn pan(&self) -> Option<PanMode> {
        Some(PanMode::Axis(self.bar.axis))
    }

    fn min_target(&self, _: &MineralUi<'_, Self::Kit>) -> MinTarget {
        MinTarget::None
    }

    fn layout(&self, ui: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        self.bar.measure(ui, c)
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let before = self.value.to_f64();
        let mut v = [before];
        self.bar.update(ui, frame, &mut v, (&self.salt, &self.label));
        *self.value = range::to_numeric(v[0]);
        let changed = (before - self.value.to_f64()).abs() > f64::EPSILON;
        WidgetResponse::new(frame.response.clone(), ()).with_changed(changed)
    }

    fn semantics(&self, i18n: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        Some(
            self.bar
                .node(i18n, self.key.id(&self.salt), &self.label, self.value.to_f64()),
        )
    }
}
