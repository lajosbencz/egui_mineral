use crate::msg;
use egui::{Sense, emath::Numeric};
use egui_mineral::{
    i18n::I18n,
    input::PanMode,
    kit,
    layout::{Axis, Constraints, Measure, MinTarget},
    range::{self, RangeModel},
    semantic::{AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use super::bar::Bar;
use crate::{Fluent, Size};

/// Fluent `RangeSlider` (`react-range-slider-preview`): two thumbs on one rail bound to `[start, end]` of any
/// [`Numeric`], kept ordered.
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct RangeSlider<'a, N = f64> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    values: &'a mut [N; 2],
    bar: Bar,
}

impl<'a, N: Numeric> RangeSlider<'a, N> {
    /// `range` sets the bounds and step, whole for integer `N`; `label` names the pair, each thumb is `<label> start` / `<label> end`.
    /// Agent key `<id_salt>-range-slider`, thumbs `<id_salt>-slider-start` and `<id_salt>-slider-end`.
    pub fn new(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        range: impl Into<RangeModel>,
        values: &'a mut [N; 2],
    ) -> Self {
        Self {
            salt: id_salt.into(),
            key: egui_mineral::agent_key!("range-slider"),
            label: label.into(),
            values,
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
        /// Replaces the `<id_salt>-range-slider` agent key.
        agent_key(key)
    }
}

impl<N: Numeric> MineralWidget for RangeSlider<'_, N> {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt);

    fn enabled(&self) -> bool {
        !self.bar.disabled
    }

    fn sense(&self) -> Sense {
        self.bar.sense(2)
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
        let before = self.values.map(Numeric::to_f64);
        let mut v = [before[0], before[1].max(before[0])];
        self.bar.update(ui, frame, &mut v, (&self.salt, &self.label));
        *self.values = v.map(range::to_numeric);
        let after = self.values.map(Numeric::to_f64);
        let changed = before.iter().zip(&after).any(|(a, b)| (a - b).abs() > f64::EPSILON);
        WidgetResponse::new(frame.response.clone(), ()).with_changed(changed)
    }

    fn semantics(&self, i18n: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let [start, end] = self.values.map(Numeric::to_f64);
        let value = i18n.tr(msg::RangeValue {
            start: self.bar.text(i18n, start).into(),
            end: self.bar.text(i18n, end).into(),
        });
        let mut n = SemanticNode::new(self.key.id(&self.salt), Role::Group, &self.label).value(value);
        n.state.disabled = self.bar.disabled;
        Some(n)
    }
}
