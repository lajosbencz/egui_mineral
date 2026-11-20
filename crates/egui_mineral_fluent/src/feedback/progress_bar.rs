use crate::msg;
use egui::{Color32, CornerRadius, Rect, vec2};
use egui_mineral::{
    i18n::{Direction, I18n},
    kit,
    layout::{Constraints, Measure},
    semantic::{AgentKey, KeyedSalt, NumericRange, Role, SemanticNode},
    widget::{MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::ColorToken::{
    ColorCompoundBrandBackground, ColorNeutralBackground6, ColorPaletteDarkOrangeBackground3,
    ColorPaletteGreenBackground3, ColorPaletteRedBackground3,
};
use crate::{
    Fluent, FluentExt,
    inputs::Validation,
    spec::{defaults::progress_bar as d, tokens::geometry},
    widget::{paint, tween},
};

/// `ProgressBar` `thickness`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ProgressThickness {
    /// `medium`: 2.
    #[default]
    Medium,
    /// `large`: 4.
    Large,
}

impl ProgressThickness {
    /// Every variant.
    pub const ALL: &'static [Self] = &[Self::Medium, Self::Large];

    const fn height(self) -> f32 {
        match self {
            Self::Medium => d::THICKNESS_MEDIUM,
            Self::Large => d::THICKNESS_LARGE,
        }
    }
}

/// `ProgressBar` `shape`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ProgressShape {
    /// `rounded`: `borderRadiusMedium`.
    #[default]
    Rounded,
    /// `square`: no rounding.
    Square,
}

impl ProgressShape {
    /// Every variant.
    pub const ALL: &'static [Self] = &[Self::Rounded, Self::Square];
}

/// Fluent `ProgressBar`: a full-width track with a fill, or a sweeping bar without a value.
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct ProgressBar {
    salt: KeyedSalt,
    key: AgentKey,
    label: Option<String>,
    value: Option<f32>,
    max: f32,
    thickness: ProgressThickness,
    shape: ProgressShape,
    validation: Option<Validation>,
}

impl ProgressBar {
    /// `value` of 0 to `max` (1 unless set); a non-finite value counts as 0. Agent key `<id_salt>-progress-bar`.
    pub fn determinate(id_salt: impl Into<KeyedSalt>, value: f32) -> Self {
        let value = kit::sanitize::finite(value, "ProgressBar value").unwrap_or(0.0);
        Self::build(id_salt.into(), Some(value))
    }

    /// No `value`: a bar sweeps along the track. Agent key `<id_salt>-progress-bar`.
    pub fn indeterminate(id_salt: impl Into<KeyedSalt>) -> Self {
        Self::build(id_salt.into(), None)
    }

    fn build(salt: KeyedSalt, value: Option<f32>) -> Self {
        Self {
            salt,
            key: AgentKey::Kind("progress-bar"),
            label: None,
            value,
            max: 1.0,
            thickness: ProgressThickness::Medium,
            shape: ProgressShape::Rounded,
            validation: None,
        }
    }

    /// `max`: the value of a full bar, default 1; a non-positive one counts as the smallest positive.
    pub fn max(mut self, max: f32) -> Self {
        self.max = kit::sanitize::at_least(max, f32::MIN_POSITIVE.., "ProgressBar max");
        self
    }

    /// Accessible name; default the `mineral-progress` message.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Default [`ProgressThickness::Medium`].
    pub fn thickness(mut self, thickness: ProgressThickness) -> Self {
        self.thickness = thickness;
        self
    }

    /// Default [`ProgressShape::Rounded`].
    pub fn shape(mut self, shape: ProgressShape) -> Self {
        self.shape = shape;
        self
    }

    /// `color`: error, warning or success fill of a determinate bar; default brand.
    pub fn validation(mut self, validation: Validation) -> Self {
        self.validation = Some(validation);
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-progress-bar` agent key.
        agent_key(key)
    }

    fn fraction(&self) -> Option<f32> {
        self.value.map(|v| (v / self.max).clamp(0.0, 1.0))
    }
}

fn span(rect: Rect, dir: Direction, from: f32, to: f32) -> Rect {
    let x = dir.start_x(rect, egui::Rangef::new(from, to));
    Rect::from_x_y_ranges(x..=x + (to - from), rect.y_range())
}

impl MineralWidget for ProgressBar {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt, hover, no_target);

    /// Fills the main axis.
    fn layout(&self, ui: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        let w = if c.max.x.is_finite() {
            c.max.x
        } else {
            ui.available_width()
        };
        c.constrain(vec2(w, self.thickness.height())).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let rt = ui.rt();
        let dir = ui.i18n().direction();
        let rect = frame.zones.visual;
        let radius = match self.shape {
            ProgressShape::Rounded => geometry::BORDER_RADIUS_MEDIUM.min(rect.height() / 2.0),
            ProgressShape::Square => geometry::BORDER_RADIUS_NONE,
        };
        let r = CornerRadius::from(radius);
        let track = t.color(ColorNeutralBackground6);
        let p = ui.painter().with_clip_rect(rect);
        p.rect_filled(rect, r, track);
        let fill = t.color(match (self.value, self.validation) {
            (Some(_), Some(Validation::Error)) => ColorPaletteRedBackground3,
            (Some(_), Some(Validation::Warning)) => ColorPaletteDarkOrangeBackground3,
            (Some(_), Some(Validation::Success)) => ColorPaletteGreenBackground3,
            _ => ColorCompoundBrandBackground,
        });
        let w = rect.width();
        let id = ui.make_persistent_id(self.salt.id_salt());
        if let Some(f) = self.fraction() {
            let duration = if f > d::ZERO_THRESHOLD {
                d::FILL_DURATION
            } else {
                std::time::Duration::ZERO
            };
            let shown = tween::tween(rt.core(), id, f, duration, d::FILL_EASING);
            if shown > 0.0 {
                p.rect_filled(span(rect, dir, 0.0, w * shown), r, fill);
            }
        } else {
            let (from, width) = if rt.core().motion().reduced() {
                (0.0, w)
            } else {
                egui_mineral::motion::request_frame(ui.ctx());
                let cycle = d::INDETERMINATE_CYCLE.as_secs_f64();
                let k = (ui.input(|i| i.time).rem_euclid(cycle) / cycle) as f32;
                let width = w * d::INDETERMINATE_WIDTH;
                (-width + (w + width) * k, width)
            };
            let bar = span(rect, dir, from, from + width);
            p.rect_filled(bar, r, fill);
            paint::gradient_h(&p, bar, [track, Color32::TRANSPARENT, track]);
        }
        WidgetResponse::new(frame.response.clone(), ())
    }

    fn semantics(&self, i18n: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let name = crate::tr_or(i18n, self.label.as_deref(), msg::PROGRESS);
        let mut node = SemanticNode::new(self.key.id(&self.salt), Role::ProgressIndicator, name);
        if let Some(v) = self.value {
            let v = f64::from(v.clamp(0.0, self.max));
            node.state.range = Some(NumericRange {
                value: v,
                min: 0.0,
                max: f64::from(self.max),
                step: None,
            });
            let pct = (v / f64::from(self.max) * 100.0).round();
            node.value = Some(i18n.tr(msg::Percent { value: pct.into() }));
        }
        Some(node)
    }
}
