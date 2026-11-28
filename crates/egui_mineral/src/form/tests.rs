use std::cell::RefCell;

use super::*;
use crate::{
    select::Selection,
    test_support::{TestKit, en_us, harness},
};

crate::i18n::messages!(mod tmsg = "src/form/tests.ftl", crate = crate);

thread_local! {
    // (salt key, label, required) of each field a widget showed.
    static SHOWN: RefCell<Vec<(String, String, bool)>> = const { RefCell::new(Vec::new()) };
}

fn shown() -> Vec<(String, String, bool)> {
    SHOWN.with_borrow_mut(std::mem::take)
}

fn record<V: FieldInput, Kd: FieldKind<V>>(
    ui: &mut MineralUi<'_, TestKit>,
    f: &mut FieldCtx<'_, V, Kd>,
) -> egui::Response {
    SHOWN.with_borrow_mut(|s| s.push((f.salt().key().to_owned(), f.label().to_owned(), f.required())));
    ui.allocate_response(egui::Vec2::ZERO, egui::Sense::hover())
}

impl<V: FieldInput, Kd: FieldKind<V>> FieldWidget<Kd, V> for TestKit {
    fn show(ui: &mut MineralUi<'_, Self>, f: &mut FieldCtx<'_, V, Kd>) -> egui::Response {
        record(ui, f)
    }
}

#[expect(clippy::ptr_arg, reason = "a `Validator<String>` callback takes `&String`")]
fn is_email(e: &String, _: &dyn LocaleFormat) -> Result<(), ValidationError> {
    if e.contains('@') {
        Ok(())
    } else {
        Err(ValidationError::new("email", "Not an email"))
    }
}

#[derive(Default, Form)]
#[form(check = Self::cross)]
struct Signup {
    #[form(label = "Email", required, check = is_email)]
    email: String,
    #[form(required, check = MinLength(3), masked, on = Change)]
    pw: String,
    #[form(label = "Repeat", masked)]
    pw2: String,
    #[form(label = "Extra", kind = Switch)]
    extra: bool,
    #[form(kind = Date)]
    start: Option<crate::date::Date>,
    #[form(skip)]
    need_extra: bool,
}

impl Signup {
    fn cross(&self, v: &mut Validation<'_, Self>) {
        if self.pw != self.pw2 {
            v.error(Self::PW2, ValidationError::new("mismatch", "Passwords differ"));
        }
        if self.need_extra {
            v.field(Self::EXTRA).required();
        }
    }

    fn valid() -> Self {
        Self {
            email: "a@b".into(),
            pw: "abc".into(),
            pw2: "abc".into(),
            ..Self::default()
        }
    }
}

fn code(state: &FormState, path: impl Into<FieldPath>) -> Option<String> {
    state.field(path)?.error().map(|e| e.code.to_string())
}

#[test]
fn derive_declares_keys_rules_and_presentation() {
    assert_eq!((Signup::EMAIL.name(), Signup::EXTRA.index()), ("email", 3));
    let r = Signup::default().report(&en_us());
    assert_eq!(r.first_invalid(), Some(&Signup::EMAIL.into()));
    assert!(r.is_required(Signup::EMAIL) && r.is_required(Signup::PW) && !r.is_required(Signup::PW2));
    assert_eq!(
        r.error(Signup::EMAIL).map(|e| e.code.as_ref()),
        Some(i18n::msg::REQUIRED.id())
    );
    assert!(Signup::valid().report(&en_us()).is_valid());

    let s = Signup::default();
    let pw = Signup::PW.spec(&s);
    assert_eq!(
        pw.label(),
        &Label::from("Pw"),
        "labels default to the humanized field name"
    );
    assert!(pw.options().masked);
    assert_eq!(pw.validates_on(), ValidateOn::Change);
    assert_eq!(Signup::EMAIL.spec(&s).label(), &Label::from("Email"));
    let _: FieldKey<Signup, bool, kind::Switch> = Signup::EXTRA;
    let _: FieldKey<Signup, Option<crate::date::Date>, kind::Date> = Signup::START;
    let _: FieldKey<Signup, String, kind::Text> = Signup::EMAIL;
}

#[derive(Form)]
struct Item {
    #[form(check = NumberRange::at_least(1.0))]
    qty: u32,
}

#[derive(Form)]
#[form(check = Self::non_empty)]
struct Order {
    #[form(nested)]
    main_item: Item,
    #[form(list)]
    items: Vec<Item>,
}

impl Order {
    fn non_empty(&self, v: &mut Validation<'_, Self>) {
        if self.items.is_empty() {
            v.form_error(ValidationError::new("empty", "No items"));
        }
    }
}

#[test]
fn nested_and_list_paths_report_headless() {
    let order = Order {
        main_item: Item { qty: 0 },
        items: vec![Item { qty: 2 }, Item { qty: 0 }],
    };
    let r = order.report(&en_us());
    let first = r.first_invalid().expect("invalid");
    assert_eq!(*first, Order::MAIN_ITEM.then(Item::QTY));
    assert_eq!(first.salt().key(), "main-item-qty");
    assert!(first.starts_with(&Order::MAIN_ITEM.into()));
    assert!(r.error(Order::ITEMS.at(1).then(Item::QTY)).is_some());
    assert!(r.error(Order::ITEMS.at(0).then(Item::QTY)).is_none());
    let empty = Order {
        main_item: Item { qty: 1 },
        items: Vec::new(),
    }
    .report(&en_us());
    assert_eq!(empty.form_error().map(|e| e.code.as_ref()), Some("empty"));
}

#[derive(Default, Form)]
#[form(messages = tmsg)]
struct Contact {
    email: String,
    phone: String,
}

#[test]
fn messages_convention_resolves_labels_and_attributes() {
    let ctx = egui::Context::default();
    let i18n = crate::i18n::I18n::with_format(
        &ctx,
        crate::i18n::locale!("en-US"),
        crate::i18n::AsciiFormat::default(),
        &[tmsg::CATALOG],
    )
    .expect("catalog");
    let c = Contact::default();
    let (email, phone) = (Contact::EMAIL.spec(&c), Contact::PHONE.spec(&c));
    assert_eq!(email.label().text(&i18n), "E-mail");
    let help = |s: &FieldSpec<String>| s.supporting_label().and_then(|l| l.optional(&i18n));
    assert_eq!(help(&email).as_deref(), Some("We never share it"));
    assert_eq!(phone.label().text(&i18n), "Phone");
    assert_eq!(help(&phone), None, "a missing attribute hides the help line");
}

#[derive(Default)]
struct App {
    state: FormState,
    value: Signup,
    press: bool,
    edit_pw: Option<&'static str>,
    show_extra: bool,
    pending: bool,
    submitted: bool,
    first_invalid: Option<FieldPath>,
}

fn show(ui: &mut MineralUi<'_, TestKit>, app: &mut App) {
    let press = std::mem::take(&mut app.press);
    let edit = app.edit_pw.take();
    let show_extra = app.show_extra;
    let mut form = Form::new("f", &mut app.state, &mut app.value).locale(Arc::new(en_us()));
    if app.pending {
        form = form.pending_submit();
    }
    let r = form.show_with(
        ui,
        |f| {
            if let Some(text) = edit {
                f.custom(Signup::PW, |ui, c| {
                    text.clone_into(c.value());
                    c.committed();
                    record(ui, c)
                });
            }
            if !show_extra {
                // Placed elsewhere (another wizard step): `rest` skips it.
                f.mark(Signup::EXTRA);
            }
            f.rest();
        },
        |_, _| press,
    );
    app.submitted |= r.submitted;
    // `Harness::run` runs the repaint pass a submit requests too.
    if r.first_invalid.is_some() {
        app.first_invalid = r.first_invalid;
    }
}

#[test]
fn rest_shows_each_declared_field_once_in_order() {
    let mut h = harness(show, App::default());
    h.state_mut().value.show_extra = true;
    shown();
    h.run();
    let first: Vec<_> = shown().into_iter().take(4).collect();
    assert_eq!(
        first,
        [
            ("f-email".into(), "Email".into(), true),
            ("f-pw".into(), "Pw".into(), true),
            ("f-pw2".into(), "Repeat".into(), false),
            ("f-extra".into(), "Extra".into(), false),
        ]
    );
}

#[test]
fn sync_submit_validates_then_finishes_itself() {
    let mut h = harness(show, App::default());
    h.state_mut().value.value = Signup {
        email: "nope".into(),
        pw2: "xyz".into(),
        ..Signup::valid()
    };
    h.run();
    assert_eq!(
        code(&h.state().value.state, Signup::EMAIL),
        None,
        "errors show after touch or submit"
    );
    h.state_mut().value.press = true;
    h.run();
    let app = &h.state().value;
    assert!(!app.submitted);
    assert_eq!(app.first_invalid, Some(Signup::EMAIL.into()));
    assert_eq!(code(&app.state, Signup::EMAIL).as_deref(), Some("email"));
    assert_eq!(code(&app.state, Signup::PW2).as_deref(), Some("mismatch"));

    h.state_mut().value.value = Signup::valid();
    h.run();
    assert_eq!(
        code(&h.state().value.state, Signup::PW2),
        None,
        "a fixed cross-field error clears"
    );
    h.state_mut().value.edit_pw = Some("abc");
    h.run();
    assert!(h.state().value.state.is_dirty());
    h.state_mut().value.press = true;
    h.run();
    let app = &h.state().value;
    assert!(app.submitted, "valid values submit");
    assert!(
        !app.state.submitting() && !app.state.is_dirty(),
        "a sync submit finishes itself"
    );
}

#[test]
fn pending_submit_stays_in_flight_and_keeps_server_errors() {
    let mut h = harness(show, App::default());
    let app = &mut h.state_mut().value;
    app.value = Signup::valid();
    app.pending = true;
    app.press = true;
    h.run();
    let app = &mut h.state_mut().value;
    assert!(app.submitted && app.state.submitting());
    app.submitted = false;
    app.press = true;
    h.run();
    let app = &mut h.state_mut().value;
    assert!(!app.submitted, "no double submit");
    app.state.finish_submit(Err(vec![
        SubmitError::on_field(Signup::PW, ValidationError::new("taken", "Taken")),
        SubmitError::on_form(ValidationError::new("down", "Server down")),
    ]));
    app.press = true;
    h.run();
    let app = &h.state().value;
    assert!(!app.submitted, "a server error blocks the next submit");
    assert_eq!(code(&app.state, Signup::PW).as_deref(), Some("taken"));
    h.state_mut().value.edit_pw = Some("xyz");
    h.run();
    let app = &h.state().value;
    assert!(
        app.state.field(Signup::PW).is_some_and(|f| f.error().is_none()),
        "an edit answers it"
    );
}

#[test]
fn hidden_invalid_field_reports_and_touches_by_path() {
    let mut h = harness(show, App::default());
    let app = &mut h.state_mut().value;
    app.value = Signup {
        need_extra: true,
        ..Signup::valid()
    };
    app.press = true;
    h.run();
    let app = &mut h.state_mut().value;
    assert!(!app.submitted);
    assert_eq!(app.first_invalid, Some(Signup::EXTRA.into()));
    assert_eq!(
        app.state.form_error().map(|e| e.code.as_ref()),
        Some(i18n::msg::FORM_HIDDEN_ERRORS.id())
    );
    assert!(!app.state.touch(Signup::EXTRA), "the step's field still fails");
    assert!(app.state.touch(Signup::EMAIL));
    app.value.extra = true;
    app.state.request_submit();
    h.run();
    assert!(h.state().value.submitted, "a requested submit runs on the next pass");
}

#[test]
fn rest_descends_into_nested_values_and_list_items() {
    let order = Order {
        main_item: Item { qty: 1 },
        items: vec![Item { qty: 1 }, Item { qty: 0 }],
    };
    let mut h = harness(
        |ui, (state, order): &mut (FormState, Order)| {
            #[expect(
                clippy::redundant_closure_for_method_calls,
                reason = "the method path is not general over the `FormUi` lifetimes"
            )]
            Form::new("o", state, order).show_with(ui, |f| f.rest(), |_, _| false);
        },
        (FormState::default(), order),
    );
    shown();
    h.run();
    let keys: Vec<_> = shown().into_iter().map(|(k, _, _)| k).take(3).collect();
    assert_eq!(keys, ["o-main-item-qty", "o-items-0-qty", "o-items-1-qty"]);
    let state = &h.state().value.0;
    assert!(state.field(Order::ITEMS.at(1).then(Item::QTY)).is_some());
}

#[test]
fn review_regressions() {
    // A click plus a pending request submits once.
    let mut h = harness(show, App::default());
    let app = &mut h.state_mut().value;
    app.value = Signup::valid();
    app.state.request_submit();
    app.press = true;
    h.run();
    h.state_mut().value.submitted = false;
    h.run();
    assert!(!h.state().value.submitted, "the request was consumed by the click pass");

    // A server error on a field not shown keeps the form invalid with the hint; set_error paths touch.
    let app = &mut h.state_mut().value;
    app.state
        .set_error(Signup::EXTRA, Some(ValidationError::new("taken", "Taken")));
    app.press = true;
    h.run();
    let app = &mut h.state_mut().value;
    assert!(!app.submitted);
    assert_eq!(
        app.state.form_error().map(|e| e.code.as_ref()),
        Some(i18n::msg::FORM_HIDDEN_ERRORS.id())
    );
    assert!(!app.state.touch(Signup::EXTRA));
    app.state.set_error(Signup::EXTRA, None);
    h.run();
    assert_eq!(
        h.state().value.state.form_error(),
        None,
        "the hint clears with the hidden error"
    );
    assert!(
        !h.state_mut().value.state.touch(FieldPath::from(Item::QTY)),
        "no match is not a pass"
    );
}

fn plugin_form() -> DynForm {
    DynForm::new([
        DynField::new("name", DynKind::Text(TextOptions::default()), "Name")
            .required(true)
            .check(MinLength(3)),
        DynField::new("level", DynKind::Number(NumberOptions::default()), "Level").check(NumberRange::at_most(5.0)),
        DynField::new("tags", DynKind::MultiPick(Choices::default()), "Tags").initial(DynValue::Flag(true)),
        DynField::new("name", DynKind::Check, "Repeated key"),
    ])
}

#[test]
fn dynamic_form_validates_its_data_rules() {
    let mut form = plugin_form();
    assert_eq!(form.fields().len(), 2 + 1, "the repeated key is dropped");
    assert_eq!(
        form.value("tags"),
        Some(&DynValue::Picks(Selection::default())),
        "a mismatched initial is ignored"
    );
    let report = form.report(&en_us());
    assert_eq!(report.first_invalid(), Some(&FieldPath::named("name")));
    assert!(report.is_required(FieldPath::named("name")));
    assert!(form.set("name", DynValue::Text("ab".into())));
    assert!(!form.set("name", DynValue::Flag(true)), "another kind's value");
    assert!(!form.set("missing", DynValue::Flag(true)));
    let report = form.report(&en_us());
    assert_eq!(
        report.error(FieldPath::named("name")).map(|e| e.code.as_ref()),
        Some("mineral-min-length")
    );
    form.set("name", DynValue::Text("abc".into()));
    form.set("level", DynValue::Number(9.0));
    let report = form.report(&en_us());
    assert_eq!(report.first_invalid(), Some(&FieldPath::named("level")));
}

#[test]
fn dynamic_form_renders_through_the_kit_and_submits() {
    let mut h = harness(
        |ui, (state, form, submitted, press): &mut (FormState, DynForm, bool, bool)| {
            let press = std::mem::take(press);
            #[expect(
                clippy::redundant_closure_for_method_calls,
                reason = "the method path is not general over the `FormUi` lifetimes"
            )]
            let r = Form::new("p", state, form).show_with(ui, |f| f.rest(), |_, _| press);
            *submitted |= r.submitted;
        },
        (FormState::default(), plugin_form(), false, true),
    );
    shown();
    h.run();
    let fields = shown();
    let keys: Vec<_> = fields
        .iter()
        .map(|(k, l, r)| (k.as_str(), l.as_str(), *r))
        .take(3)
        .collect();
    assert_eq!(
        keys,
        [
            ("p-name", "Name", true),
            ("p-level", "Level", false),
            ("p-tags", "Tags", false)
        ]
    );
    assert!(!h.state().value.2, "the blank required name blocks");
    let state = &h.state().value.0;
    assert!(
        state
            .field(FieldPath::named("name"))
            .and_then(FieldState::error)
            .is_some()
    );
    let app = &mut h.state_mut().value;
    app.1.set("name", DynValue::Text("Ann".into()));
    app.3 = true;
    h.run();
    assert!(h.state().value.2);
}
