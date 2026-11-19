use std::time::Duration;

use egui::{Margin, Response};
use egui_mineral::{
    anchor::{Align, Anchor, Side},
    input::{HoverDelays, HoverIntent},
    kit::{self, state::TempState},
    overlay::Popup,
    semantic::{self, AgentKey, KeyedSalt, Role, SemanticNode},
};

use super::overlay::{CAPTION1, Look, arrow, framed, text};
use crate::ColorToken::{
    ColorNeutralBackground1, ColorNeutralBackgroundStatic, ColorNeutralForeground1,
    ColorNeutralForegroundStaticInverted,
};
use crate::{
    FluentExt, Rt,
    spec::{defaults::tooltip as d, tokens::geometry as g},
    theme::Elevation,
};

/// How the tooltip text relates to its trigger (`relationship`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TooltipRelationship {
    /// `label`: the text is the trigger's accessible name.
    Label,
    /// `description`: the text is the trigger's accessible description.
    #[default]
    Description,
    /// `inaccessible`: only for pointer users, hidden from assistive technology.
    Inaccessible,
}

/// Tooltip `appearance`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TooltipAppearance {
    /// Neutral surface.
    #[default]
    Normal,
    /// `inverted`: the static dark surface.
    Inverted,
}

impl TooltipAppearance {
    /// Every variant.
    pub const ALL: [Self; 2] = [Self::Normal, Self::Inverted];
}

/// Result of [`Tooltip::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct TooltipResponse {
    /// Surface while visible.
    pub response: Option<Response>,
}

#[derive(Clone, Default)]
struct HoverState {
    intent: HoverIntent<bool>,
    open: bool,
    escaped: bool,
}

impl TempState for HoverState {}

/// Fluent `Tooltip`: a text hint shown while its trigger is hovered or focused.
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct Tooltip {
    salt: KeyedSalt,
    key: AgentKey,
    content: String,
    relationship: TooltipRelationship,
    appearance: TooltipAppearance,
    arrow: bool,
    side: Side,
    align: Align,
    show_delay: Duration,
    hide_delay: Duration,
}

impl Tooltip {
    /// `content` is the tooltip text; above the trigger, centered, no arrow. Agent key `<id_salt>-tooltip`.
    pub fn new(id_salt: impl Into<KeyedSalt>, content: impl Into<String>) -> Self {
        Self {
            salt: id_salt.into(),
            key: AgentKey::Kind("tooltip"),
            content: content.into(),
            relationship: TooltipRelationship::Description,
            appearance: TooltipAppearance::Normal,
            arrow: false,
            side: Side::Top,
            align: Align::Center,
            show_delay: d::SHOW_DELAY,
            hide_delay: d::HIDE_DELAY,
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-tooltip` key.
        agent_key(key)
    }

    /// Default [`TooltipRelationship::Description`].
    pub fn relationship(mut self, relationship: TooltipRelationship) -> Self {
        self.relationship = relationship;
        self
    }

    /// Default [`TooltipAppearance::Normal`].
    pub fn appearance(mut self, appearance: TooltipAppearance) -> Self {
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

    /// Hover time before it shows; default 250ms.
    pub fn show_delay(mut self, show_delay: Duration) -> Self {
        self.show_delay = show_delay;
        self
    }

    /// Time it stays after the pointer and focus leave; default 250ms.
    pub fn hide_delay(mut self, hide_delay: Duration) -> Self {
        self.hide_delay = hide_delay;
        self
    }

    /// Shows while `trigger` is hovered or focused, after the delays; Escape hides it.
    pub fn show(self, rt: &Rt, trigger: &Response) -> TooltipResponse {
        let ctx = trigger.ctx.clone();
        let id = trigger.id.with(("mineral::tooltip", self.salt.id_salt()));
        self.relate(trigger);
        let mut st = HoverState::load(&ctx, id);
        let delays = HoverDelays {
            open: self.show_delay,
            close: self.hide_delay,
        };
        let engaged = trigger.hovered() || trigger.has_focus();
        st.escaped &= engaged;
        st.intent
            .follow(&ctx, &mut st.open, (engaged && !st.escaped).into(), delays);
        if st.open && kit::overlay::consume_escape(&ctx) {
            st.open = false;
            st.escaped = true;
            st.intent.cancel();
        }
        let mut out = TooltipResponse { response: None };
        if st.open {
            out.response = Some(self.surface(rt, trigger));
        }
        st.store(&ctx, id);
        out
    }

    fn relate(&self, trigger: &Response) {
        let ctx = &trigger.ctx;
        if !ctx.has_accesskit_node(trigger.id) {
            return;
        }
        ctx.accesskit_node_builder(trigger.id, |b| match self.relationship {
            TooltipRelationship::Label => {
                if b.label().is_none() {
                    b.set_label(self.content.as_str());
                }
            }
            TooltipRelationship::Description => {
                if b.description().is_none() {
                    b.set_description(self.content.as_str());
                }
            }
            TooltipRelationship::Inaccessible => {}
        });
    }

    fn surface(&self, rt: &Rt, trigger: &Response) -> Response {
        let t = rt.tokens();
        let height = self.arrow.then_some(d::ARROW_HEIGHT);
        let (fill, fg) = match self.appearance {
            TooltipAppearance::Normal => (ColorNeutralBackground1, ColorNeutralForeground1),
            TooltipAppearance::Inverted => (ColorNeutralBackgroundStatic, ColorNeutralForegroundStaticInverted),
        };
        let gap = d::OFFSET + height.unwrap_or(0.0);
        let area = Popup::new(self.salt.child("area"), Anchor::new(self.side, self.align).gap(gap)).show_at(
            rt,
            trigger.rect,
            |ui, placement| {
                let look = Look {
                    fill: t.color(fill),
                    radius: g::BORDER_RADIUS_MEDIUM,
                    elevation: Elevation::E8,
                };
                let border = g::STROKE_WIDTH_THIN as i8;
                let margin = Margin {
                    left: d::PADDING_X as i8 + border,
                    right: d::PADDING_X as i8 + border,
                    top: d::PADDING_TOP as i8 + border,
                    bottom: d::PADDING_BOTTOM as i8 + border,
                };
                let shown = framed(ui, &t, &look, margin, |ui| {
                    ui.set_max_width(d::MAX_WIDTH - f32::from(margin.left + margin.right));
                    text(ui, rt, &self.content, CAPTION1, t.color(fg));
                });
                if let Some(h) = height {
                    arrow(ui, placement, shown.response.rect, look.radius, h, look.fill);
                }
                shown.response.rect
            },
        );
        if self.relationship != TooltipRelationship::Inaccessible {
            semantic::emit(rt.ctx(), Some(area.response.id), area.inner, || {
                SemanticNode::new(self.key.id(&self.salt), Role::Tooltip, &self.content)
            });
        }
        area.response
    }
}
