use crate::Material;
use crate::MaterialExt;
use crate::Rt;
use crate::adaptive::MIN_INTERACTIVE_DP;
use crate::widget::modal_focus::ModalFocus;
use egui::{
    Color32, Context, CornerRadius, Frame, Id, Key, Margin, Rangef, Rect, ScrollArea, Sense, Shadow, UiBuilder, Vec2,
    pos2, vec2,
};
use egui_mineral::kit::state::TempState;
use egui_mineral::overlay::OverlayResponse;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::Availability;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    i18n::Direction,
    kit,
    layout::{Axis, Edge},
    overlay::Dismiss,
    range::{RangeModel, Track},
    semantic::{self, Role, SemanticNode, StableId},
};

use crate::spec::ElevationLevel;
use crate::widget::{
    common::{self, Interaction},
    edge_surface::{self, EdgeSurface},
    parts,
};
use crate::{
    spec::{ColorRole, ShapeRole, TypeScale, defaults, tokens::sheet_bottom},
    tokens::Tokens,
};

fn standard_shadow(t: &Tokens, level: ElevationLevel, offset: [i8; 2]) -> Shadow {
    Shadow {
        offset,
        ..t.elevation_shadow(level)
    }
}

#[derive(Clone, Copy, Default)]
struct BottomState {
    detent: usize,
    grab: Option<f32>,
    live: Option<f32>,
}

impl TempState for BottomState {}

#[derive(Default)]
struct HandleOut {
    drag_started: Option<f32>,
    drag_stopped: bool,
    // points/s
    release_vy: f32,
    clicked: bool,
    step: i32,
}

/// MD3 bottom sheet, standard or modal (Compose `BottomSheetScaffold`, `ModalBottomSheet`).
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct BottomSheet {
    salt: KeyedSalt,
    id: Id,
    modal: bool,
    detents: Vec<f32>,
    handle: bool,
    label: Option<String>,
}

impl BottomSheet {
    /// One detent at half the screen height, with a drag handle; agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            id: Id::unique(("mineral::bottom_sheet", id_salt.id_salt())),
            salt: id_salt,
            modal: false,
            detents: vec![0.5],
            handle: true,
            label: None,
        }
    }

    /// Accessible name of the sheet; default "Bottom sheet".
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    /// Resting heights as fractions of the screen height, each clamped to 0.1..=1; an empty list keeps the
    /// current detents.
    #[track_caller]
    pub fn detents(mut self, fractions: impl IntoIterator<Item = f32>) -> Self {
        let fractions: Vec<f32> = fractions
            .into_iter()
            .map(|f| egui_mineral::kit::sanitize::within(f, 0.1..=1.0, "detent"))
            .collect();
        if fractions.is_empty() {
            egui_mineral::kit::sanitize::report(format_args!("detents must be non-empty"));
        } else {
            self.detents = fractions;
        }
        self
    }

    /// Shows the drag handle; default `true`.
    pub fn drag_handle(mut self, drag_handle: bool) -> Self {
        self.handle = drag_handle;
        self
    }

    /// Standard sheet: coexists with the content behind it.
    pub fn show<R>(
        self,
        rt: &Rt,
        open: &mut bool,
        add: impl FnOnce(&mut MineralUi<'_, Material>) -> R,
    ) -> OverlayResponse<R> {
        self.run(rt, open, add)
    }

    /// Modal sheet: scrim and dismissal by Escape, scrim press or swipe.
    pub fn show_modal<R>(
        mut self,
        rt: &Rt,
        open: &mut bool,
        add: impl FnOnce(&mut MineralUi<'_, Material>) -> R,
    ) -> OverlayResponse<R> {
        self.modal = true;
        let fid = kit::scoped_id(rt.core(), self.id).with("focus");
        ModalFocus::scope(rt.ctx(), fid, open, None, |_, open| self.run(rt, open, add))
    }

    fn run<R>(
        mut self,
        rt: &Rt,
        open: &mut bool,
        add: impl FnOnce(&mut MineralUi<'_, Material>) -> R,
    ) -> OverlayResponse<R> {
        let ctx = rt.ctx();
        self.id = kit::scoped_id(rt.core(), self.id);
        let t = rt.tokens();
        let Some(e) = edge_surface::slide(rt, self.id, *open) else {
            BottomState::remove(ctx, self.id);
            return OverlayResponse::closed();
        };
        let screen = ctx.content_rect();
        let mut st = BottomState::load(ctx, self.id);
        let heights: Vec<f32> = self.detents.iter().map(|f| f * screen.height()).collect();
        st.detent = st.detent.min(heights.len() - 1);
        // No MineralUi in `run`.
        if let (Some(grab), Some(pan)) = (st.grab, rt.core().input().pan) {
            st.live = Some((screen.bottom() - (pan.pos.y - grab)).clamp(0.0, screen.height()));
        }
        let height_id = self.id.with("height");
        let h = match st.live {
            Some(l) => ctx.animate_value_with_time(height_id, l, 0.0),
            None => ctx.animate_value_with_time(
                height_id,
                heights[st.detent],
                rt.core().motion().duration(t.duration.medium[1]).as_secs_f32(),
            ),
        };
        let w = screen.width().min(defaults::sheet::MAX_WIDTH);
        let top = screen.bottom() - h + (1.0 - e) * h;
        let name = crate::tr_or(rt.i18n(), self.label.as_deref(), "mineral-sheet-bottom");
        let surface = EdgeSurface {
            id: self.id,
            name: &name,
            pos: pos2(screen.center().x - w / 2.0, top),
            scrim: self.modal.then(|| t.scrim(e)),
        };
        let mut handle_out = HandleOut::default();
        let mut add = Some(add);
        let shown = surface.show(rt, |ui| {
            let mut frame = Frame::new()
                .fill(t.role(sheet_bottom::DOCKED_CONTAINER_COLOR))
                .corner_radius(sheet_bottom::DOCKED_CONTAINER_SHAPE.corner_radius(&t.shape, Direction::Ltr));
            // Compose gives only the standard (scaffold) sheet a shadow.
            if !self.modal {
                frame = frame.shadow(standard_shadow(
                    &t,
                    sheet_bottom::DOCKED_STANDARD_CONTAINER_ELEVATION,
                    [0, -1],
                ));
            }
            let framed = frame.show(ui, |ui| {
                ui.set_min_size(vec2(w, h));
                ui.set_max_size(vec2(w, h));
                let mut header = 0.0;
                if self.handle {
                    header = HANDLE_ZONE;
                    handle_out = paint_handle(&mut rt.ui(ui), &self.salt, w, st.detent, heights.len(), &t);
                }
                // Compose sheets add no content padding: the content provides it.
                ScrollArea::vertical()
                    .id_salt((self.id, "scroll"))
                    .max_height((h - header).max(0.0))
                    .auto_shrink([false, false])
                    .show(ui, |ui| add.take().map(|f| f(&mut rt.ui(ui))))
                    .inner
            });
            semantic::emit(ui.ctx(), None, framed.response.rect, || {
                SemanticNode::new(StableId::of(&self.salt, "bottom-sheet"), Role::Dialog, &name)
            });
            framed.inner
        });

        let close = apply_handle(rt, &mut st, &handle_out, &heights, h).or(shown.close);
        let dismissed = edge_surface::dismiss(open, close);
        if dismissed.is_some() {
            st = BottomState::default();
        }
        st.store(ctx, self.id);
        OverlayResponse::shown(shown.response, shown.inner, dismissed)
    }
}

fn apply_handle(rt: &Rt, st: &mut BottomState, handle_out: &HandleOut, heights: &[f32], h: f32) -> Option<Dismiss> {
    let mut dismissed = None;
    if let Some(g) = handle_out.drag_started {
        st.grab = Some(g);
    }
    if handle_out.drag_stopped {
        let live = st.live.take().unwrap_or(h);
        st.grab = None;
        let vy = handle_out.release_vy;
        let lowest = heights.iter().copied().fold(f32::INFINITY, f32::min);
        let fling = rt.core().policies().fling;
        if live <= lowest
            && fling.should_dismiss(
                egui_mineral::motion::Release {
                    pos: lowest - live,
                    velocity: vy,
                },
                lowest,
            )
        {
            dismissed = Some(Dismiss::Swipe);
        } else {
            let to = fling.project(egui_mineral::motion::Release {
                pos: live,
                velocity: -vy,
            });
            if let Some((i, _)) = heights
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| (*a - to).abs().total_cmp(&(*b - to).abs()))
            {
                st.detent = i;
            }
        }
    }
    if handle_out.clicked && heights.len() > 1 {
        st.detent = (st.detent + 1) % heights.len();
    }
    if handle_out.step > 0 {
        st.detent = (st.detent + 1).min(heights.len().saturating_sub(1));
    } else if handle_out.step < 0 {
        if st.detent == 0 {
            dismissed = Some(Dismiss::Swipe);
        } else {
            st.detent -= 1;
        }
    }
    dismissed
}

// Compose BottomSheetDefaults.DragHandle
const HANDLE_ZONE: f32 = sheet_bottom::DOCKED_DRAG_HANDLE_HEIGHT + 2.0 * defaults::sheet::DRAG_HANDLE_VERTICAL_PADDING;

fn paint_handle(
    ui: &mut MineralUi<'_, Material>,
    salt: &KeyedSalt,
    w: f32,
    detent: usize,
    detents: usize,
    t: &Tokens,
) -> HandleOut {
    let rt = ui.rt();
    let ctx = rt.ctx();
    let (zone, mut resp) = ui.allocate_exact_size(vec2(w, HANDLE_ZONE), Sense::click_and_drag());
    ui.pass_input().tap_on(&mut resp);
    let hid = resp.id;
    let center = zone.center();
    let i = Interaction::of(&resp, Availability::Enabled);
    let color = t.role(sheet_bottom::DOCKED_DRAG_HANDLE_COLOR);
    // Keyboard and pointer feedback in a minimum-interactive circle; Compose's handle has none.
    let target = Rect::from_center_size(center, Vec2::splat(HANDLE_ZONE));
    let p = ui.painter();
    common::state_focus(p, target, CornerRadius::same(t.shape.full), color, i, t);
    let handle = Rect::from_center_size(
        center,
        vec2(
            sheet_bottom::DOCKED_DRAG_HANDLE_WIDTH,
            sheet_bottom::DOCKED_DRAG_HANDLE_HEIGHT,
        ),
    );
    p.rect_filled(
        handle,
        common::radius(handle, defaults::sheet::DRAG_HANDLE_SHAPE.radius(&t.shape)),
        color,
    );
    if resp.hovered() {
        ctx.set_cursor_icon(egui::CursorIcon::Grab);
    }
    let mut out = HandleOut {
        // Pointer offset from the sheet top, which is the zone top.
        drag_started: resp
            .drag_started()
            .then(|| resp.interact_pointer_pos().map_or(0.0, |p| p.y - zone.top())),
        drag_stopped: resp.drag_stopped(),
        release_vy: ui
            .pass_input()
            .pan_on(&resp)
            .filter(|p| p.ended)
            .map_or(0.0, |p| p.velocity.y),
        clicked: resp.clicked(),
        step: 0,
    };
    if resp.has_focus() {
        kit::focus::lock_arrows(ctx, hid, kit::focus::Arrows::Axis(Axis::Vertical));
        for k in kit::input::plain_key_presses(ctx) {
            match k {
                Key::ArrowUp => out.step += 1,
                Key::ArrowDown => out.step -= 1,
                _ => {}
            }
        }
    }
    semantic::emit(ctx, Some(hid), zone, || {
        let name = crate::tr(rt.i18n(), "mineral-sheet-handle");
        let value = rt.i18n().tr(
            "mineral-sheet-detent",
            &[("index", (detent + 1).into()), ("count", detents.into())],
        );
        SemanticNode::new(StableId::of(salt, "drag-handle"), Role::Button, name)
            .value(value)
            .clickable(Availability::Enabled)
    });
    out
}

// No Compose implementation or tokens; values from m3.material.io/components/side-sheets/specs
const SIDE_WIDTH: f32 = 360.0;
const SIDE_WIDTHS: Rangef = Rangef { min: 256.0, max: 400.0 };
const SIDE_HEADER_HEIGHT: f32 = 64.0;
const SIDE_PADDING: f32 = 24.0;
const SIDE_CLOSE_END: f32 = 8.0;
const SIDE_HEADLINE_FONT: TypeScale = TypeScale::TitleLarge;
const SIDE_HEADLINE_COLOR: ColorRole = ColorRole::OnSurfaceVariant;
const SIDE_CONTAINER_COLOR: ColorRole = sheet_bottom::DOCKED_CONTAINER_COLOR;
const SIDE_MODAL_SHAPE: ShapeRole = ShapeRole::CornerLargeStart;
// 40dp container in the 48dp minimum interactive size

/// MD3 side sheet. A standard sheet docks into the `Ui` it is shown in at the logical end edge
/// (see [`SideSheet::edge`]) and takes its width from the remaining content, like `egui::Panel`; a modal sheet overlays the
/// viewport with a scrim.
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct SideSheet {
    salt: KeyedSalt,
    id: Id,
    headline: Option<String>,
    close_button: bool,
    width: f32,
    widths: Rangef,
    resizable: bool,
    edge: Edge,
    scroll: bool,
    padding: Margin,
}

impl SideSheet {
    /// 360dp wide with a close button; agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            id: Id::unique(("mineral::side_sheet", id_salt.id_salt())),
            salt: id_salt,
            headline: None,
            close_button: true,
            width: SIDE_WIDTH,
            widths: SIDE_WIDTHS,
            resizable: false,
            edge: Edge::End,
            scroll: true,
            padding: Margin::from(vec2(SIDE_PADDING, 0.0)),
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    /// Title row text.
    pub fn headline(mut self, headline: impl Into<String>) -> Self {
        self.headline = Some(headline.into());
        self
    }

    /// Close icon button in the title row; default `true`.
    pub fn close_button(mut self, close_button: bool) -> Self {
        self.close_button = close_button;
        self
    }

    /// Width in dp, clamped into [`Self::width_range`]; the initial width of a resizable sheet.
    #[track_caller]
    pub fn width(mut self, width: f32) -> Self {
        self.width = egui_mineral::kit::sanitize::at_least(width, 0.0.., "side sheet width");
        self
    }

    /// Width bounds in dp; default MD3 256..=400. A reversed range is swapped.
    #[track_caller]
    pub fn width_range(mut self, widths: impl Into<Rangef>) -> Self {
        let w = widths.into();
        let min = egui_mineral::kit::sanitize::at_least(w.min.min(w.max), 0.0.., "side sheet width_range");
        self.widths = Rangef::new(min, w.max.max(w.min).max(min));
        self
    }

    /// Standard and modal sheet: a drag handle on its inner edge resizes it within [`Self::width_range`]
    /// (arrow keys when focused); the width persists in egui memory per sheet id. Default `false`.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Logical edge the sheet sits at, docked and modal; default [`Edge::End`].
    pub fn edge(mut self, edge: Edge) -> Self {
        self.edge = edge;
        self
    }

    /// Wraps the content in a vertical `ScrollArea`; default `true`. Without it the content gets
    /// the full remaining height and scrolls itself.
    pub fn scroll(mut self, scroll: bool) -> Self {
        self.scroll = scroll;
        self
    }

    /// Margin around the content; default 24dp at the sides, none above and below.
    pub fn content_padding(mut self, padding: Margin) -> Self {
        self.padding = padding;
        self
    }

    /// Standard sheet: reserves its animated width in `ui`, so show it before the content sharing
    /// the space.
    pub fn show<R>(
        self,
        ui: &mut MineralUi<'_, Material>,
        open: &mut bool,
        add: impl FnOnce(&mut MineralUi<'_, Material>) -> R,
    ) -> OverlayResponse<R> {
        let rt = ui.rt();
        self.run(rt, open, |s, e, name| s.docked_surface(ui, e, name, add))
    }

    /// Modal sheet over the screen edge, with scrim.
    pub fn show_modal<R>(
        self,
        rt: &Rt,
        open: &mut bool,
        add: impl FnOnce(&mut MineralUi<'_, Material>) -> R,
    ) -> OverlayResponse<R> {
        let fid = kit::scoped_id(rt.core(), self.id).with("focus");
        ModalFocus::scope(rt.ctx(), fid, open, None, |_, open| {
            self.run(rt, open, |s, e, name| s.modal_surface(rt, e, name, add))
        })
    }

    fn run<R>(
        mut self,
        rt: &Rt,
        open: &mut bool,
        surface: impl FnOnce(&Self, f32, &str) -> (Option<R>, egui::Response, Option<Dismiss>),
    ) -> OverlayResponse<R> {
        self.id = kit::scoped_id(rt.core(), self.id);
        let Some(e) = edge_surface::slide(rt, self.id, *open) else {
            return OverlayResponse::closed();
        };
        let name = crate::tr_or(rt.i18n(), self.headline.as_deref(), "mineral-sheet-side");
        let (inner, response, closed) = surface(&self, e, &name);
        let dismissed = edge_surface::dismiss(open, closed);
        OverlayResponse::shown(response, inner, dismissed)
    }

    fn modal_surface<R>(
        &self,
        rt: &Rt,
        e: f32,
        name: &str,
        add: impl FnOnce(&mut MineralUi<'_, Material>) -> R,
    ) -> (Option<R>, egui::Response, Option<Dismiss>) {
        let ctx = rt.ctx();
        let t = rt.tokens();
        let screen = ctx.content_rect();
        let dir = rt.i18n().direction();
        let width_id = self.id.with("width");
        let width = self.widths.clamp(self.persisted_width(ctx, width_id));
        let w = width.min(screen.width());
        let hidden = (1.0 - e) * w;
        let x = self.side(dir).start_x(screen, Rangef::new(-hidden, -hidden + w));
        let mut closed = false;
        let shown = EdgeSurface {
            id: self.id,
            name,
            pos: pos2(x, screen.top()),
            scrim: Some(t.scrim(e)),
        }
        .show(rt, |ui| {
            let framed = Frame::new()
                .fill(t.role(SIDE_CONTAINER_COLOR))
                .corner_radius(self.modal_shape().corner_radius(&t.shape, dir))
                .show(ui, |ui| {
                    ui.set_min_size(vec2(w, screen.height()));
                    ui.set_max_size(vec2(w, screen.height()));
                    self.body(&mut rt.ui(ui), w, screen.height(), &mut closed, add)
                });
            self.emit(ui.ctx(), framed.response.rect, Role::Dialog, name);
            if self.resizable && e >= 1.0 {
                let resized = self.resize_handle(&rt.ui(ui), framed.response.rect, width);
                if resized.to_bits() != width.to_bits() {
                    ctx.data_mut(|d| d.insert_persisted(width_id, resized));
                }
            }
            framed.inner
        });
        let closed = if closed { Some(Dismiss::Close) } else { shown.close };
        (shown.inner, shown.response, closed)
    }

    fn docked_surface<R>(
        &self,
        ui: &mut MineralUi<'_, Material>,
        e: f32,
        name: &str,
        add: impl FnOnce(&mut MineralUi<'_, Material>) -> R,
    ) -> (Option<R>, egui::Response, Option<Dismiss>) {
        let rt = ui.rt();
        let t = rt.tokens();
        let dir = rt.i18n().direction();
        let width_id = self.id.with("width");
        let width = self.widths.clamp(self.persisted_width(ui.ctx(), width_id));
        let w = width.min(ui.available_rect_before_wrap().width());
        let panel = match (dir, self.edge) {
            (Direction::Ltr, Edge::End) | (Direction::Rtl, Edge::Start) => egui::Panel::right((self.id, "dock")),
            (Direction::Rtl, Edge::End) | (Direction::Ltr, Edge::Start) => egui::Panel::left((self.id, "dock")),
        };
        let mut closed = false;
        let r = panel
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame::NONE)
            .exact_size(w * e)
            .show(ui, |ui| {
                // The sheet keeps its full width and slides: the panel reveals its start edge.
                let shown = ui.max_rect();
                let sheet = Rect::from_min_size(
                    pos2(
                        self.inner_side(dir).start_x(shown, Rangef::new(0.0, 0.0 + w)),
                        shown.top(),
                    ),
                    vec2(w, shown.height()),
                );
                ui.painter().rect_filled(shown, 0, t.role(SIDE_CONTAINER_COLOR));
                let mut sheet_ui = ui.new_child(UiBuilder::new().max_rect(sheet));
                let inner = self.body(&mut rt.ui(&mut sheet_ui), w, sheet.height(), &mut closed, add);
                self.emit(ui.ctx(), shown, Role::Complementary, name);
                if self.resizable && e >= 1.0 {
                    let resized = self.resize_handle(&rt.ui(ui), sheet, width);
                    if resized.to_bits() != width.to_bits() {
                        ui.data_mut(|d| d.insert_persisted(width_id, resized));
                    }
                }
                inner
            });
        (r.inner, r.response, closed.then_some(Dismiss::Close))
    }

    fn persisted_width(&self, ctx: &Context, id: Id) -> f32 {
        if self.resizable {
            ctx.data_mut(|d| *d.get_persisted_mut_or(id, self.width))
        } else {
            self.width
        }
    }

    // Direction whose start is the side the sheet sits at.
    fn side(&self, dir: Direction) -> Direction {
        match self.edge {
            Edge::End => dir.reversed(),
            Edge::Start => dir,
        }
    }

    // Direction whose start is the sheet's inner (content-facing) side.
    fn inner_side(&self, dir: Direction) -> Direction {
        match self.edge {
            Edge::End => dir,
            Edge::Start => dir.reversed(),
        }
    }

    fn modal_shape(&self) -> ShapeRole {
        match self.edge {
            Edge::End => SIDE_MODAL_SHAPE,
            Edge::Start => ShapeRole::CornerLargeEnd,
        }
    }

    // Inner edge handle, the sheet growing toward the content.
    fn resize_handle(&self, ui: &MineralUi<'_, Material>, sheet: Rect, width: f32) -> f32 {
        let rt = ui.rt();
        let dir = rt.i18n().direction();
        let edge = self.inner_side(dir).start_x(sheet, Rangef::point(0.0));
        let hit = MIN_INTERACTIVE_DP / 2.0;
        let rect = Rect::from_x_y_ranges(edge - hit / 2.0..=edge + hit / 2.0, sheet.y_range());
        let range = RangeModel::new(f64::from(self.widths.min), f64::from(self.widths.max));
        let (handle, v) = egui_mineral::splitter::size_handle(
            ui,
            self.id.with("resize"),
            rect,
            match self.edge {
                Edge::End => Track::horizontal(dir).reversed(),
                Edge::Start => Track::horizontal(dir),
            },
            &range,
            f64::from(width),
            1.0,
        );
        crate::containment::drag_handle::paint(ui, &handle, pos2(edge, sheet.center().y), Axis::Horizontal, false);
        let label = crate::tr(rt.i18n(), "mineral-sheet-resize");
        semantic::emit(ui.ctx(), Some(handle.id), rect, || {
            SemanticNode::new(StableId::of(&self.salt, "resize"), Role::Splitter, label)
                .ranged(range.numeric(v), Availability::Enabled)
        });
        #[expect(clippy::cast_possible_truncation, reason = "within the f32 width range")]
        let v = v as f32;
        v
    }

    fn emit(&self, ctx: &Context, rect: Rect, role: Role, name: &str) {
        semantic::emit(ctx, None, rect, || {
            SemanticNode::new(StableId::of(&self.salt, "side-sheet"), role, name)
        });
    }

    fn body<R>(
        &self,
        ui: &mut MineralUi<'_, Material>,
        w: f32,
        h: f32,
        closed: &mut bool,
        add: impl FnOnce(&mut MineralUi<'_, Material>) -> R,
    ) -> Option<R> {
        let rt = ui.rt();
        let t = rt.tokens();
        let dir = rt.i18n().direction();
        let header = self.headline.is_some() || self.close_button;
        if header {
            let (row, _) = ui.allocate_exact_size(vec2(w, SIDE_HEADER_HEIGHT), Sense::hover());
            let close_w = if self.close_button {
                SIDE_CLOSE_END + MIN_INTERACTIVE_DP
            } else {
                0.0
            };
            if let Some(h) = &self.headline {
                let max_w = (w - SIDE_PADDING - close_w).max(0.0);
                let g = parts::type_wrapped(
                    ui.painter(),
                    h,
                    SIDE_HEADLINE_FONT,
                    t.role(SIDE_HEADLINE_COLOR),
                    max_w,
                    1,
                );
                let x = dir.start_x(row, Rangef::new(SIDE_PADDING, SIDE_PADDING + g.size().x));
                ui.painter()
                    .galley(pos2(x, row.center().y - g.size().y / 2.0), g, Color32::PLACEHOLDER);
            }
            if self.close_button {
                let slot = parts::action_slot(dir.reversed(), row, SIDE_CLOSE_END);
                let layout = *ui.layout();
                *closed = egui_mineral::layout::child(ui, self.salt.child("close"), slot, layout, |ui| {
                    parts::close_button(ui, self.salt.child("close"))
                })
                .inner;
            }
        }
        let used = if header { SIDE_HEADER_HEIGHT } else { 0.0 };
        let margin = self.padding;
        if !self.scroll {
            let body = vec2(w, (h - used).max(0.0));
            let inner = (body - margin.sum()).max(Vec2::ZERO);
            return ui
                .allocate_ui(body, |ui| {
                    Frame::new()
                        .inner_margin(margin)
                        .show(ui, |ui| {
                            ui.set_min_size(inner);
                            ui.set_max_size(inner);
                            add(&mut rt.ui(ui))
                        })
                        .inner
                })
                .inner
                .into();
        }
        let mut add = Some(add);
        ScrollArea::vertical()
            .id_salt((self.id, "scroll"))
            .max_height((h - used).max(0.0))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                Frame::new()
                    .inner_margin(margin)
                    .show(ui, |ui| add.take().map(|f| f(&mut rt.ui(ui))))
                    .inner
            })
            .inner
    }
}
