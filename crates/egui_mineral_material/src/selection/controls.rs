use crate::Material;
use crate::MaterialExt;
use crate::MaterialUiExt as _;
use crate::Rt;
use crate::adaptive::Density;
use egui_mineral::i18n::I18n;
use egui_mineral::i18n::Sides;
use egui_mineral::kit::state::TempState;
use egui_mineral::semantic::AgentKey;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;
use std::sync::Arc;

use egui::{
    Color32, CornerRadius, Galley, Id, Pos2, Rangef, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2,
    epaint::MarginF32, vec2,
};
use egui_mineral::widget::Availability;
use egui_mineral::{
    focus::{self, RovingFocus},
    i18n::Direction,
    kit,
    layout::{Axis, Constraints, Measure, MinTarget},
    select::Check,
    semantic::{self, Role, SemanticNode, StableId},
    shortcut::Mnemonic,
    widget::{MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::{
    Icon,
    color::ColorScheme,
    motion,
    spec::{
        ColorRole, TypeScale,
        defaults::{checkbox as cbx, radio_button as rbx},
        tokens::{checkbox as cb, radio_button as rb, switch as sw},
    },
    tokens::Tokens,
};
use crate::{
    SelectionResponse,
    widget::{
        common::{self, Interaction, with_alpha},
        indication, parts,
    },
};

// Compose LocalContentColor
const RIPPLE_COLOR: ColorRole = ColorRole::OnSurface;
const LABEL_COLOR: ColorRole = ColorRole::OnSurface;

fn label_color(t: &Tokens, enabled: bool, disabled_opacity: f32) -> Color32 {
    let c = t.role(LABEL_COLOR);
    if enabled { c } else { with_alpha(c, disabled_opacity) }
}

// Compose CheckboxWithTextSample
struct ControlRow {
    canvas: Vec2,
    label: Option<Arc<Galley>>,
    dir: Direction,
}

impl ControlRow {
    // The label wraps to fit a `width` wide row.
    fn new(
        ui: &MineralUi<'_, Material>,
        canvas: Vec2,
        label: Option<&str>,
        mnemonic: Option<Mnemonic>,
        color: Color32,
        width: f32,
    ) -> Self {
        let rt = ui.rt();
        let label = label.map(|l| {
            let format = cbx::LABEL_FONT.format(ui.ctx(), color);
            let mut job = parts::format_job(rt.core(), l, &format, mnemonic);
            job.wrap.max_width = (width - canvas.x - cbx::LABEL_SPACE).max(0.0);
            ui.painter().layout_job(job)
        });
        Self {
            canvas,
            label,
            dir: rt.i18n().direction(),
        }
    }

    // Space above the label and above the canvas: both center on the first label line.
    fn insets(&self) -> (f32, f32) {
        let first = self
            .label
            .as_ref()
            .and_then(|g| g.rows.first())
            .map_or(self.canvas.y, |r| r.rect().height());
        (
            ((self.canvas.y - first) / 2.0).max(0.0),
            ((first - self.canvas.y) / 2.0).max(0.0),
        )
    }

    fn size(&self) -> Vec2 {
        self.label.as_ref().map_or(self.canvas, |g| {
            let (text_top, _) = self.insets();
            vec2(
                self.canvas.x + cbx::LABEL_SPACE + g.size().x,
                self.canvas.y.max(text_top + g.size().y),
            )
        })
    }

    // Never below the canvas: the spec sizes do not shrink (docs/divergences.md).
    fn measure(&self, size: Vec2, overflow: f32) -> Measure {
        let m = Measure::from(size.max(self.canvas));
        if self.label.is_none() {
            return m;
        }
        let reserve = overflow.max(0.0);
        let Sides { left, right } = self.dir.sides(reserve, 0.0);
        m.margin(MarginF32 {
            left,
            right,
            ..MarginF32::ZERO
        })
    }

    fn row(&self, visual: Rect) -> Rect {
        Rect::from_min_size(visual.min, vec2(visual.width(), self.size().y))
    }

    fn canvas(&self, visual: Rect) -> Rect {
        let row = self.row(visual);
        let x = self.dir.start_x(row, Rangef::new(0.0, 0.0 + self.canvas.x));
        Rect::from_min_size(egui::pos2(x, row.top() + self.insets().1), self.canvas)
    }

    fn text_x(&self, visual: Rect, w: f32) -> f32 {
        let from = self.canvas.x + cbx::LABEL_SPACE;
        self.dir.start_x(self.row(visual), Rangef::new(from, from + w))
    }

    fn paint_label(&self, ui: &Ui, visual: Rect) {
        if let Some(g) = &self.label {
            let x = self.text_x(visual, g.size().x);
            let y = self.row(visual).top() + self.insets().0;
            ui.painter()
                .galley(egui::pos2(x, y), Arc::clone(g), Color32::PLACEHOLDER);
        }
    }
}

// Re-wrapping a label at its own measured width must not break it again.
const WRAP_SLACK: f32 = 0.5;

// Compose CheckboxImpl, RadioButton, Switch
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Control {
    Checkbox,
    Radio,
    Switch,
}

impl Control {
    pub(crate) fn canvas(self) -> Vec2 {
        match self {
            Self::Checkbox => Vec2::splat(cbx::BOX_SIZE + 2.0 * cbx::PADDING),
            Self::Radio => Vec2::splat(rb::ICON_SIZE + 2.0 * rbx::PADDING),
            Self::Switch => vec2(sw::TRACK_WIDTH, sw::TRACK_HEIGHT),
        }
    }

    pub(crate) fn layer(self, density: Density) -> f32 {
        let (size, content) = match self {
            Self::Checkbox => (cb::STATE_LAYER_SIZE, cbx::BOX_SIZE),
            Self::Radio => (rb::STATE_LAYER_SIZE, rb::ICON_SIZE),
            Self::Switch => (sw::STATE_LAYER_SIZE, sw::TRACK_HEIGHT),
        };
        density.dense_to(size, content)
    }

    // Compose minimumInteractiveComponentSize
    pub(crate) fn node(self, rt: &Rt, center: Pos2) -> Rect {
        Rect::from_center_size(
            center,
            self.canvas()
                .max(Vec2::splat(crate::adaptive::MIN_INTERACTIVE.points(rt.core()))),
        )
    }

    pub(crate) fn indication(
        self,
        ui: &MineralUi<'_, Material>,
        response: &Response,
        node: Rect,
        disabled: bool,
        t: &Tokens,
    ) {
        indication::paint_unbounded(
            ui,
            response,
            node,
            self.layer(ui.density()) / 2.0,
            t.role(RIPPLE_COLOR),
            Interaction::of(response, Availability::disabled_if(disabled)),
            t,
        );
    }
}

// Compose rememberUpdatedState
fn color_fraction(rt: &crate::Rt, id: Id, on: bool, enabled: bool, spring: egui_mineral::motion::Spring) -> f32 {
    if enabled {
        egui_mineral::motion::spring(rt.core(), id, f32::from(on), spring).clamp(0.0, 1.0)
    } else {
        let target = f32::from(u8::from(on));
        egui_mineral::motion::snap(rt.ctx(), id, target);
        target
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct CheckMotion {
    shown: Option<Check>,
    off_at: f64,
    held: [f32; 2],
}

impl TempState for CheckMotion {}

#[derive(Clone, Copy)]
struct CheckColors {
    fill: Color32,
    border: Color32,
    mark: Color32,
}

impl CheckColors {
    fn of(on: bool, error: bool, enabled: bool, c: &ColorScheme) -> Self {
        let r = |role: ColorRole| role.resolve(c);
        let none = Color32::TRANSPARENT;
        match (on, enabled) {
            (true, true) => {
                let fill = r(if error {
                    cb::SELECTED_ERROR_CONTAINER_COLOR
                } else {
                    cb::SELECTED_CONTAINER_COLOR
                });
                Self {
                    fill,
                    border: fill,
                    mark: r(if error {
                        cb::SELECTED_ERROR_ICON_COLOR
                    } else {
                        cb::SELECTED_ICON_COLOR
                    }),
                }
            }
            (false, true) => Self {
                fill: none,
                border: r(if error {
                    cb::UNSELECTED_ERROR_OUTLINE_COLOR
                } else {
                    cb::UNSELECTED_OUTLINE_COLOR
                }),
                mark: none,
            },
            (true, false) => {
                let fill = with_alpha(
                    r(cb::SELECTED_DISABLED_CONTAINER_COLOR),
                    cb::SELECTED_DISABLED_CONTAINER_OPACITY,
                );
                Self {
                    fill,
                    border: fill,
                    mark: r(cb::SELECTED_ICON_COLOR),
                }
            }
            (false, false) => Self {
                fill: none,
                border: with_alpha(
                    r(cb::UNSELECTED_DISABLED_OUTLINE_COLOR),
                    cb::UNSELECTED_DISABLED_CONTAINER_OPACITY,
                ),
                mark: none,
            },
        }
    }

    fn lerp(self, to: Self, k: f32) -> Self {
        Self {
            fill: self.fill.lerp_to_gamma(to.fill, k),
            border: self.border.lerp_to_gamma(to.border, k),
            mark: self.mark.lerp_to_gamma(to.mark, k),
        }
    }
}

// Compose CheckboxImpl
fn check_motion(rt: &crate::Rt, id: Id, state: Check) -> [f32; 2] {
    let ctx = rt.ctx();
    let key = id.with("check-motion");
    let mut m = CheckMotion::load(ctx, key);
    let fractions = advance_check(rt, id, &mut m, state);
    m.store(ctx, key);
    fractions
}

fn advance_check(rt: &crate::Rt, id: Id, m: &mut CheckMotion, state: Check) -> [f32; 2] {
    let ctx = rt.ctx();
    let (draw, gravity) = (id.with("check-draw"), id.with("check-gravity"));
    let gravity_of = |s: Check| f32::from(u8::from(s == Check::Mixed));
    let now = ctx.input(|i| i.time);
    if let Some(from) = m.shown
        && from != state
    {
        if state == Check::Unchecked {
            m.off_at = now;
            m.held = [
                egui_mineral::motion::spring(rt.core(), draw, 1.0, motion::DEFAULT_SPATIAL),
                egui_mineral::motion::spring(rt.core(), gravity, gravity_of(from), motion::DEFAULT_SPATIAL),
            ];
        } else if from == Check::Unchecked {
            egui_mineral::motion::snap(ctx, gravity, gravity_of(state));
        }
    }
    m.shown = Some(state);
    if state == Check::Unchecked {
        let wait = cbx::SNAP_DELAY.as_secs_f64() - (now - m.off_at);
        if wait > 0.0
            && !rt.core().motion().reduced()
            && let Ok(wait) = std::time::Duration::try_from_secs_f64(wait)
        {
            ctx.request_repaint_after(wait);
            return m.held;
        }
        egui_mineral::motion::snap(ctx, draw, 0.0);
        egui_mineral::motion::snap(ctx, gravity, 0.0);
        return [0.0, 0.0];
    }
    [
        egui_mineral::motion::spring(rt.core(), draw, 1.0, motion::DEFAULT_SPATIAL),
        egui_mineral::motion::spring(rt.core(), gravity, gravity_of(state), motion::DEFAULT_SPATIAL),
    ]
}

// Compose drawBox
fn paint_box(p: &egui::Painter, bx: Rect, colors: CheckColors) {
    let r = CornerRadius::from(cbx::RADIUS);
    if colors.fill == colors.border {
        p.rect_filled(bx, r, colors.fill);
    } else {
        let w = cbx::STROKE_WIDTH;
        p.rect_filled(
            bx.shrink(w),
            CornerRadius::from((cbx::RADIUS - w).max(0.0)),
            colors.fill,
        );
        p.rect_stroke(bx, r, Stroke::new(w, colors.border), StrokeKind::Inside);
    }
}

// Compose drawCheck
fn paint_check(p: &egui::Painter, bx: Rect, draw: f32, gravity: f32, color: Color32) {
    if draw <= 0.0 || color.a() == 0 {
        return;
    }
    let mid = cbx::CHECK_CENTER;
    let at = |x: f32, y: f32| bx.min + vec2(x, y) * bx.width();
    let pts = [
        at(cbx::CHECK_START[0], egui::lerp(cbx::CHECK_START[1]..=mid, gravity)),
        at(
            egui::lerp(cbx::CHECK_CROSS[0]..=mid, gravity),
            egui::lerp(cbx::CHECK_CROSS[1]..=mid, gravity),
        ),
        at(cbx::CHECK_END[0], egui::lerp(cbx::CHECK_END[1]..=mid, gravity)),
    ];
    let lengths = [pts[0].distance(pts[1]), pts[1].distance(pts[2])];
    let mut left = (lengths[0] + lengths[1]) * draw.min(1.0);
    let mut path = vec![pts[0]];
    for (seg, len) in pts.windows(2).zip(lengths) {
        if left <= 0.0 {
            break;
        }
        let k = (left / len).min(1.0);
        path.push(seg[0].lerp(seg[1], k));
        left -= len;
    }
    // Square caps.
    let half = cbx::STROKE_WIDTH / 2.0;
    // One point when the box has no width.
    let (Some(&[first, second]), Some(&[prev, last])) = (path.first_chunk(), path.last_chunk()) else {
        return;
    };
    if let Some(head) = path.first_mut() {
        *head += (first - second).normalized() * half;
    }
    if let Some(tail) = path.last_mut() {
        *tail += (last - prev).normalized() * half;
    }
    p.line(path, Stroke::new(cbx::STROKE_WIDTH, color));
}

// Compose CheckboxImpl
pub(crate) fn paint_checkbox(
    ui: &MineralUi<'_, Material>,
    id: Id,
    center: Pos2,
    state: Check,
    error: bool,
    enabled: bool,
    t: &Tokens,
) {
    let rt = ui.rt();
    let on = state != Check::Unchecked;
    let spring = if on {
        motion::DEFAULT_EFFECTS
    } else {
        motion::FAST_EFFECTS
    };
    let k = color_fraction(rt, id.with("check-color"), on, enabled, spring);
    let colors =
        CheckColors::of(false, error, enabled, &t.color).lerp(CheckColors::of(true, error, enabled, &t.color), k);
    let [draw, gravity] = check_motion(rt, id, state);
    let bx = Rect::from_center_size(center, Vec2::splat(cbx::BOX_SIZE));
    paint_box(ui.painter(), bx, colors);
    paint_check(ui.painter(), bx, draw, gravity, colors.mark);
}

fn radio_color(selected: bool, enabled: bool, c: &ColorScheme) -> Color32 {
    match (selected, enabled) {
        (true, true) => rb::SELECTED_ICON_COLOR.resolve(c),
        (false, true) => rb::UNSELECTED_ICON_COLOR.resolve(c),
        (true, false) => with_alpha(
            rb::DISABLED_SELECTED_ICON_COLOR.resolve(c),
            rb::DISABLED_SELECTED_ICON_OPACITY,
        ),
        (false, false) => with_alpha(
            rb::DISABLED_UNSELECTED_ICON_COLOR.resolve(c),
            rb::DISABLED_UNSELECTED_ICON_OPACITY,
        ),
    }
}

// Compose RadioButton
pub(crate) fn paint_radio(
    ui: &egui_mineral::widget::MineralUi<'_, crate::Material>,
    id: Id,
    center: Pos2,
    selected: bool,
    enabled: bool,
    t: &Tokens,
) {
    let rt = ui.rt();
    let k = color_fraction(rt, id.with("radio-color"), selected, enabled, motion::DEFAULT_EFFECTS);
    let color = radio_color(false, enabled, &t.color).lerp_to_gamma(radio_color(true, enabled, &t.color), k);
    let dot = egui_mineral::motion::spring(
        rt.core(),
        id.with("radio-dot"),
        if selected { rbx::DOT_SIZE / 2.0 } else { 0.0 },
        motion::FAST_SPATIAL,
    );
    let stroke = rbx::STROKE_WIDTH;
    let p = ui.painter();
    p.circle_stroke(center, rb::ICON_SIZE / 2.0 - stroke / 2.0, Stroke::new(stroke, color));
    if dot - stroke / 2.0 > 0.0 {
        p.circle_filled(center, dot - stroke / 2.0, color);
    }
}

#[derive(Clone, Copy)]
struct SwitchColors {
    track: Color32,
    border: Color32,
    thumb: Color32,
    icon: Color32,
}

impl SwitchColors {
    fn of(on: bool, enabled: bool, c: &ColorScheme) -> Self {
        let r = |role: ColorRole| role.resolve(c);
        let over = |role: ColorRole, alpha: f32| c.surface.blend(with_alpha(r(role), alpha));
        match (on, enabled) {
            (true, true) => Self {
                track: r(sw::SELECTED_TRACK_COLOR),
                border: Color32::TRANSPARENT,
                thumb: r(sw::SELECTED_HANDLE_COLOR),
                icon: r(sw::SELECTED_ICON_COLOR),
            },
            // `uncheckedBorderColor` is the focus outline token.
            (false, true) => Self {
                track: r(sw::UNSELECTED_TRACK_COLOR),
                border: r(sw::UNSELECTED_FOCUS_TRACK_OUTLINE_COLOR),
                thumb: r(sw::UNSELECTED_HANDLE_COLOR),
                icon: r(sw::UNSELECTED_ICON_COLOR),
            },
            (true, false) => Self {
                track: over(sw::DISABLED_SELECTED_TRACK_COLOR, sw::DISABLED_TRACK_OPACITY),
                border: Color32::TRANSPARENT,
                thumb: over(sw::DISABLED_SELECTED_HANDLE_COLOR, sw::DISABLED_SELECTED_HANDLE_OPACITY),
                icon: over(sw::DISABLED_SELECTED_ICON_COLOR, sw::DISABLED_SELECTED_ICON_OPACITY),
            },
            (false, false) => Self {
                track: over(sw::DISABLED_UNSELECTED_TRACK_COLOR, sw::DISABLED_TRACK_OPACITY),
                border: over(sw::DISABLED_UNSELECTED_TRACK_OUTLINE_COLOR, sw::DISABLED_TRACK_OPACITY),
                thumb: over(
                    sw::DISABLED_UNSELECTED_HANDLE_COLOR,
                    sw::DISABLED_UNSELECTED_HANDLE_OPACITY,
                ),
                icon: over(sw::DISABLED_UNSELECTED_ICON_COLOR, sw::DISABLED_UNSELECTED_ICON_OPACITY),
            },
        }
    }
}

const TRAVEL: f32 = sw::TRACK_WIDTH - sw::TRACK_HEIGHT;

fn switch_offset_id(id: Id) -> Id {
    id.with("switch-offset")
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct SwitchLook {
    pub on: bool,
    /// Check and close icons in the thumb (`thumbContent`).
    pub icons: bool,
    pub enabled: bool,
    /// Pressed or dragged: the thumb grows to its pressed size.
    pub pressed: bool,
    /// Thumb travel held by a drag, from the start anchor.
    pub drag: Option<f32>,
}

// Compose Switch
pub(crate) fn paint_switch(ui: &MineralUi<'_, Material>, id: Id, track: Rect, s: SwitchLook, t: &Tokens) -> Rect {
    let rt = ui.rt();
    let dir = rt.i18n().direction();
    let sign = dir.sign();
    let offset = s.drag.unwrap_or_else(|| {
        egui_mineral::motion::spring(
            rt.core(),
            switch_offset_id(id),
            if s.on { TRAVEL } else { 0.0 },
            motion::FAST_SPATIAL,
        )
    });
    let target = if s.pressed && s.enabled {
        sw::PRESSED_HANDLE_WIDTH
    } else if s.on || s.icons {
        sw::SELECTED_HANDLE_WIDTH
    } else {
        sw::UNSELECTED_HANDLE_WIDTH
    };
    let size = egui_mineral::motion::spring(rt.core(), id.with("switch-thumb"), target, motion::FAST_SPATIAL).max(0.0);
    let c = SwitchColors::of(s.on, s.enabled, &t.color);
    let shape = sw::TRACK_SHAPE.corner_radius(&t.shape, dir);
    let p = ui.painter();
    p.rect_filled(track, shape, c.track);
    p.rect_stroke(
        track,
        shape,
        Stroke::new(sw::TRACK_OUTLINE_WIDTH, c.border),
        StrokeKind::Inside,
    );
    let start = dir.start_edge(track);
    let center = egui::pos2(start + sign * (sw::TRACK_HEIGHT / 2.0 + offset), track.center().y);
    let thumb = Rect::from_center_size(center, Vec2::splat(size));
    p.rect_filled(thumb, sw::HANDLE_SHAPE.corner_radius(&t.shape, Direction::Ltr), c.thumb);
    if s.icons {
        let (icon, size): (Icon, f32) = if s.on {
            (ui.icons().check, sw::SELECTED_ICON_SIZE)
        } else {
            (ui.icons().close, sw::UNSELECTED_ICON_SIZE)
        };
        common::icon(p, center, icon, size, c.icon);
    }
    thumb
}

/// MD3 checkbox (Compose `TriStateCheckbox`); `indeterminate` shows a dash and activates to checked.
/// Never narrower than its box, even under a tighter constraint.
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct CheckButton<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    checked: &'a mut bool,
    label: String,
    label_visible: bool,
    indeterminate: bool,
    error: Option<String>,
    disabled: bool,
    mnemonic: Option<Mnemonic>,
}

impl<'a> CheckButton<'a> {
    /// `label` is the visible text and accessible name; a click flips `checked` and reports `changed`. Agent key `<id_salt>-checkbox`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, checked: &'a mut bool) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            key: AgentKey::Kind("checkbox"),
            salt: id_salt,
            checked,
            label: label.into(),
            label_visible: true,
            indeterminate: false,
            error: None,
            disabled: false,
            mnemonic: None,
        }
    }

    /// `false` keeps the label for assistive technology only.
    pub fn label_visible(mut self, label_visible: bool) -> Self {
        self.label_visible = label_visible;
        self
    }

    /// Alt+letter toggles the control; the letter is underlined while mnemonics are visible.
    pub fn mnemonic(mut self, mnemonic: char) -> Self {
        self.mnemonic = Some(Mnemonic::new(mnemonic));
        self
    }

    /// Shows the dash; the next activation checks the box and clears it.
    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.indeterminate = indeterminate;
        self
    }

    crate::widget::error_setters! {
        /// Error colors and `message` below the box; marks the node invalid.
        |s| s.error
    }

    crate::widget::setters! {
        /// Greys out the control and ignores input.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-checkbox` agent key.
        agent_key(key)
    }

    fn row(&self, ui: &MineralUi<'_, Material>, t: &Tokens, width: f32) -> ControlRow {
        ControlRow::new(
            ui,
            Control::Checkbox.canvas(),
            self.label_visible.then_some(self.label.as_str()),
            self.mnemonic,
            label_color(t, !self.disabled, cb::UNSELECTED_DISABLED_CONTAINER_OPACITY),
            width,
        )
    }

    fn error_galley(&self, ui: &MineralUi<'_, Material>, t: &Tokens) -> Option<Arc<Galley>> {
        let rt = ui.rt();
        self.error
            .as_ref()
            .map(|m| parts::type_galley(rt.core(), ui.painter(), m, TypeScale::BodySmall, t.color.error, None))
    }

    fn state(&self) -> Check {
        match (self.indeterminate, *self.checked) {
            (true, _) => Check::Mixed,
            (false, true) => Check::Checked,
            (false, false) => Check::Unchecked,
        }
    }
}

impl MineralWidget for CheckButton<'_> {
    type Kit = Material;
    type State = ();
    type Output = ();

    egui_mineral::kit::widget_meta!(salt, enabled);

    fn access_key(&self) -> Option<Mnemonic> {
        self.mnemonic
    }

    fn min_target(&self) -> MinTarget {
        crate::adaptive::MIN_INTERACTIVE
    }

    fn layout(&self, ui: &MineralUi<'_, Material>, c: Constraints, (): &Self::State) -> Measure {
        let t = ui.tokens();
        let overflow = (Control::Checkbox.layer(ui.density()) - Control::Checkbox.canvas().x) / 2.0;
        let row = self.row(ui, &t, c.max.x - overflow.max(0.0));
        let mut size = row.size();
        if let Some(e) = self.error_galley(ui, &t) {
            size = vec2(
                size.x.max(row.canvas.x + cbx::LABEL_SPACE + e.size().x),
                size.y + e.size().y,
            );
        }
        row.measure(c.constrain(size), overflow)
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Material>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let changed = parts::press(ui, frame, Availability::disabled_if(self.disabled)).clicked;
        if changed {
            *self.checked = self.indeterminate || !*self.checked;
            self.indeterminate = false;
        }
        let row = self.row(ui, &t, frame.zones.visual.width() + WRAP_SLACK);
        let center = row.canvas(frame.zones.visual).center();
        let r = &frame.response;
        paint_checkbox(ui, r.id, center, self.state(), self.error.is_some(), !self.disabled, &t);
        Control::Checkbox.indication(ui, r, Control::Checkbox.node(ui.rt(), center), self.disabled, &t);
        row.paint_label(ui, frame.zones.visual);
        if let Some(e) = self.error_galley(ui, &t) {
            let x = row.text_x(frame.zones.visual, e.size().x);
            let y = row.row(frame.zones.visual).bottom();
            ui.painter().galley(egui::pos2(x, y), e, Color32::PLACEHOLDER);
        }
        WidgetResponse::new(frame.response.clone(), ()).with_changed(changed)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(self.key.id(&self.salt), Role::CheckBox, &self.label)
            .clickable(Availability::disabled_if(self.disabled));
        n.state.invalid = self.error.is_some();
        n.value.clone_from(&self.error);
        n.state.checked = Some(self.state());
        Some(n)
    }
}
/// MD3 radio button (Compose `RadioButton`). Use [`RadioGroup`] for arrow-key navigation. Never
/// narrower than its icon, even under a tighter constraint.
/// MD3 radio button (Compose `RadioButton`). Use [`RadioGroup`] for arrow-key navigation.
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Radio {
    salt: KeyedSalt,
    key: AgentKey,
    selected: bool,
    label: String,
    label_visible: bool,
    disabled: bool,
    tab_stop: bool,
    mnemonic: Option<Mnemonic>,
}

impl Radio {
    /// Display-only radio; [`RadioGroup`] owns selection. Agent key `<id_salt>-radio`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            key: AgentKey::Kind("radio"),
            salt: id_salt,
            selected: false,
            label: label.into(),
            label_visible: true,
            disabled: false,
            tab_stop: true,
            mnemonic: None,
        }
    }

    /// Filled dot.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub(crate) fn tab_stop(mut self, tab_stop: bool) -> Self {
        self.tab_stop = tab_stop;
        self
    }

    /// `false` keeps the label for assistive technology only.
    pub fn label_visible(mut self, label_visible: bool) -> Self {
        self.label_visible = label_visible;
        self
    }

    /// Alt+letter toggles the control; the letter is underlined while mnemonics are visible.
    pub fn mnemonic(mut self, mnemonic: char) -> Self {
        self.mnemonic = Some(Mnemonic::new(mnemonic));
        self
    }

    crate::widget::setters! {
        /// Greys out the radio and ignores input.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-radio` agent key.
        agent_key(key)
    }

    fn row(&self, ui: &MineralUi<'_, Material>, t: &Tokens, width: f32) -> ControlRow {
        ControlRow::new(
            ui,
            Control::Radio.canvas(),
            self.label_visible.then_some(self.label.as_str()),
            self.mnemonic,
            label_color(t, !self.disabled, rb::DISABLED_UNSELECTED_ICON_OPACITY),
            width,
        )
    }
}

impl MineralWidget for Radio {
    type Kit = Material;
    type State = ();
    type Output = ();

    egui_mineral::kit::widget_meta!(salt, enabled);

    fn sense(&self) -> Sense {
        focus::roving_sense(
            Sense::click(),
            if self.tab_stop {
                focus::TabStop::Stop
            } else {
                focus::TabStop::Skip
            },
        )
    }

    fn access_key(&self) -> Option<Mnemonic> {
        self.mnemonic
    }

    fn min_target(&self) -> MinTarget {
        crate::adaptive::MIN_INTERACTIVE
    }

    fn layout(&self, ui: &MineralUi<'_, Material>, c: Constraints, (): &Self::State) -> Measure {
        let t = ui.tokens();
        let overflow = (Control::Radio.layer(ui.density()) - Control::Radio.canvas().x) / 2.0;
        let row = self.row(ui, &t, c.max.x - overflow.max(0.0));
        row.measure(c.constrain(row.size()), overflow)
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Material>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        kit::focus::follow_press(&frame.response);
        let changed = parts::press(ui, frame, Availability::disabled_if(self.disabled)).clicked && !self.selected;
        let row = self.row(ui, &t, frame.zones.visual.width() + WRAP_SLACK);
        let center = row.canvas(frame.zones.visual).center();
        let r = &frame.response;
        paint_radio(ui, r.id, center, self.selected || changed, !self.disabled, &t);
        Control::Radio.indication(ui, r, Control::Radio.node(ui.rt(), center), self.disabled, &t);
        row.paint_label(ui, frame.zones.visual);
        WidgetResponse::new(frame.response.clone(), ()).with_changed(changed)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(self.key.id(&self.salt), Role::RadioButton, &self.label)
            .clickable(Availability::disabled_if(self.disabled));
        n.state.checked = Some(self.selected.into());
        Some(n)
    }
}

/// Option of a [`RadioGroup`].
#[derive(Clone, Debug)]
#[must_use]
pub struct RadioOption<T> {
    value: T,
    label: String,
    disabled: bool,
    salt: Option<egui_mineral::semantic::KeyedSalt>,
}

impl<T> RadioOption<T> {
    crate::widget::setters! {
        /// Default the index.
        item_salt
    }

    /// `label` is the visible text and accessible name of `value`.
    pub fn new(value: T, label: impl Into<String>) -> Self {
        Self {
            value,
            label: label.into(),
            disabled: false,
            salt: None,
        }
    }

    crate::widget::setters! {
        /// Greys out this option and skips it in keyboard navigation.
        disabled
    }
}

/// <https://www.w3.org/WAI/ARIA/apg/patterns/radio/>
#[derive(Debug)]
#[must_use = "call .show(..)"]
pub struct RadioGroup<'a, T> {
    salt: KeyedSalt,
    label: String,
    value: &'a mut T,
    options: Vec<RadioOption<T>>,
    axis: Axis,
    disabled: bool,
}

impl<'a, T: PartialEq + Clone> RadioGroup<'a, T> {
    /// `label` names the group for assistive technology.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, value: &'a mut T) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            label: label.into(),
            value,
            options: Vec::new(),
            axis: Axis::Vertical,
            disabled: false,
        }
    }

    /// Appends one option.
    pub fn option(mut self, option: impl Into<RadioOption<T>>) -> Self {
        self.options.push(option.into());
        self
    }

    /// Appends options.
    pub fn options(mut self, options: impl IntoIterator<Item = impl Into<RadioOption<T>>>) -> Self {
        self.options.extend(options.into_iter().map(Into::into));
        self
    }

    /// Default [`Axis::Vertical`].
    pub fn axis(mut self, axis: Axis) -> Self {
        self.axis = axis;
        self
    }

    crate::widget::setters! {
        /// Greys out every option.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    /// Lays out the radios along the axis; a pick writes `value` and reports `changed`.
    pub fn show(self, ui: &mut MineralUi<'_, Material>) -> SelectionResponse {
        let rt = ui.rt();
        let Self {
            salt,
            label,
            value,
            options,
            axis,
            disabled,
        } = self;
        let mut ids = Vec::with_capacity(options.len());
        let mut changed = false;
        let mut picked = None;
        let group_id = ui.make_persistent_id(salt.id_salt());
        let enabled = |i: usize| !disabled && options.get(i).is_some_and(|o| !o.disabled);
        let selected = options.iter().position(|o| o.value == *value);
        let tab_stop = focus::tab_stop(ui.ctx(), group_id, selected, options.len(), enabled);
        let body = |ui: &mut Ui| {
            for (i, o) in options.iter().enumerate() {
                let r = rt.ui(ui).mineral(
                    Radio::new(crate::widget::item_salt(&salt, o.salt.as_ref(), None, i), &o.label)
                        .selected(o.value == *value)
                        .tab_stop(i == tab_stop)
                        .disabled(!enabled(i)),
                );
                if r.changed() {
                    *value = o.value.clone();
                    changed = true;
                    picked = Some(i);
                }
                ids.push(r.id);
            }
        };
        let group = ui
            .push_id(salt.id_salt(), |ui| match axis {
                Axis::Horizontal => ui.horizontal_wrapped(body).response,
                Axis::Vertical => ui.vertical(body).response,
            })
            .inner;
        if let Some((i, o)) = RovingFocus::new(axis)
            .update_group(ui.core(), group_id, &ids, enabled)
            .and_then(|i| Some((i, options.get(i)?)))
        {
            *value = o.value.clone();
            changed = true;
            picked = Some(i);
        }
        semantic::emit(ui.ctx(), Some(group.id), group.rect, || {
            let mut n = SemanticNode::new(StableId::of(&salt, "radio-group"), Role::RadioGroup, &label);
            n.state.disabled = disabled;
            n
        });
        SelectionResponse::new(group, picked, changed)
    }
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct SwitchState {
    drag: Option<f32>,
}

/// MD3 switch (Compose `Switch`): the thumb grows from 16dp (off) to 24dp (on or with icons) and
/// 28dp while pressed or dragged; it can be dragged between its anchors. Never narrower than its
/// track, even under a tighter constraint.
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Switch<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    checked: &'a mut bool,
    label: String,
    label_visible: bool,
    icons: bool,
    disabled: bool,
    mnemonic: Option<Mnemonic>,
}

impl<'a> Switch<'a> {
    /// `label` is the visible text and accessible name; a click flips `checked` and reports `changed`. Agent key `<id_salt>-switch`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, checked: &'a mut bool) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            key: AgentKey::Kind("switch"),
            salt: id_salt,
            checked,
            label: label.into(),
            label_visible: true,
            icons: false,
            disabled: false,
            mnemonic: None,
        }
    }

    /// `false` keeps the label for assistive technology only.
    pub fn label_visible(mut self, label_visible: bool) -> Self {
        self.label_visible = label_visible;
        self
    }

    /// Alt+letter toggles the control; the letter is underlined while mnemonics are visible.
    pub fn mnemonic(mut self, mnemonic: char) -> Self {
        self.mnemonic = Some(Mnemonic::new(mnemonic));
        self
    }

    /// Check and close icons in the thumb (`thumbContent`).
    pub fn icons(mut self, icons: bool) -> Self {
        self.icons = icons;
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

    fn row(&self, ui: &MineralUi<'_, Material>, t: &Tokens, width: f32) -> ControlRow {
        ControlRow::new(
            ui,
            Control::Switch.canvas(),
            self.label_visible.then_some(self.label.as_str()),
            self.mnemonic,
            label_color(t, !self.disabled, sw::DISABLED_UNSELECTED_HANDLE_OPACITY),
            width,
        )
    }
}

impl MineralWidget for Switch<'_> {
    type Kit = Material;
    type State = SwitchState;
    type Output = ();

    egui_mineral::kit::widget_meta!(salt, enabled);

    fn sense(&self) -> Sense {
        Sense::click_and_drag()
    }

    fn pan_axis(&self) -> Option<Axis> {
        Some(Axis::Horizontal)
    }

    fn access_key(&self) -> Option<Mnemonic> {
        self.mnemonic
    }

    fn min_target(&self) -> MinTarget {
        crate::adaptive::MIN_INTERACTIVE
    }

    fn layout(&self, ui: &MineralUi<'_, Material>, c: Constraints, _: &Self::State) -> Measure {
        let t = ui.tokens();
        let overflow = (Control::Switch.layer(ui.density()) - sw::TRACK_HEIGHT) / 2.0;
        let row = self.row(ui, &t, c.max.x - overflow.max(0.0));
        row.measure(c.constrain(row.size()), overflow)
    }

    fn update(
        &mut self,
        ui: &mut MineralUi<'_, Material>,
        frame: &WidgetFrame<'_>,
        state: &mut SwitchState,
    ) -> WidgetResponse {
        let rt = ui.rt();
        let t = ui.tokens();
        let ctx = ui.ctx().clone();
        let r = &frame.response;
        let row = self.row(ui, &t, frame.zones.visual.width() + WRAP_SLACK);
        let sign = row.dir.sign();
        let offset_id = switch_offset_id(r.id);
        let mut changed = parts::press(ui, frame, Availability::disabled_if(self.disabled)).clicked;
        if !self.disabled {
            if r.drag_started() {
                state.drag = Some(egui_mineral::motion::spring(
                    rt.core(),
                    offset_id,
                    if *self.checked { TRAVEL } else { 0.0 },
                    motion::FAST_SPATIAL,
                ));
            }
            if let Some(d) = &mut state.drag {
                *d = (*d + sign * r.drag_delta().x).clamp(0.0, TRAVEL);
                egui_mineral::motion::snap(&ctx, offset_id, *d);
            }
            if r.drag_stopped()
                && let Some(d) = state.drag.take()
            {
                let v = sign * ctx.input(|i| i.pointer.velocity().x);
                let fling = rt.core().policies().fling;
                let settled = if *self.checked {
                    !fling.should_dismiss(
                        egui_mineral::motion::Release {
                            pos: TRAVEL - d,
                            velocity: -v,
                        },
                        TRAVEL,
                    )
                } else {
                    fling.should_dismiss(egui_mineral::motion::Release { pos: d, velocity: v }, TRAVEL)
                };
                changed |= settled != *self.checked;
            }
        }
        if changed {
            *self.checked = !*self.checked;
        }
        let look = SwitchLook {
            on: *self.checked,
            icons: self.icons,
            enabled: !self.disabled,
            pressed: r.is_pointer_button_down_on() || state.drag.is_some(),
            drag: state.drag,
        };
        let thumb = paint_switch(ui, r.id, row.canvas(frame.zones.visual), look, &t);
        Control::Switch.indication(ui, r, thumb, self.disabled, &t);
        row.paint_label(ui, frame.zones.visual);
        WidgetResponse::new(r.clone(), ()).with_changed(changed)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, _: &SwitchState) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(self.key.id(&self.salt), Role::Switch, &self.label)
            .clickable(Availability::disabled_if(self.disabled));
        n.state.checked = Some((*self.checked).into());
        Some(n)
    }
}

#[cfg(all(test, feature = "bundled-icons"))]
mod tests {
    use super::*;

    fn painted(width: f32) -> usize {
        let ctx = egui::Context::default();
        let out = crate::test_support::run(&ctx, egui::RawInput::default(), |ui| {
            let bx = Rect::from_min_size(Pos2::ZERO, Vec2::splat(width));
            paint_check(ui.painter(), bx, 1.0, 0.0, Color32::WHITE);
        });
        out.shapes.len()
    }

    #[test]
    fn check_mark_in_a_zero_width_box_paints_nothing() {
        assert_eq!(painted(cbx::BOX_SIZE), 1, "a sized box paints the mark");
        assert_eq!(painted(0.0), 0);
    }
}
