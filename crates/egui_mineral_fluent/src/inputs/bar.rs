use crate::msg;
use egui::{Color32, CornerRadius, CursorIcon, Pos2, Rangef, Rect, Response, Sense, Stroke, Vec2, pos2, vec2};
use egui_mineral::{
    focus::{self, TabStop},
    i18n::{Direction, I18n},
    kit::state::TempState,
    layout::{Axis, Constraints, Measure},
    range::{RangeModel, Track, nearest_thumb},
    semantic::{self, KeyedSalt, Role, SemanticNode, StableId},
    widget::{Availability, MineralUi, WidgetFrame},
};

use crate::ColorToken::{
    ColorCompoundBrandBackground, ColorCompoundBrandBackgroundHover, ColorCompoundBrandBackgroundPressed,
    ColorNeutralBackground1, ColorNeutralBackgroundDisabled, ColorNeutralForegroundDisabled, ColorNeutralStroke1,
    ColorNeutralStrokeAccessible,
};
use crate::{
    Fluent, FluentExt, Size,
    spec::{defaults::slider as sl, tokens::geometry as g},
    widget::{
        focus as ring,
        state::{Ramp, State},
    },
};

// Slider and RangeSlider share rail, thumbs and input handling; `values` is one thumb or the ordered pair.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Bar {
    pub range: RangeModel,
    pub size: Size,
    pub axis: Axis,
    pub disabled: bool,
}

#[derive(Clone, Copy, Debug, Default)]
struct Active(Option<usize>);

impl TempState for Active {}

#[derive(Clone, Copy, Debug)]
struct Geo {
    axis: Axis,
    track: Track,
    span: Rangef,
    cross: f32,
    thumb: f32,
    inner: f32,
    rail: f32,
}

impl Geo {
    fn len(&self) -> f32 {
        self.span.span()
    }

    fn coord(&self, d: f32) -> f32 {
        if self.track.max_at_start() {
            self.span.max - d
        } else {
            self.span.min + d
        }
    }

    fn pt(&self, a: f32) -> Pos2 {
        match self.axis {
            Axis::Horizontal => pos2(a, self.cross),
            Axis::Vertical => pos2(self.cross, a),
        }
    }

    fn along(&self, p: Pos2) -> f32 {
        match self.axis {
            Axis::Horizontal => p.x,
            Axis::Vertical => p.y,
        }
    }

    // `thumbPositionVar`: `clamp(innerThumbRadius, progress, 100% - innerThumbRadius)`.
    fn thumb_center(&self, frac: f64) -> Pos2 {
        let d = (frac as f32 * self.len()).clamp(self.inner, (self.len() - self.inner).max(self.inner));
        self.pt(self.coord(d))
    }

    fn segment(&self, from: f64, to: f64) -> Rect {
        let (a, b) = (self.coord(from as f32 * self.len()), self.coord(to as f32 * self.len()));
        let (lo, hi) = (a.min(b), a.max(b));
        match self.axis {
            Axis::Horizontal => Rect::from_min_max(
                pos2(lo, self.cross - self.rail / 2.0),
                pos2(hi, self.cross + self.rail / 2.0),
            ),
            Axis::Vertical => Rect::from_min_max(
                pos2(self.cross - self.rail / 2.0, lo),
                pos2(self.cross + self.rail / 2.0, hi),
            ),
        }
    }

    fn thumb_rect(&self, frac: f64) -> Rect {
        Rect::from_center_size(self.thumb_center(frac), Vec2::splat(self.thumb))
    }
}

impl Bar {
    fn metrics(&self) -> (f32, f32, f32, f32) {
        match self.size {
            Size::Small => (
                sl::THUMB_SMALL,
                sl::INNER_THUMB_RADIUS_SMALL,
                sl::RAIL_SMALL,
                sl::MIN_CROSS_SMALL,
            ),
            Size::Medium | Size::Large => (
                sl::THUMB_MEDIUM,
                sl::INNER_THUMB_RADIUS_MEDIUM,
                sl::RAIL_MEDIUM,
                sl::MIN_CROSS_MEDIUM,
            ),
        }
    }

    fn geo(&self, visual: Rect, dir: Direction) -> Geo {
        let (thumb, inner, rail, _) = self.metrics();
        let (track, full, cross) = match self.axis {
            Axis::Horizontal => (Track::horizontal(dir), visual.x_range(), visual.center().y),
            Axis::Vertical => (Track::vertical(), visual.y_range(), visual.center().x),
        };
        let min = full.min + thumb / 2.0;
        Geo {
            axis: self.axis,
            track,
            span: Rangef::new(min, (full.max - thumb / 2.0).max(min)),
            cross,
            thumb,
            inner,
            rail,
        }
    }

    pub(crate) fn measure(&self, ui: &MineralUi<'_, Fluent>, c: Constraints) -> Measure {
        let (thumb, _, _, cross) = self.metrics();
        let max = match self.axis {
            Axis::Horizontal => c.max.x,
            Axis::Vertical => c.max.y,
        };
        // Unconstrained, a horizontal slider fills the width and a vertical one keeps `minHeight`.
        let fill = match self.axis {
            Axis::Horizontal => ui.available_size().x,
            Axis::Vertical => sl::MIN_LENGTH,
        };
        let main = [max, fill]
            .into_iter()
            .find(|v| v.is_finite())
            .unwrap_or(sl::MIN_LENGTH)
            .max(sl::MIN_LENGTH);
        c.constrain(match self.axis {
            Axis::Horizontal => vec2(main, cross),
            Axis::Vertical => vec2(thumb, main),
        })
        .into()
    }

    pub(crate) fn sense(&self, thumbs: usize) -> Sense {
        if self.disabled {
            Sense::hover()
        } else {
            focus::roving_sense(Sense::click_and_drag(), TabStop::at(thumbs, 1))
        }
    }

    pub(crate) fn text(&self, i18n: &I18n, v: f64) -> String {
        i18n.format().number_lossy(v, digits(&self.range))
    }

    pub(crate) fn node(&self, i18n: &I18n, id: StableId, name: &str, v: f64) -> SemanticNode {
        SemanticNode::new(id, Role::Slider, name)
            .value(self.text(i18n, v))
            .ranged(self.range.numeric(v), Availability::disabled_if(self.disabled))
    }

    fn colors(&self, s: State, t: &crate::theme::Tokens) -> [Color32; 4] {
        if self.disabled {
            let off = t.color(ColorNeutralForegroundDisabled);
            return [t.color(ColorNeutralBackgroundDisabled), off, off, off];
        }
        let brand = Ramp::new(
            ColorCompoundBrandBackground,
            ColorCompoundBrandBackgroundHover,
            ColorCompoundBrandBackgroundPressed,
            ColorCompoundBrandBackground,
        )
        .pick(s, t);
        [
            t.color(ColorNeutralStrokeAccessible),
            brand,
            brand,
            t.color(ColorNeutralStroke1),
        ]
    }

    fn order(values: &mut [f64], moved: usize) {
        if let [a, b] = values {
            if moved == 0 {
                *a = a.min(*b);
            } else {
                *b = b.max(*a);
            }
        }
    }

    // `values` in, edited `values` out; `salt`/`label` name the per-thumb nodes of a pair.
    pub(crate) fn update(
        &self,
        ui: &mut MineralUi<'_, Fluent>,
        frame: &WidgetFrame<'_>,
        values: &mut [f64],
        names: (&KeyedSalt, &str),
    ) {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let dir = ui.i18n().direction();
        let geo = self.geo(frame.zones.visual, dir);
        let r = &frame.response;
        let single = values.len() == 1;
        for v in values.iter_mut() {
            *v = self.range.snap(*v);
        }
        let thumbs: Vec<Response> = values
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let sense = if self.disabled {
                    Sense::hover()
                } else {
                    focus::roving_sense(Sense::click_and_drag(), TabStop::at(single, true))
                };
                frame
                    .pan
                    .apply(ui.interact(geo.thumb_rect(self.range.fraction(*v)), r.id.with(("thumb", i)), sense))
            })
            .collect();
        let stop = |i: usize| {
            if single {
                r.id
            } else {
                thumbs.get(i).map_or(r.id, |th| th.id)
            }
        };
        let state_id = r.id.with("active");
        let mut active = Active::load_or(ctx, state_id, Active::default).0;
        if !self.disabled {
            let range = self.range;
            let value_at = move |p: Pos2| range.value_at(geo.span, geo.along(p), geo.track);
            let started = thumbs.iter().position(Response::drag_started).or_else(|| {
                (r.drag_started() || r.clicked()).then(|| {
                    let at = r.interact_pointer_pos().map_or(0.0, value_at);
                    nearest_thumb(values, at).unwrap_or(0)
                })
            });
            if started.is_some() {
                active = started;
            }
            let dragging = r.dragged() || r.clicked() || thumbs.iter().any(Response::dragged);
            if let (Some(i), true) = (active, dragging)
                && let Some(p) = ctx.pointer_interact_pos()
                && let Some(v) = values.get_mut(i)
            {
                *v = value_at(p);
                Self::order(values, i);
                ctx.memory_mut(|m| m.request_focus(stop(i)));
            }
            if !dragging && !r.drag_stopped() {
                active = None;
            }
            if let Some(p) = frame.tapped
                && !r.clicked()
            {
                let i = nearest_thumb(values, value_at(p)).unwrap_or(0);
                values[i] = value_at(p);
                Self::order(values, i);
            }
            for i in 0..values.len() {
                values[i] = self.range.drive(rt.core(), stop(i), values[i], geo.track);
                Self::order(values, i);
            }
            if r.hovered() || thumbs.iter().any(|th| th.hovered() || th.dragged()) {
                ctx.set_cursor_icon(if active.is_some() {
                    CursorIcon::Grabbing
                } else {
                    CursorIcon::PointingHand
                });
            }
        }
        Active(active).store(ctx, state_id);

        self.paint(ui, frame, &geo, values, &thumbs, names);
    }

    fn paint(
        &self,
        ui: &mut MineralUi<'_, Fluent>,
        frame: &WidgetFrame<'_>,
        geo: &Geo,
        values: &[f64],
        thumbs: &[Response],
        names: (&KeyedSalt, &str),
    ) {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let t = ui.tokens();
        let r = &frame.response;
        let single = values.len() == 1;
        let hovered = r.hovered() || thumbs.iter().any(Response::hovered);
        let pressed = r.is_pointer_button_down_on() || thumbs.iter().any(Response::is_pointer_button_down_on);
        let s = if self.disabled {
            State::Disabled
        } else if pressed {
            State::Pressed
        } else if hovered {
            State::Hover
        } else {
            State::Rest
        };
        let [rail, progress, thumb, edge] = self.colors(s, &t);
        let fractions: Vec<f64> = values.iter().map(|v| self.range.fraction(*v)).collect();
        let (from, to) = match fractions.as_slice() {
            [a, b] => (*a, *b),
            [v] => (0.0, *v),
            _ => (0.0, 0.0),
        };
        let p = ui.painter();
        let radius = CornerRadius::from(g::BORDER_RADIUS_X_LARGE.min(geo.rail / 2.0));
        let rail_rect = geo.segment(0.0, 1.0);
        p.rect_filled(rail_rect, radius, rail);
        p.with_clip_rect(geo.segment(from, to))
            .rect_filled(rail_rect, radius, progress);
        let bg = t.color(ColorNeutralBackground1);
        for tick in self.range.ticks(1000).into_iter().skip(1) {
            let at = geo.coord(self.range.fraction(tick) as f32 * geo.len());
            let tick_rect = match geo.axis {
                Axis::Horizontal => {
                    Rect::from_min_max(pos2(at - sl::TICK_WIDTH, rail_rect.top()), pos2(at, rail_rect.bottom()))
                }
                Axis::Vertical => {
                    Rect::from_min_max(pos2(rail_rect.left(), at), pos2(rail_rect.right(), at + sl::TICK_WIDTH))
                }
            };
            p.rect_filled(tick_rect, 0.0, bg);
        }
        for f in &fractions {
            let c = geo.thumb_center(*f);
            let rad = geo.thumb / 2.0;
            p.circle_filled(c, rad, bg);
            p.circle_filled(c, rad - geo.thumb * sl::THUMB_INSET, thumb);
            let w = geo.thumb * sl::THUMB_BORDER;
            p.circle_stroke(c, rad - w / 2.0, Stroke::new(w, edge));
        }
        let focused = if single {
            focus::visible(ctx, r.id)
        } else {
            thumbs.iter().any(|th| focus::visible(ctx, th.id))
        };
        if focused && !self.disabled {
            let outset = match geo.axis {
                Axis::Horizontal => vec2(sl::FOCUS_OFFSET_ACROSS, sl::FOCUS_OFFSET_ALONG),
                Axis::Vertical => vec2(-sl::FOCUS_OFFSET_ACROSS, sl::FOCUS_OFFSET_ALONG),
            };
            ring::outline(
                p,
                frame.zones.visual.expand2(outset),
                CornerRadius::from(g::BORDER_RADIUS_MEDIUM),
                &t,
            );
        }
        if !single {
            for (i, th) in thumbs.iter().enumerate() {
                semantic::emit(ctx, Some(th.id), th.rect, || {
                    let (salt, label) = names;
                    let label = label.into();
                    let name = if i == 0 {
                        rt.i18n().tr(msg::RangeStart { label })
                    } else {
                        rt.i18n().tr(msg::RangeEnd { label })
                    };
                    let mut n = self.node(
                        rt.i18n(),
                        StableId::of(salt, if i == 0 { "slider-start" } else { "slider-end" }),
                        &name,
                        values.get(i).copied().unwrap_or_default(),
                    );
                    n.depth = 1;
                    n
                });
            }
        }
    }
}

pub(crate) fn digits(range: &RangeModel) -> Option<i16> {
    match range.step_size() {
        Some(s) => Some(
            s.to_string()
                .split_once('.')
                .map_or(0, |(_, f)| i16::try_from(f.len()).map_or(6, |n| n.min(6))),
        ),
        None if range.significant_digits().is_some() => None,
        None => Some(2),
    }
}
