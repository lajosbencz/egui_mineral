use crate::msg;
use egui::Sense;
use egui_mineral::{
    i18n::I18n,
    kit,
    layout::{Axis, Constraints, Measure, MinTarget},
    range::RangeModel,
    semantic::{AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use super::bar::Bar;
use crate::{Fluent, Size};

/// Value of a [`RangeSlider`]: the two thumbs, `start <= end`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RangeValue {
    /// Lower thumb.
    pub start: f64,
    /// Upper thumb.
    pub end: f64,
}

impl RangeValue {
    /// Thumbs at `start` and `end`.
    pub fn new(start: f64, end: f64) -> Self {
        Self { start, end }
    }
}

/// Fluent `RangeSlider` (`react-range-slider-preview`): two thumbs on one rail bound to a [`RangeValue`].
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct RangeSlider<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    value: &'a mut RangeValue,
    bar: Bar,
}

impl<'a> RangeSlider<'a> {
    /// `range` sets the bounds and step; `label` names the pair, each thumb is `<label> start` / `<label> end`.
    /// Agent key `<id_salt>-range-slider`, thumbs `<id_salt>-slider-start` and `<id_salt>-slider-end`.
    pub fn new(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        range: impl Into<RangeModel>,
        value: &'a mut RangeValue,
    ) -> Self {
        Self {
            salt: id_salt.into(),
            key: AgentKey::Kind("range-slider"),
            label: label.into(),
            value,
            bar: Bar {
                range: range.into(),
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

impl MineralWidget for RangeSlider<'_> {
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

    fn pan_axis(&self) -> Option<Axis> {
        Some(self.bar.axis)
    }

    fn min_target(&self, _: &MineralUi<'_, Self::Kit>) -> MinTarget {
        MinTarget::None
    }

    fn layout(&self, ui: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        self.bar.measure(ui, c)
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let before = *self.value;
        let mut v = [before.start, before.end.max(before.start)];
        self.bar.update(ui, frame, &mut v, (&self.salt, &self.label));
        *self.value = RangeValue::new(v[0], v[1]);
        let changed = (before.start - v[0]).abs() > f64::EPSILON || (before.end - v[1]).abs() > f64::EPSILON;
        WidgetResponse::new(frame.response.clone(), ()).with_changed(changed)
    }

    fn semantics(&self, i18n: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let RangeValue { start, end } = *self.value;
        let value = i18n.tr(msg::RangeValue {
            start: self.bar.text(i18n, start).into(),
            end: self.bar.text(i18n, end).into(),
        });
        let mut n = SemanticNode::new(self.key.id(&self.salt), Role::Group, &self.label).value(value);
        n.state.disabled = self.bar.disabled;
        Some(n)
    }
}
