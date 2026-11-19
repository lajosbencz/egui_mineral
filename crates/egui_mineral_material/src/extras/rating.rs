use egui_mineral::semantic::AgentKey;
use std::num::NonZeroU8;

use crate::Material;
use crate::MaterialExt;
use egui::{CursorIcon, Pos2, Rangef, Rect, Sense, Vec2, pos2, vec2};
use egui_mineral::i18n::I18n;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::Availability;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    i18n::Direction,
    layout::{Constraints, Measure},
    range::{RangeModel, Track},
    semantic::{Action, Role, SemanticNode},
    widget::{MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::widget::{
    common::{self, Interaction},
    indication,
};
use crate::{
    Icon,
    spec::{ShapeRole, tokens::standard_icon_button as m},
};
use crate::{
    Size,
    actions::{IconButtonWidth, icon_button},
};

// No MD3 counterpart

/// Star rating, optionally half steps and clearable; non-MD3 (MUI `Rating` pattern).
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Rating<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    value: &'a mut f64,
    max: NonZeroU8,
    half: bool,
    size: Size,
    icon: Option<Icon>,
    clearable: bool,
    read_only: bool,
    disabled: bool,
}

impl<'a> Rating<'a> {
    /// Five stars bound to `value`, 0 for none. Agent key `<id_salt>-rating`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, value: &'a mut f64) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            key: AgentKey::Kind("rating"),
            salt: id_salt,
            label: label.into(),
            value,
            max: NonZeroU8::MIN.saturating_add(4),
            half: false,
            size: Size::Small,
            icon: None,
            clearable: true,
            read_only: false,
            disabled: false,
        }
    }

    /// Icon count; default 5.
    pub fn count(mut self, count: NonZeroU8) -> Self {
        self.max = count;
        self
    }

    /// Half-star steps.
    pub fn half(mut self, half: bool) -> Self {
        self.half = half;
        self
    }

    /// Icon button size of each star.
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    fn cell(&self) -> (Vec2, f32) {
        icon_button::metrics(self.size, IconButtonWidth::Narrow)
    }

    /// Star icon; default [`crate::IconSet::star`].
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Clicking the current value clears to 0; default `true`.
    pub fn clearable(mut self, clearable: bool) -> Self {
        self.clearable = clearable;
        self
    }

    /// Shows the value without accepting input; the node stays enabled.
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    crate::widget::setters! {
        /// Greys out the stars and ignores input.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-rating` agent key.
        agent_key(key)
    }

    fn model(&self) -> RangeModel {
        RangeModel::new(0.0, f64::from(self.max.get())).step(if self.half { 0.5 } else { 1.0 })
    }

    fn interactive(&self) -> bool {
        !self.read_only && !self.disabled
    }

    fn value_at(&self, rect: Rect, p: Pos2, dir: Direction) -> f64 {
        egui_mineral::kit::rating::value_at(
            rect,
            p,
            dir,
            self.cell().0.x,
            self.max.get(),
            egui_mineral::kit::rating::Step::half_if(self.half),
        )
    }

    fn text(&self, i18n: &egui_mineral::i18n::I18n, v: f64) -> String {
        let f = i18n.format();
        let digits = i16::from(self.half);
        i18n.tr(
            "mineral-rating-value",
            &[
                ("value", f.number_lossy(v, Some(digits)).into()),
                ("max", f.integer(i64::from(self.max.get())).into()),
            ],
        )
    }
}

impl MineralWidget for Rating<'_> {
    type Kit = Material;
    type State = ();
    type Output = ();

    egui_mineral::kit::widget_meta!(salt, enabled);

    fn sense(&self) -> Sense {
        if self.interactive() {
            Sense::click()
        } else {
            Sense::hover()
        }
    }

    fn layout(&self, _ui: &MineralUi<'_, Material>, c: Constraints, (): &Self::State) -> Measure {
        let cell = self.cell().0;
        c.constrain(vec2(f32::from(self.max.get()) * cell.x, cell.y)).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Material>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let t = rt.tokens();
        let dir = rt.i18n().direction();
        let rect = frame.zones.visual;
        let r = &frame.response;
        let model = self.model();
        let before = *self.value;
        let mut v = model.snap(*self.value);
        let mut hover = None;

        if self.interactive() {
            if let Some(p) = r.hover_pos().filter(|p| frame.zones.hit.contains(*p)) {
                hover = Some(self.value_at(rect, p, dir));
                ctx.set_cursor_icon(CursorIcon::PointingHand);
            }
            let press = frame.press_pos();
            if let Some(p) = press {
                let n = self.value_at(rect, p, dir);
                v = if self.clearable && (n - v).abs() < f64::EPSILON {
                    0.0
                } else {
                    n
                };
                r.request_focus();
            }
            v = model.drive(rt.core(), r.id, v, Track::horizontal(dir));
        }
        if (v - model.snap(before)).abs() > f64::EPSILON {
            *self.value = v;
        }

        let shown = hover.unwrap_or(v) as f32;
        let (cell, size) = self.cell();
        let (full, empty) = if self.disabled {
            let d = common::with_alpha(t.role(m::DISABLED_COLOR), m::DISABLED_OPACITY);
            (d, d)
        } else {
            (t.role(m::SELECTED_COLOR), t.role(m::UNSELECTED_COLOR))
        };
        let star = |i: u8| {
            let from = f32::from(i) * cell.x;
            let x = dir.start_x(rect, Rangef::new(from, from + cell.x));
            Rect::from_min_size(pos2(x, rect.top()), cell)
        };
        // The state layer marks the star a click or arrow key acts from.
        let marked = if hover.is_some() { shown } else { v as f32 };
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss, reason = "0..max")]
        let at = (marked.ceil() as u8).clamp(1, self.max.get()) - 1;
        let layer = if f32::from(at) < shown { full } else { empty };
        let shape = ShapeRole::CornerFull.corner_radius(&t.shape, dir);
        indication::paint(
            ui,
            r,
            star(at),
            shape,
            layer,
            Interaction::of(r, Availability::enabled_if(self.interactive())),
            &t,
        );
        let icon = self.icon.unwrap_or(ui.icons().star);
        let p = ui.painter();
        for i in 0..self.max.get() {
            let star = Rect::from_center_size(star(i).center(), Vec2::splat(size));
            let part = (shown - f32::from(i)).clamp(0.0, 1.0);
            if part < 1.0 {
                common::icon(p, star.center(), Icon::single(icon.regular()), size, empty);
            }
            if part > 0.0 {
                let w = star.width() * part;
                let clip = Rect::from_min_size(
                    pos2(dir.start_x(star, Rangef::new(0.0, 0.0 + w)), star.top()),
                    vec2(w, star.height()),
                );
                common::icon(
                    &p.with_clip_rect(clip),
                    star.center(),
                    Icon::single(icon.filled()),
                    size,
                    full,
                );
            }
        }
        WidgetResponse::new(r.clone(), ()).with_changed((before - *self.value).abs() > f64::EPSILON)
    }

    fn semantics(&self, i18n: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let v = *self.value;
        let mut n = SemanticNode::new(self.key.id(&self.salt), Role::Slider, &self.label).value(self.text(i18n, v));
        n.state.disabled = self.disabled;
        n.state.read_only = self.read_only;
        n.state.range = Some(self.model().numeric(v));
        if !self.disabled {
            n.actions.push(Action::Focus);
        }
        if self.interactive() {
            n.actions
                .extend([Action::SetValue, Action::Increment, Action::Decrement]);
        }
        Some(n)
    }
}
