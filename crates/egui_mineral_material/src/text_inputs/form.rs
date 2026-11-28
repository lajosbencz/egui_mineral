use crate::Material;
use crate::MaterialExt;
use crate::msg;
use egui::emath::Numeric;
use egui_mineral::date::Date;
use egui_mineral::form::{
    FieldCtx, FieldInput, FieldKind, FieldWidget, Form, FormFields, FormFooter, FormModel, FormResponse, FormRow,
    FormUi, LabelSlot, NumberOptions, kind,
};
use egui_mineral::range::RangeModel;
use egui_mineral::select::{ItemKey, Selection};
use egui_mineral::semantic::{self, KeySalt, Role, SemanticNode, StableId};
use egui_mineral::widget::MineralUi;

use crate::actions::PushButton;
use crate::selection::{CheckButton, DatePicker, DisplayMode, Select, Switch, ValueSlider};
use crate::spec::tokens::filled_text_field as m;
use crate::text_inputs::TextField;
use crate::widget::parts;

mod sealed {
    pub trait Sealed {}
    impl<T> Sealed for egui_mineral::form::Form<'_, T> {}
    impl<T> Sealed for egui_mineral::form::FormUi<'_, '_, '_, crate::Material, T> {}
}

/// Shows a core [`Form`](struct@Form) with Material fields ([`FieldWidget`] for each core [`kind`]); sealed. The submit is a
/// filled [`PushButton`] after the fields; form errors use the field error style; a required field's label gets a
/// `*`.
pub trait FormExt<T>: sealed::Sealed {
    /// `add` places the fields.
    fn show<R>(
        self,
        ui: &mut MineralUi<'_, Material>,
        add: impl FnOnce(&mut FormUi<'_, '_, '_, Material, T>) -> R,
    ) -> FormResponse<R>;

    /// Every declared field in declaration order.
    fn show_all(self, ui: &mut MineralUi<'_, Material>) -> FormResponse<()>
    where
        T: FormFields<Material>;
}

impl<T: FormModel> FormExt<T> for Form<'_, T> {
    fn show<R>(
        self,
        ui: &mut MineralUi<'_, Material>,
        add: impl FnOnce(&mut FormUi<'_, '_, '_, Material, T>) -> R,
    ) -> FormResponse<R> {
        self.show_with(ui, add, footer)
    }

    fn show_all(self, ui: &mut MineralUi<'_, Material>) -> FormResponse<()>
    where
        T: FormFields<Material>,
    {
        #[expect(
            clippy::redundant_closure_for_method_calls,
            reason = "the method path is not general over the `FormUi` lifetimes"
        )]
        self.show_with(ui, |f| f.rest(), footer)
    }
}

/// Material extras of a form's field placement; sealed.
pub trait FormUiExt: sealed::Sealed {
    /// Title small heading of the fields that follow; a [`Role::Heading`] node keyed `<form salt>-<salt>`.
    fn section(&mut self, salt: impl KeySalt, title: &str) -> egui::Response;
}

impl<T> FormUiExt for FormUi<'_, '_, '_, Material, T> {
    fn section(&mut self, salt: impl KeySalt, title: &str) -> egui::Response {
        let key = StableId::of(self.form_salt(), salt);
        self.row(FormRow::Section, |ui, _| heading(ui, key, title))
    }
}

// Every core kind; Number needs `extras` (NumberField).
impl egui_mineral::form::DynWidgets for Material {
    const WIDGETS: egui_mineral::form::DynTable<Self> = egui_mineral::form::DynTable {
        text: Some(FieldWidget::show),
        check: Some(FieldWidget::show),
        switch: Some(FieldWidget::show),
        #[cfg(feature = "extras")]
        number: Some(FieldWidget::show),
        #[cfg(not(feature = "extras"))]
        number: None,
        slider: Some(FieldWidget::show),
        date: Some(FieldWidget::show),
        pick: Some(FieldWidget::show),
        multi_pick: Some(FieldWidget::show),
    };
}

// TextField; Enter in a single row submits.
impl FieldWidget<kind::Text, String> for Material {
    fn show(ui: &mut MineralUi<'_, Self>, f: &mut FieldCtx<'_, String, kind::Text>) -> egui::Response {
        let (error, label) = decor(ui, f);
        let visible = own_label(f);
        let o = f.spec().options();
        let placeholder = o.placeholder.as_ref().map(|p| p.text(ui.i18n()));
        let supporting = f.supporting();
        let mut tf = TextField::new(f.salt(), label, f.value())
            .label_visible(visible)
            .masked(o.masked)
            .rows(o.rows)
            .maybe_error(error);
        if let Some(p) = placeholder {
            tf = tf.placeholder(p);
        }
        if let Some(h) = supporting {
            tf = tf.supporting(h);
        }
        let r = ui.mineral(tf);
        if r.changed() {
            f.changed();
        }
        if r.lost_focus() {
            f.blurred();
            if o.rows == 1 && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                f.submit();
            }
        }
        r.into_response()
    }
}

impl FieldWidget<kind::Check, bool> for Material {
    fn show(ui: &mut MineralUi<'_, Self>, f: &mut FieldCtx<'_, bool, kind::Check>) -> egui::Response {
        let (error, label) = decor(ui, f);
        let visible = own_label(f);
        let supporting = f.supporting().filter(|_| error.is_none());
        let salt = f.salt();
        let rt = ui.rt();
        let (response, changed, lost_focus) = ui
            .scope(|ui| {
                let r = rt.ui(ui).mineral(
                    CheckButton::new(salt, label, f.value())
                        .maybe_error(error)
                        .label_visible(visible),
                );
                if let Some(h) = supporting {
                    let t = rt.tokens();
                    parts::type_label(ui, h, m::SUPPORTING_FONT, t.role(m::SUPPORTING_COLOR), 0);
                }
                (r.response().clone(), r.changed(), r.lost_focus())
            })
            .inner;
        pick(f, changed, lost_focus);
        response
    }
}

// MD3 switches have no error state: the error or help text goes below.
impl FieldWidget<kind::Switch, bool> for Material {
    fn show(ui: &mut MineralUi<'_, Self>, f: &mut FieldCtx<'_, bool, kind::Switch>) -> egui::Response {
        let (error, label) = decor(ui, f);
        let visible = own_label(f);
        let supporting = f.supporting();
        let salt = f.salt();
        let (response, changed, lost_focus) = below(ui, error.as_deref(), supporting, |ui| {
            let r = ui.mineral(Switch::new(salt, label, f.value()).label_visible(visible));
            (r.response().clone(), r.changed(), r.lost_focus())
        });
        pick(f, changed, lost_focus);
        response
    }
}

impl<K: ItemKey> FieldWidget<kind::Pick, Option<K>> for Material {
    fn show(ui: &mut MineralUi<'_, Self>, f: &mut FieldCtx<'_, Option<K>, kind::Pick>) -> egui::Response {
        let (error, label) = decor(ui, f);
        let visible = own_label(f);
        let items = choices(ui, f.spec().options().items());
        let supporting = f.supporting();
        let mut select = Select::optional(f.salt(), label, f.value())
            .label_visible(visible)
            .items(items)
            .maybe_error(error);
        if let Some(h) = supporting {
            select = select.supporting(h);
        }
        let r = ui.mineral(select);
        pick(f, r.changed(), r.lost_focus());
        r.response().clone()
    }
}

impl<K: ItemKey> FieldWidget<kind::MultiPick, Selection<K>> for Material {
    fn show(ui: &mut MineralUi<'_, Self>, f: &mut FieldCtx<'_, Selection<K>, kind::MultiPick>) -> egui::Response {
        let (error, label) = decor(ui, f);
        let visible = own_label(f);
        let items = choices(ui, f.spec().options().items());
        let supporting = f.supporting();
        let mut select = Select::multi(f.salt(), label, f.value())
            .label_visible(visible)
            .items(items)
            .maybe_error(error);
        if let Some(h) = supporting {
            select = select.supporting(h);
        }
        let r = ui.mineral(select);
        pick(f, r.changed(), r.lost_focus());
        r.response().clone()
    }
}

// Docked DatePicker in input mode.
impl FieldWidget<kind::Date, Option<Date>> for Material {
    fn show(ui: &mut MineralUi<'_, Self>, f: &mut FieldCtx<'_, Option<Date>, kind::Date>) -> egui::Response {
        let (error, label) = decor(ui, f);
        let visible = own_label(f);
        let supporting = f.supporting();
        let r = date_picker(ui, f.salt(), label, visible, error, supporting, f.value());
        pick(f, r.0, false);
        r.1
    }
}

impl FieldWidget<kind::Date, Date> for Material {
    fn show(ui: &mut MineralUi<'_, Self>, f: &mut FieldCtx<'_, Date, kind::Date>) -> egui::Response {
        let (error, label) = decor(ui, f);
        let visible = own_label(f);
        let supporting = f.supporting();
        let mut value = Some(*f.value());
        let (changed, response) = date_picker(ui, f.salt(), label, visible, error, supporting, &mut value);
        if let Some(d) = value {
            *f.value() = d;
        }
        pick(f, changed, false);
        response
    }
}

/// [`NumberField`](crate::extras::NumberField); a step or committed text commits the field. Without
/// [`FieldSpec::range`](egui_mineral::form::FieldSpec::range) it spans `N`.
#[cfg(feature = "extras")]
impl<N: Numeric + FieldInput> FieldWidget<kind::Number, N> for Material
where
    kind::Number: FieldKind<N, Options = NumberOptions>,
{
    fn show(ui: &mut MineralUi<'_, Self>, f: &mut FieldCtx<'_, N, kind::Number>) -> egui::Response {
        let (error, label) = decor(ui, f);
        let visible = own_label(f);
        let range = range_of::<N>(f.spec().options());
        let supporting = f.supporting();
        let mut field = crate::extras::NumberField::new(f.salt(), label, range, f.value())
            .maybe_error(error)
            .label_visible(visible);
        if let Some(h) = supporting {
            field = field.supporting(h);
        }
        let r = ui.mineral(field);
        pick(f, r.changed(), r.lost_focus());
        r.into_response()
    }
}

// MD3 sliders have no error state: the error or help text goes below. Without a range it spans `N`.
impl<N: Numeric + FieldInput> FieldWidget<kind::Slider, N> for Material
where
    kind::Slider: FieldKind<N, Options = NumberOptions>,
{
    fn show(ui: &mut MineralUi<'_, Self>, f: &mut FieldCtx<'_, N, kind::Slider>) -> egui::Response {
        let (error, label) = decor(ui, f);
        let range = range_of::<N>(f.spec().options());
        let supporting = f.supporting();
        let salt = f.salt();
        let (response, changed, lost_focus) = below(ui, error.as_deref(), supporting, |ui| {
            let r = ui.mineral(ValueSlider::new(salt, label, range, f.value()));
            (r.response().clone(), r.changed(), r.lost_focus())
        });
        pick(f, changed, lost_focus);
        response
    }
}

fn range_of<N: Numeric>(o: &NumberOptions) -> RangeModel {
    o.range
        .unwrap_or_else(|| RangeModel::new(N::MIN.to_f64(), N::MAX.to_f64()))
}

fn choices<K: Clone>(ui: &MineralUi<'_, Material>, items: &[(K, egui_mineral::form::Label)]) -> Vec<(K, String)> {
    items.iter().map(|(k, l)| (k.clone(), l.text(ui.i18n()))).collect()
}

fn date_picker(
    ui: &mut MineralUi<'_, Material>,
    salt: egui_mineral::semantic::KeyedSalt,
    label: String,
    visible: bool,
    error: Option<String>,
    supporting: Option<&str>,
    value: &mut Option<Date>,
) -> (bool, egui::Response) {
    let mut picker = DatePicker::new(salt, label, value)
        .label_visible(visible)
        .input_mode(DisplayMode::Input)
        .maybe_error(error);
    if let Some(h) = supporting {
        picker = picker.supporting(h);
    }
    let r = picker.show(ui);
    (r.changed, r.response)
}

// `add`, then the error, or else the help text, as text field supporting text.
fn below<R>(
    ui: &mut MineralUi<'_, Material>,
    error: Option<&str>,
    supporting: Option<&str>,
    add: impl FnOnce(&mut MineralUi<'_, Material>) -> R,
) -> R {
    let rt = ui.rt();
    ui.scope(|ui| {
        let ui = &mut rt.ui(ui);
        let r = add(ui);
        let t = rt.tokens();
        if let Some(e) = error {
            parts::type_label(ui, e, m::SUPPORTING_FONT, t.role(m::ERROR_SUPPORTING_COLOR), 0);
        } else if let Some(h) = supporting {
            parts::type_label(ui, h, m::SUPPORTING_FONT, t.role(m::SUPPORTING_COLOR), 0);
        }
        r
    })
    .inner
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
fn own_label<V: FieldInput, Kd: FieldKind<V>>(f: &FieldCtx<'_, V, Kd>) -> bool {
    f.label_slot() != LabelSlot::External
}

// Shown error text and the label, starred when required.
fn decor<V: FieldInput, Kd: FieldKind<V>>(
    ui: &MineralUi<'_, Material>,
    f: &FieldCtx<'_, V, Kd>,
) -> (Option<String>, String) {
    let label = if f.required() {
        format!("{}*", f.label())
    } else {
        f.label().to_owned()
    };
    (f.visible_error().map(|e| e.message(ui.i18n())), label)
}

// A pick commits the field, as a blur does for text.
fn pick<V: FieldInput, Kd: FieldKind<V>>(f: &mut FieldCtx<'_, V, Kd>, changed: bool, lost_focus: bool) {
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
