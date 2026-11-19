use crate::Material;
use crate::MaterialExt;
use crate::Rt;
use egui::{Color32, CursorIcon, Pos2, Rangef, Rect, Sense, Ui, Vec2, epaint::CornerRadiusF32, pos2};
use egui_mineral::i18n::I18n;
use egui_mineral::semantic::AgentKey;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    focus,
    layout::{Axis, Constraints, Measure, MinTarget},
    range::{RangeModel, Track, nearest_thumb},
    semantic::{self, Role, SemanticNode, StableId},
    widget::{Availability, MineralWidget, PanGate, WidgetFrame, WidgetResponse},
};

use crate::MaterialUiExt as _;
use crate::extras::NumberField;
use crate::text_inputs::TextFieldVariant;
use crate::text_inputs::text_field::field_h;
use egui_mineral::i18n::Direction;
use egui_mineral::widget::SizeExt as _;
use crate::widget::parts;
use crate::{
    MaterialIcon,
    spec::{
        defaults::slider as sx,
        tokens::{plain_tooltip as tip, slider as tok},
    },
    tokens::Tokens,
};
use crate::{
    Size,
    widget::common::{self, with_alpha},
};

const MAX_TICKS: usize = 64;
// No MD3 value; five bodyLarge digits with the field paddings
const VALUE_FIELD_WIDTH: f32 = 88.0;
const OUTER_ICON: f32 = 24.0;
const OUTER_ICON_GAP: f32 = 8.0;

#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct ValueSliderState {
    active: Option<usize>,
}

fn metrics(size: Size) -> sx::SliderMetrics {
    match size {
        Size::XSmall => sx::XS,
        Size::Small => sx::S,
        Size::Medium => sx::M,
        Size::Large => sx::L,
        Size::XLarge => sx::XL,
    }
}

#[derive(Clone, Copy)]
struct TrackAxis {
    track: Track,
    across: f32,
    span: Rangef,
}

impl TrackAxis {
    fn axis(self) -> Axis {
        self.track.axis()
    }

    fn pt(self, along: f32, off: f32) -> Pos2 {
        self.axis().pos2(along, self.across + off)
    }

    fn along(self, p: Pos2) -> f32 {
        self.axis().main(p.to_vec2())
    }

    fn screen(self, a: f32) -> f32 {
        if self.track.max_at_start() {
            self.span.max - a
        } else {
            self.span.min + a
        }
    }

    fn rect(self, a0: f32, a1: f32, thick: f32) -> Rect {
        let c = self.across;
        self.axis().rect(
            Rangef::new(a0.min(a1), a0.max(a1)),
            Rangef::new(c - thick / 2.0, c + thick / 2.0),
        )
    }

    fn segment(self, a0: f32, a1: f32, thick: f32, r0: f32, r1: f32) -> (Rect, CornerRadiusF32) {
        let rect = self.rect(self.screen(a0), self.screen(a1), thick);
        let max = rect.width().min(rect.height()) / 2.0;
        let (lo, hi) = if self.track.max_at_start() { (r1, r0) } else { (r0, r1) };
        let (lo, hi) = (lo.min(max), hi.min(max));
        let r = match self.axis() {
            Axis::Vertical => CornerRadiusF32 {
                nw: lo,
                ne: lo,
                sw: hi,
                se: hi,
            },
            Axis::Horizontal => CornerRadiusF32 {
                nw: lo,
                sw: lo,
                ne: hi,
                se: hi,
            },
        };
        (rect, r)
    }

    fn size(self, along: f32, across: f32) -> Vec2 {
        self.axis().vec2(along, across)
    }
}

/// MD3 slider, one thumb or a range (Compose `Slider`, `RangeSlider`).
#[must_use = "show with ui.mineral(..)"]
pub struct ValueSlider<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    values: &'a mut [f64],
    range: RangeModel,
    ticks: bool,
    indicator: bool,
    format: Option<Box<dyn Fn(f64) -> String + 'a>>,
    disabled: bool,
    size: Size,
    centered: bool,
    axis: Axis,
    icons: EndIcons,
    value_field: bool,
}

#[derive(Clone, Copy, Debug, Default)]
struct EndIcons {
    min: Option<MaterialIcon>,
    max: Option<MaterialIcon>,
}

egui_mineral::kit::opaque_debug!(ValueSlider<'_>);

// The range's fraction digits, else the step's (then the increment's) decimals; the value's own
// (`None`) on a significant-digit grid; else 2.
pub(crate) fn fraction_digits(range: &RangeModel) -> Option<i16> {
    let decimals = |s: f64| {
        s.to_string()
            .split_once('.')
            .map_or(0, |(_, f)| i16::try_from(f.len()).map_or(6, |n| n.min(6)))
    };
    if let Some(d) = range.fraction_digit_count() {
        return Some(i16::from(d));
    }
    match range.step_size().or(range.increment_size()) {
        Some(s) => Some(decimals(s)),
        None if range.significant_digits().is_some() => None,
        None => Some(2),
    }
}

impl<'a> ValueSlider<'a> {
    /// One thumb bound to `value`; `range` sets the bounds and step. Agent key `<id_salt>-slider`.
    pub fn new(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        range: impl Into<RangeModel>,
        value: &'a mut f64,
    ) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self::with_values(id_salt, label, range.into(), std::slice::from_mut(value))
    }

    /// Two thumbs bound to `values`, kept ordered. Agent key `<id_salt>-slider`.
    pub fn range(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        range: impl Into<RangeModel>,
        values: &'a mut [f64; 2],
    ) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self::with_values(id_salt, label, range.into(), values)
    }

    fn with_values(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        range: RangeModel,
        values: &'a mut [f64],
    ) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        let label = label.into();
        Self {
            key: AgentKey::Kind("slider"),
            salt: id_salt,
            label,
            values,
            range,
            ticks: false,
            indicator: true,
            format: None,
            disabled: false,
            size: Size::XSmall,
            centered: false,
            axis: Axis::Horizontal,
            icons: EndIcons::default(),
            value_field: false,
        }
    }

    /// Tick marks at each step.
    pub fn ticks(mut self, ticks: bool) -> Self {
        self.ticks = ticks;
        self
    }

    /// Value label above the thumb while dragged or hovered; default `true`.
    pub fn indicator(mut self, indicator: bool) -> Self {
        self.indicator = indicator;
        self
    }

    /// Value label text; default the locale number format.
    pub fn format(mut self, format: impl Fn(f64) -> String + 'a) -> Self {
        self.format = Some(Box::new(format));
        self
    }

    /// Number field after the track bound to the same value, labeled and named like the slider.
    /// Single thumb, horizontal only. Default `false`.
    pub fn value_field(mut self, value_field: bool) -> Self {
        self.value_field = value_field;
        self
    }

    crate::widget::setters! {
        /// Greys out the slider and ignores input.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-slider` agent key.
        agent_key(key)
    }

    /// Default [`Size::XSmall`], the Compose `Slider`.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Single-thumb only: the active track grows from the center of the range.
    pub fn centered(mut self, centered: bool) -> Self {
        self.centered = centered;
        self
    }

    /// Default [`Axis::Horizontal`].
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    /// Icon at the min end: inset in the track for sizes with a track icon, else beside it.
    pub fn icon(mut self, icon: MaterialIcon) -> Self {
        self.icons.min = Some(icon);
        self
    }

    /// Icon at the max end: inset in the track for sizes with a track icon, else beside it.
    pub fn trailing_icon(mut self, trailing_icon: MaterialIcon) -> Self {
        self.icons.max = Some(trailing_icon);
        self
    }

    fn text(&self, i18n: &I18n, v: f64) -> String {
        if let Some(f) = &self.format {
            return f(v);
        }
        i18n.format().number_lossy(v, fraction_digits(&self.range))
    }

    fn order(&mut self, moved: usize) {
        if let [a, b] = self.values {
            if moved == 0 {
                *a = a.min(*b);
            } else {
                *b = b.max(*a);
            }
        }
    }

    fn outer_icons(&self) -> bool {
        metrics(self.size).icon.is_none()
    }

    fn cross(&self) -> f32 {
        let m = metrics(self.size);
        m.handle.max(m.track)
    }

    fn has_value_field(&self) -> bool {
        self.value_field && self.values.len() == 1 && self.axis == Axis::Horizontal
    }

    fn is_centered(&self) -> bool {
        self.centered && self.values.len() == 1
    }
}

fn tab_stop(focusable: bool) -> focus::TabStop {
    if focusable {
        focus::TabStop::Stop
    } else {
        focus::TabStop::Skip
    }
}

struct SliderLayout {
    m: sx::SliderMetrics,
    axis: TrackAxis,
    geo: Geometry,
    lo: f32,
    hi: f32,
}

struct PaintCx<'a> {
    t: &'a Tokens,
    colors: &'a Colors,
    lay: &'a SliderLayout,
    r: &'a egui::Response,
    thumbs: &'a [egui::Response],
    active: Option<usize>,
}

impl ValueSlider<'_> {
    fn slider_layout(&self, rt: &Rt, rect: Rect) -> SliderLayout {
        let m = metrics(self.size);
        let full = self.axis.main_range(rect);
        let track_dir = match self.axis {
            Axis::Horizontal => Track::horizontal(rt.i18n().direction()),
            Axis::Vertical => Track::vertical(),
        };
        let slot = OUTER_ICON + OUTER_ICON_GAP;
        let (lo_icon, hi_icon) = {
            let (min_icon, max_icon) = (self.icons.min.is_some(), self.icons.max.is_some());
            if track_dir.max_at_start() {
                (max_icon, min_icon)
            } else {
                (min_icon, max_icon)
            }
        };
        let outer = self.outer_icons();
        let lo = full.min + if outer && lo_icon { slot } else { 0.0 };
        let hi = full.max - if outer && hi_icon { slot } else { 0.0 };
        // Compose `SliderLayout`: the track is inset by half the thumb.
        let track = Rangef::new(lo + tok::HANDLE_WIDTH / 2.0, hi - tok::HANDLE_WIDTH / 2.0);
        let axis = TrackAxis {
            track: track_dir,
            across: self.axis.cross(rect.center().to_vec2()),
            span: track,
        };
        let geo = Geometry {
            len: track.span(),
            corner: m.corner.unwrap_or(m.track / 2.0),
            shrink: m.corner.is_some() || self.is_centered(),
            ticks: self
                .range
                .ticks(MAX_TICKS)
                .into_iter()
                .map(|v| self.range.fraction(v) as f32)
                .collect(),
            centered: self.is_centered(),
            range: self.values.len() > 1,
        };
        SliderLayout { m, axis, geo, lo, hi }
    }

    fn stop_id(&self, r: &egui::Response, thumbs: &[egui::Response], i: usize) -> egui::Id {
        if self.values.len() == 1 {
            r.id
        } else {
            thumbs.get(i).map_or(r.id, |th| th.id)
        }
    }

    // The single slider's track is its Tab stop; a range slider's thumbs are one each (APG multi-thumb
    // slider), so each takes keyboard and AccessKit input.
    fn thumbs(&self, ui: &Ui, r: &egui::Response, pan: PanGate, lay: &SliderLayout) -> Vec<egui::Response> {
        let (axis, m) = (lay.axis, lay.m);
        let single = self.values.len() == 1;
        let thumb_rect = |v: f64| {
            let at = axis.screen(lay.geo.at(self.range.fraction(v) as f32));
            Rect::from_center_size(axis.pt(at, 0.0), axis.size(tok::HANDLE_WIDTH, m.handle))
        };
        self.values
            .iter()
            .enumerate()
            .map(|(i, v)| {
                let sense = if self.disabled {
                    Sense::hover()
                } else {
                    focus::roving_sense(Sense::click_and_drag(), tab_stop(!single))
                };
                pan.apply(ui.interact(thumb_rect(*v), r.id.with(("thumb", i)), sense))
            })
            .collect()
    }

    fn drive_input(
        &mut self,
        rt: &Rt,
        frame: &WidgetFrame<'_>,
        state: &mut ValueSliderState,
        thumbs: &[egui::Response],
        lay: &SliderLayout,
    ) {
        let ctx = rt.ctx();
        let r = &frame.response;
        let (axis, track, track_dir) = (lay.axis, lay.axis.span, lay.axis.track);
        let range = self.range;
        let value_at = move |p: Pos2| range.value_at(track, axis.along(p), track_dir);
        let pointer = thumbs.iter().position(egui::Response::drag_started).or_else(|| {
            (r.drag_started() || r.clicked()).then(|| {
                let at = r.interact_pointer_pos().map_or(0.0, value_at);
                nearest_thumb(self.values, at).unwrap_or(0)
            })
        });
        if pointer.is_some() {
            state.active = pointer;
        }
        let dragging = r.dragged() || r.clicked() || thumbs.iter().any(egui::Response::dragged);
        // `active` outlives a frame; the thumb count may have dropped since.
        if let (Some(i), true) = (state.active, dragging)
            && let Some(p) = ctx.pointer_interact_pos()
            && let Some(v) = self.values.get_mut(i)
        {
            *v = value_at(p);
            self.order(i);
            ctx.memory_mut(|m| m.request_focus(self.stop_id(r, thumbs, i)));
        }
        if !dragging && !r.drag_stopped() {
            state.active = None;
        }
        if let Some(p) = frame.tapped
            && !r.clicked()
        {
            let i = nearest_thumb(self.values, value_at(p)).unwrap_or(0);
            self.values[i] = value_at(p);
            self.order(i);
        }
        for i in 0..self.values.len() {
            let id = self.stop_id(r, thumbs, i);
            self.values[i] = self.range.drive(rt.core(), id, self.values[i], track_dir);
            self.order(i);
        }
        if thumbs.iter().any(|th| th.hovered() || th.dragged()) || r.hovered() {
            ctx.set_cursor_icon(if state.active.is_some() {
                CursorIcon::Grabbing
            } else {
                CursorIcon::Grab
            });
        }
    }

    fn paint_track(&self, p: &egui::Painter, pc: &PaintCx<'_>) {
        let (t, colors, lay) = (pc.t, pc.colors, pc.lay);
        let (axis, m) = (lay.axis, lay.m);
        let fractions: Vec<f32> = self.values.iter().map(|v| self.range.fraction(*v) as f32).collect();
        let (start, end) = match fractions.as_slice() {
            [a, b] => (*a, *b),
            [v] => (0.0, *v),
            _ => (0.0, 0.0),
        };
        let inset = m.icon.zip(self.icons.min.or(self.icons.max)).is_some();
        let plan = lay.geo.plan(start, end, !inset);
        for seg in &plan.segments {
            let color = if seg.active {
                colors.active_track
            } else {
                colors.inactive_track
            };
            let (rect, radius) = axis.segment(seg.a0, seg.a1, m.track, seg.r0, seg.r1);
            p.rect_filled(rect, radius, color);
        }
        for a in &plan.stops {
            p.circle_filled(
                axis.pt(axis.screen(*a), 0.0),
                tok::STOP_INDICATOR_SIZE / 2.0,
                colors.active_track,
            );
        }
        if self.ticks {
            for (a, active) in &plan.ticks {
                let color = if *active {
                    colors.active_tick
                } else {
                    colors.inactive_tick
                };
                p.circle_filled(axis.pt(axis.screen(*a), 0.0), tok::STOP_INDICATOR_SIZE / 2.0, color);
            }
        }
        if let Some(size) = m.icon {
            plan.paint_icons(p, axis, self.icons, size, colors);
        }
        if self.outer_icons() {
            let slot = OUTER_ICON + OUTER_ICON_GAP;
            let col = if self.disabled {
                common::dim(t.color.on_surface_variant, t)
            } else {
                t.color.on_surface_variant
            };
            for (icon, at_lo) in [
                (self.icons.min, !axis.track.max_at_start()),
                (self.icons.max, axis.track.max_at_start()),
            ] {
                if let Some(icon) = icon {
                    let a = if at_lo {
                        lay.lo - slot + OUTER_ICON / 2.0
                    } else {
                        lay.hi + slot - OUTER_ICON / 2.0
                    };
                    common::icon(p, axis.pt(a, 0.0), icon, OUTER_ICON, col);
                }
            }
        }
    }

    fn paint_thumbs(&self, ui: &MineralUi<'_, Material>, pc: &PaintCx<'_>) {
        let rt = ui.rt();
        let (t, r, lay) = (pc.t, pc.r, pc.lay);
        let ctx = ui.ctx();
        let p = ui.painter();
        let tip_painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, r.id.with("value")));
        let single_held = self.values.len() == 1 && r.is_pointer_button_down_on();
        for (i, th) in pc.thumbs.iter().enumerate() {
            let pressed = pc.active == Some(i) || th.is_pointer_button_down_on() || single_held;
            let width = if self.disabled {
                tok::DISABLED_HANDLE_WIDTH
            } else if pressed {
                tok::PRESSED_HANDLE_WIDTH
            } else if focus::visible(&th.ctx, self.stop_id(r, pc.thumbs, i)) {
                tok::FOCUS_HANDLE_WIDTH
            } else if th.hovered() {
                tok::HOVER_HANDLE_WIDTH
            } else {
                tok::HANDLE_WIDTH
            };
            let thumb = Rect::from_center_size(th.rect.center(), lay.axis.size(width, lay.m.handle));
            p.rect_filled(thumb, tok::HANDLE_SHAPE.scale().radius(&t.shape), pc.colors.thumb);
            // Compose `Label`: shown while the thumb is hovered, pressed or dragged.
            if self.indicator
                && !self.disabled
                && (pressed || th.hovered())
                && let Some(v) = self.values.get(i)
            {
                value_label(rt, &tip_painter, t, th.rect, &self.text(rt.i18n(), *v));
            }
        }
    }

    fn emit_thumb_nodes(&self, rt: &Rt, pc: &PaintCx<'_>) {
        let ctx = rt.ctx();
        if self.values.len() < 2 {
            return;
        }
        for (i, th) in pc.thumbs.iter().enumerate() {
            semantic::emit(ctx, Some(th.id), th.rect, || {
                let msg = if i == 0 {
                    "mineral-range-start"
                } else {
                    "mineral-range-end"
                };
                let name = rt.i18n().tr(msg, &[("label", (&self.label).into())]);
                let mut n = self.node(
                    rt.i18n(),
                    self.key.id(&self.salt).sub(if i == 0 { "start" } else { "end" }),
                    name,
                    self.values.get(i).copied().unwrap_or_default(),
                );
                n.depth = 1;
                n
            });
        }
    }
}

impl MineralWidget for ValueSlider<'_> {
    type Kit = Material;
    type State = ValueSliderState;
    type Output = ();

    egui_mineral::kit::widget_meta!(salt, enabled);

    /// Focusable only as a single slider; a range slider takes focus per thumb.
    fn sense(&self) -> Sense {
        focus::roving_sense(Sense::click_and_drag(), tab_stop(self.values.len() == 1))
    }

    fn pan_axis(&self) -> Option<Axis> {
        Some(self.axis)
    }

    fn min_target(&self) -> MinTarget {
        crate::adaptive::MIN_INTERACTIVE
    }

    /// Fills the main axis like Compose `fillMaxWidth` / `fillMaxHeight`.
    fn layout(&self, ui: &MineralUi<'_, Material>, c: Constraints, _: &Self::State) -> Measure {
        let max = self.axis.main(c.max);
        let main = if max.is_finite() {
            max
        } else {
            self.axis.main(ui.available_size())
        };
        let cross = if self.has_value_field() {
            self.cross().max(field_h(ui.density(), TextFieldVariant::Outlined, false))
        } else {
            self.cross()
        };
        c.constrain(self.axis.vec2(main, cross)).into()
    }

    fn update(
        &mut self,
        ui: &mut MineralUi<'_, Material>,
        frame: &WidgetFrame<'_>,
        state: &mut ValueSliderState,
    ) -> WidgetResponse {
        let rt = ui.rt();
        let t = ui.tokens();
        let (track, field) = self.split_value_field(rt, frame.zones.visual);
        let lay = self.slider_layout(rt, track);
        let r = &frame.response;
        let before: Vec<f64> = self.values.to_vec();
        for v in self.values.iter_mut() {
            *v = self.range.snap(*v);
        }
        let thumbs = self.thumbs(ui, r, frame.pan, &lay);
        if !self.disabled {
            self.drive_input(rt, frame, state, &thumbs, &lay);
        }

        let colors = Colors::of(&t, !self.disabled);
        let pc = PaintCx {
            t: &t,
            colors: &colors,
            lay: &lay,
            r,
            thumbs: &thumbs,
            active: state.active,
        };
        self.paint_track(ui.painter(), &pc);
        self.paint_thumbs(ui, &pc);
        self.emit_thumb_nodes(rt, &pc);
        if let Some(field) = field {
            self.show_value_field(ui, field);
        }
        let changed = before
            .iter()
            .zip(self.values.iter())
            .any(|(a, b)| (a - b).abs() > f64::EPSILON);
        WidgetResponse::new(r.clone(), ()).with_changed(changed)
    }

    fn semantics(&self, i18n: &I18n, _: &WidgetResponse, _: &ValueSliderState) -> Option<SemanticNode> {
        Some(if let [a, b] = &*self.values {
            let value = i18n.tr(
                "mineral-range-value",
                &[
                    ("start", self.text(i18n, *a).into()),
                    ("end", self.text(i18n, *b).into()),
                ],
            );
            let mut n = SemanticNode::new(self.key.id(&self.salt), Role::Group, &self.label).value(value);
            n.state.disabled = self.disabled;
            n
        } else {
            self.node(
                i18n,
                self.key.id(&self.salt),
                self.label.clone(),
                self.values.first().copied().unwrap_or_default(),
            )
        })
    }
}

impl ValueSlider<'_> {
    // The track rect and, with `value_field`, the rect of the field at the end.
    fn split_value_field(&self, rt: &Rt, visual: Rect) -> (Rect, Option<Rect>) {
        if !self.has_value_field() {
            return (visual, None);
        }
        let width = VALUE_FIELD_WIDTH.min(visual.width() / 2.0);
        let gap = crate::adaptive::grid(2);
        let dir = rt.i18n().direction();
        let (track, field) = match dir {
            Direction::Ltr => (
                Rect::from_min_max(visual.min, pos2(visual.right() - width - gap, visual.bottom())),
                Rect::from_min_max(pos2(visual.right() - width, visual.top()), visual.max),
            ),
            Direction::Rtl => (
                Rect::from_min_max(pos2(visual.left() + width + gap, visual.top()), visual.max),
                Rect::from_min_max(visual.min, pos2(visual.left() + width, visual.bottom())),
            ),
        };
        (track, Some(field))
    }

    fn show_value_field(&mut self, ui: &mut MineralUi<'_, Material>, rect: Rect) {
        let rt = ui.rt();
        let (salt, range, disabled) = (self.salt.clone(), self.range, self.disabled);
        let field = NumberField::new(salt.child("value"), self.label.clone(), range, &mut self.values[0])
            .variant(TextFieldVariant::Outlined)
            .label_visible(false)
            .step_buttons(false)
            .disabled(disabled)
            .agent_key(StableId::of(&salt, "value"))
            .preferred_width(rect.width());
        ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
            rt.ui(ui).mineral(field);
        });
    }

    fn node(&self, i18n: &I18n, id: StableId, name: String, v: f64) -> SemanticNode {
        SemanticNode::new(id, Role::Slider, name)
            .value(self.text(i18n, v))
            .ranged(self.range.numeric(v), Availability::disabled_if(self.disabled))
    }
}

// Compose SliderDefaults.colors()
struct Colors {
    thumb: Color32,
    active_track: Color32,
    inactive_track: Color32,
    active_tick: Color32,
    inactive_tick: Color32,
}

impl Colors {
    fn of(t: &Tokens, enabled: bool) -> Self {
        let c = &t.color;
        if enabled {
            Self {
                thumb: t.role(tok::HANDLE_COLOR),
                active_track: t.role(tok::ACTIVE_TRACK_COLOR),
                inactive_track: t.role(tok::INACTIVE_TRACK_COLOR),
                active_tick: t.role(tok::INACTIVE_TRACK_COLOR),
                inactive_tick: t.role(tok::ACTIVE_TRACK_COLOR),
            }
        } else {
            let active = with_alpha(
                t.role(tok::DISABLED_ACTIVE_TRACK_COLOR),
                tok::DISABLED_ACTIVE_TRACK_OPACITY,
            );
            let inactive = with_alpha(
                t.role(tok::DISABLED_INACTIVE_TRACK_COLOR),
                tok::DISABLED_INACTIVE_TRACK_OPACITY,
            );
            Self {
                thumb: c.surface.blend(with_alpha(
                    t.role(tok::DISABLED_HANDLE_COLOR),
                    tok::DISABLED_HANDLE_OPACITY,
                )),
                active_track: active,
                inactive_track: inactive,
                active_tick: inactive,
                inactive_tick: active,
            }
        }
    }
}

fn value_label(rt: &Rt, p: &egui::Painter, t: &Tokens, anchor: Rect, text: &str) {
    let color = t.role(tip::SUPPORTING_TEXT_COLOR);
    let galley = parts::type_galley(rt.core(), p, text, tip::SUPPORTING_TEXT_FONT, color, None);
    let pad = Vec2::from(sx::VALUE_PADDING);
    let size = (galley.size() + 2.0 * pad).max(Vec2::from(sx::VALUE_MIN_SIZE));
    let rect = Rect::from_min_size(
        pos2(
            anchor.center().x - size.x / 2.0,
            anchor.top() - sx::VALUE_SPACE - size.y,
        ),
        size,
    );
    let radius = tip::CONTAINER_SHAPE.scale().radius(&t.shape);
    p.rect_filled(rect, common::radius(rect, radius), t.role(tip::CONTAINER_COLOR));
    p.galley(rect.min + pad, galley, color);
}

// Compose SliderDefaults.drawTrack
struct Geometry {
    len: f32,
    corner: f32,
    shrink: bool,
    ticks: Vec<f32>,
    centered: bool,
    range: bool,
}

struct Segment {
    a0: f32,
    a1: f32,
    r0: f32,
    r1: f32,
    active: bool,
}

struct Plan {
    segments: Vec<Segment>,
    stops: Vec<f32>,
    ticks: Vec<(f32, bool)>,
}

impl Geometry {
    fn at(&self, f: f32) -> f32 {
        let edge = [self.ticks.first(), self.ticks.last()]
            .into_iter()
            .flatten()
            .any(|e| (e - f).abs() <= f32::EPSILON);
        if self.ticks.is_empty() || edge {
            self.len * f
        } else {
            self.corner + (self.len - 2.0 * self.corner) * f
        }
    }

    fn plan(&self, start: f32, end: f32, stops: bool) -> Plan {
        let (w, corner, inside) = (self.len, self.corner, sx::INSIDE_CORNER);
        let (s_v, e_v) = (self.at(start), self.at(end));
        let center = w / 2.0;
        let gap = tok::ACTIVE_HANDLE_LEADING_SPACE;
        let half = tok::HANDLE_WIDTH / 2.0;
        let start_gap = match (self.centered, self.range) {
            (true, _) => gap + if end > 0.5 { 0.0 } else { half },
            (false, true) => gap + half,
            (false, false) => 0.0,
        };
        let end_gap = gap + if self.centered && end < 0.5 { 0.0 } else { half };
        let pad = !self.shrink || !self.ticks.is_empty();
        let pad_corner = if pad { corner } else { 0.0 };
        let mut plan = Plan {
            segments: Vec::new(),
            stops: Vec::new(),
            ticks: Vec::new(),
        };

        let low_end = if self.centered { e_v.min(center) } else { s_v };
        if (self.centered || self.range) && low_end > start_gap + pad_corner {
            plan.segments.push(Segment {
                a0: 0.0,
                a1: low_end - start_gap,
                r0: corner,
                r1: inside,
                active: false,
            });
            plan.stops.push(corner);
        }
        let high_start = if self.centered { e_v.max(center) } else { e_v };
        if high_start < w - end_gap - pad_corner {
            plan.segments.push(Segment {
                a0: high_start + end_gap,
                a1: w,
                r0: inside,
                r1: corner,
                active: false,
            });
            plan.stops.push(w - corner);
        }
        let active_start = if self.centered {
            low_end + if low_end < center { start_gap } else { 0.0 }
        } else if self.range {
            s_v + start_gap
        } else {
            0.0
        };
        let active_end = if self.centered {
            high_start - if high_start > center { end_gap } else { 0.0 }
        } else {
            e_v - end_gap
        };
        let r_start = if self.centered || self.range { inside } else { corner };
        if active_end - active_start > if pad { r_start } else { 0.0 } {
            plan.segments.push(Segment {
                a0: active_start,
                a1: active_end,
                r0: r_start,
                r1: inside,
                active: true,
            });
        }
        if !stops {
            plan.stops.clear();
        }

        let around = |x: f32, at: f32, g: f32| (at - g..=at + g).contains(&x);
        let (center_gap, end_gap_at) = if e_v > center {
            (start_gap, end_gap)
        } else {
            (end_gap, start_gap)
        };
        let last = self.ticks.len().saturating_sub(1);
        for (i, f) in self.ticks.iter().enumerate() {
            if stops && (i == last || (i == 0 && (self.centered || self.range))) {
                continue;
            }
            let x = egui::lerp(corner..=w - corner, *f);
            let skip = (self.centered && around(x, center, center_gap))
                || (self.range && around(x, s_v, start_gap))
                || around(x, e_v, if self.centered { end_gap_at } else { end_gap });
            if !skip {
                plan.ticks.push((x, (active_start..=active_end).contains(&x)));
            }
        }
        plan
    }
}

impl Plan {
    fn paint_icons(&self, p: &egui::Painter, axis: TrackAxis, icons: EndIcons, size: f32, colors: &Colors) {
        let first = self.segments.iter().min_by(|a, b| a.a0.total_cmp(&b.a0));
        let last = self.segments.iter().max_by(|a, b| a.a1.total_cmp(&b.a1));
        let pad = sx::ICON_PADDING;
        for (icon, seg, at_min) in [(icons.min, first, true), (icons.max, last, false)] {
            let (Some(icon), Some(seg)) = (icon, seg) else { continue };
            if size >= seg.a1 - seg.a0 - 2.0 * pad {
                continue;
            }
            let a = if at_min {
                seg.a0 + pad + size / 2.0
            } else {
                seg.a1 - pad - size / 2.0
            };
            let color = if seg.active {
                colors.active_tick
            } else {
                colors.inactive_tick
            };
            common::icon(p, axis.pt(axis.screen(a), 0.0), icon, size, color);
        }
    }
}

#[cfg(all(test, feature = "bundled-icons"))]
mod tests {
    use egui::{Context, Event, PointerButton, pos2, vec2};
    use egui_mineral::input::synth;

    use super::*;

    fn pass(ctx: &Context, rt: &crate::Rt, events: Vec<Event>, two: bool) {
        let (mut one, mut pair) = (50.0, [20.0, 80.0]);
        crate::test_support::pass(ctx, rt, vec2(400.0, 200.0), events, |ui| {
            if two {
                ui.mineral(ValueSlider::range("s", "S", RangeModel::new(0.0, 100.0), &mut pair));
            } else {
                ui.mineral(ValueSlider::new("s", "S", RangeModel::new(0.0, 100.0), &mut one));
            }
        });
    }

    #[test]
    fn drag_survives_the_thumb_count_dropping() {
        let ctx = Context::default();
        let rt = crate::test_support::runtime(&ctx);
        rt.set_collect_semantics(true);
        pass(&ctx, &rt, Vec::new(), true);
        let r = rt
            .tree(egui::ViewportId::ROOT)
            .and_then(|t| t.find_key("s-slider").map(|n| n.rect))
            .expect("slider");
        let at = pos2(r.left() + r.width() * 0.97, r.center().y);
        let press = synth::pointer_down(at, PointerButton::Primary);
        pass(&ctx, &rt, vec![Event::PointerMoved(at), press], true);
        pass(&ctx, &rt, vec![Event::PointerMoved(at - vec2(20.0, 0.0))], true);
        pass(&ctx, &rt, vec![Event::PointerMoved(at - vec2(40.0, 0.0))], false);
    }
}
