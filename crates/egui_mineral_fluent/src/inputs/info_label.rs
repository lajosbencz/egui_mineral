use crate::msg;
use crate::tokens::ShadowTint;
use egui::{CornerRadius, Frame, Margin, Sense, Stroke, Vec2};
use egui_mineral::{
    anchor::{Align, Anchor, Side},
    i18n::I18n,
    kit::{self, state::TempState},
    layout::{Constraints, Measure},
    overlay::Popup,
    semantic::{KeyedSalt, Role, SemanticNode, StableId},
    widget::{Availability, Composite, Interaction, MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use super::label::FieldLabel;
use crate::spec::ColorToken::{
    ColorNeutralBackground1, ColorNeutralForeground1, ColorNeutralForeground2, ColorNeutralForeground2BrandHover,
    ColorNeutralForeground2BrandPressed, ColorNeutralForeground2BrandSelected, ColorNeutralStroke1,
    ColorTransparentBackgroundHover, ColorTransparentBackgroundPressed, ColorTransparentBackgroundSelected,
};
use crate::{
    Fluent, FluentExt, Size,
    spec::{
        defaults::info_label as il,
        tokens::{geometry as g, typography as ty},
    },
    theme::Elevation,
    widget::{focus, paint},
};

/// Fluent `InfoLabel`: a [`FieldLabel`] followed by an info button that toggles a popover with `info`.
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct InfoLabel {
    salt: KeyedSalt,
    label: String,
    info: String,
    size: Size,
    required: bool,
    disabled: bool,
}

impl InfoLabel {
    /// `label` is the visible text, `info` the popover text. Keys `<id_salt>-label`, `<id_salt>-info-button`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, info: impl Into<String>) -> Self {
        Self {
            salt: id_salt.into(),
            label: label.into(),
            info: info.into(),
            size: Size::Medium,
            required: false,
            disabled: false,
        }
    }

    /// Size of the label and the info icon; default [`Size::Medium`].
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Appends the required mark to the label.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    crate::widget::setters! {
        /// Greys out the label.
        disabled
    }
}

impl Composite for InfoLabel {
    type Kit = Fluent;
    type Output = ();

    kit::widget_meta!(salt);

    fn compose(&mut self, ui: &mut MineralUi<'_, Fluent>, _: Constraints) -> WidgetResponse {
        let rt = ui.rt();
        let t = ui.tokens();
        let open_id = ui.make_persistent_id(self.salt.id_salt()).with("open");
        let mut open = bool::load(ui.ctx(), open_id);
        let row = ui.horizontal(|ui| {
            let ui = &mut rt.ui(ui);
            ui.mineral(
                FieldLabel::new(self.salt.child("label"), &self.label)
                    .agent_key(StableId::of(&self.salt, "label"))
                    .size(self.size)
                    .required(self.required)
                    .disabled(self.disabled),
            );
            ui.mineral(InfoButton {
                salt: self.salt.child("info"),
                size: self.size,
                expanded: open,
            })
        });
        let button = row.inner;
        if button.response().clicked() {
            open = !open;
        }
        let (text_size, line) = match self.size {
            Size::Large => (ty::FONT_SIZE_BASE_300, ty::LINE_HEIGHT_BASE_300),
            Size::Small | Size::Medium => (ty::FONT_SIZE_BASE_200, ty::LINE_HEIGHT_BASE_200),
        };
        Popup::new(
            self.salt.child("popover"),
            Anchor::new(Side::Top, Align::Start).gap(g::SPACING_VERTICAL_S),
        )
        .role(Role::Dialog)
        .name(&self.label)
        .show(rt, button.response(), &mut open, |ui, _| {
            let pad = g::SPACING_HORIZONTAL_L;
            Frame::new()
                .fill(t.color(ColorNeutralBackground1))
                .stroke(Stroke::new(g::STROKE_WIDTH_THIN, t.color(ColorNeutralStroke1)))
                .corner_radius(CornerRadius::from(g::BORDER_RADIUS_MEDIUM))
                .inner_margin(Margin::same(pad as i8))
                .shadow(t.shadow(Elevation::E16, ShadowTint::Neutral).ambient)
                .show(ui, |ui| {
                    ui.set_max_width(ui.max_rect().width().min(il::POPOVER_MAX_WIDTH - 2.0 * pad));
                    let format = crate::typography::format(
                        ui.ctx(),
                        crate::typography::TypeSpec {
                            size: text_size,
                            line_height: line,
                            weight: ty::FONT_WEIGHT_REGULAR,
                        },
                        t.color(ColorNeutralForeground1),
                    );
                    ui.label(egui::text::LayoutJob::single_section(self.info.clone(), format));
                });
        });
        open.store(ui.ctx(), open_id);
        WidgetResponse::new(row.response, ())
    }
}

// react-infolabel useInfoButtonStyles.styles.ts
struct InfoButton {
    salt: KeyedSalt,
    size: Size,
    expanded: bool,
}

impl InfoButton {
    fn icon(&self) -> f32 {
        match self.size {
            Size::Small => 12.0,
            Size::Medium => 16.0,
            Size::Large => 20.0,
        }
    }

    fn margin(&self) -> f32 {
        match self.size {
            Size::Large => il::BUTTON_MARGIN_LARGE,
            Size::Small | Size::Medium => 0.0,
        }
    }

    fn padding(&self) -> f32 {
        match self.size {
            Size::Large => g::SPACING_VERTICAL_XXS,
            Size::Small | Size::Medium => g::SPACING_VERTICAL_XS,
        }
    }
}

impl MineralWidget for InfoButton {
    type Kit = Fluent;
    type State = ();
    type Output = ();

    kit::widget_meta!(salt, no_target);

    fn sense(&self) -> Sense {
        Sense::click()
    }

    fn layout(&self, _: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        let side = self.icon() + 2.0 * self.padding();
        c.constrain(Vec2::new(side, side + 2.0 * self.margin())).into()
    }

    fn update(&mut self, ui: &mut MineralUi<'_, Fluent>, frame: &WidgetFrame<'_>, (): &mut ()) -> WidgetResponse {
        let t = ui.tokens();
        let r = &frame.response;
        let i = Interaction::of(r, Availability::Enabled);
        let rect = frame.zones.visual.expand2(Vec2::new(0.0, -self.margin()));
        let (fill, color) = if i.pressed {
            (ColorTransparentBackgroundPressed, ColorNeutralForeground2BrandPressed)
        } else if self.expanded {
            (ColorTransparentBackgroundSelected, ColorNeutralForeground2BrandSelected)
        } else if i.hovered {
            (ColorTransparentBackgroundHover, ColorNeutralForeground2BrandHover)
        } else {
            (ColorTransparentBackgroundSelected, ColorNeutralForeground2)
        };
        let radius = CornerRadius::from(g::BORDER_RADIUS_MEDIUM);
        let p = ui.painter();
        if i.pressed || i.hovered || self.expanded {
            p.rect_filled(rect, radius, t.color(fill));
        }
        let filled = i.pressed || i.hovered || self.expanded;
        paint::icon(p, rect.center(), ui.icons().info, filled, self.icon(), t.color(color));
        if i.focused {
            focus::outline(p, rect, radius, &t);
        }
        WidgetResponse::new(r.clone(), ())
    }

    fn semantics(&self, i18n: &I18n, _: &WidgetResponse, (): &()) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(
            StableId::of(&self.salt, "button"),
            Role::Button,
            crate::tr(i18n, msg::INFO_LABEL_INFO),
        )
        .clickable(Availability::Enabled);
        n.state.expanded = Some(self.expanded);
        Some(n)
    }
}
