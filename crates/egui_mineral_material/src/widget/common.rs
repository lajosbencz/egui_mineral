use egui::{Color32, CornerRadius, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, epaint::TextShape};

use egui_mineral::anchor::{Align, Anchor, Side};
use egui_mineral::icon::IconStyle;
use egui_mineral::widget::InteractionKind;

use crate::{Icon, spec::ElevationLevel, tokens::Tokens};

pub(crate) use egui_mineral::widget::Interaction;

// MD3 state layer opacity
pub(crate) trait StateLayer {
    fn layer_opacity(self, t: &Tokens) -> f32;
}

impl StateLayer for Interaction {
    fn layer_opacity(self, t: &Tokens) -> f32 {
        let s = &t.state;
        self.dominant().map_or(0.0, |kind| match kind {
            InteractionKind::Dragged => s.dragged,
            InteractionKind::Pressed => s.pressed,
            InteractionKind::Focused => s.focus,
            InteractionKind::Hovered => s.hover,
        })
    }
}

// `rest`, `pressed` and `disabled` default to the (PRESSED_/DISABLED_)CONTAINER_ELEVATION tokens.
macro_rules! elevation_tokens {
    ($m:ident; hovered: $hovered:ident, focused: $focused:ident) => {
        $crate::widget::common::elevation_tokens!(
            $m;
            rest: CONTAINER_ELEVATION,
            hovered: $hovered,
            focused: $focused,
            pressed: PRESSED_CONTAINER_ELEVATION,
            disabled: DISABLED_CONTAINER_ELEVATION
        )
    };
    (
        $m:ident;
        rest: $rest:ident,
        hovered: $hovered:ident,
        focused: $focused:ident,
        pressed: $pressed:ident,
        disabled: $disabled:ident
    ) => {
        $crate::actions::button::Elevation {
            rest: $m::$rest,
            hovered: $m::$hovered,
            focused: $m::$focused,
            pressed: $m::$pressed,
            disabled: $m::$disabled,
        }
    };
}
pub(crate) use elevation_tokens;

// No spec token: free space kept between anchored popups and the viewport edge.
const POPUP_VIEWPORT_MARGIN: f32 = 8.0;

pub(crate) fn popup_anchor(side: Side, align: Align) -> Anchor {
    Anchor::new(side, align).padding(POPUP_VIEWPORT_MARGIN)
}

pub(crate) fn with_alpha(c: Color32, a: f32) -> Color32 {
    c.gamma_multiply(a)
}

pub(crate) fn dim(c: Color32, t: &Tokens) -> Color32 {
    with_alpha(c, t.state.disabled_content)
}

pub(crate) fn disabled_content(t: &Tokens) -> Color32 {
    dim(t.color.on_surface, t)
}

pub(crate) fn disabled_container(t: &Tokens) -> Color32 {
    with_alpha(t.color.on_surface, t.state.disabled_container)
}

pub(crate) fn radius(rect: Rect, r: u8) -> CornerRadius {
    CornerRadius::from(f32::from(r).min(rect.height().min(rect.width()) / 2.0))
}

pub(crate) fn state_layer(
    p: &Painter,
    rect: Rect,
    r: impl Into<CornerRadius>,
    content: Color32,
    i: Interaction,
    t: &Tokens,
) {
    let o = i.layer_opacity(t);
    if o > 0.0 {
        p.rect_filled(rect, r, with_alpha(content, o));
    }
}

pub(crate) fn state_focus(
    p: &Painter,
    rect: Rect,
    r: impl Into<CornerRadius>,
    content: Color32,
    i: Interaction,
    t: &Tokens,
) {
    state_focus_inset(p, rect, r, content, i, t, 0.0);
}

pub(crate) fn state_focus_inset(
    p: &Painter,
    rect: Rect,
    r: impl Into<CornerRadius>,
    content: Color32,
    i: Interaction,
    t: &Tokens,
    inset: f32,
) {
    let r = r.into();
    state_layer(p, rect, r, content, i, t);
    if i.focused {
        focus_ring(p, rect.shrink(inset), r, t);
    }
}

pub(crate) fn elevation_shape(
    rect: Rect,
    r: impl Into<CornerRadius>,
    level: ElevationLevel,
    t: &Tokens,
) -> Option<Shape> {
    (level != ElevationLevel::Level0).then(|| t.elevation_shadow(level).as_shape(rect, r).into())
}

pub(crate) fn elevated_frame<R>(
    ui: &mut egui::Ui,
    level: ElevationLevel,
    t: &Tokens,
    frame: egui::Frame,
    add: impl FnOnce(&mut egui::Ui) -> R,
) -> egui::InnerResponse<R> {
    let shadow = ui.painter().add(Shape::Noop);
    let framed = frame.show(ui, add);
    if let Some(s) = elevation_shape(framed.response.rect, frame.corner_radius, level, t) {
        ui.painter().set(shadow, s);
    }
    framed
}

pub(crate) fn elevation(p: &Painter, rect: Rect, r: impl Into<CornerRadius>, level: ElevationLevel, t: &Tokens) {
    if let Some(s) = elevation_shape(rect, r, level, t) {
        p.add(s);
    }
}

pub(crate) fn selection_icon(icon: Icon, selected: bool) -> Icon {
    Icon::single(icon.glyph(IconStyle::filled_if(selected)))
}

// Filled form: the Material default.
pub(crate) fn icon(p: &Painter, center: Pos2, icon: Icon, size: f32, color: Color32) {
    p.add(icon_shape(p, center, icon, size, color, 0.0));
}

pub(crate) fn icon_shape(p: &Painter, center: Pos2, icon: Icon, size: f32, color: Color32, angle: f32) -> TextShape {
    egui_mineral::kit::icon::shape(p.ctx(), center, icon.filled(), size, color, angle)
}

const FOCUS_RING_OFFSET: f32 = 2.0;
const FOCUS_RING_WIDTH: f32 = 3.0;

pub(crate) fn focus_ring(p: &Painter, rect: Rect, r: impl Into<CornerRadius>, t: &Tokens) {
    p.rect_stroke(
        rect.expand(FOCUS_RING_OFFSET),
        r,
        Stroke::new(FOCUS_RING_WIDTH, t.color.secondary),
        StrokeKind::Outside,
    );
}
