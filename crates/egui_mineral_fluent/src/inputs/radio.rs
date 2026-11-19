use egui::{Color32, CornerRadius, Sense, Stroke, Ui};
use egui_mineral::{
    focus::{self, RovingFocus},
    i18n::I18n,
    kit,
    layout::{Axis, Constraints, Measure, MinTarget},
    semantic::{self, AgentKey, KeyedSalt, Role, SemanticNode, StableId},
    shortcut::Mnemonic,
    widget::{Availability, Interaction, MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use super::choice::{Position, Spec, WRAP_SLACK};
use crate::ColorToken::{
    ColorCompoundBrandForeground1, ColorCompoundBrandForeground1Hover, ColorCompoundBrandForeground1Pressed,
    ColorCompoundBrandStroke, ColorCompoundBrandStrokeHover, ColorCompoundBrandStrokePressed, ColorNeutralForeground1,
    ColorNeutralForeground2, ColorNeutralForeground3, ColorNeutralForegroundDisabled, ColorNeutralStrokeAccessible,
    ColorNeutralStrokeAccessibleHover, ColorNeutralStrokeAccessiblePressed, ColorNeutralStrokeDisabled,
};
use crate::{
    Fluent, FluentExt,
    spec::{
        defaults::radio as rb,
        tokens::{geometry as g, typography as ty},
    },
    widget::{
        focus as ring,
        state::{Ramp, State},
    },
};

/// Side of the label a [`Radio`] puts it on (`labelPosition`); mirrored in right-to-left locales.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RadioLabelPosition {
    /// `after`: trailing the circle.
    #[default]
    After,
    /// `below`: under the circle, centered.
    Below,
}

/// Fluent `Radio`: one choice, selected when `selected`; a click reports `changed`. [`RadioGroup`] binds a value
/// and navigates by arrow keys.
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Radio {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    selected: bool,
    position: RadioLabelPosition,
    label_visible: bool,
    tab_stop: bool,
    mnemonic: Option<Mnemonic>,
    disabled: bool,
}

impl Radio {
    /// `label` is the visible text and accessible name; agent key `<id_salt>-radio`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        Self {
            salt: id_salt.into(),
            key: AgentKey::Kind("radio"),
            label: label.into(),
            selected: false,
            position: RadioLabelPosition::After,
            label_visible: true,
            tab_stop: true,
            mnemonic: None,
            disabled: false,
        }
    }

    /// `checked`.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Default [`RadioLabelPosition::After`].
    pub fn label_position(mut self, position: RadioLabelPosition) -> Self {
        self.position = position;
        self
    }

    /// `false` keeps the label for assistive technology only.
    pub fn label_visible(mut self, label_visible: bool) -> Self {
        self.label_visible = label_visible;
        self
    }

    /// `false` reaches the radio by arrow keys only; a group has one Tab stop.
    pub fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }

    /// Alt+letter selects the radio; the letter is underlined while mnemonics are visible.
    pub fn mnemonic(mut self, mnemonic: char) -> Self {
        self.mnemonic = Some(Mnemonic::new(mnemonic));
        self
    }

    crate::widget::setters! {
        /// Greys out the radio and ignores input.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-radio` agent key.
        agent_key(key)
    }

    fn spec(&self) -> Spec {
        Spec {
            indicator: egui::Vec2::splat(rb::INDICATOR),
            position: match self.position {
                RadioLabelPosition::After => Position::After,
                RadioLabelPosition::Below => Position::Below,
            },
            line_height: ty::LINE_HEIGHT_BASE_300,
        }
    }

    fn plan(&self, ui: &MineralUi<'_, Fluent>, max: f32) -> super::choice::Plan {
        let spec = self.spec();
        let label = self.label_visible.then(|| {
            let format = crate::typography::format(
                ui.ctx(),
                crate::typography::TypeSpec {
                    size: ty::FONT_SIZE_BASE_300,
                    line_height: ty::LINE_HEIGHT_BASE_300,
                    weight: ty::FONT_WEIGHT_REGULAR,
                },
                Color32::PLACEHOLDER,
            );
            spec.label(ui.core(), ui.painter(), &self.label, &format, self.mnemonic, max)
        });
        spec.plan(label)
    }

    // react-radio useRadioStyles.styles.ts useInputBaseClassName: label, indicator border and dot color.
    fn colors(&self, s: State, t: &crate::theme::Tokens) -> [Color32; 3] {
        let by = |r: Ramp| r.pick(s, t);
        if self.selected {
            [
                by(Ramp::new(
                    ColorNeutralForeground1,
                    ColorNeutralForeground1,
                    ColorNeutralForeground1,
                    ColorNeutralForegroundDisabled,
                )),
                by(Ramp::new(
                    ColorCompoundBrandStroke,
                    ColorCompoundBrandStrokeHover,
                    ColorCompoundBrandStrokePressed,
                    ColorNeutralStrokeDisabled,
                )),
                by(Ramp::new(
                    ColorCompoundBrandForeground1,
                    ColorCompoundBrandForeground1Hover,
                    ColorCompoundBrandForeground1Pressed,
                    ColorNeutralForegroundDisabled,
                )),
            ]
        } else {
            [
                by(Ramp::new(
                    ColorNeutralForeground3,
                    ColorNeutralForeground2,
                    ColorNeutralForeground1,
                    ColorNeutralForegroundDisabled,
                )),
                by(Ramp::new(
                    ColorNeutralStrokeAccessible,
                    ColorNeutralStrokeAccessibleHover,
                    ColorNeutralStrokeAccessiblePressed,
                    ColorNeutralStrokeDisabled,
                )),
                Color32::TRANSPARENT,
            ]
        }
    }
}

impl MineralWidget for Radio {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt, enabled);

    fn sense(&self) -> Sense {
        focus::roving_sense(
            Sense::click(),
            if self.tab_stop {
                focus::TabStop::Stop
            } else {
                focus::TabStop::Skip
            },
        )
    }

    fn access_key(&self) -> Option<Mnemonic> {
        self.mnemonic
    }

    fn min_target(&self) -> MinTarget {
        MinTarget::None
    }

    fn layout(&self, ui: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        c.constrain(self.plan(ui, c.max.x).size).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let r = &frame.response;
        let availability = Availability::disabled_if(self.disabled);
        kit::input::hover_cursor(ui.core(), r, availability);
        kit::focus::follow_press(r);
        let changed = r.clicked() && !self.disabled && !self.selected;
        self.selected |= changed;
        let i = Interaction::of(r, availability);
        let s = State::of(i);
        let [label, border, dot] = self.colors(s, &t);
        let visual = frame.zones.visual;
        let placed = self
            .plan(ui, visual.width() + WRAP_SLACK)
            .place(visual, ui.i18n().direction());
        let p = ui.painter();
        let center = placed.indicator.center();
        p.circle_stroke(
            center,
            rb::INDICATOR / 2.0 - g::STROKE_WIDTH_THIN / 2.0,
            Stroke::new(g::STROKE_WIDTH_THIN, border),
        );
        if self.selected {
            p.circle_filled(center, rb::INDICATOR * rb::DOT_SCALE / 2.0, dot);
        }
        if let Some((at, galley)) = placed.text {
            p.galley(at, galley, label);
        }
        if i.focused {
            ring::outline(p, visual, CornerRadius::from(g::BORDER_RADIUS_MEDIUM), &t);
        }
        WidgetResponse::new(r.clone(), ()).with_changed(changed)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(self.key.id(&self.salt), Role::RadioButton, &self.label)
            .clickable(Availability::disabled_if(self.disabled));
        n.state.checked = Some(self.selected.into());
        Some(n)
    }
}

/// Option of a [`RadioGroup`].
#[derive(Clone, Debug)]
#[must_use = "add to a RadioGroup"]
pub struct RadioOption<T> {
    value: T,
    label: String,
    disabled: bool,
    salt: Option<KeyedSalt>,
}

impl<T> RadioOption<T> {
    /// Picking the option writes `value`.
    pub fn new(value: T, label: impl Into<String>) -> Self {
        Self {
            value,
            label: label.into(),
            disabled: false,
            salt: None,
        }
    }

    crate::widget::setters! {
        item_salt
    }

    crate::widget::setters! {
        /// Greys the option out and ignores input.
        disabled
    }
}

/// `layout` of a [`RadioGroup`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RadioLayout {
    /// `vertical`: a column.
    #[default]
    Vertical,
    /// `horizontal`: a wrapping row.
    Horizontal,
    /// `horizontal-stacked`: a wrapping row of radios with the label below.
    HorizontalStacked,
}

/// Outcome of [`RadioGroup::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct RadioGroupResponse {
    /// Group response; `mark_changed` when `changed`.
    pub response: egui::Response,
    /// Agrees with `response.changed()`; `selected` is `Some`.
    pub changed: bool,
    /// Index picked this frame.
    pub selected: Option<usize>,
}

/// Fluent `RadioGroup`: single selection with one Tab stop and arrow-key navigation.
#[derive(Debug)]
#[must_use = "show with RadioGroup::show"]
pub struct RadioGroup<'a, T> {
    salt: KeyedSalt,
    label: String,
    value: &'a mut T,
    options: Vec<RadioOption<T>>,
    layout: RadioLayout,
    disabled: bool,
}

impl<'a, T: PartialEq + Clone> RadioGroup<'a, T> {
    /// `label` names the group for assistive technology; node key `<id_salt>-radio-group`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, value: &'a mut T) -> Self {
        Self {
            salt: id_salt.into(),
            label: label.into(),
            value,
            options: Vec::new(),
            layout: RadioLayout::Vertical,
            disabled: false,
        }
    }

    /// Adds an option; radio keys are `<id_salt>-<index>-radio`.
    pub fn option(mut self, option: impl Into<RadioOption<T>>) -> Self {
        self.options.push(option.into());
        self
    }

    /// Adds options in order.
    pub fn options(mut self, options: impl IntoIterator<Item = impl Into<RadioOption<T>>>) -> Self {
        self.options.extend(options.into_iter().map(Into::into));
        self
    }

    /// Default [`RadioLayout::Vertical`].
    pub fn layout(mut self, layout: RadioLayout) -> Self {
        self.layout = layout;
        self
    }

    crate::widget::setters! {
        /// Disables every radio.
        disabled
    }

    /// Lays out the radios; a pick writes `value` and reports `changed`.
    pub fn show(self, ui: &mut MineralUi<'_, Fluent>) -> RadioGroupResponse {
        let rt = ui.rt();
        let Self {
            salt,
            label,
            value,
            options,
            layout,
            disabled,
        } = self;
        let axis = match layout {
            RadioLayout::Vertical => Axis::Vertical,
            RadioLayout::Horizontal | RadioLayout::HorizontalStacked => Axis::Horizontal,
        };
        let position = match layout {
            RadioLayout::HorizontalStacked => RadioLabelPosition::Below,
            RadioLayout::Vertical | RadioLayout::Horizontal => RadioLabelPosition::After,
        };
        let mut ids = Vec::with_capacity(options.len());
        let mut changed = false;
        let mut picked = None;
        let group_id = ui.make_persistent_id(salt.id_salt());
        let enabled = |i: usize| !disabled && options.get(i).is_some_and(|o| !o.disabled);
        let selected = options.iter().position(|o| o.value == *value);
        let tab_stop = focus::tab_stop(ui.ctx(), group_id, selected, options.len(), enabled);
        let body = |ui: &mut Ui| {
            for (i, o) in options.iter().enumerate() {
                let r = rt.ui(ui).mineral(
                    Radio::new(o.salt.clone().unwrap_or_else(|| salt.child(i)), &o.label)
                        .selected(o.value == *value)
                        .label_position(position)
                        .tab_stop(i == tab_stop)
                        .disabled(!enabled(i)),
                );
                if r.changed() {
                    *value = o.value.clone();
                    changed = true;
                    picked = Some(i);
                }
                ids.push(r.id);
            }
        };
        let group = ui
            .push_id(salt.id_salt(), |ui| match axis {
                Axis::Horizontal => ui.horizontal_wrapped(body).response,
                Axis::Vertical => ui.vertical(body).response,
            })
            .inner;
        if let Some((i, o)) = RovingFocus::new(axis)
            .update_group(ui.core(), group_id, &ids, enabled)
            .and_then(|i| Some((i, options.get(i)?)))
        {
            *value = o.value.clone();
            changed = true;
            picked = Some(i);
        }
        semantic::emit(ui.ctx(), Some(group.id), group.rect, || {
            let mut n = SemanticNode::new(StableId::of(&salt, "radio-group"), Role::RadioGroup, &label);
            n.state.disabled = disabled;
            n
        });
        let mut response = group;
        if changed {
            response.mark_changed();
        }
        RadioGroupResponse {
            response,
            changed,
            selected: picked,
        }
    }
}
