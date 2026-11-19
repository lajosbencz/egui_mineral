use egui::Sense;
use egui_mineral::{
    i18n::I18n,
    kit,
    layout::{Axis, Constraints, Measure, MinTarget},
    range::RangeModel,
    semantic::{AgentKey, KeyedSalt, SemanticNode},
    widget::{MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use super::bar::Bar;
use crate::{Fluent, Size};

/// Fluent `Slider`: one thumb on a rail bound to `value`.
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Slider<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    value: &'a mut f64,
    bar: Bar,
}

impl<'a> Slider<'a> {
    /// `range` sets the bounds and step; `label` is the accessible name. Agent key `<id_salt>-slider`.
    pub fn new(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        range: impl Into<RangeModel>,
        value: &'a mut f64,
    ) -> Self {
        Self {
            salt: id_salt.into(),
            key: AgentKey::Kind("slider"),
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
        /// Replaces the `<id_salt>-slider` agent key.
        agent_key(key)
    }
}

impl MineralWidget for Slider<'_> {
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

    fn min_target(&self) -> MinTarget {
        MinTarget::None
    }

    fn layout(&self, ui: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        self.bar.measure(ui, c)
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let before = *self.value;
        let mut v = [before];
        self.bar.update(ui, frame, &mut v, (&self.salt, &self.label));
        *self.value = v[0];
        WidgetResponse::new(frame.response.clone(), ()).with_changed((before - v[0]).abs() > f64::EPSILON)
    }

    fn semantics(&self, i18n: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        Some(self.bar.node(i18n, self.key.id(&self.salt), &self.label, *self.value))
    }
}
