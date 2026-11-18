use crate::Material;
use crate::MaterialExt;
use std::sync::Arc;

use egui::{FontFamily, FontId, Galley, OpenUrl, Rangef, Rect, Stroke, StrokeKind, Vec2, vec2};
use egui_mineral::i18n::I18n;
use egui_mineral::semantic::AgentKey;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::Availability;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    i18n::Direction,
    layout::{Constraints, Measure},
    semantic::{Role, SemanticNode},
    widget::{MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::spec::{
    ColorRole, ShapeRole, TypeScale, defaults,
    tokens::{divider, filled_text_field, outlined_card},
};
use crate::widget::{
    common::{self},
    indication,
    paint::paint_widget,
    parts,
};

// No MD3 counterparts

const KEY_TYPE: TypeScale = TypeScale::LabelMedium;
// MD3 extra small shape
const KEY_SHAPE: ShapeRole = ShapeRole::CornerExtraSmall;
// Two divider widths of the outline color
const KEY_DEPTH: f32 = 2.0 * divider::THICKNESS;
const LINK_UNDERLINE: f32 = filled_text_field::ACTIVE_INDICATOR_HEIGHT;

/// Keyboard key caps for a shortcut hint; non-MD3.
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Kbd {
    salt: KeyedSalt,
    key: AgentKey,
    keys: Vec<String>,
}

impl Kbd {
    /// Each item of `keys` renders as one key cap; agent key `<id_salt>-kbd`.
    pub fn new(id_salt: impl Into<KeyedSalt>, keys: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            key: AgentKey::Kind("kbd"),
            salt: id_salt,
            keys: keys.into_iter().map(Into::into).collect(),
        }
    }
    crate::widget::setters! {
        /// Replaces the `<id_salt>-kbd` agent key.
        agent_key(key)
    }

    fn caps(&self, ui: &MineralUi<'_, Material>) -> (Vec<Arc<Galley>>, Vec<f32>, f32, f32) {
        let rt = ui.rt();
        let t = rt.tokens();
        // Inner padding, gap between caps and the cap's vertical slack: the extra small spacing.
        let pad = crate::adaptive::grid(1);
        let font = FontId::new(KEY_TYPE.size(), FontFamily::Monospace);
        let h = KEY_TYPE.line_height() + pad;
        let label = t.role(ColorRole::OnSurface);
        let galleys: Vec<_> = self
            .keys
            .iter()
            .map(|k| ui.painter().layout_no_wrap(k.clone(), font.clone(), label))
            .collect();
        let widths = galleys.iter().map(|g| (g.size().x + 2.0 * pad).max(h)).collect();
        (galleys, widths, h, pad)
    }

    fn extent(&self, ui: &MineralUi<'_, Material>, c: Constraints) -> Vec2 {
        let (_, widths, h, pad) = self.caps(ui);
        let total = widths.iter().sum::<f32>() + pad * widths.len().saturating_sub(1) as f32;
        c.constrain(vec2(total, h + KEY_DEPTH))
    }

    fn paint(&self, ui: &mut MineralUi<'_, Material>, rect: Rect) {
        let rt = ui.rt();
        if !ui.is_rect_visible(rect) {
            return;
        }
        let t = rt.tokens();
        let label = t.role(ColorRole::OnSurface);
        let (galleys, widths, h, pad) = self.caps(ui);
        let dir = rt.i18n().direction();
        let r = KEY_SHAPE.corner_radius(&t.shape, Direction::Ltr);
        let outline = t.role(outlined_card::OUTLINE_COLOR);
        let p = ui.painter();
        let mut offset = 0.0;
        for (g, w) in galleys.into_iter().zip(widths) {
            let cap = Rect::from_min_size(
                egui::pos2(dir.start_x(rect, Rangef::new(offset, offset + w)), rect.top()),
                vec2(w, h),
            );
            p.rect_filled(cap.translate(vec2(0.0, KEY_DEPTH)), r, outline);
            p.rect_filled(cap, r, t.role(ColorRole::SurfaceContainerHigh));
            p.rect_stroke(
                cap,
                r,
                Stroke::new(outlined_card::OUTLINE_WIDTH, outline),
                StrokeKind::Inside,
            );
            p.galley(cap.center() - g.size() / 2.0, g, label);
            offset += w + pad;
        }
    }

    fn node(&self, _: &egui_mineral::i18n::I18n) -> SemanticNode {
        SemanticNode::new(self.key.id(&self.salt), Role::Code, self.keys.join(" "))
    }
}

paint_widget!(Kbd);

/// Text hyperlink; non-MD3 (MUI `Link` pattern). With [`TextLink::url`] activation opens it, without one
/// it is an in-app link read with `.clicked()`.
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct TextLink {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    url: Option<String>,
    new_tab: bool,
    scale: TypeScale,
    disabled: bool,
}

impl TextLink {
    /// `label` is the visible text and accessible name. Agent key `<id_salt>-link`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            key: AgentKey::Kind("link"),
            salt: id_salt,
            label: label.into(),
            url: None,
            new_tab: false,
            scale: TypeScale::BodyMedium,
            disabled: false,
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-link` agent key.
        agent_key(key)
    }

    /// Activation opens `url` (egui `OpenUrl`).
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Opens the [`Self::url`] in a new tab (egui `OpenUrl::new_tab`).
    pub fn new_tab(mut self, new_tab: bool) -> Self {
        self.new_tab = new_tab;
        self
    }

    /// Text role; default [`TypeScale::BodyMedium`].
    pub fn scale(mut self, scale: TypeScale) -> Self {
        self.scale = scale;
        self
    }

    crate::widget::setters! {
        /// Greys out the link; it cannot open.
        disabled
    }
}

impl MineralWidget for TextLink {
    type Kit = Material;
    type State = ();
    type Output = ();

    egui_mineral::kit::widget_meta!(salt, enabled);

    fn layout(&self, ui: &MineralUi<'_, Material>, constraints: Constraints, (): &Self::State) -> Measure {
        let rt = ui.rt();
        let g = parts::type_galley(
            rt.core(),
            ui.painter(),
            &self.label,
            self.scale,
            egui::Color32::PLACEHOLDER,
            None,
        );
        constraints.constrain(g.size()).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Material>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let r = &frame.response;
        let rect = frame.zones.visual;
        let press = parts::press(ui, frame, Availability::disabled_if(self.disabled));
        let clicked = press.clicked;
        if let Some(url) = &self.url {
            if clicked {
                ui.ctx().open_url(if self.new_tab {
                    OpenUrl::new_tab(url)
                } else {
                    OpenUrl::same_tab(url)
                });
            }
            ui.ctx().accesskit_node_builder(r.id, |b| b.set_url(url.as_str()));
        }
        let color = if self.disabled {
            common::disabled_content(&t)
        } else {
            t.role(defaults::button::TEXT_LABEL_COLOR)
        };
        let shape = KEY_SHAPE.corner_radius(&t.shape, Direction::Ltr);
        indication::paint(ui, r, rect, shape, color, press.interaction, &t);
        let format = egui::TextFormat {
            underline: Stroke::new(LINK_UNDERLINE, color),
            ..self.scale.format(ui.ctx(), color)
        };
        let g = ui
            .painter()
            .layout_job(egui::text::LayoutJob::single_section(self.label.clone(), format));
        ui.painter().galley(rect.center() - g.size() / 2.0, g, color);
        WidgetResponse::new(r.clone(), ())
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(self.key.id(&self.salt), Role::Link, &self.label)
            .clickable(Availability::disabled_if(self.disabled));
        n.value.clone_from(&self.url);
        Some(n)
    }
}
