use egui::{Pos2, Rangef, Rect, Sense, vec2};
use egui_mineral::{
    i18n::{I18n, Sides},
    kit,
    layout::{Constraints, Measure, MinTarget},
    semantic::{self, AgentKey, KeyedSalt, Role, SemanticNode, StableId},
    widget::{Availability, MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use super::textbox::{self, Chrome, Content, FieldOutput, Fill, InputAppearance};
use crate::{
    Fluent, FluentExt, Size,
    icons::{ICON_DISMISS, ICON_SEARCH},
    spec::{
        defaults::{input as inp, search_box as sb},
        tokens::geometry as g,
    },
    widget::paint,
};

/// Fluent `SearchBox`: an [`Input`](super::Input) with a search icon and a dismiss button that clears the text
/// while focused; Escape clears it too.
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct SearchBox<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    value: &'a mut String,
    appearance: InputAppearance,
    size: Size,
    placeholder: Option<String>,
    disabled: bool,
}

impl<'a> SearchBox<'a> {
    /// `label` is the accessible name; agent key `<id_salt>-search-box`, dismiss button `<id_salt>-clear-button`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, value: &'a mut String) -> Self {
        Self {
            salt: id_salt.into(),
            key: AgentKey::Kind("search-box"),
            label: label.into(),
            value,
            appearance: InputAppearance::Outline,
            size: Size::Medium,
            placeholder: None,
            disabled: false,
        }
    }

    /// Default [`InputAppearance::Outline`].
    pub fn appearance(mut self, appearance: InputAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// Default [`Size::Medium`].
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Hint shown while empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    crate::widget::setters! {
        /// Greys out the field and ignores input.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-search-box` agent key.
        agent_key(key)
    }

    fn chrome(&self) -> Chrome {
        Chrome {
            appearance: self.appearance,
            size: self.size,
            invalid: false,
            disabled: self.disabled,
        }
    }

    // react-search useSearchBoxStyles.styles.ts useRootStyles: horizontal padding; useDismissStyles: icon size.
    fn root_pad(&self) -> f32 {
        match self.size {
            Size::Small => g::SPACING_HORIZONTAL_S_NUDGE,
            Size::Medium => g::SPACING_HORIZONTAL_S,
            Size::Large => g::SPACING_HORIZONTAL_M_NUDGE,
        }
    }

    fn dismiss_icon(&self) -> f32 {
        match self.size {
            Size::Small => sb::DISMISS_ICON_SMALL,
            Size::Medium => sb::DISMISS_ICON,
            Size::Large => sb::DISMISS_ICON_LARGE,
        }
    }
}

impl MineralWidget for SearchBox<'_> {
    type Kit = Fluent;
    type State = ();
    type Output = FieldOutput;

    kit::widget_meta!(salt, enabled, frame_sense);

    fn min_target(&self) -> MinTarget {
        MinTarget::None
    }

    fn layout(&self, _: &MineralUi<'_, Fluent>, c: Constraints, (): &()) -> Measure {
        let width = if c.max.x.is_finite() {
            c.max.x
        } else {
            inp::DEFAULT_WIDTH
        };
        c.constrain(vec2(width.min(sb::MAX_WIDTH), self.chrome().metrics().height))
            .into()
    }

    fn update(
        &mut self,
        ui: &mut MineralUi<'_, Fluent>,
        frame: &WidgetFrame<'_>,
        (): &mut (),
    ) -> WidgetResponse<FieldOutput> {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let t = ui.tokens();
        let dir = ui.i18n().direction();
        let base = ui.make_persistent_id(self.salt.id_salt());
        let edit_id = base.with("edit");
        let visual = frame.zones.visual;
        let inner = visual.shrink(g::STROKE_WIDTH_THIN);
        let focused = ctx.memory(|m| m.has_focus(edit_id));
        let mut m = self.chrome().metrics();
        let root = self.root_pad();
        m.root = root;
        m.input = g::SPACING_HORIZONTAL_S_NUDGE;
        m.gap = 0.0;
        let icon = self.dismiss_icon();
        let slot = Rect::from_center_size(
            Pos2::new(
                dir.reversed().start_x(inner, Rangef::new(root, root + icon)) + icon / 2.0,
                inner.center().y,
            ),
            vec2(icon, icon),
        );
        let held =
            ctx.pointer_interact_pos().is_some_and(|p| slot.contains(p)) && ctx.input(|i| i.pointer.primary_down());
        let dismiss = !self.disabled && !self.value.is_empty() && (focused || held);
        let end = root + if dismiss { g::SPACING_HORIZONTAL_M + icon } else { 0.0 };
        let Sides { left, right } = dir.sides(root + m.icon + m.input, end);
        let text = Rect::from_min_max(
            Pos2::new(inner.left() + left, inner.top()),
            Pos2::new((inner.right() - right).max(inner.left() + left), inner.bottom()),
        );
        let escape = focused && !self.disabled && kit::overlay::consume_escape_from(ctx, edit_id);
        if escape {
            self.value.clear();
        }
        let shown = textbox::show(
            ui,
            self.value,
            &frame.response,
            &Fill {
                base,
                frame: visual,
                chrome: self.chrome(),
                content: Content {
                    before: Some(ICON_SEARCH),
                    after: None,
                },
                placeholder: self.placeholder.as_deref(),
                mask: false,
                rows: None,
                reserve_end: 0.0,
                text_rect: Some(text),
                metrics: Some(m),
            },
        );
        let mut changed = shown.response.changed() || escape;
        if dismiss {
            let r = ui.interact(
                slot.expand(g::SPACING_HORIZONTAL_XS),
                base.with("dismiss"),
                Sense::click() - Sense::FOCUSABLE,
            );
            let color = t.color(crate::ColorToken::ColorNeutralForeground3);
            paint::icon(ui.painter(), slot.center(), ICON_DISMISS, false, icon, color);
            if r.clicked() {
                self.value.clear();
                changed = true;
                ctx.memory_mut(|mem| mem.request_focus(edit_id));
            }
            semantic::emit(ctx, Some(r.id), r.rect, || {
                SemanticNode::new(
                    StableId::of(&self.salt, "clear-button"),
                    Role::Button,
                    crate::tr(rt.i18n(), "mineral-search-clear"),
                )
                .pressable(Availability::Enabled)
            });
        }
        let out = FieldOutput {
            submitted: textbox::submitted(&shown.response),
            blurred: shown.response.lost_focus(),
        };
        let mut response = shown.response;
        if changed && !response.changed() {
            response.mark_changed();
        }
        WidgetResponse::new(response, out).with_changed(changed)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse<FieldOutput>, (): &()) -> Option<SemanticNode> {
        Some(
            SemanticNode::new(self.key.id(&self.salt), Role::SearchInput, &self.label)
                .value(self.value.clone())
                .editable(Availability::disabled_if(self.disabled)),
        )
    }
}
