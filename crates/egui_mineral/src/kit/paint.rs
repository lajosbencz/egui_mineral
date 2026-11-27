//! Shapes, text layout jobs and state indication shared by stylekits: glyphs and tabular figures, per-state
//! values ([`VisualState`], [`StateRamp`]), state-layer overlays ([`LayerOpacity`], [`overlay`]) and focus strokes
//! ([`FocusStroke`]). Kits pass their token values in.

use egui::{
    Color32, Context, CornerRadius, Pos2, Rect, Stroke, StrokeKind, TextFormat,
    emath::Rot2,
    epaint::{RectShape, TextShape},
    text::LayoutJob,
    vec2,
};

use crate::{
    icon::Glyph,
    widget::{Interaction, InteractionKind},
};

/// Pointer state a per-state value is picked by, CSS-style: rest, `:hover`, `:active`, disabled.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VisualState {
    /// No pointer interaction.
    Rest,
    /// Pointer over.
    Hover,
    /// Pressed; outranks hover.
    Pressed,
    /// Disabled; outranks the rest.
    Disabled,
}

impl VisualState {
    /// State of `i`; focus and drag do not change it.
    pub const fn of(i: Interaction) -> Self {
        if i.disabled {
            Self::Disabled
        } else if i.pressed {
            Self::Pressed
        } else if i.hovered {
            Self::Hover
        } else {
            Self::Rest
        }
    }

    /// Hovered or pressed.
    pub const fn engaged(self) -> bool {
        matches!(self, Self::Hover | Self::Pressed)
    }
}

/// One property's value per [`VisualState`], e.g. a color token per state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct StateRamp<T> {
    /// At rest.
    pub rest: T,
    /// Hovered.
    pub hover: T,
    /// Pressed.
    pub pressed: T,
    /// Disabled.
    pub disabled: T,
}

impl<T> StateRamp<T> {
    /// Values in state order.
    pub const fn new(rest: T, hover: T, pressed: T, disabled: T) -> Self {
        Self {
            rest,
            hover,
            pressed,
            disabled,
        }
    }

    /// Value of `state`.
    pub const fn get(&self, state: VisualState) -> &T {
        match state {
            VisualState::Rest => &self.rest,
            VisualState::Hover => &self.hover,
            VisualState::Pressed => &self.pressed,
            VisualState::Disabled => &self.disabled,
        }
    }

    /// Replaces the disabled value.
    #[must_use]
    pub fn disabled(self, disabled: T) -> Self {
        Self { disabled, ..self }
    }

    /// Ramp of `f` applied to every value.
    pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> StateRamp<U> {
        StateRamp::new(f(self.rest), f(self.hover), f(self.pressed), f(self.disabled))
    }
}

impl<T> StateRamp<Option<T>> {
    /// No value in any state, e.g. a transparent fill.
    pub const NONE: Self = Self::new(None, None, None, None);

    /// Every state set.
    pub const fn some(rest: T, hover: T, pressed: T, disabled: T) -> Self {
        Self::new(Some(rest), Some(hover), Some(pressed), Some(disabled))
    }
}

/// Opacity of an MD3-style state layer per [`Interaction::dominant`] kind, each 0..=1.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LayerOpacity {
    /// Hovered.
    pub hover: f32,
    /// Focused.
    pub focus: f32,
    /// Pressed.
    pub pressed: f32,
    /// Dragged.
    pub dragged: f32,
}

impl LayerOpacity {
    /// Opacity for `i`; 0 without a dominant interaction or while disabled.
    pub fn of(&self, i: Interaction) -> f32 {
        i.dominant().map_or(0.0, |kind| match kind {
            InteractionKind::Dragged => self.dragged,
            InteractionKind::Pressed => self.pressed,
            InteractionKind::Focused => self.focus,
            InteractionKind::Hovered => self.hover,
        })
    }
}

/// Fill of `rect` in `color` at `opacity`, laid over what is painted below; `None` at 0.
pub fn overlay(rect: Rect, r: impl Into<CornerRadius>, color: Color32, opacity: f32) -> Option<RectShape> {
    (opacity > 0.0).then(|| RectShape::filled(rect, r, color.gamma_multiply(opacity)))
}

/// Focus indicator stroke around or inside a shape.
#[derive(Clone, Copy, Debug, PartialEq)]
#[must_use = "build a shape with .shape(..)"]
pub struct FocusStroke {
    /// Width in points and color, both from kit tokens.
    pub stroke: Stroke,
    /// Gap from the shape's edge; negative moves the stroke inward.
    pub offset: f32,
    /// Side of the offset edge the stroke lies on.
    pub kind: StrokeKind,
}

impl FocusStroke {
    /// `stroke` outside the shape's edge.
    pub const fn outside(stroke: Stroke) -> Self {
        Self {
            stroke,
            offset: 0.0,
            kind: StrokeKind::Outside,
        }
    }

    /// `stroke` inside the shape's edge.
    pub const fn inside(stroke: Stroke) -> Self {
        Self {
            stroke,
            offset: 0.0,
            kind: StrokeKind::Inside,
        }
    }

    /// Default 0; non-finite is 0.
    pub fn offset(mut self, offset: f32) -> Self {
        self.offset = crate::kit::sanitize::finite(offset, "FocusStroke::offset").unwrap_or(0.0);
        self
    }

    /// Stroke of the shape `rect` with corners `r`.
    pub fn shape(self, rect: Rect, r: impl Into<CornerRadius>) -> RectShape {
        RectShape::stroke(rect.expand(self.offset), r, self.stroke, self.kind)
    }
}

/// Glyph drawn centered on a point by its ink, not its line box.
#[derive(Clone, Copy, Debug, PartialEq)]
#[must_use = "build a shape with .shape(..)"]
pub struct GlyphShape {
    glyph: Glyph,
    size: f32,
    color: Color32,
    angle: f32,
}

impl GlyphShape {
    /// `glyph` at `size` points in `color`, unrotated.
    pub const fn new(glyph: Glyph, size: f32, color: Color32) -> Self {
        Self {
            glyph,
            size,
            color,
            angle: 0.0,
        }
    }

    /// Rotation around the center, radians; default 0.
    #[track_caller]
    pub fn angle(mut self, angle: f32) -> Self {
        self.angle = crate::kit::sanitize::finite(angle, "GlyphShape::angle").unwrap_or(0.0);
        self
    }

    /// Text shape centered on `center`; the stylekit adds it to its painter.
    pub fn shape(self, ctx: &Context, center: Pos2) -> TextShape {
        let Self {
            glyph,
            size,
            color,
            angle,
        } = self;
        let galley = ctx.fonts_mut(|f| f.layout(glyph.ch().to_string(), glyph.font_id(size), color, f32::INFINITY));
        let mid_y = galley
            .rows
            .iter()
            .flat_map(|row| {
                row.glyphs
                    .iter()
                    .map(move |g| row.pos.y + g.pos.y + g.uv_rect.offset.y + g.uv_rect.size.y / 2.0)
            })
            .next()
            .unwrap_or_else(|| galley.size().y / 2.0);
        let anchor = vec2(galley.size().x / 2.0, mid_y);
        TextShape::new(center - Rot2::from_angle(angle) * anchor, galley, color).with_angle(angle)
    }
}

/// Job of `text` in `format` with every ASCII digit in a cell as wide as the font's widest digit,
/// flush to the cell's end, so a number keeps its width while its digits change. Stands in for
/// OpenType `tnum`, which epaint cannot request.
pub fn tabular_figures(ctx: &Context, text: &str, format: &TextFormat) -> LayoutJob {
    let widths = ctx
        .fonts_mut(|f| ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'].map(|d| f.glyph_width(&format.font_id, d)));
    let cell = widths.iter().copied().fold(0.0, f32::max);
    let mut job = LayoutJob::default();
    let mut buf = [0; 4];
    for c in text.chars() {
        let pad = c
            .to_digit(10)
            .filter(|_| c.is_ascii_digit())
            .and_then(|d| widths.get(d as usize))
            .map_or(0.0, |w| cell - w);
        job.append(c.encode_utf8(&mut buf), pad, format.clone());
    }
    job
}

#[cfg(test)]
mod tests {
    use super::{LayerOpacity, StateRamp, VisualState};
    use crate::widget::Interaction;

    #[test]
    fn states_rank_disabled_then_pressed_then_hover() {
        let i = Interaction {
            hovered: true,
            pressed: true,
            ..Interaction::default()
        };
        assert_eq!(VisualState::of(i), VisualState::Pressed);
        assert_eq!(
            VisualState::of(Interaction { disabled: true, ..i }),
            VisualState::Disabled
        );
        let ramp = StateRamp::some(1, 2, 3, 4).disabled(None);
        assert_eq!(
            (*ramp.get(VisualState::Pressed), *ramp.get(VisualState::Disabled)),
            (Some(3), None)
        );
        let layers = LayerOpacity {
            hover: 0.08,
            pressed: 0.1,
            ..LayerOpacity::default()
        };
        assert!((layers.of(i) - 0.1).abs() < f32::EPSILON);
        assert_eq!(
            layers.of(Interaction { disabled: true, ..i }).to_bits(),
            0.0_f32.to_bits()
        );
    }

    use egui::{FontId, TextFormat, text::LayoutJob};

    #[test]
    fn digits_share_one_advance() {
        let ctx = crate::test_support::ctx();
        let format = TextFormat::simple(FontId::proportional(14.0), egui::Color32::WHITE);
        let mut widths = Vec::new();
        crate::test_support::run(&ctx, egui::RawInput::default(), |ui| {
            let width = |job| ui.fonts_mut(|f| f.layout_job(job)).size().x;
            widths = ["f 111", "f 000", "f 478"]
                .map(|s| width(super::tabular_figures(ui.ctx(), s, &format)))
                .to_vec();
            let plain = LayoutJob::single_section("f 000".to_owned(), format.clone());
            assert!(widths[0] >= width(plain) && widths[0] > 0.0);
        });
        assert!(widths.iter().all(|w| (w - widths[0]).abs() < 0.01), "{widths:?}");
    }
}
