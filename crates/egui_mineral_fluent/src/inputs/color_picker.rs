use egui::{
    Color32, CornerRadius, Key, Mesh, Modifiers, Pos2, Rect, Sense, Shape, Stroke, StrokeKind, ecolor::Hsva, vec2,
};
use egui_mineral::{
    i18n::I18n,
    kit,
    layout::{Axis, Constraints, Measure, MinTarget},
    semantic::{AgentKey, KeyedSalt, NumericRange, Role, SemanticNode},
    widget::{Availability, Composite, Interaction, MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::ColorToken::{
    ColorNeutralBackground1, ColorNeutralForeground4, ColorNeutralStroke1, ColorStrokeFocus2, ColorTransparentStroke,
};
use crate::{
    Fluent, FluentExt,
    spec::{
        defaults::color_picker as cp,
        tokens::geometry::{
            BORDER_RADIUS_CIRCULAR, BORDER_RADIUS_MEDIUM, BORDER_RADIUS_NONE, SPACING_VERTICAL_XS, STROKE_WIDTH_THICK,
            STROKE_WIDTH_THIN,
        },
    },
    theme::{Elevation, Tokens},
    widget::paint,
};

/// `shape` of the color area and sliders.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ColorShape {
    /// `rounded`: `borderRadiusMedium`.
    #[default]
    Rounded,
    /// `square`: no rounding.
    Square,
}

impl ColorShape {
    /// Every variant.
    pub const ALL: &'static [Self] = &[Self::Rounded, Self::Square];

    fn radius(self) -> CornerRadius {
        CornerRadius::from(match self {
            Self::Rounded => BORDER_RADIUS_MEDIUM,
            Self::Square => BORDER_RADIUS_NONE,
        })
    }
}

/// Fluent `ColorPicker`: a `ColorArea` for saturation and value, a hue `ColorSlider` and, optionally, an
/// `AlphaSlider`, bound to one HSVA color.
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct ColorPicker<'a> {
    salt: KeyedSalt,
    label: String,
    value: &'a mut Hsva,
    shape: ColorShape,
    alpha: bool,
    disabled: bool,
}

impl<'a> ColorPicker<'a> {
    /// `label` prefixes the names of the parts. Agent keys `<id_salt>-color-area`, `-hue-slider`, `-alpha-slider`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, value: &'a mut Hsva) -> Self {
        Self {
            salt: id_salt.into(),
            label: label.into(),
            value,
            shape: ColorShape::Rounded,
            alpha: false,
            disabled: false,
        }
    }

    /// Default [`ColorShape::Rounded`].
    pub fn shape(mut self, shape: ColorShape) -> Self {
        self.shape = shape;
        self
    }

    /// Adds the `AlphaSlider`.
    pub fn alpha(mut self, alpha: bool) -> Self {
        self.alpha = alpha;
        self
    }

    crate::widget::setters! {
        /// Ignores input.
        disabled
    }
}

impl Composite for ColorPicker<'_> {
    type Kit = Fluent;
    type Output = ();

    kit::widget_meta!(salt);

    fn compose(&mut self, ui: &mut MineralUi<'_, Fluent>, _: Constraints) -> WidgetResponse<()> {
        ui.spacing_mut().item_spacing.y = SPACING_VERTICAL_XS;
        let mut v = *self.value;
        let part = |kind, name: &str| Part {
            salt: self.salt.child(kind),
            picker: self.salt.clone(),
            kind,
            name: format!("{} {name}", self.label),
            shape: self.shape,
            disabled: self.disabled,
        };
        let area = ui.mineral(part("color-area", "saturation and value").area(&mut v));
        let hue = ui.mineral(part("hue-slider", "hue").channel(&mut v, Channel::Hue));
        let alpha = self
            .alpha
            .then(|| ui.mineral(part("alpha-slider", "alpha").channel(&mut v, Channel::Alpha)));
        let changed = area.changed() || hue.changed() || alpha.as_ref().is_some_and(WidgetResponse::changed);
        if changed {
            *self.value = v;
        }
        WidgetResponse::new(area.into_response(), ()).with_changed(changed)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Channel {
    Hue,
    Alpha,
}

struct Part {
    salt: KeyedSalt,
    picker: KeyedSalt,
    kind: &'static str,
    name: String,
    shape: ColorShape,
    disabled: bool,
}

impl Part {
    fn area(self, value: &mut Hsva) -> Handle<'_> {
        Handle {
            part: self,
            value,
            channel: None,
        }
    }

    fn channel(self, value: &mut Hsva, channel: Channel) -> Handle<'_> {
        Handle {
            part: self,
            value,
            channel: Some(channel),
        }
    }
}

struct Handle<'v> {
    part: Part,
    value: &'v mut Hsva,
    channel: Option<Channel>,
}

// Rail and thumb centers sit inside the rail by half a thumb.
fn track(rect: Rect) -> Rect {
    rect.shrink2(vec2(cp::THUMB_SIZE / 2.0, 0.0))
}

impl Handle<'_> {
    fn pointer(&self, rect: Rect, pos: Pos2, v: &mut Hsva) {
        match self.channel {
            None => {
                let a = rect;
                v.s = ((pos.x - a.left()) / a.width()).clamp(0.0, 1.0);
                v.v = (1.0 - (pos.y - a.top()) / a.height()).clamp(0.0, 1.0);
            }
            Some(c) => {
                let t = track(rect);
                let k = ((pos.x - t.left()) / t.width().max(1.0)).clamp(0.0, 1.0);
                match c {
                    Channel::Hue => v.h = k,
                    Channel::Alpha => v.a = k,
                }
            }
        }
    }

    // `ColorArea` / `ColorSlider` `onKeyDown`: arrows step `1`, Shift `10` percent.
    fn keys(&self, ctx: &egui::Context, v: &mut Hsva) -> bool {
        let mut moved = false;
        for (mods, step) in [(Modifiers::SHIFT, cp::KEY_STEP_LARGE), (Modifiers::NONE, cp::KEY_STEP)] {
            for (key, dx, dy) in [
                (Key::ArrowRight, 1.0, 0.0),
                (Key::ArrowLeft, -1.0, 0.0),
                (Key::ArrowUp, 0.0, 1.0),
                (Key::ArrowDown, 0.0, -1.0),
            ] {
                if !ctx.input_mut(|i| i.consume_key(mods, key)) {
                    continue;
                }
                moved = true;
                match self.channel {
                    None => {
                        v.s = (v.s + dx * step).clamp(0.0, 1.0);
                        v.v = (v.v + dy * step).clamp(0.0, 1.0);
                    }
                    Some(Channel::Hue) => v.h = (v.h + (dx + dy) * step).clamp(0.0, 1.0),
                    Some(Channel::Alpha) => v.a = (v.a + (dx + dy) * step).clamp(0.0, 1.0),
                }
            }
        }
        moved
    }

    fn progress(&self) -> f32 {
        match self.channel {
            None => self.value.s,
            Some(Channel::Hue) => self.value.h,
            Some(Channel::Alpha) => self.value.a,
        }
    }

    fn thumb_color(&self) -> Color32 {
        let opaque = Hsva { a: 1.0, ..*self.value };
        match self.channel {
            Some(Channel::Hue) => Hsva::new(self.value.h, 1.0, 1.0, 1.0).into(),
            _ => Color32::from(opaque),
        }
    }
}

impl MineralWidget for Handle<'_> {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    fn id_salt(&self) -> egui::IdSalt {
        self.part.salt.id_salt()
    }

    fn enabled(&self) -> bool {
        !self.part.disabled
    }

    fn sense(&self) -> Sense {
        Sense::click_and_drag()
    }

    fn pan_axis(&self) -> Option<Axis> {
        self.channel.map(|_| Axis::Horizontal)
    }

    fn min_target(&self) -> MinTarget {
        MinTarget::None
    }

    fn layout(&self, _: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        let size = match self.channel {
            None => vec2(cp::AREA_MIN, cp::AREA_MIN),
            Some(_) => vec2(cp::AREA_MIN, cp::SLIDER_MIN_HEIGHT),
        };
        c.constrain(size).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let r = &frame.response;
        let rect = frame.zones.visual;
        let mut v = *self.value;
        let mut changed = false;
        if !self.part.disabled {
            if let Some(pos) = r
                .interact_pointer_pos()
                .filter(|_| if frame.pan.active() { r.dragged() } else { r.is_pointer_button_down_on() } || r.clicked())
            {
                self.pointer(rect, pos, &mut v);
                changed = true;
            }
            if r.has_focus() {
                changed |= self.keys(ui.ctx(), &mut v);
            }
        }
        let changed = changed && v != *self.value;
        if changed {
            *self.value = v;
        }
        let i = Interaction::of(r, Availability::disabled_if(self.part.disabled));
        self.paint(ui, rect, i, &t);
        WidgetResponse::new(r.clone(), ()).with_changed(changed)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let v = *self.value;
        let (value, text) = match self.channel {
            None => (
                f64::from(v.s) * 100.0,
                format!("{:.0}%, {:.0}%", v.s * 100.0, v.v * 100.0),
            ),
            Some(Channel::Hue) => (f64::from(v.h) * 360.0, format!("{:.0}", v.h * 360.0)),
            Some(Channel::Alpha) => (f64::from(v.a) * 100.0, format!("{:.0}%", v.a * 100.0)),
        };
        let max = if self.channel == Some(Channel::Hue) {
            360.0
        } else {
            100.0
        };
        Some(
            SemanticNode::new(
                AgentKey::Kind(self.part.kind).id(&self.part.picker),
                Role::Slider,
                &self.part.name,
            )
            .value(text)
            .ranged(
                NumericRange {
                    value,
                    min: 0.0,
                    max,
                    step: Some(1.0),
                },
                Availability::disabled_if(self.part.disabled),
            ),
        )
    }
}

impl Handle<'_> {
    fn paint(&self, ui: &MineralUi<'_, Fluent>, rect: Rect, i: Interaction, t: &Tokens) {
        let p = ui.painter();
        let radius = self.part.shape.radius();
        let v = *self.value;
        let thumb_center = if self.channel.is_none() {
            Pos2::new(rect.left() + rect.width() * v.s, rect.bottom() - rect.height() * v.v)
        } else {
            let tr = track(rect);
            Pos2::new(tr.left() + tr.width() * self.progress(), rect.center().y)
        };
        match self.channel {
            None => {
                let hue: Color32 = Hsva::new(v.h, 1.0, 1.0, 1.0).into();
                p.add(corners(rect, Color32::WHITE, hue, Color32::BLACK, Color32::BLACK));
                p.rect_stroke(
                    rect,
                    radius,
                    Stroke::new(STROKE_WIDTH_THIN, t.color(ColorNeutralStroke1)),
                    StrokeKind::Inside,
                );
            }
            Some(c) => {
                let rail = Rect::from_center_size(rect.center(), vec2(rect.width(), cp::RAIL_SIZE));
                match c {
                    Channel::Hue => {
                        for (k, pair) in cp::HUE_STOPS.windows(2).enumerate() {
                            let w = rail.width() / (cp::HUE_STOPS.len() - 1) as f32;
                            let seg = Rect::from_min_size(
                                Pos2::new(rail.left() + w * k as f32, rail.top()),
                                vec2(w, rail.height()),
                            );
                            let [a, b] = [pair[0], pair[1]].map(|[r, g, bl]| Color32::from_rgb(r, g, bl));
                            p.add(corners(seg, a, b, a, b));
                        }
                    }
                    Channel::Alpha => {
                        checkerboard(p, rail);
                        let solid: Color32 = Hsva { a: 1.0, ..v }.into();
                        p.add(corners(rail, Color32::TRANSPARENT, solid, Color32::TRANSPARENT, solid));
                        p.rect_stroke(
                            rail,
                            radius,
                            Stroke::new(STROKE_WIDTH_THIN, t.color(ColorNeutralStroke1)),
                            StrokeKind::Inside,
                        );
                    }
                }
                p.rect_stroke(
                    rail,
                    radius,
                    Stroke::new(STROKE_WIDTH_THIN, t.color(ColorTransparentStroke)),
                    StrokeKind::Outside,
                );
            }
        }
        // useThumbStyles: `strokeWidthThin` `colorNeutralForeground4` border, `shadow4`, `strokeWidthThick` ring.
        let thumb = Rect::from_center_size(thumb_center, vec2(cp::THUMB_SIZE, cp::THUMB_SIZE));
        let circle = CornerRadius::from(BORDER_RADIUS_CIRCULAR);
        paint::shadow(p, thumb, circle, Elevation::E4, t);
        let fill = if self.channel == Some(Channel::Alpha) {
            t.color(ColorNeutralBackground1)
        } else {
            self.thumb_color()
        };
        p.rect_filled(thumb, circle, fill);
        p.rect_stroke(
            thumb,
            circle,
            Stroke::new(STROKE_WIDTH_THIN, t.color(ColorNeutralForeground4)),
            StrokeKind::Inside,
        );
        let ring = Stroke::new(STROKE_WIDTH_THICK, t.color(ColorNeutralBackground1));
        if self.channel == Some(Channel::Alpha) {
            p.rect_filled(
                thumb.shrink(STROKE_WIDTH_THICK),
                circle,
                Color32::from(Hsva { a: v.a, ..v }),
            );
        }
        p.rect_stroke(thumb.shrink(STROKE_WIDTH_THIN), circle, ring, StrokeKind::Inside);
        if i.focused {
            p.rect_stroke(
                thumb,
                circle,
                Stroke::new(STROKE_WIDTH_THICK, t.color(ColorStrokeFocus2)),
                StrokeKind::Outside,
            );
        }
    }
}

// Quad with a color per corner, bilinearly interpolated: top-left, top-right, bottom-left, bottom-right.
fn corners(rect: Rect, tl: Color32, tr: Color32, bl: Color32, br: Color32) -> Shape {
    let mut mesh = Mesh::default();
    let white = egui::epaint::WHITE_UV;
    for (pos, color) in [
        (rect.left_top(), tl),
        (rect.right_top(), tr),
        (rect.left_bottom(), bl),
        (rect.right_bottom(), br),
    ] {
        mesh.vertices.push(egui::epaint::Vertex { pos, uv: white, color });
    }
    mesh.indices.extend([0, 1, 2, 1, 2, 3]);
    Shape::mesh(mesh)
}

fn checkerboard(p: &egui::Painter, rect: Rect) {
    let cell = cp::CHECKER_CELL;
    p.rect_filled(rect, 0.0, Color32::WHITE);
    let clipped = p.with_clip_rect(rect);
    let mut y = rect.top();
    let mut row = 0_u32;
    while y < rect.bottom() {
        let mut x = rect.left();
        let mut col = 0_u32;
        while x < rect.right() {
            if (row + col) % 2 == 1 {
                let cell_rect = Rect::from_min_size(Pos2::new(x, y), vec2(cell, cell));
                clipped.rect_filled(cell_rect, 0.0, Color32::from_gray(0xcc));
            }
            x += cell;
            col += 1;
        }
        y += cell;
        row += 1;
    }
}
