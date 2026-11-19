use crate::Material;
use crate::MaterialExt;
use crate::actions::button::{ContainerVariant, Space};
use egui_mineral::i18n::I18n;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;
use std::f32::consts::PI;

use egui::{KeyboardShortcut, Response, vec2};
use egui_mineral::widget::Availability;
use egui_mineral::{
    layout::{Constraints, Measure, MinTarget},
    semantic::{Role, SemanticNode, StableId},
    widget::{MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::selection::menu::Menu;
use crate::widget::{
    common::{self},
    parts,
};
use crate::{
    Icon, motion,
    spec::{
        ShapeScale, defaults,
        tokens::{
            split_button_large, split_button_medium, split_button_small, split_button_x_large, split_button_x_small,
            state,
        },
    },
};
use crate::{
    Size,
    actions::button::{self, ButtonVariant, Joined, PushButton},
};

#[derive(Clone, Copy, Debug)]
struct Metrics {
    height: f32,
    gap: f32,
    lead: Space,
    trail: Space,
    trail_icon: f32,
    inner: ShapeScale,
    inner_pressed: ShapeScale,
}

macro_rules! metrics {
    ($m:ident) => {
        Metrics {
            height: $m::CONTAINER_HEIGHT,
            gap: $m::BETWEEN_SPACE,
            lead: Space {
                leading: $m::LEADING_BUTTON_LEADING_SPACE,
                trailing: $m::LEADING_BUTTON_TRAILING_SPACE,
            },
            trail: Space {
                leading: $m::TRAILING_BUTTON_LEADING_SPACE,
                trailing: $m::TRAILING_BUTTON_TRAILING_SPACE,
            },
            trail_icon: $m::TRAILING_ICON_SIZE,
            inner: $m::INNER_CORNER_CORNER_SIZE,
            inner_pressed: $m::INNER_PRESSED_CORNER_CORNER_SIZE,
        }
    };
}

// Compose SplitButtonDefaults.*For(buttonHeight)
fn metrics(size: Size) -> Metrics {
    match size {
        Size::XSmall => metrics!(split_button_x_small),
        Size::Small => metrics!(split_button_small),
        Size::Medium => metrics!(split_button_medium),
        Size::Large => metrics!(split_button_large),
        Size::XLarge => metrics!(split_button_x_large),
    }
}

/// Result of [`SplitButton::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SplitButtonResponse {
    /// Leading (main) button response.
    pub response: Response,
    /// Main button clicked this frame.
    pub leading_clicked: bool,
    /// Index of the clicked menu item.
    pub clicked: Option<usize>,
    /// Menu state after this frame.
    pub open: bool,
}

/// M3 Expressive split button. The trailing part opens `menu`; while open it turns into a
/// circle and its chevron flips.
#[must_use = "call .show(..)"]
pub struct SplitButton {
    salt: KeyedSalt,
    label: String,
    icon: Option<Icon>,
    menu: Menu,
    menu_label: Option<String>,
    variant: ContainerVariant,
    size: Size,
    disabled: bool,
    shortcut: Option<KeyboardShortcut>,
}

egui_mineral::kit::opaque_debug!(SplitButton);

impl SplitButton {
    /// `label` is the main button text; `menu` opens from the trailing button; agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, menu: Menu) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            label: label.into(),
            icon: None,
            menu,
            menu_label: None,
            variant: ContainerVariant::default(),
            size: Size::Small,
            disabled: false,
            shortcut: None,
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    /// Accessible name of the trailing menu button; default `mineral-split-button-menu`.
    pub fn menu_label(mut self, menu_label: impl Into<String>) -> Self {
        self.menu_label = Some(menu_label.into());
        self
    }

    /// Leading icon of the main button.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Activates the main button like a click while enabled and visible.
    pub fn shortcut(mut self, shortcut: KeyboardShortcut) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    /// Default [`ContainerVariant::Filled`].
    pub fn variant(mut self, variant: ContainerVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Default [`Size::Small`].
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    crate::widget::setters! {
        /// Greys out both parts.
        disabled
    }

    /// Paints both parts and the menu while open.
    pub fn show(self, ui: &mut MineralUi<'_, Material>) -> SplitButtonResponse {
        let rt = ui.rt();
        let m = metrics(self.size);
        let mut lead = PushButton::new(self.salt.child("lead"), &self.label)
            .agent_key(StableId::of(&self.salt, "button"))
            .variant(self.variant.into())
            .size(self.size)
            .disabled(self.disabled);
        if let Some(icon) = self.icon {
            lead = lead.icon(icon);
        }
        if let Some(shortcut) = self.shortcut {
            lead = lead.shortcut(shortcut);
        }
        lead.space = Some(m.lead);
        lead.min_width = Some(defaults::split_button::MIN_WIDTH);
        // Compose reserves the interactive minimum around the whole split button.
        lead.min_target = MinTarget::None;
        lead.joined = Some(Joined {
            start: true,
            end: false,
            inner: m.inner,
            inner_pressed: m.inner_pressed,
            checked_full: false,
        });
        let trail = Trailing {
            salt: self.salt.child("menu"),
            key: StableId::of(&self.salt, "split-menu-button"),
            label: crate::tr_or(rt.i18n(), self.menu_label.as_deref(), "mineral-split-button-menu"),
            variant: self.variant.into(),
            size: self.size,
            metrics: m,
            disabled: self.disabled,
        };
        let dir = rt.i18n().direction();
        let min = crate::adaptive::MIN_INTERACTIVE.points(ui.core());
        let row = vec2(ui.available_width(), m.height.max(min));
        let (lead_r, trail_r) = ui
            .scope(|ui| {
                ui.spacing_mut().item_spacing.x = m.gap;
                ui.allocate_ui_with_layout(row, dir.row(), |ui| (rt.ui(ui).mineral(lead), rt.ui(ui).mineral(trail)))
                    .inner
            })
            .inner;
        let menu_item = if self.disabled {
            None
        } else {
            self.menu.emit_anchor(false).show(rt, trail_r.response()).clicked
        };
        let open = Menu::is_open(ui.ctx(), trail_r.response());
        SplitButtonResponse {
            leading_clicked: lead_r.clicked(),
            response: lead_r.into_response(),
            clicked: menu_item,
            open,
        }
    }
}

struct Trailing {
    salt: KeyedSalt,
    key: StableId,
    label: String,
    variant: ButtonVariant,
    size: Size,
    metrics: Metrics,
    disabled: bool,
}

impl Trailing {
    fn open(&self, ctx: &egui::Context, r: &Response) -> bool {
        !self.disabled && Menu::is_open(ctx, r)
    }
}

impl MineralWidget for Trailing {
    type Kit = Material;
    type State = ();
    type Output = ();

    egui_mineral::kit::widget_meta!(salt, enabled, no_target);

    fn layout(&self, _ui: &MineralUi<'_, Material>, c: Constraints, (): &Self::State) -> Measure {
        let m = &self.metrics;
        let w = (m.trail.leading + m.trail_icon + m.trail.trailing).max(defaults::split_button::MIN_WIDTH);
        c.constrain(vec2(w, m.height)).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Material>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let t = rt.tokens();
        let m = self.metrics;
        let r = &frame.response;
        let rect = frame.zones.visual;
        let i = parts::press(ui, frame, Availability::disabled_if(self.disabled)).interaction;
        let open = self.open(ctx, r);
        let s = button::style(self.variant, None, !self.disabled, &t);
        let h = rect.height().min(rect.width());
        let full = button::radius(&t, ShapeScale::Full, h);
        let inner_target = match (i.pressed, open) {
            (true, _) => button::radius(&t, m.inner_pressed, h),
            (false, true) => full,
            (false, false) => button::radius(&t, m.inner, h),
        };
        let inner = button::morph(ui, r.id.with("inner"), inner_target, false);
        let dir = rt.i18n().direction();
        let cr = button::sided(inner, full, dir);
        button::Surface::of(rect, cr, s, self.size.metrics().outline).paint(ui, &frame.response, s.content, i, &t);
        let p = ui.painter();
        if open {
            p.rect_filled(
                rect,
                cr,
                common::with_alpha(s.content, state::PRESSED_STATE_LAYER_OPACITY),
            );
        }
        // Compose `horizontalCenterOptically`: shift toward the rounder side, within the padding.
        let ppp = ctx.pixels_per_point();
        let shift =
            (defaults::split_button::CENTER_OPTICALLY * (inner - full)).clamp(-m.trail.leading, m.trail.trailing);
        let shift = (shift * ppp).round() / ppp;
        let center = rect.center() + vec2(dir.sign() * shift, 0.0);
        let turn = egui_mineral::motion::spring(
            rt.core(),
            r.id.with("chevron"),
            f32::from(open),
            motion::DEFAULT_EFFECTS,
        );
        p.add(common::icon_shape(
            p,
            center,
            ui.icons().keyboard_arrow_down,
            m.trail_icon,
            s.content,
            turn * PI,
        ));
        WidgetResponse::new(r.clone(), ())
    }

    fn semantics(&self, _: &I18n, r: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(self.key.clone(), Role::Button, &self.label)
            .clickable(Availability::disabled_if(self.disabled));
        n.state.expanded = Some(Menu::is_open(&r.response().ctx, r.response()));
        Some(n)
    }
}
