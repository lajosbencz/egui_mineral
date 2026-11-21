use std::time::Duration;

use egui::Theme;
use serde::{Deserialize, Serialize};

use crate::{
    color::{ColorScheme, Contrast, SchemeSpec},
    spec::{
        ElevationLevel, ShapeScale,
        tokens::{motion, scrim, state},
    },
};

fn unit_interval(name: &str, v: f32) -> Result<f32, String> {
    if (0.0..=1.0).contains(&v) {
        Ok(v)
    } else {
        Err(format!("{name} must be within 0..=1, got {v}"))
    }
}

/// Corner radius scale in points (Compose `ShapeDefaults`); missing keys deserialize to the default.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
#[non_exhaustive]
pub struct Shape {
    /// [`ShapeScale::None`].
    pub none: u8,
    /// [`ShapeScale::ExtraSmall`].
    pub extra_small: u8,
    /// [`ShapeScale::Small`].
    pub small: u8,
    /// [`ShapeScale::Medium`].
    pub medium: u8,
    /// [`ShapeScale::Large`].
    pub large: u8,
    /// [`ShapeScale::LargeIncreased`].
    pub large_increased: u8,
    /// [`ShapeScale::ExtraLarge`].
    pub extra_large: u8,
    /// [`ShapeScale::ExtraLargeIncreased`].
    pub extra_large_increased: u8,
    /// [`ShapeScale::ExtraExtraLarge`].
    pub extra_extra_large: u8,
    /// [`ShapeScale::Full`]; `u8::MAX` rounds to a pill.
    pub full: u8,
}

impl Default for Shape {
    fn default() -> Self {
        Self {
            none: corner(ShapeScale::None),
            extra_small: corner(ShapeScale::ExtraSmall),
            small: corner(ShapeScale::Small),
            medium: corner(ShapeScale::Medium),
            large: corner(ShapeScale::Large),
            large_increased: corner(ShapeScale::LargeIncreased),
            extra_large: corner(ShapeScale::ExtraLarge),
            extra_large_increased: corner(ShapeScale::ExtraLargeIncreased),
            extra_extra_large: corner(ShapeScale::ExtraExtraLarge),
            full: u8::MAX,
        }
    }
}

impl Shape {
    // A radius the spec fixes (Compose RoundedCornerShape(n.dp)), scaled by `extra_small` over its 4dp default:
    // the default theme paints `spec`, a sharper or rounder scale carries it along.
    pub(crate) fn follow_extra_small(&self, spec: egui::CornerRadius) -> egui::CornerRadius {
        let base = u16::from(corner(ShapeScale::ExtraSmall)).max(1);
        let scale = |r: u8| {
            let v = (u16::from(r) * u16::from(self.extra_small) + base / 2) / base;
            u8::try_from(v).unwrap_or(u8::MAX)
        };
        egui::CornerRadius {
            nw: scale(spec.nw),
            ne: scale(spec.ne),
            sw: scale(spec.sw),
            se: scale(spec.se),
        }
    }
}

fn corner(scale: ShapeScale) -> u8 {
    #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "clamped to u8")]
    scale
        .dp()
        .map_or(u8::MAX, |dp| dp.round().clamp(0.0, f32::from(u8::MAX)) as u8)
}

/// State-layer opacities (Compose `StateTokens`); missing keys deserialize to the default.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "StateLayersRaw")]
#[non_exhaustive]
pub struct StateLayers {
    /// Hover layer opacity, 0..=1 (`StateTokens.HoverStateLayerOpacity`).
    pub hover: f32,
    /// Focus layer opacity, 0..=1 (`StateTokens.FocusStateLayerOpacity`).
    pub focus: f32,
    /// Pressed layer opacity, 0..=1 (`StateTokens.PressedStateLayerOpacity`).
    pub pressed: f32,
    /// Dragged layer opacity, 0..=1 (`StateTokens.DraggedStateLayerOpacity`).
    pub dragged: f32,
    /// Disabled container opacity, 0..=1.
    pub disabled_container: f32,
    /// Disabled content opacity, 0..=1.
    pub disabled_content: f32,
    /// Scrim opacity, 0..=1 (`ScrimTokens.ContainerOpacity`).
    pub scrim: f32,
}

#[derive(Deserialize)]
#[serde(default)]
struct StateLayersRaw {
    hover: f32,
    focus: f32,
    pressed: f32,
    dragged: f32,
    disabled_container: f32,
    disabled_content: f32,
    scrim: f32,
}

impl Default for StateLayersRaw {
    fn default() -> Self {
        let d = StateLayers::default();
        Self {
            hover: d.hover,
            focus: d.focus,
            pressed: d.pressed,
            dragged: d.dragged,
            disabled_container: d.disabled_container,
            disabled_content: d.disabled_content,
            scrim: d.scrim,
        }
    }
}

impl TryFrom<StateLayersRaw> for StateLayers {
    type Error = String;

    fn try_from(r: StateLayersRaw) -> Result<Self, String> {
        Ok(Self {
            hover: unit_interval("state.hover", r.hover)?,
            focus: unit_interval("state.focus", r.focus)?,
            pressed: unit_interval("state.pressed", r.pressed)?,
            dragged: unit_interval("state.dragged", r.dragged)?,
            disabled_container: unit_interval("state.disabled_container", r.disabled_container)?,
            disabled_content: unit_interval("state.disabled_content", r.disabled_content)?,
            scrim: unit_interval("state.scrim", r.scrim)?,
        })
    }
}

impl Default for StateLayers {
    fn default() -> Self {
        Self {
            hover: state::HOVER_STATE_LAYER_OPACITY,
            focus: state::FOCUS_STATE_LAYER_OPACITY,
            pressed: state::PRESSED_STATE_LAYER_OPACITY,
            dragged: state::DRAGGED_STATE_LAYER_OPACITY,
            disabled_container: 0.12,
            disabled_content: 0.38,
            scrim: scrim::CONTAINER_OPACITY,
        }
    }
}

/// MD3 duration scale; serialized as seconds, missing keys deserialize to the default.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "DurationsRaw", into = "DurationsRaw")]
#[non_exhaustive]
pub struct Durations {
    /// `MotionTokens.DurationShort1..4`.
    pub short: [Duration; 4],
    /// `MotionTokens.DurationMedium1..4`.
    pub medium: [Duration; 4],
    /// `MotionTokens.DurationLong1..4`.
    pub long: [Duration; 4],
    /// `MotionTokens.DurationExtraLong1..4`.
    pub extra_long: [Duration; 4],
}

#[derive(Serialize, Deserialize)]
// `None` takes the exact default; seconds as f32 would round it.
struct DurationsRaw {
    short: Option<[f32; 4]>,
    medium: Option<[f32; 4]>,
    long: Option<[f32; 4]>,
    extra_long: Option<[f32; 4]>,
}

impl TryFrom<DurationsRaw> for Durations {
    type Error = String;

    fn try_from(r: DurationsRaw) -> Result<Self, String> {
        let d = Self::default();
        let check = |name: &str, v: Option<[f32; 4]>, default| -> Result<[Duration; 4], String> {
            let Some(v) = v else { return Ok(default) };
            let mut out = [Duration::ZERO; 4];
            for (o, secs) in out.iter_mut().zip(v) {
                *o = Duration::try_from_secs_f32(secs).map_err(|e| format!("{name}: {secs}: {e}"))?;
            }
            Ok(out)
        };
        Ok(Self {
            short: check("duration.short", r.short, d.short)?,
            medium: check("duration.medium", r.medium, d.medium)?,
            long: check("duration.long", r.long, d.long)?,
            extra_long: check("duration.extra_long", r.extra_long, d.extra_long)?,
        })
    }
}

impl From<Durations> for DurationsRaw {
    fn from(d: Durations) -> Self {
        let secs = |v: [Duration; 4]| Some(v.map(|d| d.as_secs_f32()));
        Self {
            short: secs(d.short),
            medium: secs(d.medium),
            long: secs(d.long),
            extra_long: secs(d.extra_long),
        }
    }
}

impl Default for Durations {
    fn default() -> Self {
        Self {
            short: [
                motion::DURATION_SHORT1,
                motion::DURATION_SHORT2,
                motion::DURATION_SHORT3,
                motion::DURATION_SHORT4,
            ],
            medium: [
                motion::DURATION_MEDIUM1,
                motion::DURATION_MEDIUM2,
                motion::DURATION_MEDIUM3,
                motion::DURATION_MEDIUM4,
            ],
            long: [
                motion::DURATION_LONG1,
                motion::DURATION_LONG2,
                motion::DURATION_LONG3,
                motion::DURATION_LONG4,
            ],
            extra_long: [
                motion::DURATION_EXTRA_LONG1,
                motion::DURATION_EXTRA_LONG2,
                motion::DURATION_EXTRA_LONG3,
                motion::DURATION_EXTRA_LONG4,
            ],
        }
    }
}

/// Resolved design tokens installed on the context: color, shape, state layers, durations. Only `color` is
/// required when deserializing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Tokens {
    /// Resolved scheme; its theme mode is fixed.
    pub color: ColorScheme,
    /// Corner radii in dp.
    #[serde(default)]
    pub shape: Shape,
    /// State-layer opacities.
    #[serde(default)]
    pub state: StateLayers,
    /// Motion durations.
    #[serde(default)]
    pub duration: Durations,
}

impl Tokens {
    // Container and content of a non-default tone.
    #[cfg(feature = "extras")]
    pub(crate) fn tone(&self, tone: egui_mineral::tone::Tone) -> Option<(egui::Color32, egui::Color32)> {
        use crate::spec::ColorRole;
        use egui_mineral::tone::Tone;
        match tone {
            Tone::Custom { container, content } => Some((container, content)),
            Tone::Error => Some((
                self.role(ColorRole::ErrorContainer),
                self.role(ColorRole::OnErrorContainer),
            )),
            _ => None,
        }
    }

    /// `role` in this scheme.
    pub fn role(&self, role: crate::spec::ColorRole) -> egui::Color32 {
        self.color.role(role)
    }

    /// `role` at `alpha` in 0..=1.
    pub fn role_alpha(&self, role: crate::spec::ColorRole, alpha: f32) -> egui::Color32 {
        crate::widget::common::with_alpha(self.role(role), alpha)
    }

    /// Scrim (`scrim` color at `state.scrim`) faded by `visible` in 0..=1.
    pub fn scrim(&self, visible: f32) -> egui::Color32 {
        self.color.scrim.gamma_multiply(self.state.scrim * visible)
    }

    /// Default non-color tokens around `color`.
    pub fn new(color: ColorScheme) -> Self {
        Self {
            color,
            shape: Shape::default(),
            state: StateLayers::default(),
            duration: Durations::default(),
        }
    }

    /// Tokens around the [`BASELINE_SEED`](crate::color::BASELINE_SEED) scheme.
    pub fn baseline(theme: Theme, contrast: Contrast) -> Self {
        Self::new(SchemeSpec::default().contrast(contrast).scheme(theme))
    }

    /// Mode of [`Self::color`].
    pub const fn theme(&self) -> Theme {
        self.color.theme
    }

    /// Shadow of `level`, via [`Self::shadow_dp`].
    pub fn elevation_shadow(&self, level: ElevationLevel) -> egui::Shadow {
        self.shadow_dp(level.dp())
    }

    /// Shadow of an elevation in dp (0 to 127), for elevations animated between levels.
    #[track_caller]
    pub fn shadow_dp(&self, dp: f32) -> egui::Shadow {
        // 127 dp: the blur (2 * dp) fits a `u8`.
        let dp = egui_mineral::kit::sanitize::within(dp, 0.0..=127.0, "shadow dp");
        egui::Shadow {
            offset: [0, dp.min(8.0) as i8],
            #[expect(clippy::cast_sign_loss, reason = "dp is clamped to 0..=127 above")]
            blur: (dp * 2.0) as u8,
            spread: 0,
            color: self.color.shadow.gamma_multiply(0.25),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_spec_radii_follow_the_extra_small_step() {
        let spec = egui::CornerRadius::same(3);
        let mut shape = Shape::default();
        assert_eq!(shape.follow_extra_small(spec), spec, "default theme paints the spec");
        shape.extra_small = 8;
        assert_eq!(shape.follow_extra_small(spec), egui::CornerRadius::same(6));
        shape.extra_small = 0;
        assert_eq!(shape.follow_extra_small(spec), egui::CornerRadius::ZERO);
    }
}
