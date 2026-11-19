use crate::Material;
use crate::MaterialExt;
use crate::widget::common::elevation_tokens;
use egui::{Color32, Margin, Rangef, Sense, Shape, Stroke, StrokeKind, UiBuilder};
use egui_mineral::semantic::AgentKey;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::Availability;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    kit,
    semantic::{self, Role, SemanticNode},
};

use crate::actions::button::Elevation;
use crate::widget::{
    common::{self, Interaction},
    indication,
};
use crate::{
    spec::{
        ShapeRole,
        tokens::{elevated_card, filled_card, outlined_card},
    },
    tokens::Tokens,
};

/// Compose `ElevatedCard`, `Card` and `OutlinedCard` styles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum CardVariant {
    #[default]
    /// `ElevatedCard`: low surface container with a shadow.
    Elevated,
    /// `Card`: highest surface container, flat.
    Filled,
    /// `OutlinedCard`: surface with an outline.
    Outlined,
}

// Compose CardDefaults.*CardColors, *CardElevation, outlinedCardBorder
struct CardStyle {
    container: Color32,
    outline: Option<Color32>,
    elevation: Elevation,
    shape: ShapeRole,
}

fn card_style(variant: CardVariant, enabled: bool, t: &Tokens) -> CardStyle {
    match variant {
        CardVariant::Elevated => {
            use elevated_card as m;
            CardStyle {
                container: if enabled {
                    t.role(m::CONTAINER_COLOR)
                } else {
                    t.role(m::DISABLED_CONTAINER_COLOR)
                        .blend(t.role_alpha(m::DISABLED_CONTAINER_COLOR, m::DISABLED_CONTAINER_OPACITY))
                },
                outline: None,
                elevation: elevation_tokens!(m; rest: CONTAINER_ELEVATION, hovered: HOVER_CONTAINER_ELEVATION, focused: FOCUS_CONTAINER_ELEVATION, pressed: PRESSED_CONTAINER_ELEVATION, disabled: DISABLED_CONTAINER_ELEVATION),
                shape: m::CONTAINER_SHAPE,
            }
        }
        CardVariant::Filled => {
            use filled_card as m;
            CardStyle {
                container: if enabled {
                    t.role(m::CONTAINER_COLOR)
                } else {
                    t.role(m::CONTAINER_COLOR)
                        .blend(t.role_alpha(m::DISABLED_CONTAINER_COLOR, m::DISABLED_CONTAINER_OPACITY))
                },
                outline: None,
                elevation: elevation_tokens!(m; rest: CONTAINER_ELEVATION, hovered: HOVER_CONTAINER_ELEVATION, focused: FOCUS_CONTAINER_ELEVATION, pressed: PRESSED_CONTAINER_ELEVATION, disabled: DISABLED_CONTAINER_ELEVATION),
                shape: m::CONTAINER_SHAPE,
            }
        }
        CardVariant::Outlined => {
            use outlined_card as m;
            CardStyle {
                container: t.role(m::CONTAINER_COLOR),
                outline: Some(if enabled {
                    t.role(m::OUTLINE_COLOR)
                } else {
                    t.role(elevated_card::CONTAINER_COLOR)
                        .blend(t.role_alpha(m::DISABLED_OUTLINE_COLOR, m::DISABLED_OUTLINE_OPACITY))
                }),
                // Compose `outlinedCardElevation` keeps the rest level on press, focus and hover.
                elevation: elevation_tokens!(m; rest: CONTAINER_ELEVATION, hovered: CONTAINER_ELEVATION, focused: CONTAINER_ELEVATION, pressed: CONTAINER_ELEVATION, disabled: DISABLED_CONTAINER_ELEVATION),
                shape: m::CONTAINER_SHAPE,
            }
        }
    }
}

/// MD3 card (Compose `Card`, `ElevatedCard`, `OutlinedCard`). Like Compose it wraps its content,
/// adds no padding of its own and draws its outline inside its bounds, so every variant has the
/// same geometry. Size it with [`Self::width`]; pad the content with [`Self::content_padding`], usually
/// [`Self::CONTENT_PADDING`].
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct Card {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    variant: CardVariant,
    clickable: bool,
    disabled: bool,
    width: Option<Rangef>,
    padding: Margin,
}

/// Result of [`Card::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct CardResponse<R> {
    /// Card surface; clickable only with [`Card::clickable`].
    pub response: egui::Response,
    /// Output of the content closure.
    pub inner: R,
    /// Clicked this frame while clickable and enabled.
    pub clicked: bool,
}

impl Card {
    /// MD3 card content padding (m3.material.io cards specs, 16dp); Compose leaves it to the content.
    pub const CONTENT_PADDING: Margin = Margin::same(16);

    /// `label` names the card for assistive technology.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            key: AgentKey::Kind("card"),
            salt: id_salt,
            label: label.into(),
            variant: CardVariant::default(),
            clickable: false,
            disabled: false,
            width: None,
            padding: Margin::ZERO,
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-card` agent key.
        agent_key(key)
    }

    /// Default [`CardVariant::Elevated`].
    pub fn variant(mut self, variant: CardVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Senses clicks with a state layer; the node becomes a button.
    pub fn clickable(mut self, clickable: bool) -> Self {
        self.clickable = clickable;
        self
    }

    crate::widget::setters! {
        /// Disabled container colors; clicks are ignored.
        disabled
    }

    /// Outer width bounds in dp, padding included: the card follows its content within them and content wraps at
    /// the upper one; equal bounds fix the width. Not finite or below 1 becomes 1; a reversed range is swapped.
    #[track_caller]
    pub fn width(mut self, width: impl Into<Rangef>) -> Self {
        let w = width.into();
        let min = egui_mineral::kit::sanitize::at_least(w.min.min(w.max), 1.0.., "Card width");
        let max = w.max.max(w.min);
        self.width = Some(Rangef::new(min, if max.is_nan() { min } else { max.max(min) }));
        self
    }

    /// Space between the container edge and the content.
    pub fn content_padding(mut self, padding: Margin) -> Self {
        self.padding = padding;
        self
    }

    /// Paints the card around `add`; content color is `onSurface`.
    pub fn show<R>(
        self,
        ui: &mut MineralUi<'_, Material>,
        add: impl FnOnce(&mut MineralUi<'_, Material>) -> R,
    ) -> CardResponse<R> {
        let rt = ui.rt();
        let t = rt.tokens();
        let s = card_style(self.variant, !self.disabled, &t);
        let r = s.shape.corner_radius(&t.shape, rt.i18n().direction());
        // Compose `contentColorFor` the card containers.
        let content_color = t.color.on_surface;
        let sense = if self.clickable && !self.disabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let mut max = ui.available_rect_before_wrap();
        if let Some(w) = self.width.filter(|w| w.max.is_finite()) {
            // At the layout's next position: the right end in a right-to-left row.
            let x = ui
                .layout()
                .horizontal_placement()
                .align_size_within_range(w.max, max.x_range());
            max = egui::Rect::from_x_y_ranges(x, max.y_range());
        }
        // Sensed before the content, so content widgets keep their own clicks.
        let mut card = ui.new_child(
            UiBuilder::new()
                .id_salt(self.salt.id_salt())
                .max_rect(max)
                .layout(rt.i18n().direction().column())
                .sense(sense),
        );
        let shadow = card.painter().add(Shape::Noop);
        let fill = card.painter().add(Shape::Noop);
        let mut content = card.new_child(
            UiBuilder::new()
                .max_rect(max - self.padding)
                .layout(rt.i18n().direction().column()),
        );
        if let Some(w) = self.width {
            let pad = self.padding.sum().x;
            content.set_min_width((w.min - pad).max(0.0));
            if w.max.is_finite() {
                content.set_max_width((w.max - pad).max(0.0));
            }
        }
        if self.disabled {
            content.multiply_opacity(t.state.disabled_content);
        }
        if sense.senses_click() {
            // Selectable text would sit on top of the card and take its clicks.
            content.style_mut().interaction.selectable_labels = false;
        }
        let inner = add(&mut rt.ui(&mut content));
        let rect = content.min_rect() + self.padding;
        card.advance_cursor_after_rect(rect);
        let mut response = card.response();
        ui.pass_input().tap_on(&mut response);
        drop(card);
        ui.advance_cursor_after_rect(rect);

        let i = if self.clickable {
            Interaction::of(&response, Availability::disabled_if(self.disabled))
        } else {
            Interaction::default().with_disabled(self.disabled)
        };
        let p = ui.painter();
        if let Some(sh) = common::elevation_shape(rect, r, s.elevation.level(i), &t) {
            p.set(shadow, sh);
        }
        p.set(fill, Shape::rect_filled(rect, r, s.container));
        if self.clickable {
            kit::input::hover_cursor(ui.core(), &response, Availability::disabled_if(self.disabled));
            indication::paint(ui, &response, rect, r, content_color, i, &t);
        }
        // Compose draws the border over the content, inside the bounds.
        if let Some(o) = s.outline {
            p.rect_stroke(
                rect,
                r,
                Stroke::new(outlined_card::OUTLINE_WIDTH, o),
                StrokeKind::Inside,
            );
        }
        let clicked = self.clickable && !self.disabled && response.clicked();
        semantic::emit(ui.ctx(), Some(response.id), rect, || {
            let availability = Availability::disabled_if(self.disabled);
            if self.clickable {
                SemanticNode::new(self.key.id(&self.salt), Role::Button, &self.label).clickable(availability)
            } else {
                let mut n = SemanticNode::new(self.key.id(&self.salt), Role::Group, &self.label);
                n.state.disabled = self.disabled;
                n
            }
        });
        CardResponse {
            response,
            inner,
            clicked,
        }
    }
}
