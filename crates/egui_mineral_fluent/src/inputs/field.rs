use std::sync::Arc;

use egui::{Align, Color32, Galley, Pos2, Response, Sense, UiBuilder, text::LayoutJob, vec2};
use egui_mineral::{
    layout::Axis,
    semantic::{self, KeyedSalt, Role, SemanticNode, StableId},
    widget::MineralUi,
};

use super::label::FieldLabel;
use crate::ColorToken::{
    ColorNeutralForeground3, ColorPaletteDarkOrangeForeground1, ColorPaletteGreenForeground1,
    ColorPaletteRedForeground1,
};
use crate::{
    Fluent, FluentExt, FluentIcon, Size,
    icons::{ICON_CHECKMARK_CIRCLE, ICON_ERROR_CIRCLE, ICON_WARNING},
    spec::{
        defaults::field as fd,
        tokens::{geometry as g, typography as ty},
    },
    widget::paint,
};

/// `validationState` of a [`Field`] with a message.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Validation {
    /// `error`: red message, marks the control invalid.
    Error,
    /// `warning`.
    Warning,
    /// `success`.
    Success,
}

impl Validation {
    // react-field useFieldStyles.styles.ts useValidationMessageIconStyles
    fn icon(self) -> (FluentIcon, crate::ColorToken) {
        match self {
            Self::Error => (ICON_ERROR_CIRCLE, ColorPaletteRedForeground1),
            Self::Warning => (ICON_WARNING, ColorPaletteDarkOrangeForeground1),
            Self::Success => (ICON_CHECKMARK_CIRCLE, ColorPaletteGreenForeground1),
        }
    }
}

/// What a [`Field`] passes to its control, to set on it (`aria-invalid`, `disabled`, `size`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FieldContext {
    /// `aria-invalid` of the control: the state is [`Validation::Error`].
    pub invalid: bool,
    /// `disabled` of the control.
    pub disabled: bool,
    /// `required` of the control; the label carries the mark.
    pub required: bool,
    /// `size` of the control, the label's size.
    pub size: Size,
}

/// Outcome of [`Field::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FieldResponse<R> {
    /// Spans label, control and messages; has no node of its own.
    pub response: Response,
    /// What the control closure returned.
    pub inner: R,
}

/// Fluent `Field`: a label, hint and validation message around one control. Node keys
/// `<id_salt>-label`, `<id_salt>-validation-message`, `<id_salt>-hint`.
#[derive(Clone, Debug)]
#[must_use = "show with Field::show"]
pub struct Field {
    salt: KeyedSalt,
    label: Option<String>,
    hint: Option<String>,
    validation: Option<(Validation, String)>,
    orientation: Axis,
    size: Size,
    required: bool,
    disabled: bool,
    label_width: Option<f32>,
}

impl Field {
    /// Without a label a horizontal field keeps its 67% control column.
    pub fn new(id_salt: impl Into<KeyedSalt>) -> Self {
        Self {
            salt: id_salt.into(),
            label: None,
            hint: None,
            validation: None,
            orientation: Axis::Vertical,
            size: Size::Medium,
            required: false,
            disabled: false,
            label_width: None,
        }
    }

    /// Width of the label column of a horizontal field, label gap included; default 33% of the available width.
    pub fn label_width(mut self, label_width: f32) -> Self {
        self.label_width = Some(label_width);
        self
    }

    /// Label above, or in the 33% column when horizontal; node key `<id_salt>-label`.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// `hint`: secondary text under the control.
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// `validationState` and `validationMessage`.
    pub fn validation(mut self, state: Validation, message: impl Into<String>) -> Self {
        self.validation = Some((state, message.into()));
        self
    }

    /// `orientation`: [`Axis::Vertical`] (default) puts the label above, [`Axis::Horizontal`] in a 33% column.
    pub fn orientation(mut self, orientation: Axis) -> Self {
        self.orientation = orientation;
        self
    }

    /// Size of the label and the context for the control; default [`Size::Medium`].
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Appends the required mark to the label.
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Greys out the label and tells the control.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    fn context(&self) -> FieldContext {
        FieldContext {
            invalid: matches!(self.validation, Some((Validation::Error, _))),
            disabled: self.disabled,
            required: self.required,
            size: self.size,
        }
    }

    // useLabelStyles vertical, verticalLarge, horizontal, horizontalSmall, horizontalLarge: (above, below).
    fn label_padding(&self) -> (f32, f32) {
        match (self.orientation, self.size) {
            (Axis::Vertical, Size::Large) => (fd::LABEL_PADDING_VERTICAL_LARGE, fd::LABEL_PADDING_VERTICAL_LARGE),
            (Axis::Vertical, _) => (g::SPACING_VERTICAL_XXS, g::SPACING_VERTICAL_XXS),
            (Axis::Horizontal, Size::Small) => (g::SPACING_VERTICAL_XS, g::SPACING_VERTICAL_XS),
            (Axis::Horizontal, Size::Medium) => (g::SPACING_VERTICAL_S_NUDGE, g::SPACING_VERTICAL_S_NUDGE),
            (Axis::Horizontal, Size::Large) => (fd::LABEL_PADDING_HORIZONTAL_LARGE, fd::LABEL_PADDING_HORIZONTAL_LARGE),
        }
    }

    fn show_label(&self, ui: &mut MineralUi<'_, Fluent>, text: &str) {
        let (above, below) = self.label_padding();
        let extra = match self.orientation {
            Axis::Vertical if self.size == Size::Large => g::SPACING_VERTICAL_XS,
            Axis::Vertical => g::SPACING_VERTICAL_XXS,
            Axis::Horizontal => 0.0,
        };
        ui.add_space(above);
        ui.mineral(
            FieldLabel::new(self.salt.child("label"), text)
                .agent_key(StableId::of(&self.salt, "label"))
                .size(self.size)
                .required(self.required)
                .disabled(self.disabled),
        );
        ui.add_space(below + extra);
    }

    // `useSecondaryTextBaseClassName`: caption1, `marginTop` XXS; the icon hangs in the start padding.
    fn secondary(
        &self,
        ui: &mut MineralUi<'_, Fluent>,
        name: &'static str,
        text: &str,
        color: Color32,
        icon: Option<(FluentIcon, Color32)>,
    ) {
        let dir = ui.i18n().direction();
        let indent = icon.map_or(0.0, |_| fd::VALIDATION_ICON + g::SPACING_HORIZONTAL_XS);
        let format = crate::typography::format(
            ui.ctx(),
            ty::FONT_SIZE_BASE_200,
            ty::LINE_HEIGHT_BASE_200,
            ty::FONT_WEIGHT_REGULAR,
            Color32::PLACEHOLDER,
        );
        let mut job = LayoutJob::single_section(text.to_owned(), format);
        job.wrap.max_width = (ui.available_width() - indent).max(0.0);
        let galley: Arc<Galley> = ui.painter().layout_job(job);
        ui.add_space(g::SPACING_VERTICAL_XXS);
        let width = ui.available_width();
        let (rect, r) = ui.allocate_exact_size(vec2(width, galley.size().y), Sense::hover());
        let x = dir.start_x(rect, egui::Rangef::new(indent, indent + galley.size().x));
        ui.painter().galley(Pos2::new(x, rect.top()), galley, color);
        if let Some((icon, icon_color)) = icon {
            let at = dir.start_x(rect, egui::Rangef::new(0.0, fd::VALIDATION_ICON));
            let center = Pos2::new(
                at + fd::VALIDATION_ICON / 2.0,
                rect.top() + ty::LINE_HEIGHT_BASE_200 / 2.0,
            );
            paint::icon(ui.painter(), center, icon, true, fd::VALIDATION_ICON, icon_color);
        }
        semantic::emit(ui.ctx(), Some(r.id), r.rect, || {
            SemanticNode::new(StableId::of(&self.salt, name), Role::Label, text)
        });
    }

    fn show_messages(&self, ui: &mut MineralUi<'_, Fluent>) {
        let t = ui.tokens();
        if let Some((state, message)) = &self.validation {
            let (icon, icon_color) = state.icon();
            let color = t.color(match state {
                Validation::Error => ColorPaletteRedForeground1,
                Validation::Warning | Validation::Success => ColorNeutralForeground3,
            });
            self.secondary(
                ui,
                "validation-message",
                message,
                color,
                Some((icon, t.color(icon_color))),
            );
        }
        if let Some(hint) = &self.hint {
            self.secondary(ui, "hint", hint, t.color(ColorNeutralForeground3), None);
        }
    }

    fn column<R>(&self, rt: &crate::Rt, ui: &mut egui::Ui, add: impl FnOnce(&mut MineralUi<'_, Fluent>) -> R) -> R {
        let ui = &mut rt.ui(ui);
        let r = add(ui);
        self.show_messages(ui);
        r
    }

    /// Lays out the label, `add` (the control, given the [`FieldContext`]) and the messages.
    pub fn show<R>(
        self,
        ui: &mut MineralUi<'_, Fluent>,
        add: impl FnOnce(&mut MineralUi<'_, Fluent>, FieldContext) -> R,
    ) -> FieldResponse<R> {
        let rt = ui.rt();
        let dir = ui.i18n().direction();
        let cx = self.context();
        let total = ui.available_width();
        let out = ui
            .push_id(self.salt.id_salt(), |ui| match self.orientation {
                Axis::Horizontal => {
                    let share = self.label_width.unwrap_or(total * fd::HORIZONTAL_LABEL_SHARE).clamp(0.0, total);
                    ui.scope_builder(UiBuilder::new().layout(dir.row().with_cross_align(Align::Min)), |ui| {
                        if let Some(label) = &self.label {
                            let column = (share - g::SPACING_HORIZONTAL_M - ui.spacing().item_spacing.x).max(0.0);
                            ui.allocate_ui_with_layout(
                                vec2(column, 0.0),
                                dir.column(),
                                |ui| {
                                    ui.set_min_width(column);
                                    self.show_label(&mut rt.ui(ui), label);
                                },
                            );
                            ui.add_space(g::SPACING_HORIZONTAL_M);
                        } else {
                            ui.add_space(share);
                        }
                        ui.scope_builder(UiBuilder::new().layout(dir.column()), |ui| {
                            ui.set_width((total - share).max(0.0));
                            self.column(rt, ui, |ui| add(ui, cx))
                        })
                        .inner
                    })
                }
                Axis::Vertical => ui.scope_builder(UiBuilder::new().layout(dir.column()), |ui| {
                    if let Some(label) = &self.label {
                        self.show_label(&mut rt.ui(ui), label);
                    }
                    self.column(rt, ui, |ui| add(ui, cx))
                }),
            })
            .inner;
        FieldResponse {
            response: out.response,
            inner: out.inner,
        }
    }
}
