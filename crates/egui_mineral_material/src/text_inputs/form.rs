use crate::Material;
use crate::MaterialExt;
use crate::msg;
#[cfg(feature = "extras")]
use egui::emath::Numeric;
use egui_mineral::date::Date;
#[cfg(feature = "extras")]
use egui_mineral::form::Blank;
use egui_mineral::form::{FieldCtx, FieldSpec, Form, FormFooter, FormResponse, FormRow, FormUi, LabelSlot};
#[cfg(feature = "extras")]
use egui_mineral::range::RangeModel;
use egui_mineral::select::{ItemKey, Selection};
use egui_mineral::semantic::{self, KeySalt, KeyedSalt, Role, SemanticNode, StableId};
use egui_mineral::widget::MineralUi;

use crate::actions::PushButton;
use crate::selection::{CheckButton, DatePicker, DisplayMode, Select};
use crate::spec::tokens::filled_text_field as m;
use crate::text_inputs::TextField;
use crate::widget::parts;

mod sealed {
    pub trait Sealed {}
    impl<T> Sealed for egui_mineral::form::Form<'_, T> {}
    impl Sealed for egui_mineral::form::FormUi<'_, '_, crate::Material> {}
}

/// Shows a core [`Form`] with stylekit fields; sealed.
pub trait FormExt<T>: sealed::Sealed {
    /// `add` places the fields with [`FormUiExt`]; the submit is a filled [`PushButton`] after them, form errors use
    /// the field error style.
    fn show<R>(
        self,
        ui: &mut MineralUi<'_, Material>,
        add: impl FnOnce(&mut FormUi<'_, '_, Material>, &mut T) -> R,
    ) -> FormResponse<R>;
}

impl<T> FormExt<T> for Form<'_, T> {
    fn show<R>(
        self,
        ui: &mut MineralUi<'_, Material>,
        add: impl FnOnce(&mut FormUi<'_, '_, Material>, &mut T) -> R,
    ) -> FormResponse<R> {
        self.show_with(ui, add, footer)
    }
}

/// Typed fields of a Material form; sealed. `build` adds the spec's required flag, help text and validators; a
/// required field's label gets a `*`.
pub trait FormUiExt: sealed::Sealed {
    /// Title small heading of the fields that follow; a [`Role::Heading`] node keyed `<form salt>-<salt>`.
    fn section(&mut self, salt: impl KeySalt, title: &str) -> egui::Response;

    /// [`TextField`]; masked and multi-row per [`FieldSpec::masked`] and [`FieldSpec::rows`]. Enter in a single
    /// row submits.
    fn text(
        &mut self,
        salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        value: &mut String,
        build: impl FnOnce(FieldSpec<String>) -> FieldSpec<String>,
    ) -> egui::Response;

    /// [`CheckButton`] labeled by the field label; required means checked.
    fn check(
        &mut self,
        salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        value: &mut bool,
        build: impl FnOnce(FieldSpec<bool>) -> FieldSpec<bool>,
    ) -> egui::Response;

    /// Optional [`Select`] of `(value, label)` options.
    fn select<K: ItemKey>(
        &mut self,
        salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        value: &mut Option<K>,
        options: impl IntoIterator<Item = (K, impl Into<String>)>,
        build: impl FnOnce(FieldSpec<Option<K>>) -> FieldSpec<Option<K>>,
    ) -> egui::Response;

    /// Multi [`Select`] of `(value, label)` options.
    fn multi_select<K: ItemKey>(
        &mut self,
        salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        value: &mut Selection<K>,
        options: impl IntoIterator<Item = (K, impl Into<String>)>,
        build: impl FnOnce(FieldSpec<Selection<K>>) -> FieldSpec<Selection<K>>,
    ) -> egui::Response;

    /// Docked [`DatePicker`] in input mode.
    fn date(
        &mut self,
        salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        value: &mut Option<Date>,
        build: impl FnOnce(FieldSpec<Option<Date>>) -> FieldSpec<Option<Date>>,
    ) -> egui::Response;

    /// [`NumberField`](crate::extras::NumberField) over `range`; a step or committed text commits the field.
    #[cfg(feature = "extras")]
    fn number<N: Numeric + Blank>(
        &mut self,
        salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        value: &mut N,
        range: impl Into<RangeModel>,
        build: impl FnOnce(FieldSpec<N>) -> FieldSpec<N>,
    ) -> egui::Response;
}

impl FormUiExt for FormUi<'_, '_, Material> {
    fn section(&mut self, salt: impl KeySalt, title: &str) -> egui::Response {
        let key = StableId::of(self.form_salt(), salt);
        self.row(FormRow::Section, |ui, _| heading(ui, key, title))
    }

    fn text(
        &mut self,
        salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        value: &mut String,
        build: impl FnOnce(FieldSpec<String>) -> FieldSpec<String>,
    ) -> egui::Response {
        self.field(salt, &build(FieldSpec::new(label)), value, |ui, f| {
            let (error, label) = decor(ui, f);
            let visible = own_label(f);
            let spec = f.spec();
            let mut tf = TextField::new(f.salt(), label, f.value())
                .label_visible(visible)
                .masked(spec.is_masked())
                .rows(spec.row_count())
                .maybe_error(error);
            if let Some(h) = spec.supporting_text() {
                tf = tf.supporting(h);
            }
            let r = ui.mineral(tf);
            if r.changed() {
                f.changed();
            }
            if r.lost_focus() {
                f.blurred();
                if spec.row_count() == 1 && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    f.submit();
                }
            }
            r.into_response()
        })
    }

    fn check(
        &mut self,
        salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        value: &mut bool,
        build: impl FnOnce(FieldSpec<bool>) -> FieldSpec<bool>,
    ) -> egui::Response {
        self.field(salt, &build(FieldSpec::new(label)), value, |ui, f| {
            let (error, label) = decor(ui, f);
            let visible = own_label(f);
            let spec = f.spec();
            let has_error = error.is_some();
            let salt = f.salt();
            let rt = ui.rt();
            let (response, changed, lost_focus) = ui
                .scope(|ui| {
                    let r = rt.ui(ui).mineral(
                        CheckButton::new(salt, label, f.value())
                            .maybe_error(error)
                            .label_visible(visible),
                    );
                    if let Some(h) = spec.supporting_text().filter(|_| !has_error) {
                        let t = rt.tokens();
                        parts::type_label(ui, h, m::SUPPORTING_FONT, t.role(m::SUPPORTING_COLOR), 0);
                    }
                    (r.response().clone(), r.changed(), r.lost_focus())
                })
                .inner;
            pick(f, changed, lost_focus);
            response
        })
    }

    fn select<K: ItemKey>(
        &mut self,
        salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        value: &mut Option<K>,
        options: impl IntoIterator<Item = (K, impl Into<String>)>,
        build: impl FnOnce(FieldSpec<Option<K>>) -> FieldSpec<Option<K>>,
    ) -> egui::Response {
        self.field(salt, &build(FieldSpec::new(label)), value, |ui, f| {
            let (error, label) = decor(ui, f);
            let visible = own_label(f);
            let spec = f.spec();
            let options = options.into_iter().map(|(k, l)| (k, l.into()));
            let mut select = Select::optional(f.salt(), label, f.value())
                .label_visible(visible)
                .items(options)
                .maybe_error(error);
            if let Some(h) = spec.supporting_text() {
                select = select.supporting(h);
            }
            let r = ui.mineral(select);
            pick(f, r.changed(), r.lost_focus());
            r.response().clone()
        })
    }

    fn multi_select<K: ItemKey>(
        &mut self,
        salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        value: &mut Selection<K>,
        options: impl IntoIterator<Item = (K, impl Into<String>)>,
        build: impl FnOnce(FieldSpec<Selection<K>>) -> FieldSpec<Selection<K>>,
    ) -> egui::Response {
        self.field(salt, &build(FieldSpec::new(label)), value, |ui, f| {
            let (error, label) = decor(ui, f);
            let visible = own_label(f);
            let spec = f.spec();
            let options = options.into_iter().map(|(k, l)| (k, l.into()));
            let mut select = Select::multi(f.salt(), label, f.value())
                .label_visible(visible)
                .items(options)
                .maybe_error(error);
            if let Some(h) = spec.supporting_text() {
                select = select.supporting(h);
            }
            let r = ui.mineral(select);
            pick(f, r.changed(), r.lost_focus());
            r.response().clone()
        })
    }

    fn date(
        &mut self,
        salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        value: &mut Option<Date>,
        build: impl FnOnce(FieldSpec<Option<Date>>) -> FieldSpec<Option<Date>>,
    ) -> egui::Response {
        self.field(salt, &build(FieldSpec::new(label)), value, |ui, f| {
            let (error, label) = decor(ui, f);
            let visible = own_label(f);
            let spec = f.spec();
            let mut picker = DatePicker::new(f.salt(), label, f.value())
                .label_visible(visible)
                .input_mode(DisplayMode::Input)
                .maybe_error(error);
            if let Some(h) = spec.supporting_text() {
                picker = picker.supporting(h);
            }
            let r = picker.show(ui);
            pick(f, r.changed, false);
            r.response
        })
    }

    #[cfg(feature = "extras")]
    fn number<N: Numeric + Blank>(
        &mut self,
        salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        value: &mut N,
        range: impl Into<RangeModel>,
        build: impl FnOnce(FieldSpec<N>) -> FieldSpec<N>,
    ) -> egui::Response {
        let range = range.into();
        self.field(salt, &build(FieldSpec::new(label)), value, |ui, f| {
            let (error, label) = decor(ui, f);
            let visible = own_label(f);
            let spec = f.spec();
            let mut field = crate::extras::NumberField::new(f.salt(), label, range, f.value())
                .maybe_error(error)
                .label_visible(visible);
            if let Some(h) = spec.supporting_text() {
                field = field.supporting(h);
            }
            let r = ui.mineral(field);
            pick(f, r.changed(), r.lost_focus());
            r.into_response()
        })
    }
}

// Title small, as a form grid section.
pub(crate) fn heading(ui: &mut MineralUi<'_, Material>, key: StableId, title: &str) -> egui::Response {
    let color = ui.tokens().color.on_surface_variant;
    let g = parts::type_wrapped(
        ui.painter(),
        title,
        crate::spec::TypeScale::TitleSmall,
        color,
        ui.available_width(),
        0,
    );
    let width = ui.available_width();
    let (rect, r) = ui.allocate_exact_size(egui::vec2(width, g.size().y), egui::Sense::hover());
    let x = ui.i18n().direction().start_x(rect, egui::Rangef::new(0.0, g.size().x));
    ui.painter()
        .galley(egui::pos2(x, rect.top()), g, egui::Color32::PLACEHOLDER);
    semantic::emit(ui.ctx(), Some(r.id), r.rect, || {
        SemanticNode::new(key, Role::Heading, title)
    });
    r
}

// Material has no horizontal field layout: a label column falls back to the field's own label.
fn own_label<V: ?Sized>(f: &FieldCtx<'_, V>) -> bool {
    f.label_slot() != LabelSlot::External
}

// Shown error text and the label, starred when required.
fn decor<V: ?Sized>(ui: &MineralUi<'_, Material>, f: &FieldCtx<'_, V>) -> (Option<String>, String) {
    let spec = f.spec();
    let label = if spec.is_required() {
        format!("{}*", spec.label())
    } else {
        spec.label().to_owned()
    };
    (f.visible_error().map(|e| e.message(ui.i18n())), label)
}

// A pick commits the field, as a blur does for text.
fn pick<V: ?Sized>(f: &mut FieldCtx<'_, V>, changed: bool, lost_focus: bool) {
    if changed {
        f.committed();
    } else if lost_focus {
        f.blurred();
    }
}

fn footer(ui: &mut MineralUi<'_, Material>, f: &FormFooter<'_>) -> bool {
    let rt = ui.rt();
    // Form-level errors read as a text field's error supporting text.
    if let Some(e) = f.error {
        let t = rt.tokens();
        let color = t.role(m::ERROR_SUPPORTING_COLOR);
        let message = e.message(ui.i18n());
        parts::type_label(ui, &message, m::SUPPORTING_FONT, color, 0);
    }
    let label = f
        .submit_label
        .map_or_else(|| crate::tr(rt.i18n(), msg::SUBMIT), str::to_owned);
    rt.ui(ui)
        .mineral(PushButton::new(f.form_salt.child("submit"), label).disabled(!f.enabled))
        .clicked()
}
