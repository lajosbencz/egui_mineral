use egui::{Color32, CornerRadius, Sense, Stroke, StrokeKind};
use egui_mineral::{
    focus::{TabStop, roving_sense},
    i18n::I18n,
    kit,
    layout::{Constraints, Measure, MinTarget},
    select::Check,
    semantic::{AgentKey, KeyedSalt, Role, SemanticNode},
    shortcut::Mnemonic,
    widget::{Availability, Interaction, MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use super::choice::{Position, Spec, WRAP_SLACK};
use crate::ColorToken::{
    ColorCompoundBrandBackground, ColorCompoundBrandBackgroundHover, ColorCompoundBrandBackgroundPressed,
    ColorCompoundBrandForeground1, ColorCompoundBrandForeground1Hover, ColorCompoundBrandForeground1Pressed,
    ColorCompoundBrandStroke, ColorCompoundBrandStrokeHover, ColorCompoundBrandStrokePressed, ColorNeutralForeground1,
    ColorNeutralForeground2, ColorNeutralForeground3, ColorNeutralForegroundDisabled, ColorNeutralForegroundInverted,
    ColorNeutralStrokeAccessible, ColorNeutralStrokeAccessibleHover, ColorNeutralStrokeAccessiblePressed,
    ColorNeutralStrokeDisabled,
};
use crate::{
    Fluent, FluentExt, Size,
    spec::{
        defaults::checkbox as cb,
        tokens::{geometry as g, typography as ty},
    },
    widget::{
        focus, paint,
        state::{Ramp, State},
    },
};

/// Side of the label a [`Checkbox`] puts it on (`labelPosition`); mirrored in right-to-left locales.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CheckboxLabelPosition {
    /// `before`: leading the box.
    Before,
    /// `after`: trailing the box.
    #[default]
    After,
}

/// Checkbox `shape`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CheckboxShape {
    /// `square`: `borderRadiusSmall`.
    #[default]
    Square,
    /// `circular`.
    Circular,
}

/// Fluent `Checkbox`: a click flips the bound value and reports `changed`; [`Checkbox::mixed`] shows the
/// `mixed` state, which the next activation resolves to checked.
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Checkbox<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    checked: &'a mut bool,
    mixed: bool,
    size: Size,
    shape: CheckboxShape,
    position: CheckboxLabelPosition,
    label_visible: bool,
    mnemonic: Option<Mnemonic>,
    disabled: bool,
    pub(crate) tab_stop: Option<TabStop>,
}

impl<'a> Checkbox<'a> {
    /// `label` is the visible text and accessible name; agent key `<id_salt>-checkbox`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, checked: &'a mut bool) -> Self {
        Self {
            salt: id_salt.into(),
            key: AgentKey::Kind("checkbox"),
            label: label.into(),
            checked,
            mixed: false,
            size: Size::Medium,
            shape: CheckboxShape::Square,
            position: CheckboxLabelPosition::After,
            label_visible: true,
            mnemonic: None,
            disabled: false,
            tab_stop: None,
        }
    }

    /// `checked="mixed"`: the dash, whatever the bound value.
    pub fn mixed(mut self, mixed: bool) -> Self {
        self.mixed = mixed;
        self
    }

    /// [`Size::Medium`] (default) or [`Size::Large`]; [`Size::Small`] is medium, as in React.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Default [`CheckboxShape::Square`].
    pub fn shape(mut self, shape: CheckboxShape) -> Self {
        self.shape = shape;
        self
    }

    /// Default [`CheckboxLabelPosition::After`].
    pub fn label_position(mut self, position: CheckboxLabelPosition) -> Self {
        self.position = position;
        self
    }

    /// `false` keeps the label for assistive technology only.
    pub fn label_visible(mut self, label_visible: bool) -> Self {
        self.label_visible = label_visible;
        self
    }

    /// Alt+letter toggles the box; the letter is underlined while mnemonics are visible.
    pub fn mnemonic(mut self, mnemonic: char) -> Self {
        self.mnemonic = Some(Mnemonic::new(mnemonic));
        self
    }

    crate::widget::setters! {
        /// Greys out the checkbox and ignores input.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-checkbox` agent key.
        agent_key(key)
    }

    fn state(&self) -> Check {
        match (self.mixed, *self.checked) {
            (true, _) => Check::Mixed,
            (false, true) => Check::Checked,
            (false, false) => Check::Unchecked,
        }
    }

    fn indicator(&self) -> f32 {
        match self.size {
            Size::Large => cb::INDICATOR_LARGE,
            Size::Small | Size::Medium => cb::INDICATOR_MEDIUM,
        }
    }

    fn spec(&self) -> Spec {
        Spec {
            indicator: egui::Vec2::splat(self.indicator()),
            position: match self.position {
                CheckboxLabelPosition::Before => Position::Before,
                CheckboxLabelPosition::After => Position::After,
            },
            line_height: ty::LINE_HEIGHT_BASE_300,
        }
    }

    fn plan(&self, ui: &MineralUi<'_, Fluent>, max: f32) -> super::choice::Plan {
        let spec = self.spec();
        let label = self.label_visible.then(|| {
            let format = crate::typography::format(
                ui.ctx(),
                ty::FONT_SIZE_BASE_300,
                ty::LINE_HEIGHT_BASE_300,
                ty::FONT_WEIGHT_REGULAR,
                Color32::PLACEHOLDER,
            );
            spec.label(ui.core(), ui.painter(), &self.label, &format, self.mnemonic, max)
        });
        spec.plan(label)
    }

    // react-checkbox useCheckboxStyles.styles.ts useRootStyles: label, indicator border, fill, glyph.
    fn colors(&self, s: State, t: &crate::theme::Tokens) -> [Color32; 4] {
        let none = Color32::TRANSPARENT;
        let by = |r: Ramp| r.pick(s, t);
        match self.state() {
            Check::Unchecked => [
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
                none,
                none,
            ],
            Check::Checked => [
                by(Ramp::new(
                    ColorNeutralForeground1,
                    ColorNeutralForeground1,
                    ColorNeutralForeground1,
                    ColorNeutralForegroundDisabled,
                )),
                by(Ramp::new(
                    ColorCompoundBrandBackground,
                    ColorCompoundBrandBackgroundHover,
                    ColorCompoundBrandBackgroundPressed,
                    ColorNeutralStrokeDisabled,
                )),
                by(Ramp::new(
                    ColorCompoundBrandBackground,
                    ColorCompoundBrandBackgroundHover,
                    ColorCompoundBrandBackgroundPressed,
                    ColorNeutralStrokeDisabled,
                )
                .disabled(None)),
                by(Ramp::new(
                    ColorNeutralForegroundInverted,
                    ColorNeutralForegroundInverted,
                    ColorNeutralForegroundInverted,
                    ColorNeutralForegroundDisabled,
                )),
            ],
            Check::Mixed => [
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
                none,
                by(Ramp::new(
                    ColorCompoundBrandForeground1,
                    ColorCompoundBrandForeground1Hover,
                    ColorCompoundBrandForeground1Pressed,
                    ColorNeutralForegroundDisabled,
                )),
            ],
        }
    }
}

impl MineralWidget for Checkbox<'_> {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt, enabled);

    fn access_key(&self) -> Option<Mnemonic> {
        self.mnemonic
    }

    fn sense(&self) -> Sense {
        self.tab_stop
            .map_or(Sense::click(), |t| roving_sense(Sense::click(), t))
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
        let changed = r.clicked() && !self.disabled;
        if changed {
            *self.checked = self.mixed || !*self.checked;
            self.mixed = false;
        }
        let i = Interaction::of(r, availability);
        let s = State::of(i);
        let [label, border, fill, glyph] = self.colors(s, &t);
        let visual = frame.zones.visual;
        let placed = self
            .plan(ui, visual.width() + WRAP_SLACK)
            .place(visual, ui.i18n().direction());
        let radius = match self.shape {
            CheckboxShape::Square => g::BORDER_RADIUS_SMALL,
            CheckboxShape::Circular => g::BORDER_RADIUS_CIRCULAR,
        };
        let radius = CornerRadius::from(radius.min(placed.indicator.height() / 2.0));
        let p = ui.painter();
        p.rect_filled(placed.indicator, radius, fill);
        p.rect_stroke(
            placed.indicator,
            radius,
            Stroke::new(g::STROKE_WIDTH_THIN, border),
            StrokeKind::Inside,
        );
        let glyph_size = match self.size {
            Size::Large => cb::GLYPH_LARGE,
            Size::Small | Size::Medium => cb::GLYPH_MEDIUM,
        };
        match self.state() {
            Check::Checked => paint::icon(p, placed.indicator.center(), ui.icons().checkmark, true, glyph_size, glyph),
            Check::Mixed => paint::icon(p, placed.indicator.center(), ui.icons().square, true, glyph_size, glyph),
            Check::Unchecked => {}
        }
        if let Some((at, galley)) = placed.text {
            p.galley(at, galley, label);
        }
        if i.focused {
            focus::outline(p, visual, CornerRadius::from(g::BORDER_RADIUS_MEDIUM), &t);
        }
        WidgetResponse::new(r.clone(), ()).with_changed(changed)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(self.key.id(&self.salt), Role::CheckBox, &self.label)
            .clickable(Availability::disabled_if(self.disabled));
        n.state.checked = Some(self.state());
        Some(n)
    }
}
