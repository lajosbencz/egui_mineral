use egui::emath::Numeric;
use egui_mineral::date::Date;
use egui_mineral::form::{
    FieldCtx, FieldInput, FieldKind, FieldWidget, Form, FormFields, FormFooter, FormModel, FormResponse, FormRow,
    FormUi, Label, LabelSlot, NumberOptions, kind,
};
use egui_mineral::range::RangeModel;
use egui_mineral::select::{ItemKey, Selection};
use egui_mineral::semantic::{self, KeySalt, KeyedSalt, Role, SemanticNode, StableId};
use egui_mineral::widget::MineralUi;

use crate::Fluent;
use crate::actions::{ButtonAppearance, PushButton};
use crate::feedback::{MessageBar, MessageBarIntent};
use crate::inputs::{
    Checkbox, DatePicker, Dropdown, Field, FieldContext, Input, Slider, SpinButton, Switch, Textarea, Validation,
};
use crate::msg;

mod sealed {
    pub trait Sealed {}
    impl<T> Sealed for egui_mineral::form::Form<'_, T> {}
    impl<T> Sealed for egui_mineral::form::FormUi<'_, '_, '_, crate::Fluent, T> {}
}

/// Shows a core [`Form`](struct@Form) with Fluent inputs ([`FieldWidget`] for each core [`kind`]), each control in a [`Field`]
/// carrying its label, hint and error; sealed. The submit is a primary [`PushButton`] after the fields, form errors
/// show in an error [`MessageBar`].
pub trait FormExt<T>: sealed::Sealed {
    /// `add` places the fields.
    fn show<R>(
        self,
        ui: &mut MineralUi<'_, Fluent>,
        add: impl FnOnce(&mut FormUi<'_, '_, '_, Fluent, T>) -> R,
    ) -> FormResponse<R>;

    /// Every declared field in declaration order.
    fn show_all(self, ui: &mut MineralUi<'_, Fluent>) -> FormResponse<()>
    where
        T: FormFields<Fluent>;
}

impl<T: FormModel> FormExt<T> for Form<'_, T> {
    fn show<R>(
        self,
        ui: &mut MineralUi<'_, Fluent>,
        add: impl FnOnce(&mut FormUi<'_, '_, '_, Fluent, T>) -> R,
    ) -> FormResponse<R> {
        self.show_with(ui, add, footer)
    }

    fn show_all(self, ui: &mut MineralUi<'_, Fluent>) -> FormResponse<()>
    where
        T: FormFields<Fluent>,
    {
        #[expect(
            clippy::redundant_closure_for_method_calls,
            reason = "the method path is not general over the `FormUi` lifetimes"
        )]
        self.show_with(ui, |f| f.rest(), footer)
    }
}

/// Fluent extras of a form's field placement; sealed.
pub trait FormUiExt: sealed::Sealed {
    /// `subtitle2` heading of the fields that follow; a [`Role::Heading`] node keyed `<form salt>-<salt>`.
    fn section(&mut self, salt: impl KeySalt, title: &str) -> egui::Response;
}

impl<T> FormUiExt for FormUi<'_, '_, '_, Fluent, T> {
    fn section(&mut self, salt: impl KeySalt, title: &str) -> egui::Response {
        let key = StableId::of(self.form_salt(), salt);
        self.row(FormRow::Section, |ui, _| heading(ui, key, title))
    }
}

// Input, or a Textarea past one row; masked is a password input. Enter in an Input submits.
impl FieldWidget<kind::Text, String> for Fluent {
    fn show(ui: &mut MineralUi<'_, Self>, f: &mut FieldCtx<'_, String, kind::Text>) -> egui::Response {
        let o = f.spec().options();
        let label = f.label();
        let single = o.rows == 1;
        let placeholder = o.placeholder.as_ref().map(|p| p.text(ui.i18n()));
        let r = wrapped(ui, f, true, |ui, salt, text, cx| {
            if single {
                let mut input = Input::new(salt, label, text)
                    .password(o.masked)
                    .validation(cx.validation);
                if let Some(p) = placeholder {
                    input = input.placeholder(p);
                }
                ui.mineral(input).into_response()
            } else {
                let mut area = Textarea::new(salt, label, text)
                    .rows(u8::try_from(o.rows).unwrap_or(u8::MAX))
                    .validation(cx.validation);
                if let Some(p) = placeholder {
                    area = area.placeholder(p);
                }
                ui.mineral(area).into_response()
            }
        });
        if r.changed() {
            f.changed();
        }
        if r.lost_focus() {
            f.blurred();
            if single && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                f.submit();
            }
        }
        r
    }
}

// In a label column the column carries the label, not the control.
impl FieldWidget<kind::Check, bool> for Fluent {
    fn show(ui: &mut MineralUi<'_, Self>, f: &mut FieldCtx<'_, bool, kind::Check>) -> egui::Response {
        let label = f.label();
        let own = f.label_slot() == LabelSlot::Own;
        let r = wrapped(ui, f, !own, |ui, salt, checked, _| {
            ui.mineral(Checkbox::new(salt, label, checked).label_visible(own))
                .into_response()
        });
        pick(f, &r);
        r
    }
}

impl FieldWidget<kind::Switch, bool> for Fluent {
    fn show(ui: &mut MineralUi<'_, Self>, f: &mut FieldCtx<'_, bool, kind::Switch>) -> egui::Response {
        let label = f.label();
        let own = f.label_slot() == LabelSlot::Own;
        let r = wrapped(ui, f, !own, |ui, salt, checked, _| {
            ui.mineral(Switch::new(salt, label, checked).label_visible(own))
                .into_response()
        });
        pick(f, &r);
        r
    }
}

impl<K: ItemKey> FieldWidget<kind::Pick, Option<K>> for Fluent {
    fn show(ui: &mut MineralUi<'_, Self>, f: &mut FieldCtx<'_, Option<K>, kind::Pick>) -> egui::Response {
        let label = f.label();
        let items = choices(ui, f.spec().options().items());
        let r = wrapped(ui, f, true, |ui, salt, picked, cx| {
            ui.mineral(
                Dropdown::optional(salt, label, picked)
                    .items(items)
                    .validation(cx.validation),
            )
            .into_response()
        });
        pick(f, &r);
        r
    }
}

impl<K: ItemKey> FieldWidget<kind::MultiPick, Selection<K>> for Fluent {
    fn show(ui: &mut MineralUi<'_, Self>, f: &mut FieldCtx<'_, Selection<K>, kind::MultiPick>) -> egui::Response {
        let label = f.label();
        let items = choices(ui, f.spec().options().items());
        let r = wrapped(ui, f, true, |ui, salt, picked, cx| {
            ui.mineral(
                Dropdown::multi(salt, label, picked)
                    .items(items)
                    .validation(cx.validation),
            )
            .into_response()
        });
        pick(f, &r);
        r
    }
}

impl FieldWidget<kind::Date, Option<Date>> for Fluent {
    fn show(ui: &mut MineralUi<'_, Self>, f: &mut FieldCtx<'_, Option<Date>, kind::Date>) -> egui::Response {
        let label = f.label();
        let r = wrapped(ui, f, true, |ui, salt, day, cx| {
            ui.mineral(DatePicker::new(salt, label, day).validation(cx.validation))
                .into_response()
        });
        pick(f, &r);
        r
    }
}

impl FieldWidget<kind::Date, Date> for Fluent {
    fn show(ui: &mut MineralUi<'_, Self>, f: &mut FieldCtx<'_, Date, kind::Date>) -> egui::Response {
        let label = f.label();
        let r = wrapped(ui, f, true, |ui, salt, day, cx| {
            let mut value = Some(*day);
            let r = ui
                .mineral(DatePicker::new(salt, label, &mut value).validation(cx.validation))
                .into_response();
            if let Some(d) = value {
                *day = d;
            }
            r
        });
        pick(f, &r);
        r
    }
}

/// [`SpinButton`]; a step or committed text commits the field. Without
/// [`FieldSpec::range`](egui_mineral::form::FieldSpec::range) it spans `N`.
impl<N: Numeric + FieldInput> FieldWidget<kind::Number, N> for Fluent
where
    kind::Number: FieldKind<N, Options = NumberOptions>,
{
    fn show(ui: &mut MineralUi<'_, Self>, f: &mut FieldCtx<'_, N, kind::Number>) -> egui::Response {
        let label = f.label();
        let range = range_of::<N>(f.spec().options());
        let r = wrapped(ui, f, true, |ui, salt, n, cx| {
            ui.mineral(SpinButton::new(salt, label, range, n).validation(cx.validation))
                .into_response()
        });
        pick(f, &r);
        r
    }
}

impl<N: Numeric + FieldInput> FieldWidget<kind::Slider, N> for Fluent
where
    kind::Slider: FieldKind<N, Options = NumberOptions>,
{
    fn show(ui: &mut MineralUi<'_, Self>, f: &mut FieldCtx<'_, N, kind::Slider>) -> egui::Response {
        let label = f.label();
        let range = range_of::<N>(f.spec().options());
        let r = wrapped(ui, f, true, |ui, salt, n, _| {
            ui.mineral(Slider::new(salt, label, range, n)).into_response()
        });
        pick(f, &r);
        r
    }
}

fn range_of<N: Numeric>(o: &NumberOptions) -> RangeModel {
    o.range
        .unwrap_or_else(|| RangeModel::new(N::MIN.to_f64(), N::MAX.to_f64()))
}

fn choices<K: Clone>(ui: &MineralUi<'_, Fluent>, items: &[(K, Label)]) -> Vec<(K, String)> {
    items.iter().map(|(k, l)| (k.clone(), l.text(ui.i18n()))).collect()
}

fn wrapped<V: FieldInput, Kd: FieldKind<V>>(
    ui: &mut MineralUi<'_, Fluent>,
    f: &mut FieldCtx<'_, V, Kd>,
    labeled: bool,
    control: impl FnOnce(&mut MineralUi<'_, Fluent>, KeyedSalt, &mut V, FieldContext) -> egui::Response,
) -> egui::Response {
    let salt = f.salt();
    let mut field = Field::new(salt.child("field")).required(f.required());
    if labeled {
        field = field.label(f.label());
    }
    field = field.placed(f.label_slot());
    if let Some(h) = f.supporting() {
        field = field.hint(h);
    }
    if let Some(e) = f.visible_error() {
        field = field.validation(Validation::Error, e.message(ui.i18n()));
    }
    let value = f.value();
    field.show(ui, |ui, cx| control(ui, salt, value, cx)).inner
}

// Fluent Subtitle2, as a form grid section.
pub(crate) fn heading(ui: &mut MineralUi<'_, Fluent>, key: StableId, title: &str) -> egui::Response {
    use crate::{FluentExt as _, spec::ColorToken::ColorNeutralForeground1, typography::TypeRamp};
    let color = ui.tokens().color(ColorNeutralForeground1);
    let mut job = egui::text::LayoutJob::single_section(title.to_owned(), TypeRamp::Subtitle2.format(ui.ctx(), color));
    job.wrap.max_width = ui.available_width();
    let galley = ui.painter().layout_job(job);
    let width = ui.available_width();
    let (rect, r) = ui.allocate_exact_size(egui::vec2(width, galley.size().y), egui::Sense::hover());
    let x = ui
        .i18n()
        .direction()
        .start_x(rect, egui::Rangef::new(0.0, galley.size().x));
    ui.painter().galley(egui::pos2(x, rect.top()), galley, color);
    semantic::emit(ui.ctx(), Some(r.id), r.rect, || {
        SemanticNode::new(key, Role::Heading, title)
    });
    r
}

// A pick commits the field, as a blur does for text.
fn pick<V: FieldInput, Kd: FieldKind<V>>(f: &mut FieldCtx<'_, V, Kd>, r: &egui::Response) {
    if r.changed() {
        f.committed();
    } else if r.lost_focus() {
        f.blurred();
    }
}

fn footer(ui: &mut MineralUi<'_, Fluent>, f: &FormFooter<'_>) -> bool {
    let rt = ui.rt();
    if let Some(e) = f.error {
        let message = e.message(ui.i18n());
        MessageBar::new(f.form_salt.child("error"), message)
            .intent(MessageBarIntent::Error)
            .show(ui);
    }
    let label = f
        .submit_label
        .map_or_else(|| crate::tr(rt.i18n(), msg::SUBMIT), str::to_owned);
    ui.mineral(
        PushButton::new(f.form_salt.child("submit"), label)
            .appearance(ButtonAppearance::Primary)
            .disabled(!f.enabled),
    )
    .clicked()
}
