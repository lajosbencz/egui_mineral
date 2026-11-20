use crate::msg;
use std::time::Duration;

use egui::{Align2, Frame, Id, Margin, Panel, Rect, Response, ScrollArea, Ui, UiBuilder, Vec2, pos2, vec2};
use egui_mineral::{
    action::Clicked,
    i18n::Direction,
    kit::{self, state::TempState},
    motion::{self, Visibility},
    overlay::{self, Dismiss},
    semantic::{self, AgentKey, KeyedSalt, Role, SemanticNode},
    widget::MineralUi,
};

use super::dialog::DialogAction;
use super::overlay::{Look, ModalFocus, framed, text};
use crate::spec::ColorToken::{
    ColorBackgroundOverlay, ColorNeutralBackground1, ColorNeutralBackground3, ColorNeutralForeground1,
};
use crate::{
    Fluent, FluentExt, Rt, Size,
    actions::PushButton,
    spec::{
        defaults::drawer as d,
        tokens::{motion as mo, typography as ty},
    },
    theme::Elevation,
    widget::vocab::ButtonAppearance,
};

/// Edge a [`Drawer`] attaches to (`position`); start and end follow the text direction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DrawerPosition {
    /// `start`.
    #[default]
    Start,
    /// `end`.
    End,
    /// `bottom`.
    Bottom,
}

impl DrawerPosition {
    /// Every variant.
    pub const ALL: &'static [Self] = &[Self::Start, Self::End, Self::Bottom];

    const fn vertical(self) -> bool {
        matches!(self, Self::Bottom)
    }
}

/// `size` of a [`Drawer`]: width of a side drawer, height of a bottom one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DrawerSize {
    /// `small`: 320pt.
    Small,
    /// `medium`: 592pt.
    #[default]
    Medium,
    /// `large`: 940pt.
    Large,
    /// `full`: the whole extent.
    Full,
}

impl DrawerSize {
    /// Every variant, smallest first.
    pub const ALL: &'static [Self] = &[Self::Small, Self::Medium, Self::Large, Self::Full];

    const fn motion(self) -> Duration {
        match self {
            Self::Small => d::MOTION_SMALL,
            Self::Medium => d::MOTION_MEDIUM,
            Self::Large => d::MOTION_LARGE,
            Self::Full => d::MOTION_FULL,
        }
    }

    fn points(self, available: f32) -> f32 {
        match self {
            Self::Small => d::SIZE_SMALL,
            Self::Medium => d::SIZE_MEDIUM,
            Self::Large => d::SIZE_LARGE,
            Self::Full => available,
        }
        .min(available)
    }
}

/// Result of showing a [`Drawer`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct DrawerResponse<R> {
    /// Surface while shown.
    pub response: Option<Response>,
    /// Body closure result while shown.
    pub inner: Option<R>,
    /// Footer action clicked this frame.
    pub clicked: Option<Clicked>,
    /// Why it closed this frame.
    pub dismissed: Option<Dismiss>,
}

impl<R> DrawerResponse<R> {
    fn closed() -> Self {
        Self {
            response: None,
            inner: None,
            clicked: None,
            dismissed: None,
        }
    }
}

#[derive(Clone, Copy, Default)]
struct Heights {
    header: f32,
    footer: f32,
}

impl TempState for Heights {}

struct Body<R> {
    inner: R,
    clicked: Option<Clicked>,
    close: bool,
}

/// Fluent `OverlayDrawer` and `InlineDrawer`: header, scrolling body and footer on a screen edge.
#[derive(Clone, Debug)]
#[must_use = "call .show_modal(..) or .show(..)"]
pub struct Drawer {
    salt: KeyedSalt,
    key: AgentKey,
    id: Id,
    label: String,
    title: Option<String>,
    close_button: bool,
    position: DrawerPosition,
    size: DrawerSize,
    footer: Vec<DialogAction>,
    separator: bool,
}

impl Drawer {
    /// `label` is the accessible name. Agent key `<id_salt>-drawer`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        let salt: KeyedSalt = id_salt.into();
        Self {
            id: Id::unique(("mineral::fluent-drawer", salt.id_salt())),
            salt,
            key: egui_mineral::agent_key!("drawer"),
            label: label.into(),
            title: None,
            close_button: false,
            position: DrawerPosition::Start,
            size: DrawerSize::Medium,
            footer: Vec::new(),
            separator: true,
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-drawer` key.
        agent_key(key)
    }

    /// `DrawerHeaderTitle` heading.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Close button in the header; default `false`.
    pub fn close_button(mut self, close_button: bool) -> Self {
        self.close_button = close_button;
        self
    }

    /// Default [`DrawerPosition::Start`].
    pub fn position(mut self, position: DrawerPosition) -> Self {
        self.position = position;
        self
    }

    /// Default [`DrawerSize::Medium`].
    pub fn size(mut self, size: DrawerSize) -> Self {
        self.size = size;
        self
    }

    /// Appends a `DrawerFooter` button.
    pub fn footer_action(mut self, action: impl Into<DialogAction>) -> Self {
        self.footer.push(action.into());
        self
    }

    /// `separator` of an inline drawer; default `true`.
    pub fn separator(mut self, separator: bool) -> Self {
        self.separator = separator;
        self
    }

    /// Overlay drawer over a backdrop while `open`; Escape, a backdrop press and the close button clear it.
    pub fn show_modal<R>(
        mut self,
        rt: &Rt,
        open: &mut bool,
        add: impl FnOnce(&mut MineralUi<'_, Fluent>) -> R,
    ) -> DrawerResponse<R> {
        let ctx = rt.ctx();
        self.id = kit::scoped_id(rt.core(), self.id);
        ModalFocus::scope(ctx, self.id.with("focus"), open, |_, open| {
            let Some(e) = self.progress(rt, *open) else {
                return DrawerResponse::closed();
            };
            let t = rt.tokens();
            let screen = ctx.content_rect();
            let dir = rt.i18n().direction();
            let extent = self.extent(screen.size());
            let (align, hidden) = self.edge(dir, extent, e);
            let mut area = kit::layer::modal_area(rt.core(), self.id);
            area = area.anchor(align, hidden);
            let modal = egui::Modal {
                area,
                backdrop_color: t.color(ColorBackgroundOverlay).gamma_multiply(e),
                frame: Some(Frame::NONE),
            };
            let r = modal.show(ctx, |ui| {
                let look = Look {
                    fill: t.color(ColorNeutralBackground1),
                    radius: 0.0,
                    elevation: Elevation::E64,
                };
                let shown = framed(ui, &t, &look, Margin::ZERO, |ui| self.body(rt, ui, extent, add));
                semantic::emit(ui.ctx(), None, shown.response.rect, || {
                    SemanticNode::new(self.key.id(&self.salt), Role::Dialog, &self.label)
                });
                shown.inner
            });
            let dismissed = overlay::modal(&r).or(r.inner.close.then_some(Dismiss::Close));
            if *open && (dismissed.is_some() || r.inner.clicked.is_some()) {
                *open = false;
            }
            DrawerResponse {
                response: Some(r.response),
                inner: Some(r.inner.inner),
                clicked: r.inner.clicked,
                dismissed,
            }
        })
    }

    /// Inline drawer docked in the layout of `ui`, beside what is added after it; the close button clears `open`.
    /// Add it before the central content, like an `egui::Panel`.
    pub fn show<R>(
        mut self,
        ui: &mut MineralUi<'_, Fluent>,
        open: &mut bool,
        add: impl FnOnce(&mut MineralUi<'_, Fluent>) -> R,
    ) -> DrawerResponse<R> {
        let rt = ui.rt();
        self.id = kit::scoped_id(rt.core(), self.id);
        let Some(e) = self.progress(rt, *open) else {
            return DrawerResponse::closed();
        };
        let t = rt.tokens();
        let dir = rt.i18n().direction();
        let avail = ui.available_size();
        let extent = self.extent(avail);
        let (w, h) = (extent.x, extent.y);
        let panel_id = (self.id, "dock");
        let panel = match (self.position, dir) {
            (DrawerPosition::Start, Direction::Ltr) | (DrawerPosition::End, Direction::Rtl) => Panel::left(panel_id),
            (DrawerPosition::End, Direction::Ltr) | (DrawerPosition::Start, Direction::Rtl) => Panel::right(panel_id),
            (DrawerPosition::Bottom, _) => Panel::bottom(panel_id),
        };
        let left = matches!(
            (self.position, dir),
            (DrawerPosition::Start, Direction::Ltr) | (DrawerPosition::End, Direction::Rtl)
        );
        let grow = if self.position.vertical() { h } else { w } * e;
        let r = panel
            .resizable(false)
            .show_separator_line(false)
            .frame(Frame::NONE)
            .exact_size(grow)
            .show(ui, |ui| {
                let shown = ui.max_rect();
                ui.set_clip_rect(shown);
                let full = match self.position {
                    DrawerPosition::Bottom => Rect::from_min_size(pos2(shown.left(), shown.bottom() - h), extent),
                    _ if left => Rect::from_min_size(shown.min, extent),
                    _ => Rect::from_min_size(pos2(shown.right() - w, shown.top()), extent),
                };
                ui.painter().rect_filled(shown, 0, t.color(ColorNeutralBackground1));
                let mut child = ui.new_child(UiBuilder::new().max_rect(full));
                let body = self.body(rt, &mut child, extent, add);
                if self.separator && e >= 1.0 {
                    let line = d::SEPARATOR_WIDTH;
                    let edge = match self.position {
                        DrawerPosition::Bottom => Rect::from_min_size(shown.min, vec2(w, line)),
                        _ if left => Rect::from_min_size(pos2(shown.right() - line, shown.top()), vec2(line, h)),
                        _ => Rect::from_min_size(shown.min, vec2(line, h)),
                    };
                    ui.painter().rect_filled(edge, 0, t.color(ColorNeutralBackground3));
                }
                semantic::emit(ui.ctx(), None, shown, || {
                    SemanticNode::new(self.key.id(&self.salt), Role::Complementary, &self.label)
                });
                body
            });
        let dismissed = r.inner.close.then_some(Dismiss::Close);
        if dismissed.is_some() {
            *open = false;
        }
        DrawerResponse {
            response: Some(r.response),
            inner: Some(r.inner.inner),
            clicked: r.inner.clicked,
            dismissed,
        }
    }

    fn progress(&self, rt: &Rt, open: bool) -> Option<f32> {
        let visibility = if open { Visibility::Visible } else { Visibility::Hidden };
        let k = motion::presence(rt.core(), self.id.with("presence"), visibility, self.size.motion())?;
        Some(if open {
            mo::CURVE_DECELERATE_MID.ease(k)
        } else {
            mo::CURVE_ACCELERATE_MIN.ease(k)
        })
    }

    fn extent(&self, available: Vec2) -> Vec2 {
        if self.position.vertical() {
            vec2(available.x, self.size.points(available.y))
        } else {
            vec2(self.size.points(available.x), available.y)
        }
    }

    fn edge(&self, dir: Direction, extent: Vec2, e: f32) -> (Align2, Vec2) {
        let start_left = dir == Direction::Ltr;
        let left = (self.position == DrawerPosition::Start) == start_left;
        let slide = 1.0 - e;
        match self.position {
            DrawerPosition::Bottom => (Align2::CENTER_BOTTOM, vec2(0.0, slide * extent.y)),
            _ if left => (Align2::LEFT_CENTER, vec2(-slide * extent.x, 0.0)),
            _ => (Align2::RIGHT_CENTER, vec2(slide * extent.x, 0.0)),
        }
    }

    fn body<R>(
        &self,
        rt: &Rt,
        ui: &mut Ui,
        extent: Vec2,
        add: impl FnOnce(&mut MineralUi<'_, Fluent>) -> R,
    ) -> Body<R> {
        let t = rt.tokens();
        let dir = rt.i18n().direction();
        ui.set_min_size(extent);
        ui.set_max_size(extent);
        ui.spacing_mut().item_spacing = Vec2::ZERO;
        ui.visuals_mut().override_text_color = Some(t.color(ColorNeutralForeground1));
        let heights_id = self.id.with("heights");
        let mut heights = Heights::load(rt.ctx(), heights_id);
        let mut close = false;
        let mut clicked = None;
        let top = ui.cursor().top();
        if self.title.is_some() || self.close_button {
            let width = extent.x - 2.0 * d::PADDING_X;
            ui.add_space(d::HEADER_PADDING_TOP);
            ui.horizontal(|ui| {
                ui.add_space(d::PADDING_X);
                ui.allocate_ui_with_layout(vec2(width, 0.0), dir.row(), |ui| {
                    let reserve = if self.close_button { 32.0 + d::FOOTER_GAP } else { 0.0 };
                    ui.scope_builder(UiBuilder::new().layout(dir.column()), |ui| {
                        ui.set_max_width((width - reserve).max(0.0));
                        if let Some(title) = &self.title {
                            let format = (
                                ty::FONT_SIZE_BASE_500,
                                ty::LINE_HEIGHT_BASE_500,
                                ty::FONT_WEIGHT_SEMIBOLD,
                            );
                            text(ui, rt, title, format, t.color(ColorNeutralForeground1));
                        }
                    });
                    if self.close_button {
                        ui.with_layout(dir.reversed().row(), |ui| {
                            let label = crate::tr(rt.i18n(), msg::DRAWER_CLOSE);
                            let b = PushButton::new(self.salt.child("close"), label)
                                .appearance(ButtonAppearance::Subtle)
                                .size(Size::Medium)
                                .icon(rt.icons().dismiss)
                                .label_visible(false);
                            close = rt.ui(ui).mineral(b).response().clicked();
                        });
                    }
                });
            });
            ui.add_space(d::HEADER_PADDING_BOTTOM);
        }
        let header = ui.cursor().top() - top;
        let room = (extent.y - header - heights.footer).max(0.0);
        let first = header == 0.0;
        let last = self.footer.is_empty();
        let inner = ScrollArea::vertical()
            .id_salt(self.id.with("scroll"))
            .max_height(room)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let margin = Margin::symmetric(d::PADDING_X as i8, 0);
                Frame::NONE
                    .inner_margin(margin)
                    .show(ui, |ui| {
                        ui.set_width(extent.x - 2.0 * d::PADDING_X);
                        if first {
                            ui.add_space(d::BODY_EDGE_PADDING);
                        }
                        let inner = add(&mut rt.ui(ui));
                        if last {
                            ui.add_space(d::BODY_EDGE_PADDING);
                        }
                        inner
                    })
                    .inner
            })
            .inner;
        let before = ui.cursor().top();
        if !self.footer.is_empty() {
            ui.add_space(d::FOOTER_PADDING_TOP);
            ui.horizontal(|ui| {
                ui.add_space(d::PADDING_X);
                ui.spacing_mut().item_spacing.x = d::FOOTER_GAP;
                for (i, a) in self.footer.iter().enumerate() {
                    let b = PushButton::new(self.salt.child(("footer", i)), &a.label)
                        .appearance(a.appearance)
                        .disabled(a.disabled);
                    if rt.ui(ui).mineral(b).response().clicked() {
                        clicked = Some(a.clicked(i));
                    }
                }
            });
            ui.add_space(d::FOOTER_PADDING_BOTTOM);
        }
        let next = Heights {
            header,
            footer: ui.cursor().top() - before,
        };
        if (next.header - heights.header).abs() > 0.5 || (next.footer - heights.footer).abs() > 0.5 {
            heights = next;
            if rt.ctx().current_pass_index() == 0 {
                rt.ctx().request_discard("drawer layout changed");
            }
        }
        heights.store(rt.ctx(), heights_id);
        Body { inner, clicked, close }
    }
}
