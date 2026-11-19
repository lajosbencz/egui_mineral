use egui::{Color32, CornerRadius, Rangef, Rect, Stroke, StrokeKind, Vec2, vec2};
use egui_mineral::{
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
    ColorNeutralBackgroundDisabled, ColorNeutralForeground1, ColorNeutralForegroundDisabled,
    ColorNeutralForegroundInverted, ColorNeutralStrokeAccessible, ColorNeutralStrokeAccessibleHover,
    ColorNeutralStrokeAccessiblePressed, ColorNeutralStrokeDisabled, ColorTransparentStroke,
    ColorTransparentStrokeDisabled, ColorTransparentStrokeInteractive,
};
use crate::{
    Fluent, FluentExt, Size,
    spec::{
        defaults::switch as sw,
        tokens::{geometry as g, motion, typography as ty},
    },
    widget::{
        focus,
        state::{Ramp, State},
        tween,
    },
};

/// Side of the label a [`Switch`] puts it on (`labelPosition`); mirrored in right-to-left locales.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SwitchLabelPosition {
    /// `before`: leading the track.
    Before,
    /// `after`: trailing the track.
    #[default]
    After,
    /// `above`: over the track.
    Above,
}

/// Fluent `Switch`: a click flips the bound value and reports `changed`.
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Switch<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    checked: &'a mut bool,
    size: Size,
    position: SwitchLabelPosition,
    label_visible: bool,
    mnemonic: Option<Mnemonic>,
    disabled: bool,
}

impl<'a> Switch<'a> {
    /// `label` is the visible text and accessible name; agent key `<id_salt>-switch`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, checked: &'a mut bool) -> Self {
        Self {
            salt: id_salt.into(),
            key: AgentKey::Kind("switch"),
            label: label.into(),
            checked,
            size: Size::Medium,
            position: SwitchLabelPosition::After,
            label_visible: true,
            mnemonic: None,
            disabled: false,
        }
    }

    /// [`Size::Small`] or [`Size::Medium`] (default); [`Size::Large`] is medium, as in React.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Default [`SwitchLabelPosition::After`].
    pub fn label_position(mut self, position: SwitchLabelPosition) -> Self {
        self.position = position;
        self
    }

    /// `false` keeps the label for assistive technology only.
    pub fn label_visible(mut self, label_visible: bool) -> Self {
        self.label_visible = label_visible;
        self
    }

    /// Alt+letter toggles the switch; the letter is underlined while mnemonics are visible.
    pub fn mnemonic(mut self, mnemonic: char) -> Self {
        self.mnemonic = Some(Mnemonic::new(mnemonic));
        self
    }

    crate::widget::setters! {
        /// Greys out the switch and ignores input.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-switch` agent key.
        agent_key(key)
    }

    fn track(&self) -> Vec2 {
        match self.size {
            Size::Small => vec2(sw::TRACK_WIDTH_SMALL, sw::TRACK_HEIGHT_SMALL),
            Size::Medium | Size::Large => vec2(sw::TRACK_WIDTH_MEDIUM, sw::TRACK_HEIGHT_MEDIUM),
        }
    }

    // react-switch useLabelStyles.sizeSmall: `fontSizeBase200`.
    fn font(&self) -> (f32, f32) {
        match self.size {
            Size::Small => (ty::FONT_SIZE_BASE_200, ty::LINE_HEIGHT_BASE_200),
            Size::Medium | Size::Large => (ty::FONT_SIZE_BASE_300, ty::LINE_HEIGHT_BASE_300),
        }
    }

    fn spec(&self) -> Spec {
        Spec {
            indicator: self.track(),
            position: match self.position {
                SwitchLabelPosition::Before => Position::Before,
                SwitchLabelPosition::After => Position::After,
                SwitchLabelPosition::Above => Position::Above,
            },
            line_height: self.font().1,
        }
    }

    fn plan(&self, ui: &MineralUi<'_, Fluent>, max: f32) -> super::choice::Plan {
        let spec = self.spec();
        let label = self.label_visible.then(|| {
            let (size, line) = self.font();
            let format = crate::typography::format(ui.ctx(), size, line, ty::FONT_WEIGHT_REGULAR, Color32::PLACEHOLDER);
            spec.label(ui.core(), ui.painter(), &self.label, &format, self.mnemonic, max)
        });
        spec.plan(label)
    }

    // react-switch useSwitchStyles.styles.ts useInputBaseClassName: label, border, track fill and thumb colors.
    fn colors(&self, s: State, t: &crate::theme::Tokens) -> [Color32; 4] {
        let by = |r: Ramp| r.pick(s, t);
        let label = by(Ramp::new(
            ColorNeutralForeground1,
            ColorNeutralForeground1,
            ColorNeutralForeground1,
            ColorNeutralForegroundDisabled,
        ));
        if *self.checked {
            [
                label,
                by(Ramp::new(
                    ColorTransparentStroke,
                    ColorTransparentStrokeInteractive,
                    ColorTransparentStrokeInteractive,
                    ColorTransparentStrokeDisabled,
                )),
                by(Ramp::new(
                    ColorCompoundBrandBackground,
                    ColorCompoundBrandBackgroundHover,
                    ColorCompoundBrandBackgroundPressed,
                    ColorNeutralBackgroundDisabled,
                )),
                by(Ramp::new(
                    ColorNeutralForegroundInverted,
                    ColorNeutralForegroundInverted,
                    ColorNeutralForegroundInverted,
                    ColorNeutralForegroundDisabled,
                )),
            ]
        } else {
            let edge = by(Ramp::new(
                ColorNeutralStrokeAccessible,
                ColorNeutralStrokeAccessibleHover,
                ColorNeutralStrokeAccessiblePressed,
                ColorNeutralStrokeDisabled,
            ));
            let thumb = by(Ramp::new(
                ColorNeutralStrokeAccessible,
                ColorNeutralStrokeAccessibleHover,
                ColorNeutralStrokeAccessiblePressed,
                ColorNeutralForegroundDisabled,
            ));
            [label, edge, Color32::TRANSPARENT, thumb]
        }
    }
}

impl MineralWidget for Switch<'_> {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt, enabled);

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
        let changed = r.clicked() && !self.disabled;
        if changed {
            *self.checked = !*self.checked;
        }
        let i = Interaction::of(r, availability);
        let s = State::of(i);
        let [label, border, fill, thumb] = self.colors(s, &t);
        let visual = frame.zones.visual;
        let dir = ui.i18n().direction();
        let placed = self.plan(ui, visual.width() + WRAP_SLACK).place(visual, dir);
        let track = placed.indicator;
        let radius = CornerRadius::from(g::BORDER_RADIUS_CIRCULAR.min(track.height() / 2.0));
        let k = tween::tween(
            ui.core(),
            r.id.with("thumb"),
            f32::from(*self.checked),
            motion::DURATION_NORMAL,
            motion::CURVE_EASY_EASE,
        );
        let d = track.height() - sw::THUMB_GAP;
        let inner = track.shrink(g::STROKE_WIDTH_THIN);
        let travel = k * (track.width() - d - sw::THUMB_GAP);
        let x = dir.start_x(inner, Rangef::new(travel, travel + d));
        let p = ui.painter();
        p.rect_filled(track, radius, fill);
        p.rect_stroke(
            track,
            radius,
            Stroke::new(g::STROKE_WIDTH_THIN, border),
            StrokeKind::Inside,
        );
        p.circle_filled(
            Rect::from_min_size(egui::pos2(x, inner.top()), vec2(d, d)).center(),
            d / 2.0,
            thumb,
        );
        if let Some((at, galley)) = placed.text {
            p.galley(at, galley, label);
        }
        if i.focused {
            focus::outline(p, visual, CornerRadius::from(g::BORDER_RADIUS_MEDIUM), &t);
        }
        WidgetResponse::new(r.clone(), ()).with_changed(changed)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(self.key.id(&self.salt), Role::Switch, &self.label)
            .clickable(Availability::disabled_if(self.disabled));
        n.state.checked = Some(Check::from(*self.checked));
        Some(n)
    }
}
