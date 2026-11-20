use crate::Material;
use crate::MaterialExt;
use crate::MaterialUiExt as _;
use crate::msg;
use egui_mineral::i18n::I18n;
use egui_mineral::kit::state::{Memo, TempState};
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::Composite;
use egui_mineral::widget::MineralUi;
use std::sync::Arc;

use egui::{
    Color32, CornerRadius, CursorIcon, Key, Mesh, Pos2, Rect, Sense, Shape, Stroke, Ui,
    ecolor::{HexColor, Hsva},
    pos2, vec2,
};
use egui_mineral::widget::Availability;
use egui_mineral::{
    kit,
    layout::{Axis, Constraints, Measure, MinTarget},
    range::{RangeModel, Track},
    semantic::{NumericRange, Role, SemanticNode, StableId},
    widget::{MineralWidget, SizeExt, WidgetFrame, WidgetResponse},
};
use material_colors::{color::Rgb, hct::Hct};

use crate::extras::NumberField;
use crate::text_inputs::text_field::FieldChrome;
use crate::text_inputs::{TextField, TextFieldVariant};
use crate::widget::{
    common::{self, Interaction},
    indication, parts,
};
use crate::{
    adaptive::MIN_INTERACTIVE,
    spec::tokens::{divider, outlined_text_field, radio_button, slider, small_icon_button},
    tokens::Tokens,
    tr,
};

// Not an MD3 component: geometry comes from its sibling controls

const HCT_CHROMA: f64 = 120.0;
const GRID: usize = 16;
// Compose TextFieldDefaults.MinWidth
const DEFAULT_WIDTH: f32 = 280.0;
const AREA_ASPECT: f32 = 0.6;
const THUMB: f32 = radio_button::ICON_SIZE / 2.0;
const THUMB_LAYER: f32 = radio_button::STATE_LAYER_SIZE / 2.0;
// White over translucent black reads on any picked color
const THUMB_RING: Color32 = Color32::WHITE;
const THUMB_RING_WIDTH: f32 = 2.0;
const THUMB_OUTLINE_ALPHA: f32 = 0.4;
const SWATCH: f32 = small_icon_button::CONTAINER_HEIGHT;

/// Space the [`ColorPicker`] edits in: HSV or HCT.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum ColorSpace {
    #[default]
    /// Hue, saturation, value.
    Hsv,
    /// Hue, chroma, tone (material-color-utilities `Hct`).
    Hct,
}

impl ColorSpace {
    fn color(self, h: f64, x: f64, y: f64) -> Color32 {
        match self {
            Self::Hsv => {
                let [r, g, b] = Hsva::new((h / 360.0) as f32, x as f32, (1.0 - y) as f32, 1.0).to_srgb();
                Color32::from_rgb(r, g, b)
            }
            Self::Hct => {
                let rgb = Rgb::from(Hct::from(h, x * HCT_CHROMA, (1.0 - y) * 100.0));
                Color32::from_rgb(rgb.red, rgb.green, rgb.blue)
            }
        }
    }

    fn coords(self, c: Color32) -> (f64, f64, f64) {
        match self {
            Self::Hsv => {
                let hsv = Hsva::from_srgb([c.r(), c.g(), c.b()]);
                (f64::from(hsv.h) * 360.0, f64::from(hsv.s), 1.0 - f64::from(hsv.v))
            }
            Self::Hct => {
                let h = Hct::new(Rgb::new(c.r(), c.g(), c.b()));
                (
                    h.get_hue(),
                    (h.get_chroma() / HCT_CHROMA).clamp(0.0, 1.0),
                    1.0 - h.get_tone() / 100.0,
                )
            }
        }
    }

    fn hue_color(self, h: f64) -> Color32 {
        match self {
            Self::Hsv => self.color(h, 1.0, 0.0),
            Self::Hct => self.color(h, 48.0 / HCT_CHROMA, 0.4),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct Coords {
    hue: f64,
    x: f64,
    y: f64,
    last: Color32,
}

impl TempState for Coords {}

/// Color field with area, hue bar, swatches and optional RGB fields; non-MD3 (Ant `ColorPicker` pattern).
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct ColorPicker<'a> {
    salt: KeyedSalt,
    label: String,
    color: &'a mut Color32,
    space: ColorSpace,
    swatches: Vec<Color32>,
    disabled: bool,
    rgb_fields: bool,
    chrome: FieldChrome,
}

impl<'a> ColorPicker<'a> {
    /// `label` names the picker for assistive technology; `color` is the bound value. Agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, color: &'a mut Color32) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            label: label.into(),
            color,
            space: ColorSpace::default(),
            swatches: Vec::new(),
            disabled: false,
            rgb_fields: false,
            chrome: FieldChrome {
                variant: TextFieldVariant::Outlined,
                ..FieldChrome::default()
            },
        }
    }

    crate::text_inputs::text_field::field_chrome_setters!(no_icon);
    crate::text_inputs::text_field::field_chrome_setters!(text);

    /// Red, green and blue 0..=255 fields under the hex field, in sync with the color.
    pub fn rgb_fields(mut self, rgb_fields: bool) -> Self {
        self.rgb_fields = rgb_fields;
        self
    }

    /// Plane and hue model; default [`ColorSpace::Hsv`].
    pub fn space(mut self, space: ColorSpace) -> Self {
        self.space = space;
        self
    }

    /// Preset colors shown as clickable swatches.
    pub fn swatches(mut self, swatches: impl IntoIterator<Item = Color32>) -> Self {
        self.swatches = swatches.into_iter().collect();
        self
    }

    crate::widget::setters! {
        /// Greys out the picker and ignores input.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }
}

impl Composite for ColorPicker<'_> {
    type Kit = Material;
    type Output = ();

    egui_mineral::kit::widget_meta!(salt);

    fn compose(&mut self, ui: &mut MineralUi<'_, Material>, constraints: Constraints) -> WidgetResponse {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let width = constraints.constrain(vec2(DEFAULT_WIDTH, 0.0)).x;
        let base = ui.make_persistent_id(self.salt.id_salt());
        let coords_id = base.with(("coords", self.space as u8));
        let hex_id = base.with("hex");
        let before = *self.color;
        let mut co: Coords = Coords::try_load(ctx, coords_id)
            .filter(|c| c.last == *self.color)
            .unwrap_or_else(|| {
                let (hue, x, y) = self.space.coords(*self.color);
                Coords {
                    hue,
                    x,
                    y,
                    last: *self.color,
                }
            });
        let space = self.space;
        let t = rt.tokens();
        let inner = ui.vertical(|ui| self.area_block(&mut rt.ui(ui), &mut co, width));
        let (moved, picked) = inner.inner;
        if moved {
            *self.color = space.color(co.hue, co.x, co.y);
        }
        if let Some(c) = picked {
            *self.color = c;
            (co.hue, co.x, co.y) = space.coords(c);
        }

        let mut hex: String = String::try_load(ctx, hex_id).unwrap_or_else(|| hex6(*self.color));
        let hex_edit_key = base.with("hex-edit");
        let hex_focused = egui::Id::try_load(ctx, hex_edit_key).is_some_and(|id| ctx.memory(|m| m.has_focus(id)));
        if !hex_focused {
            hex = hex6(*self.color);
        }
        let field = TextField::new(self.salt.child("hex"), tr(rt.i18n(), msg::COLOR_HEX), &mut hex)
            .with_chrome(self.chrome.clone())
            .disabled(self.disabled)
            .agent_key(StableId::of(&self.salt, "hex-field"))
            .preferred_width(width - SWATCH - crate::adaptive::grid(4));
        let out = ui
            .horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = crate::adaptive::grid(4);
                let (slot, _) = ui.allocate_exact_size(vec2(SWATCH, 0.0), Sense::hover());
                let out = rt.ui(ui).mineral(field);
                let at = pos2(slot.center().x, out.zones().visual.center().y);
                swatch_circle(ui.painter(), at, *self.color, &t);
                out
            })
            .inner;
        if out.changed()
            && let Some(c) = parse_hex(&hex)
        {
            *self.color = c.to_opaque();
            (co.hue, co.x, co.y) = space.coords(*self.color);
        }
        if self.rgb_fields && self.rgb_row(ui, width) {
            (co.hue, co.x, co.y) = space.coords(*self.color);
        }
        co.last = *self.color;
        co.store(ctx, coords_id);
        hex.store(ctx, hex_id);
        out.response().id.store(ctx, hex_edit_key);
        let changed = *self.color != before;
        let response = inner.response;
        WidgetResponse::new(response, ()).with_changed(changed)
    }
}

impl ColorPicker<'_> {
    fn area_block(&self, ui: &mut MineralUi<'_, Material>, co: &mut Coords, width: f32) -> (bool, Option<Color32>) {
        let rt = ui.rt();
        let space = self.space;
        ui.spacing_mut().item_spacing.y = crate::adaptive::grid(4);
        let area = rt.ui(ui).mineral(Area {
            salt: self.salt.child("area"),
            key: StableId::of(&self.salt, "area"),
            label: &self.label,
            space,
            co,
            width,
            disabled: self.disabled,
        });
        let hue = rt.ui(ui).mineral(HueBar {
            salt: self.salt.child("hue"),
            key: StableId::of(&self.salt, "hue-slider"),
            space,
            hue: &mut co.hue,
            width,
            disabled: self.disabled,
        });
        let mut picked = None;
        if !self.swatches.is_empty() {
            ui.horizontal_wrapped(|ui| {
                ui.set_max_width(width);
                // The 48dp targets touch; the circles keep the target's margin apart.
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                for (i, c) in self.swatches.iter().enumerate() {
                    let r = rt.ui(ui).mineral(Swatch {
                        salt: self.salt.child(("swatch", i)),
                        key: StableId::of(&self.salt, (i, "swatch")),
                        color: *c,
                        selected: *c == *self.color,
                        disabled: self.disabled,
                    });
                    if r.clicked() {
                        picked = Some(*c);
                    }
                }
            });
        }
        (area.changed() || hue.changed(), picked)
    }

    fn rgb_row(&mut self, ui: &mut MineralUi<'_, Material>, width: f32) -> bool {
        let rt = ui.rt();
        let gap = crate::adaptive::grid(2);
        let w = (width - 2.0 * gap) / 3.0;
        let [r, g, b, _] = self.color.to_array();
        let mut channels = [r, g, b].map(f64::from);
        let changed = ui
            .horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                let mut changed = false;
                let names = [
                    ("red", msg::COLOR_RED),
                    ("green", msg::COLOR_GREEN),
                    ("blue", msg::COLOR_BLUE),
                ];
                for ((salt, msg), v) in names.into_iter().zip(channels.iter_mut()) {
                    changed |= rt
                        .ui(ui)
                        .mineral(
                            NumberField::new(
                                self.salt.child(salt),
                                tr(rt.i18n(), msg),
                                RangeModel::new(0.0, 255.0).step(1.0),
                                v,
                            )
                            .with_field_chrome(FieldChrome {
                                variant: self.chrome.variant,
                                text_style: self.chrome.text_style,
                                label_hidden: self.chrome.label_hidden,
                                ..FieldChrome::default()
                            })
                            .step_buttons(false)
                            .scrub(true)
                            .disabled(self.disabled)
                            .agent_key(StableId::of(&self.salt, salt))
                            .preferred_width(w),
                        )
                        .changed();
                }
                changed
            })
            .inner;
        if changed {
            let [r, g, b] = channels.map(channel);
            *self.color = Color32::from_rgb(r, g, b);
        }
        changed
    }
}

// Snapped to the 0..=255 integer grid by its field
fn channel(v: f64) -> u8 {
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "clamped to 0..=255"
    )]
    let c = v.round().clamp(0.0, 255.0) as u8;
    c
}

fn field_mesh(rect: Rect, colors: &[Color32]) -> Mesh {
    let mut m = Mesh::default();
    let n = GRID + 1;
    for j in 0..n {
        for i in 0..n {
            let p = pos2(
                egui::lerp(rect.x_range(), i as f32 / GRID as f32),
                egui::lerp(rect.y_range(), j as f32 / GRID as f32),
            );
            m.colored_vertex(p, colors[j * n + i]);
        }
    }
    for j in 0..GRID {
        for i in 0..GRID {
            let a = (j * n + i) as u32;
            let b = a + 1;
            let c = a + n as u32;
            let d = c + 1;
            m.add_triangle(a, b, c);
            m.add_triangle(b, d, c);
        }
    }
    m
}

fn area_colors(ctx: &egui::Context, area: egui::Id, space: ColorSpace, hue: f64) -> Arc<Vec<Color32>> {
    let key = area.with(("mineral::color_area", space as u8));
    let tag = (hue * 10.0).round() as i64;
    let mut memo = Memo::<Vec<Color32>>::load(ctx, key);
    let colors = memo.get_or_compute(&tag, || {
        let n = GRID + 1;
        (0..n * n)
            .map(|k| space.color(hue, (k % n) as f64 / GRID as f64, (k / n) as f64 / GRID as f64))
            .collect()
    });
    memo.store(ctx, key);
    colors
}

fn thumb(ui: &Ui, r: &egui::Response, at: Pos2, fill: Color32, disabled: bool, t: &Tokens) {
    let i = Interaction::of(r, Availability::disabled_if(disabled));
    let p = ui.painter();
    common::state_layer(
        p,
        Rect::from_center_size(at, egui::Vec2::splat(2.0 * THUMB_LAYER)),
        CornerRadius::from(THUMB_LAYER),
        THUMB_RING,
        i,
        t,
    );
    p.circle_filled(at, THUMB, fill);
    p.circle_stroke(at, THUMB, Stroke::new(THUMB_RING_WIDTH, THUMB_RING));
    p.circle_stroke(
        at,
        THUMB + THUMB_RING_WIDTH / 2.0,
        Stroke::new(
            divider::THICKNESS,
            common::with_alpha(Color32::BLACK, THUMB_OUTLINE_ALPHA),
        ),
    );
}

fn disabled_veil(t: &Tokens) -> Color32 {
    common::with_alpha(t.color.surface, 1.0 - slider::DISABLED_ACTIVE_TRACK_OPACITY)
}

fn gradient(xs: std::ops::RangeInclusive<f32>, y: f32, h: f32, color_at: &dyn Fn(f32) -> Color32) -> Mesh {
    const STEPS: u32 = 36;
    let mut m = Mesh::default();
    for s in 0..=STEPS {
        let x = egui::lerp(xs.clone(), s as f32 / STEPS as f32);
        let c = color_at(x);
        m.colored_vertex(pos2(x, y - h / 2.0), c);
        m.colored_vertex(pos2(x, y + h / 2.0), c);
        if s > 0 {
            let i = 2 * s;
            m.add_triangle(i - 2, i - 1, i);
            m.add_triangle(i - 1, i + 1, i);
        }
    }
    m
}

fn swatch_circle(p: &egui::Painter, at: Pos2, color: Color32, t: &Tokens) {
    p.circle_filled(at, SWATCH / 2.0, color);
    p.circle_stroke(
        at,
        SWATCH / 2.0,
        Stroke::new(divider::THICKNESS, t.role(divider::COLOR)),
    );
}

struct Area<'p> {
    salt: KeyedSalt,
    key: StableId,
    label: &'p str,
    space: ColorSpace,
    co: &'p mut Coords,
    width: f32,
    disabled: bool,
}

impl MineralWidget for Area<'_> {
    type Kit = Material;
    type State = ();
    type Output = ();

    egui_mineral::kit::widget_meta!(salt, enabled);

    fn sense(&self) -> Sense {
        Sense::click_and_drag()
    }

    fn layout(&self, _ui: &MineralUi<'_, Material>, c: Constraints, (): &Self::State) -> Measure {
        c.constrain(vec2(self.width, self.width * AREA_ASPECT)).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Material>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let t = rt.tokens();
        let rect = frame.zones.visual;
        let r = &frame.response;
        let (x0, y0) = (self.co.x, self.co.y);
        // Not mirrored in RTL; y grows downward.
        let (unit, x_track, y_track) = (
            RangeModel::new(0.0, 1.0),
            Track::horizontal_ltr(),
            Track::vertical_down(),
        );
        if !self.disabled {
            let p = frame.drag_pos();
            if let Some(p) = p {
                self.co.x = unit.value_at(rect.x_range(), p.x, x_track);
                self.co.y = unit.value_at(rect.y_range(), p.y, y_track);
                r.request_focus();
            }
            if r.has_focus() {
                kit::interact::lock_arrows(ctx, r.id, kit::interact::Arrows::Both);
                for k in kit::interact::plain_key_presses(ctx) {
                    let (v, track) = match k {
                        Key::ArrowLeft | Key::ArrowRight => (&mut self.co.x, x_track),
                        Key::ArrowUp | Key::ArrowDown => (&mut self.co.y, y_track),
                        _ => continue,
                    };
                    if let Some(next) = unit.on_key(*v, k, track) {
                        *v = next;
                    }
                }
            }
            if r.hovered() {
                ctx.set_cursor_icon(CursorIcon::Crosshair);
            }
        }
        let colors = area_colors(ctx, r.id, self.space, self.co.hue);
        let p = ui.painter();
        p.add(Shape::mesh(field_mesh(rect, &colors)));
        if self.disabled {
            p.rect_filled(rect, 0, disabled_veil(&t));
        }
        let at = pos2(
            unit.position_of(rect.x_range(), self.co.x, x_track),
            unit.position_of(rect.y_range(), self.co.y, y_track),
        );
        thumb(
            ui,
            r,
            at,
            self.space.color(self.co.hue, self.co.x, self.co.y),
            self.disabled,
            &t,
        );
        let changed = (self.co.x - x0).abs() > f64::EPSILON || (self.co.y - y0).abs() > f64::EPSILON;
        WidgetResponse::new(r.clone(), ()).with_changed(changed)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let c = self.space.color(self.co.hue, self.co.x, self.co.y);
        let n = SemanticNode::new(self.key.clone(), Role::ColorWell, self.label)
            .value(hex6(c))
            .clickable(Availability::disabled_if(self.disabled));
        Some(n)
    }
}

struct HueBar<'p> {
    salt: KeyedSalt,
    key: StableId,
    space: ColorSpace,
    hue: &'p mut f64,
    width: f32,
    disabled: bool,
}

impl MineralWidget for HueBar<'_> {
    type Kit = Material;
    type State = ();
    type Output = ();

    egui_mineral::kit::widget_meta!(salt, enabled);

    fn sense(&self) -> Sense {
        Sense::click_and_drag()
    }

    fn pan_axis(&self) -> Option<Axis> {
        Some(Axis::Horizontal)
    }

    fn min_target(&self, _: &MineralUi<'_, Self::Kit>) -> MinTarget {
        MIN_INTERACTIVE
    }

    fn layout(&self, ui: &MineralUi<'_, Material>, c: Constraints, (): &Self::State) -> Measure {
        let handle = ui.density().dense_to(
            slider::HANDLE_HEIGHT,
            slider::INACTIVE_TRACK_HEIGHT + crate::adaptive::CONTENT_PAD,
        );
        c.constrain(vec2(self.width, handle)).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Material>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let rt = ui.rt();
        let t = rt.tokens();
        let dir = rt.i18n().direction();
        let rect = frame.zones.visual;
        let r = &frame.response;
        let range = RangeModel::new(0.0, 360.0).step(1.0).jump(30.0);
        let hue_track = Track::horizontal(dir);
        let track_h = slider::INACTIVE_TRACK_HEIGHT;
        // The handle centers on the rounded track ends at the extremes.
        let track = rect.x_range().shrink(track_h / 2.0);
        let before = *self.hue;
        if !self.disabled {
            if let Some(p) = frame.drag_pos() {
                *self.hue = range.value_at(track, p.x, hue_track);
                r.request_focus();
            }
            *self.hue = range.drive(rt.core(), r.id, *self.hue, hue_track);
        }
        let i = Interaction::of(r, Availability::disabled_if(self.disabled));
        let x = range.position_of(track, *self.hue, hue_track);
        let handle_w = if i.pressed || i.focused {
            slider::PRESSED_HANDLE_WIDTH
        } else {
            slider::HANDLE_WIDTH
        };
        let gap = handle_w / 2.0 + slider::ACTIVE_HANDLE_PADDING;
        let color_at = |x: f32| self.space.hue_color(range.value_at(track, x, hue_track));
        let (y, p) = (rect.center().y, ui.painter());
        let (start, end) = (track.min, track.max);
        // Rounded track ends, then the gradient on both sides of the handle gap.
        for (cap, open) in [(start, x - gap > start), (end, x + gap < end)] {
            if open {
                p.circle_filled(pos2(cap, y), track_h / 2.0, color_at(cap));
            }
        }
        for (a, b) in [(start, x - gap), (x + gap, end)] {
            if b > a {
                p.add(Shape::mesh(gradient(a..=b, y, track_h, &color_at)));
            }
        }
        if self.disabled {
            p.rect_filled(
                Rect::from_x_y_ranges(rect.x_range(), y - track_h / 2.0..=y + track_h / 2.0),
                0,
                disabled_veil(&t),
            );
        }
        let handle_color = if self.disabled {
            t.role_alpha(slider::DISABLED_HANDLE_COLOR, slider::DISABLED_HANDLE_OPACITY)
        } else {
            t.role(slider::HANDLE_COLOR)
        };
        let handle = Rect::from_center_size(pos2(x, y), vec2(handle_w, rect.height()));
        p.rect_filled(handle, CornerRadius::from(handle_w / 2.0), handle_color);
        kit::interact::hover_cursor(rt.core(), r, Availability::disabled_if(self.disabled));
        WidgetResponse::new(r.clone(), ()).with_changed((*self.hue - before).abs() > f64::EPSILON)
    }

    fn semantics(&self, i18n: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let name = crate::tr(i18n, msg::COLOR_HUE);
        let range = NumericRange {
            value: *self.hue,
            min: 0.0,
            max: 360.0,
            step: Some(1.0),
        };
        let n = SemanticNode::new(self.key.clone(), Role::Slider, &name)
            .value(i18n.format().integer(self.hue.round() as i64))
            .ranged(range, Availability::disabled_if(self.disabled));
        Some(n)
    }
}

struct Swatch {
    salt: KeyedSalt,
    key: StableId,
    color: Color32,
    selected: bool,
    disabled: bool,
}

impl MineralWidget for Swatch {
    type Kit = Material;
    type State = ();
    type Output = ();

    egui_mineral::kit::widget_meta!(salt, enabled);

    fn min_target(&self, _: &MineralUi<'_, Self::Kit>) -> MinTarget {
        MIN_INTERACTIVE
    }

    fn layout(&self, _ui: &MineralUi<'_, Material>, c: Constraints, (): &Self::State) -> Measure {
        c.constrain(vec2(SWATCH, SWATCH)).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Material>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let r = &frame.response;
        let rect = frame.zones.visual;
        let i = parts::press(ui, frame, Availability::disabled_if(self.disabled)).interaction;
        swatch_circle(ui.painter(), rect.center(), self.color, &t);
        let round = CornerRadius::from(SWATCH / 2.0);
        indication::paint(ui, r, rect, round, t.color.on_surface, i, &t);
        if self.selected {
            // Outlined like a focused outlined text field, in the target margin.
            let w = outlined_text_field::FOCUS_OUTLINE_WIDTH;
            ui.painter()
                .circle_stroke(rect.center(), SWATCH / 2.0 + w, Stroke::new(w, t.color.primary));
        }
        WidgetResponse::new(r.clone(), ())
    }

    fn semantics(&self, i18n: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let hex = hex6(self.color);
        let name = i18n.tr(msg::ColorSwatch {
            hex: hex.as_str().into(),
        });
        let mut n =
            SemanticNode::new(self.key.clone(), Role::Button, name).clickable(Availability::disabled_if(self.disabled));
        n.state.selected = Some(self.selected);
        Some(n)
    }
}

fn hex6(c: Color32) -> String {
    HexColor::Hex6(c).to_string()
}

fn parse_hex(s: &str) -> Option<Color32> {
    let s = s.trim();
    HexColor::from_str_without_hash(s.strip_prefix('#').unwrap_or(s))
        .ok()
        .map(|h| h.color())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use egui::{Context, Id};

    use super::{ColorSpace, area_colors};

    #[test]
    fn pickers_in_one_space_keep_their_own_area_cache() {
        let ctx = Context::default();
        let (a, b) = (Id::unique("a"), Id::unique("b"));
        let first = area_colors(&ctx, a, ColorSpace::Hsv, 10.0);
        let other = area_colors(&ctx, b, ColorSpace::Hsv, 200.0);
        assert!(Arc::ptr_eq(&first, &area_colors(&ctx, a, ColorSpace::Hsv, 10.0)));
        assert!(Arc::ptr_eq(&other, &area_colors(&ctx, b, ColorSpace::Hsv, 200.0)));
    }
}
