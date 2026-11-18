use crate::Material;
use crate::MaterialExt;
use crate::Rt;
use egui::{Frame, Id, Pos2, Vec2, vec2};
use egui_mineral::kit::state::TempState;
use egui_mineral::overlay::OverlayResponse;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    kit,
    overlay::Dismiss,
    semantic::{self, Role, SemanticNode, StableId},
};

use crate::widget::parts;
use crate::{
    adaptive::MIN_INTERACTIVE,
    spec::{defaults, tokens::dialog},
};

// No MD3 counterpart

// Dialog minimum width, square
const DEFAULT_SIZE: Vec2 = Vec2::splat(defaults::dialog::MIN_WIDTH);

/// Movable, optionally resizable non-modal panel; non-MD3.
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct FloatingPanel {
    salt: KeyedSalt,
    title: String,
    default_pos: Option<Pos2>,
    default_size: Vec2,
    min_size: Option<Vec2>,
    resizable: bool,
}

impl FloatingPanel {
    /// `title` is the header text and accessible name; resizable by default. Agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>, title: impl Into<String>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            title: title.into(),
            default_pos: None,
            default_size: DEFAULT_SIZE,
            min_size: None,
            resizable: true,
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    /// First-show position in points; the user moves it after.
    pub fn default_pos(mut self, default_pos: impl Into<Pos2>) -> Self {
        self.default_pos = Some(default_pos.into());
        self
    }

    /// First-show size in points.
    pub fn default_size(mut self, default_size: impl Into<Vec2>) -> Self {
        self.default_size = default_size.into();
        self
    }

    /// Defaults to the dialog's minimum width and the height of the header.
    pub fn min_size(mut self, min_size: impl Into<Vec2>) -> Self {
        self.min_size = Some(min_size.into());
        self
    }

    /// Edge drags resize; default `true`.
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Movable window while `open`; the close button or Escape with focus inside clears `open`.
    pub fn show<R>(
        self,
        rt: &Rt,
        open: &mut bool,
        add: impl FnOnce(&mut MineralUi<'_, Material>) -> R,
    ) -> OverlayResponse<R> {
        let ctx = rt.ctx();
        let id = egui_mineral::kit::scoped_id(rt.core(), Id::unique(("mineral::floating_panel", self.salt.id_salt())));
        let inside_id = id.with("focus-inside");
        if !*open {
            bool::remove(ctx, inside_id);
            return OverlayResponse::closed();
        }
        let t = rt.tokens();
        let dir = rt.i18n().direction();
        let r = dialog::CONTAINER_SHAPE.corner_radius(&t.shape, dir);
        let pad = defaults::dialog::PADDING;
        let frame = Frame::new()
            .fill(t.role(dialog::CONTAINER_COLOR))
            .corner_radius(r)
            .shadow(t.elevation_shadow(dialog::CONTAINER_ELEVATION))
            .inner_margin(pad);
        let header = MIN_INTERACTIVE.points(rt.core());
        let min_size = self
            .min_size
            .unwrap_or(vec2(defaults::dialog::MIN_WIDTH, header + 2.0 * pad));
        let mut window = egui::Window::new(&self.title)
            .id(id)
            .title_bar(false)
            .frame(frame)
            .default_size(self.default_size)
            .min_size(min_size)
            .resizable(self.resizable)
            .collapsible(false)
            .constrain(true);
        if let Some(p) = self.default_pos {
            window = window.default_pos(p);
        }
        let end = dir.reversed().row();
        let mut dismissed = None;
        let shown = window.show(ctx, |ui| {
            let spacing = std::mem::take(&mut ui.spacing_mut().item_spacing);
            ui.horizontal(|ui| {
                ui.set_min_height(header);
                ui.with_layout(end, |ui| {
                    if parts::close_button(&mut rt.ui(ui), self.salt.child("close")) {
                        dismissed = Some(Dismiss::Close);
                    }
                    ui.with_layout(dir.row(), |ui| {
                        parts::type_label(
                            ui,
                            &self.title,
                            dialog::HEADLINE_FONT,
                            t.role(dialog::HEADLINE_COLOR),
                            1,
                        );
                    });
                });
            });
            ui.add_space(defaults::dialog::TITLE_BOTTOM_PADDING);
            egui::ScrollArea::vertical()
                .id_salt(id.with("scroll"))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = spacing;
                    ui.with_layout(dir.column(), |ui| add(&mut rt.ui(ui))).inner
                })
                .inner
        });
        let Some(shown) = shown else {
            return OverlayResponse::closed();
        };
        let rect = shown.response.rect;
        let focus_inside = ctx
            .memory(egui::Memory::focused)
            .and_then(|f| ctx.read_response(f))
            .is_some_and(|f| f.layer_id == shown.response.layer_id);
        // egui drops focus on Escape before the pass runs, so the previous pass's focus decides.
        let was_inside = bool::load(ctx, inside_id);
        if dismissed.is_none() && (focus_inside || was_inside) && kit::overlay::consume_escape(ctx) {
            dismissed = Some(Dismiss::Escape);
        }
        (focus_inside && dismissed.is_none()).store(ctx, inside_id);
        let panel_id = shown.response.id;
        semantic::emit(ctx, Some(panel_id), rect, || {
            SemanticNode::new(StableId::of(&self.salt, "panel"), Role::Dialog, &self.title)
        });
        if dismissed.is_some() {
            *open = false;
        }
        OverlayResponse::shown(shown.response, shown.inner, dismissed)
    }
}
