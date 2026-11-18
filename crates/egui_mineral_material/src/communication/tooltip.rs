use crate::MaterialExt;
use crate::Rt;
use egui::{Frame, Label, Response, Sense, Tooltip, vec2};
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::semantic::StableId;

use crate::actions::PushButton;
use crate::spec::{
    defaults,
    tokens::{plain_tooltip, rich_tooltip},
};
use crate::widget::{common, parts};

fn describe(response: &Response, text: &str) {
    if response.ctx.has_accesskit_node(response.id) {
        response.ctx.accesskit_node_builder(response.id, |b| {
            if b.description().is_none() {
                b.set_description(text);
            }
        });
    }
}

/// Result of [`PlainTooltip::show`]; `response` is `None` while hidden.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct PlainTooltipResponse {
    /// Tooltip surface while visible.
    pub response: Option<Response>,
}

/// Result of [`RichTooltip::show`]; `response` is `None` while hidden.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct RichTooltipResponse {
    /// Tooltip surface while visible.
    pub response: Option<Response>,
    /// [`RichTooltip::action`] clicked this frame.
    pub action_clicked: bool,
    /// [`RichTooltip::secondary_action`] clicked this frame.
    pub secondary_action_clicked: bool,
}

/// MD3 plain tooltip (Compose `PlainTooltip`).
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct PlainTooltip {
    text: String,
}

impl PlainTooltip {
    /// `text` is also set as the anchor's accessible description.
    pub fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }

    /// Shows while `response` is hovered (egui `Tooltip::for_enabled`).
    pub fn show(self, rt: &Rt, response: &Response) -> PlainTooltipResponse {
        use defaults::tooltip as d;
        let text = self.text;
        describe(response, &text);
        let t = rt.tokens();
        let mut tip = Tooltip::for_enabled(response);
        tip.popup = tip.popup.frame(Frame::NONE).gap(d::ANCHOR_SPACING);
        let shown = tip.show(|ui| {
            let color = t.role(plain_tooltip::SUPPORTING_TEXT_COLOR);
            let pad = vec2(d::PLAIN_HORIZONTAL_PADDING, d::PLAIN_VERTICAL_PADDING);
            let g = parts::type_wrapped(
                ui.painter(),
                &text,
                plain_tooltip::SUPPORTING_TEXT_FONT,
                color,
                d::PLAIN_MAX_WIDTH - 2.0 * pad.x,
                0,
            );
            let size = (g.size() + 2.0 * pad).max(vec2(d::MIN_WIDTH, d::MIN_HEIGHT));
            let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
            let r = plain_tooltip::CONTAINER_SHAPE.corner_radius(&t.shape, rt.i18n().direction());
            let p = ui.painter();
            p.rect_filled(rect, r, t.role(plain_tooltip::CONTAINER_COLOR));
            p.galley(rect.min + pad, g, color);
        });
        PlainTooltipResponse {
            response: shown.map(|s| s.response),
        }
    }
}

/// MD3 rich tooltip with optional title and actions (Compose `RichTooltip`).
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct RichTooltip {
    salt: KeyedSalt,
    supporting: String,
    title: Option<String>,
    actions: [Option<String>; 2],
}

impl RichTooltip {
    /// `supporting` is the body text and the anchor's accessible description; agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>, supporting: impl Into<String>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            supporting: supporting.into(),
            title: None,
            actions: [None, None],
        }
    }

    crate::widget::setters! {
        /// Prefix of the action button keys.
        agent_key(salt)
    }

    /// Bold line above the supporting text.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// First text button below the supporting text.
    pub fn action(mut self, label: impl Into<String>) -> Self {
        self.actions[0] = Some(label.into());
        self
    }

    /// Second text button, after [`Self::action`].
    pub fn secondary_action(mut self, secondary_action: impl Into<String>) -> Self {
        self.actions[1] = Some(secondary_action.into());
        self
    }

    /// Shows while `response` is hovered (egui `Tooltip::for_enabled`).
    pub fn show(self, rt: &Rt, response: &Response) -> RichTooltipResponse {
        use defaults::tooltip as d;
        use rich_tooltip as m;
        describe(response, &self.supporting);
        let t = rt.tokens();
        let radius = m::CONTAINER_SHAPE.corner_radius(&t.shape, rt.i18n().direction());
        let has_actions = self.actions.iter().any(Option::is_some);
        let mut action = None;
        let mut tip = Tooltip::for_enabled(response);
        tip.popup = tip.popup.frame(Frame::NONE).gap(d::ANCHOR_SPACING);
        let shown = tip.show(|ui| {
            common::elevated_frame(
                ui,
                m::CONTAINER_ELEVATION,
                &t,
                Frame::new()
                    .fill(t.role(m::CONTAINER_COLOR))
                    .corner_radius(radius)
                    .inner_margin(vec2(d::RICH_HORIZONTAL_PADDING, 0.0)),
                |ui| {
                    let width = d::RICH_MAX_WIDTH - 2.0 * d::RICH_HORIZONTAL_PADDING;
                    ui.set_max_width(width);
                    ui.set_min_size(vec2(d::MIN_WIDTH, d::MIN_HEIGHT) - vec2(2.0 * d::RICH_HORIZONTAL_PADDING, 0.0));
                    ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                    let painter = ui.painter().clone();
                    let galley =
                        |text: &str, font, color| parts::type_wrapped(&painter, text, font, t.role(color), width, 0);
                    // Compose `paddingFromBaseline`: space above the first baseline.
                    if let Some(s) = &self.title {
                        let g = galley(s, m::SUBHEAD_FONT, m::SUBHEAD_COLOR);
                        ui.add_space((d::HEIGHT_TO_SUBHEAD_FIRST_LINE - parts::first_baseline(&g)).max(0.0));
                        ui.add(Label::new(g));
                    }
                    let g = galley(&self.supporting, m::SUPPORTING_TEXT_FONT, m::SUPPORTING_TEXT_COLOR);
                    if self.title.is_none() && !has_actions {
                        ui.add_space(d::PLAIN_VERTICAL_PADDING);
                        ui.add(Label::new(g));
                        ui.add_space(d::PLAIN_VERTICAL_PADDING);
                    } else {
                        ui.add_space((d::HEIGHT_FROM_SUBHEAD_TO_TEXT_FIRST_LINE - parts::first_baseline(&g)).max(0.0));
                        ui.add(Label::new(g));
                        ui.add_space(d::TEXT_BOTTOM_PADDING);
                    }
                    if has_actions {
                        ui.horizontal(|ui| {
                            ui.set_min_height(d::ACTION_LABEL_MIN_HEIGHT);
                            let buttons = self.actions.iter().enumerate().filter_map(|(i, a)| {
                                let b = PushButton::new(self.salt.child(i), a.as_ref()?)
                                    .agent_key(StableId::of(&self.salt, (i, "button")));
                                Some((i, b))
                            });
                            action = crate::actions::text_actions(&mut rt.ui(ui), buttons);
                        });
                        ui.add_space(d::ACTION_LABEL_BOTTOM_PADDING);
                    }
                },
            );
        });
        RichTooltipResponse {
            response: shown.map(|s| s.response),
            action_clicked: action == Some(0),
            secondary_action_clicked: action == Some(1),
        }
    }
}
