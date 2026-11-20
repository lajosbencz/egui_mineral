use egui::{Margin, Response};
use egui_mineral::{
    anchor::{Align, Anchor, Side},
    overlay::{Dismiss, OverlayResponse, Popup},
    semantic::{self, AgentKey, KeyedSalt, Role, SemanticNode},
    widget::MineralUi,
};

use super::overlay::{FocusTrap, Look, arrow, framed};
use crate::spec::ColorToken::{
    ColorBrandBackground, ColorNeutralBackground1, ColorNeutralBackgroundStatic, ColorNeutralForeground1,
    ColorNeutralForegroundOnBrand, ColorNeutralForegroundStaticInverted,
};
use crate::{
    Fluent, FluentExt, Rt,
    spec::{defaults::popover as d, tokens::geometry as g},
    theme::Elevation,
};

/// `PopoverSurface` `size`: padding and arrow height.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PopoverSize {
    /// `small`: 12pt padding.
    Small,
    /// `medium`: 16pt padding.
    #[default]
    Medium,
    /// `large`: 20pt padding.
    Large,
}

impl PopoverSize {
    /// Every variant, smallest first.
    pub const ALL: &'static [Self] = &[Self::Small, Self::Medium, Self::Large];

    const fn padding(self) -> f32 {
        match self {
            Self::Small => d::PADDING_SMALL,
            Self::Medium => d::PADDING_MEDIUM,
            Self::Large => d::PADDING_LARGE,
        }
    }

    const fn arrow(self) -> f32 {
        match self {
            Self::Small => d::ARROW_HEIGHT_SMALL,
            Self::Medium | Self::Large => d::ARROW_HEIGHT_MEDIUM,
        }
    }
}

/// `PopoverSurface` `appearance`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PopoverAppearance {
    /// Neutral surface.
    #[default]
    Default,
    /// `brand`.
    Brand,
    /// `inverted`.
    Inverted,
}

impl PopoverAppearance {
    /// Every variant.
    pub const ALL: &'static [Self] = &[Self::Default, Self::Brand, Self::Inverted];

    pub(super) const fn colors(self) -> (crate::spec::ColorToken, crate::spec::ColorToken) {
        match self {
            Self::Default => (ColorNeutralBackground1, ColorNeutralForeground1),
            Self::Brand => (ColorBrandBackground, ColorNeutralForegroundOnBrand),
            Self::Inverted => (ColorNeutralBackgroundStatic, ColorNeutralForegroundStaticInverted),
        }
    }
}

pub(super) struct Surface<'a> {
    pub salt: &'a KeyedSalt,
    pub node: semantic::StableId,
    pub label: &'a str,
    pub side: Side,
    pub align: Align,
    pub arrow: Option<f32>,
    pub appearance: PopoverAppearance,
    pub padding: f32,
    pub radius: f32,
    pub width: Option<f32>,
    pub trap_focus: bool,
}

pub(super) fn show_anchored<R>(
    rt: &Rt,
    spec: &Surface<'_>,
    trigger: &Response,
    open: &mut bool,
    add: impl FnOnce(&mut MineralUi<'_, Fluent>) -> R,
) -> OverlayResponse<R> {
    let ctx = trigger.ctx.clone();
    let mut dismissed = None;
    if trigger.clicked() {
        *open = !*open;
        if !*open {
            dismissed = Some(Dismiss::Close);
        }
    }
    let id = trigger.id.with(("mineral::fluent-popover", spec.salt.id_salt()));
    let trap = FocusTrap::new(id);
    let (fill, fg) = spec.appearance.colors();
    let t = rt.tokens();
    let gap = spec.arrow.unwrap_or(0.0);
    let out = Popup::new(spec.salt.child("area"), Anchor::new(spec.side, spec.align).gap(gap))
        .role(Role::Dialog)
        .name(spec.label)
        .focus(trap.start())
        .show(rt, trigger, open, |ui, placement| {
            let look = Look {
                fill: t.color(fill),
                radius: g::BORDER_RADIUS_MEDIUM.max(spec.radius),
                elevation: Elevation::E16,
            };
            let pad = spec.padding as i8;
            let inner = framed(&mut *ui, &t, &look, Margin::same(pad), |ui| {
                if let Some(w) = spec.width {
                    ui.set_width(w);
                }
                ui.visuals_mut().override_text_color = Some(t.color(fg));
                if spec.trap_focus {
                    trap.begin(ui);
                }
                let r = add(&mut rt.ui(ui));
                if spec.trap_focus {
                    trap.finish(ui);
                }
                r
            });
            if let Some(height) = spec.arrow {
                arrow(ui, placement, inner.response.rect, look.radius, height, look.fill);
            }
            (inner.response.rect, inner.inner)
        });
    let dismissed = dismissed.or(out.dismissed);
    if let (Some(area), Some((rect, inner))) = (out.response, out.inner) {
        semantic::emit(&ctx, Some(area.id), rect, || {
            SemanticNode::new(spec.node.clone(), Role::Dialog, spec.label)
        });
        OverlayResponse::shown(area, Some(inner), dismissed)
    } else {
        let mut closed = OverlayResponse::closed();
        closed.dismissed = dismissed;
        closed
    }
}

/// Fluent `Popover`: a surface anchored to a trigger, opened by its clicks.
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct Popover {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    size: PopoverSize,
    appearance: PopoverAppearance,
    arrow: bool,
    side: Side,
    align: Align,
    trap_focus: bool,
}

impl Popover {
    /// `label` names the surface for assistive technology; above the trigger, centered, no arrow.
    /// Agent key `<id_salt>-popover`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        Self {
            salt: id_salt.into(),
            key: egui_mineral::agent_key!("popover"),
            label: label.into(),
            size: PopoverSize::Medium,
            appearance: PopoverAppearance::Default,
            arrow: false,
            side: Side::Top,
            align: Align::Center,
            trap_focus: false,
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-popover` key.
        agent_key(key)
    }

    /// Default [`PopoverSize::Medium`].
    pub fn size(mut self, size: PopoverSize) -> Self {
        self.size = size;
        self
    }

    /// Default [`PopoverAppearance::Default`].
    pub fn appearance(mut self, appearance: PopoverAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// `withArrow`: a caret pointing at the trigger; default `false`.
    pub fn arrow(mut self, arrow: bool) -> Self {
        self.arrow = arrow;
        self
    }

    /// Side of the trigger; default [`Side::Top`].
    pub fn side(mut self, side: Side) -> Self {
        self.side = side;
        self
    }

    /// Alignment along the trigger; default [`Align::Center`].
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// `trapFocus`: Tab cycles inside the surface; default `false`.
    pub fn trap_focus(mut self, trap_focus: bool) -> Self {
        self.trap_focus = trap_focus;
        self
    }

    /// Shown while `open`, anchored to `trigger`: its clicks toggle `open`, Escape and outside presses clear it.
    /// Opening focuses the surface start; the next Tab reaches the first control.
    pub fn show<R>(
        self,
        rt: &Rt,
        trigger: &Response,
        open: &mut bool,
        add: impl FnOnce(&mut MineralUi<'_, Fluent>) -> R,
    ) -> OverlayResponse<R> {
        let spec = Surface {
            salt: &self.salt,
            node: self.key.id(&self.salt),
            label: &self.label,
            side: self.side,
            align: self.align,
            arrow: self.arrow.then(|| self.size.arrow()),
            appearance: self.appearance,
            padding: self.size.padding(),
            radius: g::BORDER_RADIUS_MEDIUM,
            width: None,
            trap_focus: self.trap_focus,
        };
        show_anchored(rt, &spec, trigger, open, add)
    }
}
