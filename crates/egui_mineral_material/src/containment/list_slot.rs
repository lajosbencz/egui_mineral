use std::sync::Arc;

use egui::{Color32, Galley, ImageSource, Painter, Pos2, Rect, Response, Vec2, vec2};
use egui_mineral::{
    select::Check,
    semantic::{self, Role, SemanticNode, StableId},
    widget::{Availability, MineralUi},
};

use crate::selection::controls::{self, Control, SwitchLook};
use crate::spec::tokens::list;
use crate::widget::{common, parts, row_parts};
use crate::{Icon, Leading, LeadingPaint, Material, Rt, tokens::Tokens};

/// Toggle control in a [`ListSlot`]; the row reports its toggles and Space on the focused row
/// toggles the leading one first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ListControl {
    /// Checkbox in this state.
    Checkbox(Check),
    /// Radio button, selected when `true`.
    Radio(bool),
    /// Switch, on when `true`.
    Switch(bool),
}

impl ListControl {
    fn control(self) -> Control {
        match self {
            Self::Checkbox(_) => Control::Checkbox,
            Self::Radio(_) => Control::Radio,
            Self::Switch(_) => Control::Switch,
        }
    }

    fn role(self) -> Role {
        match self {
            Self::Checkbox(_) => Role::CheckBox,
            Self::Radio(_) => Role::RadioButton,
            Self::Switch(_) => Role::Switch,
        }
    }

    pub(crate) fn kind(self) -> &'static str {
        match self {
            Self::Checkbox(_) => "checkbox",
            Self::Radio(_) => "radio",
            Self::Switch(_) => "switch",
        }
    }

    pub(crate) fn state(self) -> Check {
        match self {
            Self::Checkbox(c) => c,
            Self::Radio(on) | Self::Switch(on) => on.into(),
        }
    }

    // State shown in the pass it toggles, before the app applies it.
    fn shown(self, toggled: bool) -> Check {
        match (self, toggled) {
            (_, false) => self.state(),
            (Self::Radio(_), true) => Check::Checked,
            (_, true) if self.state() == Check::Checked => Check::Unchecked,
            (_, true) => Check::Checked,
        }
    }

    // Width along the row, and the gap before the next part.
    fn extent(self, rt: &Rt) -> (f32, f32) {
        match self {
            // The minimum interactive slot pads the glyph.
            Self::Checkbox(_) | Self::Radio(_) => (row_parts::control_slot(rt), 0.0),
            Self::Switch(_) => (
                Control::Switch.canvas().x,
                crate::spec::defaults::list::LEADING_CONTENT_END_PADDING,
            ),
        }
    }
}

/// Non-interactive content of a [`ListSlot`].
#[derive(Clone, Debug)]
pub enum ListVisual {
    /// Icon or app paint in the 24dp slot.
    Leading(Leading),
    /// Letter in a 40dp circle.
    Avatar(char),
    /// Image in the 56dp square (MD3 leading image); egui image loaders resolve the source.
    Image(ImageSource<'static>),
    /// Short label, e.g. a count.
    Text(String),
}

impl ListVisual {
    fn size(&self, rt: &Rt, p: &Painter) -> Vec2 {
        match self {
            Self::Leading(_) => Vec2::splat(list::ITEM_LEADING_ICON_SIZE),
            Self::Avatar(_) => Vec2::splat(list::ITEM_LEADING_AVATAR_SIZE),
            Self::Image(_) => vec2(list::ITEM_LEADING_IMAGE_WIDTH, list::ITEM_LEADING_IMAGE_HEIGHT),
            Self::Text(s) => text(rt, p, s, Color32::PLACEHOLDER).size(),
        }
    }
}

fn text(rt: &Rt, p: &Painter, s: &str, color: Color32) -> Arc<Galley> {
    parts::type_galley(rt.core(), p, s, list::ITEM_TRAILING_SUPPORTING_TEXT_FONT, color, None)
}

/// Leading or trailing content of a [`ListItem`](super::ListItem) or a
/// `TreeView` row: an optional control at the outer edge and an
/// optional visual next to it.
#[derive(Clone, Debug, Default)]
#[must_use]
pub struct ListSlot {
    control: Option<ListControl>,
    visual: Option<ListVisual>,
}

impl ListSlot {
    /// Empty slot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replaces the control.
    pub fn control(mut self, control: ListControl) -> Self {
        self.control = Some(control);
        self
    }

    /// Replaces the visual.
    pub fn visual(mut self, visual: ListVisual) -> Self {
        self.visual = Some(visual);
        self
    }

    /// [`ListControl::Checkbox`].
    pub fn checkbox(self, state: impl Into<Check>) -> Self {
        self.control(ListControl::Checkbox(state.into()))
    }

    /// [`ListControl::Radio`].
    pub fn radio(self, selected: bool) -> Self {
        self.control(ListControl::Radio(selected))
    }

    /// [`ListControl::Switch`].
    pub fn switch(self, on: bool) -> Self {
        self.control(ListControl::Switch(on))
    }

    /// [`ListVisual::Leading`] icon.
    pub fn icon(self, icon: impl Into<Icon>) -> Self {
        self.visual(ListVisual::Leading(Leading::from(icon.into())))
    }

    /// [`ListVisual::Avatar`].
    pub fn avatar(self, letter: char) -> Self {
        self.visual(ListVisual::Avatar(letter))
    }

    /// [`ListVisual::Image`].
    pub fn image(self, source: impl Into<ImageSource<'static>>) -> Self {
        self.visual(ListVisual::Image(source.into()))
    }

    /// [`ListVisual::Leading`] app paint.
    pub fn paint(self, paint: LeadingPaint) -> Self {
        self.visual(ListVisual::Leading(paint.into()))
    }

    /// [`ListVisual::Text`].
    pub fn text(self, text: impl Into<String>) -> Self {
        self.visual(ListVisual::Text(text.into()))
    }

    pub(crate) fn control_ref(&self) -> Option<ListControl> {
        self.control
    }

    pub(crate) fn visual_ref(&self) -> Option<&ListVisual> {
        self.visual.as_ref()
    }

    // `place(offset, width, height)` maps an offset from the outer edge to a rect.
    pub(crate) fn layout(
        &self,
        rt: &Rt,
        p: &Painter,
        max_h: f32,
        mut place: impl FnMut(f32, f32, f32) -> Rect,
    ) -> SlotLayout {
        let mut at = 0.0;
        let control = self.control.map(|c| {
            let (w, gap) = c.extent(rt);
            let center = place(at, w, w.min(max_h)).center();
            at += w + gap;
            center
        });
        let visual = self.visual.as_ref().map(|v| {
            let s = v.size(rt, p);
            // Rows shorter than the visual (dense trees) scale it down.
            let k = (max_h / s.y).min(1.0);
            let r = place(at, s.x * k, s.y * k);
            at += s.x * k;
            r
        });
        SlotLayout {
            control,
            visual,
            extent: at,
        }
    }
}

pub(crate) struct SlotLayout {
    pub control: Option<Pos2>,
    pub visual: Option<Rect>,
    pub extent: f32,
}

impl From<ListControl> for ListSlot {
    fn from(control: ListControl) -> Self {
        Self::new().control(control)
    }
}

impl From<ListVisual> for ListSlot {
    fn from(visual: ListVisual) -> Self {
        Self::new().visual(visual)
    }
}

impl From<Icon> for ListSlot {
    fn from(icon: Icon) -> Self {
        Self::new().icon(icon)
    }
}

impl From<crate::MaterialIcon> for ListSlot {
    fn from(icon: crate::MaterialIcon) -> Self {
        Self::from(Icon::from(icon))
    }
}

impl From<LeadingPaint> for ListSlot {
    fn from(paint: LeadingPaint) -> Self {
        Self::new().paint(paint)
    }
}

pub(crate) fn paint_visual(
    ui: &MineralUi<'_, Material>,
    visual: &ListVisual,
    rect: Rect,
    disabled: bool,
    color: Color32,
    t: &Tokens,
) {
    let rt = ui.rt();
    let p = ui.painter();
    match visual {
        ListVisual::Leading(l) => l.paint(p, rect.center(), rect.height(), color),
        ListVisual::Avatar(ch) => {
            let (bg, fg) = if disabled {
                (common::disabled_container(t), color)
            } else {
                (
                    t.role(list::ITEM_LEADING_AVATAR_COLOR),
                    t.role(list::ITEM_LEADING_AVATAR_LABEL_COLOR),
                )
            };
            p.circle_filled(rect.center(), rect.height() / 2.0, bg);
            let g = parts::type_galley(
                rt.core(),
                p,
                &ch.to_uppercase().collect::<String>(),
                list::ITEM_LEADING_AVATAR_LABEL_FONT,
                fg,
                None,
            );
            p.galley(rect.center() - g.size() / 2.0, g, fg);
        }
        ListVisual::Image(source) => {
            let r = list::ITEM_LEADING_IMAGE_SHAPE.corner_radius(&t.shape, rt.i18n().direction());
            let tint = if disabled {
                common::dim(Color32::WHITE, t)
            } else {
                Color32::WHITE
            };
            egui::Image::new(source.clone())
                .corner_radius(r)
                .tint(tint)
                .paint_at(ui, rect);
        }
        ListVisual::Text(s) => {
            let g = text(rt, p, s, color);
            p.galley(rect.left_center() - vec2(0.0, g.size().y / 2.0), g, color);
        }
    }
}

// Painted with the state it toggles to in the same pass.
pub(crate) struct RowControl {
    pub control: ListControl,
    pub target: Response,
    pub center: Pos2,
    pub toggled: bool,
}

impl RowControl {
    pub(crate) fn new(
        ui: &MineralUi<'_, Material>,
        row: &Response,
        salt: impl egui::AsIdSalt,
        control: ListControl,
        center: Pos2,
        clip: Rect,
        disabled: bool,
    ) -> Self {
        let hit = control.control().node(ui.rt(), center).intersect(clip);
        let target = row_parts::control_target(ui, row, salt, hit, disabled);
        let toggled = target.clicked();
        Self {
            control,
            target,
            center,
            toggled,
        }
    }

    pub(crate) fn paint(&self, ui: &MineralUi<'_, Material>, disabled: bool, t: &Tokens) {
        let (id, center, enabled) = (self.target.id, self.center, !disabled);
        let control = self.control.control();
        let shown = self.control.shown(self.toggled);
        let node = match control {
            Control::Checkbox => {
                controls::paint_checkbox(ui, id, center, shown, false, enabled, t);
                control.node(ui.rt(), center)
            }
            Control::Radio => {
                controls::paint_radio(ui, id, center, shown == Check::Checked, enabled, t);
                control.node(ui.rt(), center)
            }
            Control::Switch => controls::paint_switch(
                ui,
                id,
                Rect::from_center_size(center, control.canvas()),
                SwitchLook {
                    on: shown == Check::Checked,
                    enabled,
                    pressed: self.target.is_pointer_button_down_on(),
                    ..SwitchLook::default()
                },
                t,
            ),
        };
        control.indication(ui, &self.target, node, disabled, t);
    }

    pub(crate) fn emit(&self, ctx: &egui::Context, key: StableId, name: &str, depth: usize, disabled: bool) {
        semantic::emit(ctx, Some(self.target.id), self.target.rect, || {
            let mut n =
                SemanticNode::new(key, self.control.role(), name).pressable(Availability::disabled_if(disabled));
            n.state.checked = Some(self.control.state());
            n.depth = depth;
            n
        });
    }
}
