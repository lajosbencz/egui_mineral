use std::sync::Arc;

use egui::{Color32, Galley, Pos2, Rangef, Rect, Shape, Stroke, pos2, vec2};
use egui_mineral::{
    i18n::{Direction, I18n},
    kit,
    layout::{Constraints, Measure},
    semantic::{AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::ColorToken::{
    ColorBrandStroke1, ColorBrandStroke2Contrast, ColorNeutralForeground1, ColorNeutralForegroundStaticInverted,
    ColorNeutralStrokeAlpha2, ColorNeutralStrokeOnBrand2,
};
use crate::{
    Fluent, FluentExt,
    spec::{
        defaults::spinner as d,
        tokens::{geometry, motion},
    },
    typography::TypeRamp,
};

const ARC_STEPS: u8 = 24;

/// Spinner `size`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SpinnerSize {
    /// `extra-tiny`: 16.
    ExtraTiny,
    /// `tiny`: 20.
    Tiny,
    /// `extra-small`: 24.
    ExtraSmall,
    /// `small`: 28.
    Small,
    /// `medium`: 32.
    #[default]
    Medium,
    /// `large`: 36.
    Large,
    /// `extra-large`: 40.
    ExtraLarge,
    /// `huge`: 44.
    Huge,
}

impl SpinnerSize {
    /// Every variant, smallest first.
    pub const ALL: [Self; 8] = [
        Self::ExtraTiny,
        Self::Tiny,
        Self::ExtraSmall,
        Self::Small,
        Self::Medium,
        Self::Large,
        Self::ExtraLarge,
        Self::Huge,
    ];

    const fn ring(self) -> f32 {
        match self {
            Self::ExtraTiny => d::EXTRA_TINY,
            Self::Tiny => d::TINY,
            Self::ExtraSmall => d::EXTRA_SMALL,
            Self::Small => d::SMALL,
            Self::Medium => d::MEDIUM,
            Self::Large => d::LARGE,
            Self::ExtraLarge => d::EXTRA_LARGE,
            Self::Huge => d::HUGE,
        }
    }

    const fn stroke(self) -> f32 {
        match self {
            Self::ExtraTiny | Self::Tiny | Self::ExtraSmall | Self::Small => geometry::STROKE_WIDTH_THICK,
            Self::Medium | Self::Large | Self::ExtraLarge => geometry::STROKE_WIDTH_THICKER,
            Self::Huge => geometry::STROKE_WIDTH_THICKEST,
        }
    }

    const fn label(self) -> TypeRamp {
        match self {
            Self::ExtraTiny | Self::Tiny | Self::ExtraSmall | Self::Small => TypeRamp::Body1,
            Self::Medium | Self::Large | Self::ExtraLarge => TypeRamp::Subtitle2,
            Self::Huge => TypeRamp::Subtitle1,
        }
    }
}

/// Side of the ring the label sits on (`labelPosition`); before and after follow the text direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SpinnerLabelPosition {
    /// `before`: leading.
    Before,
    /// `after`: trailing.
    #[default]
    After,
    /// `above`.
    Above,
    /// `below`.
    Below,
}

impl SpinnerLabelPosition {
    /// Every variant.
    pub const ALL: [Self; 4] = [Self::Before, Self::After, Self::Above, Self::Below];
}

/// Spinner `appearance`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SpinnerAppearance {
    /// `primary`: brand arc on a contrast ring.
    #[default]
    Primary,
    /// `inverted`: for brand and dark backgrounds.
    Inverted,
}

impl SpinnerAppearance {
    /// Every variant.
    pub const ALL: [Self; 2] = [Self::Primary, Self::Inverted];
}

/// Fluent `Spinner`: a spinning arc on a ring with an optional label; static while motion is reduced.
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Spinner {
    salt: KeyedSalt,
    key: AgentKey,
    label: Option<String>,
    label_visible: bool,
    size: SpinnerSize,
    position: SpinnerLabelPosition,
    appearance: SpinnerAppearance,
}

impl Spinner {
    /// Medium, no label; agent key `<id_salt>-spinner`.
    pub fn new(id_salt: impl Into<KeyedSalt>) -> Self {
        Self {
            salt: id_salt.into(),
            key: AgentKey::Kind("spinner"),
            label: None,
            label_visible: true,
            size: SpinnerSize::Medium,
            position: SpinnerLabelPosition::After,
            appearance: SpinnerAppearance::Primary,
        }
    }

    /// Text next to the ring, also the accessible name.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Draws the label; default `true`. Hidden, it stays the accessible name.
    pub fn label_visible(mut self, visible: bool) -> Self {
        self.label_visible = visible;
        self
    }

    /// Default [`SpinnerSize::Medium`].
    pub fn size(mut self, size: SpinnerSize) -> Self {
        self.size = size;
        self
    }

    /// Default [`SpinnerLabelPosition::After`].
    pub fn label_position(mut self, position: SpinnerLabelPosition) -> Self {
        self.position = position;
        self
    }

    /// Default [`SpinnerAppearance::Primary`].
    pub fn appearance(mut self, appearance: SpinnerAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-spinner` agent key.
        agent_key(key)
    }

    fn vertical(&self) -> bool {
        matches!(self.position, SpinnerLabelPosition::Above | SpinnerLabelPosition::Below)
    }

    fn galley(&self, ui: &MineralUi<'_, Fluent>, color: Color32) -> Option<Arc<Galley>> {
        let text = self.label.as_ref().filter(|_| self.label_visible)?;
        let format = self.size.label().format(ui.ctx(), color);
        Some(
            ui.painter()
                .layout_job(egui::text::LayoutJob::single_section(text.clone(), format)),
        )
    }
}

fn keyframe(frames: [f32; 3], phase: f32) -> f32 {
    let (k, from, to) = if phase < 0.5 {
        (motion::CURVE_EASY_EASE.ease(phase * 2.0), frames[0], frames[1])
    } else {
        (motion::CURVE_EASY_EASE.ease((phase - 0.5) * 2.0), frames[1], frames[2])
    };
    from + (to - from) * k
}

// Parts of the arc `(start, len)` inside `mask`, in degrees modulo 360.
fn clip((start, len): (f32, f32), (mask_start, mask_len): (f32, f32)) -> impl Iterator<Item = (f32, f32)> {
    [-360.0, 0.0, 360.0].into_iter().filter_map(move |shift| {
        let from = (start + shift).max(mask_start);
        let to = (start + shift + len).min(mask_start + mask_len);
        (to > from).then_some((from, to - from))
    })
}

// Arcs of the tail in degrees clockwise from the top.
fn arcs(rtl: bool, reduced: bool, phase: f32) -> Vec<(f32, f32)> {
    if reduced {
        return vec![if rtl { d::REDUCED_ARC_RTL } else { d::REDUCED_ARC_LTR }];
    }
    let (mask, seg) = if rtl {
        (d::MASK_RTL, d::SEGMENT_START_RTL)
    } else {
        (d::MASK_LTR, 0.0)
    };
    let tail = keyframe(d::TAIL_ROTATION, phase);
    let spin = 360.0 * phase;
    [d::BEFORE_ROTATION, d::AFTER_ROTATION]
        .into_iter()
        .flat_map(|frames| clip((seg + keyframe(frames, phase), d::SEGMENT), mask))
        .map(|(start, len)| (start + tail + spin, len))
        .collect()
}

fn on_circle(center: Pos2, r: f32, degrees: f32) -> Pos2 {
    let a = degrees.to_radians();
    pos2(center.x + r * a.sin(), center.y - r * a.cos())
}

impl MineralWidget for Spinner {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt, hover, no_target);

    fn layout(&self, ui: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        let ring = self.size.ring();
        let label = self.galley(ui, Color32::PLACEHOLDER).map(|g| g.size());
        let size = match label {
            None => vec2(ring, ring),
            Some(l) if self.vertical() => vec2(ring.max(l.x), ring + d::GAP + l.y),
            Some(l) => vec2(ring + d::GAP + l.x, ring.max(l.y)),
        };
        c.constrain(size).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let dir = ui.i18n().direction();
        let rect = frame.zones.visual;
        let ring = self.size.ring();
        let inverted = self.appearance == SpinnerAppearance::Inverted;
        let label_color = t.color(if inverted {
            ColorNeutralForegroundStaticInverted
        } else {
            ColorNeutralForeground1
        });
        let galley = self.galley(ui, label_color);
        let label = galley.as_ref().map_or(vec2(0.0, 0.0), |g| g.size());
        let (ring_rect, label_pos) = match (galley.is_some(), self.position) {
            (false, _) => (Rect::from_center_size(rect.center(), vec2(ring, ring)), None),
            (true, SpinnerLabelPosition::Above | SpinnerLabelPosition::Below) => {
                let above = self.position == SpinnerLabelPosition::Above;
                let (label_y, ring_y) = if above {
                    (rect.top(), rect.top() + label.y + d::GAP)
                } else {
                    (rect.top() + ring + d::GAP, rect.top())
                };
                (
                    Rect::from_min_size(pos2(rect.center().x - ring / 2.0, ring_y), vec2(ring, ring)),
                    Some(pos2(rect.center().x - label.x / 2.0, label_y)),
                )
            }
            (true, side) => {
                let (ring_at, label_at) = if side == SpinnerLabelPosition::Before {
                    (label.x + d::GAP, 0.0)
                } else {
                    (0.0, ring + d::GAP)
                };
                let x = |at: f32, w: f32| dir.start_x(rect, Rangef::new(at, at + w));
                (
                    Rect::from_min_size(pos2(x(ring_at, ring), rect.center().y - ring / 2.0), vec2(ring, ring)),
                    Some(pos2(x(label_at, label.x), rect.center().y - label.y / 2.0)),
                )
            }
        };
        if let (Some(g), Some(pos)) = (galley, label_pos) {
            ui.painter().galley(pos, g, label_color);
        }
        self.paint_ring(ui, ring_rect, dir);
        WidgetResponse::new(frame.response.clone(), ())
    }

    fn semantics(&self, i18n: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let name = crate::tr_or(i18n, self.label.as_deref(), "mineral-progress");
        Some(SemanticNode::new(
            self.key.id(&self.salt),
            Role::ProgressIndicator,
            name,
        ))
    }
}

impl Spinner {
    fn paint_ring(&self, ui: &MineralUi<'_, Fluent>, rect: Rect, dir: Direction) {
        let t = ui.tokens();
        let reduced = ui.rt().core().motion().reduced();
        let (track, tail) = match self.appearance {
            SpinnerAppearance::Primary => (ColorBrandStroke2Contrast, ColorBrandStroke1),
            SpinnerAppearance::Inverted => (ColorNeutralStrokeAlpha2, ColorNeutralStrokeOnBrand2),
        };
        let width = self.size.stroke();
        let radius = rect.width() / 2.0 - width / 2.0;
        let center = rect.center();
        let p = ui.painter();
        p.circle_stroke(center, radius, Stroke::new(width, t.color(track)));
        let phase = if reduced {
            0.0
        } else {
            egui_mineral::motion::request_frame(ui.ctx());
            let cycle = d::CYCLE.as_secs_f64();
            (ui.input(|i| i.time).rem_euclid(cycle) / cycle) as f32
        };
        let stroke = Stroke::new(width, t.color(tail));
        for (start, len) in arcs(dir == Direction::Rtl, reduced, phase) {
            let points = (0..=ARC_STEPS)
                .map(|i| on_circle(center, radius, start + len * f32::from(i) / f32::from(ARC_STEPS)))
                .collect();
            p.add(Shape::line(points, stroke));
        }
    }
}
