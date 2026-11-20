use crate::Mui;
use egui::{Key, KeyboardShortcut, Modifiers};
use egui_mineral::{
    edit,
    form::{CheckError, FieldKind, MinLength, ValidateOn, ValidationError, Validator},
    i18n::LocaleFormat,
    shortcut::{self, Scope},
};
use egui_mineral_material::{
    catalog::Component,
    icons::{ICON_ATTACH_MONEY, ICON_EMAIL, ICON_LOCK, ICON_PERSON, ICON_SEARCH, ICON_VISIBILITY},
    prelude::*,
};

use super::{Shared, caption};

pub(crate) const TOPICS: &[&str] = &[
    "Text field",
    "Number field",
    "OTP field",
    "Tags field",
    "Inline edit",
    "Date field",
    "Form",
    "File drop zone",
    "Inspector",
];

pub(crate) struct State {
    filled: String,
    outlined: String,
    icons: String,
    supporting: String,
    error: String,
    limited: String,
    money: String,
    url: String,
    multiline: String,
    grow: String,
    path: String,
    description: String,
    password: String,
    password_plain: String,
    number: f64,
    number_free: f64,
    number_int: f64,
    number_price: f64,
    number_angle: f64,
    number_err: f64,
    number_freq: f64,
    buffered_name: String,
    buffered_gain: f64,
    buffered_event: Option<EditEvent>,
    buffered_source: Option<NumberSource>,
    saved: Option<(String, f64)>,
    otp: String,
    otp_pin: String,
    otp_alpha: String,
    otp_error: String,
    tags: Vec<String>,
    tags_last: String,
    tags_limited: Vec<String>,
    tags_semi: Vec<String>,
    inline_title: String,
    inline_empty: String,
    inline_heading: String,
    date: Option<Date>,
    date_outlined: Option<Date>,
    date_bounded: Option<Date>,
    form: FormState,
    form_done: Option<String>,
    files: Vec<String>,
    rejected: Vec<String>,
    inspector_name: String,
    inspector: [f64; 4],
    inspector_open: Expansion<usize>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            filled: String::new(),
            outlined: String::new(),
            icons: String::new(),
            supporting: String::new(),
            error: String::new(),
            limited: String::new(),
            money: String::new(),
            url: String::new(),
            multiline: String::new(),
            grow: String::new(),
            path: "/home/user/projects/podcast/renders/final-mix-v3.wav".to_owned(),
            description: String::new(),
            password: String::new(),
            password_plain: String::new(),
            otp: String::new(),
            otp_pin: String::new(),
            otp_alpha: String::new(),
            otp_error: String::new(),
            tags_semi: Vec::new(),
            inline_empty: String::new(),
            date: None,
            date_outlined: None,
            date_bounded: None,
            form: FormState::default(),
            form_done: None,
            files: Vec::new(),
            rejected: Vec::new(),
            number: 5.0,
            number_free: 0.123,
            number_int: 3.0,
            number_price: 19.99,
            number_angle: 12.3,
            number_err: 150.0,
            number_freq: 440.0,
            buffered_name: "Track 1".to_owned(),
            buffered_gain: 0.0,
            buffered_event: None,
            buffered_source: None,
            saved: None,
            tags: ["rust", "egui"].map(str::to_owned).to_vec(),
            tags_last: "none".to_owned(),
            tags_limited: vec!["one".to_owned()],
            inline_title: "Click to rename me".to_owned(),
            inline_heading: "Project Mineral".to_owned(),
            inspector_name: "Rectangle 1".to_owned(),
            inspector: [120.0, 48.0, 15.0, 80.0],
            inspector_open: (0..3).fold(Expansion::default(), |mut e, i| {
                e.expand(i);
                e
            }),
        }
    }
}

pub(crate) fn show(ui: &mut Mui<'_>, s: &mut State, shared: &mut Shared) {
    text_fields(ui, s, shared);
    numbers(ui, s);
    otp(ui, s, shared);
    tags(ui, s);
    inline(ui, s, shared);
    dates(ui, s);
    form(ui, s, shared);
    files(ui, s, shared);
    inspector(ui, s);
}

fn text_fields(ui: &mut Mui<'_>, s: &mut State, shared: &mut Shared) {
    // One row per variant: outlined fields reserve the floating label's overhang above the box.
    super::section(ui, TOPICS[0], Some(<TextField<'static>>::META), |ui| {
        caption(ui, TypeScale::LabelMedium, "Filled");
        filled_fields(ui, s, shared);
        caption(ui, TypeScale::LabelMedium, "Outlined");
        outlined_fields(ui, s);
    });
}

fn filled_fields(ui: &mut Mui<'_>, s: &mut State, shared: &mut Shared) {
    super::top_row(ui, |row| {
        row.mineral(TextField::new("ti-filled", "Filled", &mut s.filled).preferred_width(260.0));
        let f = TextField::new("ti-icons", "Search", &mut s.icons)
            .placeholder("Type to search")
            .icon(ICON_SEARCH)
            .trailing_icon(ICON_PERSON, "Account")
            .supporting("Clickable trailing icon")
            .preferred_width(260.0);
        if row.mineral(f).output().trailing_clicked {
            shared.toast("Trailing icon clicked");
        }
        let err = (!s.error.is_empty() && !s.error.contains('@')).then(|| "Missing @".to_owned());
        row.mineral(
            TextField::new("ti-error", "Address", &mut s.error)
                .supporting("Format: name@host")
                .maybe_error(err)
                .preferred_width(260.0),
        );
        row.mineral(
            TextField::new("ti-money", "Amount", &mut s.money)
                .prefix("$")
                .suffix("USD")
                .icon(ICON_ATTACH_MONEY)
                .preferred_width(260.0),
        );
        row.mineral(
            TextField::new("ti-pw", "Password", &mut s.password)
                .masked(true)
                .icon(ICON_LOCK)
                .supporting("Reveal toggle")
                .preferred_width(260.0),
        );
        let mut v = String::from("Prefilled value");
        row.mineral(
            TextField::new("ti-dis", "Disabled", &mut v)
                .disabled(true)
                .preferred_width(260.0),
        );
        row.mineral(
            TextField::new("ti-path", "Path", &mut s.path)
                .elide(Elide::Start)
                .supporting("/home/user/projects/podcast/renders: the file name stays visible")
                .preferred_width(260.0),
        );
        row.mineral(
            TextField::new("ti-desc", "Description", &mut s.description)
                .rows(3)
                .text_style(TypeScale::BodyMedium)
                .supporting("BodyMedium text style")
                .preferred_width(260.0),
        );
        row.mineral(
            TextField::new("ti-multi", "Message", &mut s.multiline)
                .rows(4)
                .max_chars(200)
                .supporting("Fixed 4 rows")
                .preferred_width(320.0),
        );
    });
}

fn outlined(f: TextField<'_>) -> TextField<'_> {
    f.variant(TextFieldVariant::Outlined)
}

fn outlined_fields(ui: &mut Mui<'_>, s: &mut State) {
    super::top_row(ui, |row| {
        row.mineral(outlined(TextField::new("ti-outlined", "Outlined", &mut s.outlined)).preferred_width(260.0));
        row.mineral(
            outlined(TextField::new("ti-icons-o", "Email", &mut s.supporting))
                .icon(ICON_EMAIL)
                .supporting("We never share it")
                .preferred_width(260.0),
        );
        row.mineral(
            outlined(TextField::new("ti-limit", "Short bio", &mut s.limited))
                .max_chars(20)
                .supporting("Up to 20 characters")
                .preferred_width(260.0),
        );
        row.mineral(
            outlined(TextField::new("ti-url", "Website", &mut s.url))
                .prefix("https://")
                .suffix(".com")
                .preferred_width(260.0),
        );
        row.mineral(
            outlined(TextField::new("ti-pw-nt", "PIN", &mut s.password_plain))
                .masked(true)
                .reveal_toggle(false)
                .trailing_icon(ICON_VISIBILITY, "Show password")
                .supporting("No reveal toggle")
                .preferred_width(260.0),
        );
        let mut v = String::from("Prefilled value");
        row.mineral(
            outlined(TextField::new("ti-dis-o", "Disabled outlined", &mut v))
                .disabled(true)
                .preferred_width(260.0),
        );
        row.mineral(
            outlined(TextField::new("ti-grow", "Notes", &mut s.grow))
                .rows(2)
                .grow_rows(4)
                .supporting("Grows 2 to 6 rows")
                .preferred_width(320.0),
        );
    });
}

const SAVE: ActionId = ActionId::from_static("showcase.save");
const SAVE_KEY: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::S);

fn buffered(ui: &mut Mui<'_>, s: &mut State) {
    let ctx = ui.ctx().clone();
    shortcut::bind(&ctx, Scope::Global, SAVE_KEY, SAVE);
    let saving = shortcut::take_action(&ctx, &SAVE);
    if saving {
        edit::commit_pending(&ctx);
    }
    caption(
        ui,
        TypeScale::LabelMedium,
        "Commit on Enter, focus loss or save; Escape reverts",
    );
    super::row(ui, |row| {
        let r = row.mineral(
            TextField::new("ti-buffered", "Name", &mut s.buffered_name)
                .commit_on_submit(true)
                .preferred_width(220.0),
        );
        s.buffered_event = r.output().edit.or(s.buffered_event);
        let r = row.add(|ui| {
            ui.mineral(
                NumberField::new("ti-num-buffered", "Gain (dB)", -24.0..=24.0, &mut s.buffered_gain)
                    .commit_on_submit(true)
                    .preferred_width(220.0),
            )
        });
        s.buffered_event = r.output().edit.or(s.buffered_event);
        s.buffered_source = r.output().source.or(s.buffered_source);
        let label = format!("Save ({})", shortcut::label(&ctx, &SAVE_KEY));
        if row
            .mineral(PushButton::new("ti-save", label).variant(ButtonVariant::Tonal))
            .clicked()
        {
            shortcut::fire(&ctx, SAVE);
        }
    });
    if saving {
        s.saved = Some((s.buffered_name.clone(), s.buffered_gain));
    }
    ui.label(format!(
        "Model: {} / {} dB, last edit: {:?}, gain set by: {:?}, saved: {:?}",
        s.buffered_name, s.buffered_gain, s.buffered_event, s.buffered_source, s.saved
    ));
}

fn formatted_numbers(ui: &mut Mui<'_>, s: &mut State) {
    super::top_row(ui, |row| {
        row.add(|ui| {
            ui.mineral(
                NumberField::new(
                    "ti-num-int",
                    "Integer",
                    RangeModel::new(1.0, 99.0).step(1.0),
                    &mut s.number_int,
                )
                .variant(TextFieldVariant::Outlined)
                .preferred_width(220.0),
            )
        });
        row.add(|ui| {
            ui.mineral(
                NumberField::new(
                    "ti-num-price",
                    "Price",
                    RangeModel::new(0.0, 1000.0).step(0.01),
                    &mut s.number_price,
                )
                .variant(TextFieldVariant::Outlined)
                .format(|v| format!("${v:.2}"))
                .preferred_width(220.0),
            )
        });
        row.add(|ui| {
            ui.mineral(
                NumberField::new(
                    "ti-num-angle",
                    "Angle",
                    RangeModel::new(0.0, 360.0).increment(5.0).fraction_digits(1),
                    &mut s.number_angle,
                )
                .variant(TextFieldVariant::Outlined)
                .supporting("1 decimal, steps by 5 from any value")
                .preferred_width(220.0),
            )
        });
    });
}

fn numbers(ui: &mut Mui<'_>, s: &mut State) {
    super::section(ui, TOPICS[1], Some(<NumberField<'static>>::META), |ui| {
        super::top_row(ui, |row| {
            row.add(|ui| {
                ui.mineral(
                    NumberField::new(
                        "ti-num",
                        "Quantity",
                        RangeModel::new(0.0, 10.0).step(0.5),
                        &mut s.number,
                    )
                    .supporting("0 to 10, step 0.5")
                    .preferred_width(220.0),
                )
            });
            row.add(|ui| {
                ui.mineral(
                    NumberField::new(
                        "ti-num-free",
                        "Phase",
                        RangeModel::new(-180.0, 180.0).step(0.01),
                        &mut s.number_free,
                    )
                    .snap_typed(false)
                    .supporting("Typed values stay unsnapped")
                    .preferred_width(220.0),
                )
            });
            let err = (s.number_err > 100.0).then(|| "Above 100".to_owned());
            row.add(|ui| {
                ui.mineral(
                    NumberField::new(
                        "ti-num-err",
                        "Percent",
                        RangeModel::new(0.0, 200.0).step(10.0),
                        &mut s.number_err,
                    )
                    .maybe_error(err)
                    .preferred_width(220.0),
                )
            });
            row.add(|ui| {
                ui.mineral(
                    NumberField::new(
                        "ti-num-freq",
                        "Frequency",
                        RangeModel::new(20.0, 20_000.0).scale(Scale::Log).significant(3),
                        &mut s.number_freq,
                    )
                    .supporting("Log scale, 3 significant digits; drag to scrub")
                    .format(|v| format!("{v} Hz"))
                    .scrub(true)
                    .preferred_width(220.0),
                )
            });
            let mut v = 4.0;
            row.add(|ui| {
                ui.mineral(
                    NumberField::new("ti-num-dis", "Disabled", 0.0..=10.0, &mut v)
                        .disabled(true)
                        .preferred_width(220.0),
                )
            });
        });
        formatted_numbers(ui, s);
        ui.label(format!(
            "Values: {} / {} / {} / {} / {} / {}",
            s.number, s.number_int, s.number_price, s.number_err, s.number_freq, s.number_angle
        ));
        buffered(ui, s);
    });
}

fn otp(ui: &mut Mui<'_>, s: &mut State, shared: &mut Shared) {
    super::section(ui, TOPICS[2], Some(<OtpField<'static>>::META), |ui| {
        let f = OtpField::new("ti-otp", "Verification code (paste a full code to fill)", 6, &mut s.otp);
        if ui.mineral(f).output().completed {
            shared.toast(format!("Code complete: {} digits", s.otp.len()));
        }
        ui.mineral(OtpField::new("ti-otp-pin", "Masked PIN", 4, &mut s.otp_pin).masked(true));
        ui.mineral(
            OtpField::new("ti-otp-alpha", "Alphanumeric invite code", 5, &mut s.otp_alpha)
                .charset(Charset::Alphanumeric),
        );
        ui.mineral(OtpField::new("ti-otp-err", "Wrong code", 6, &mut s.otp_error).error("Incorrect code"));
        let mut v = String::from("123");
        ui.mineral(OtpField::new("ti-otp-dis", "Disabled", 6, &mut v).disabled(true));
    });
}

fn tags(ui: &mut Mui<'_>, s: &mut State) {
    super::section(ui, TOPICS[3], Some(<TagsField<'static>>::META), |ui| {
        let f = TagsField::new("ti-tags", "Topics", &mut s.tags)
            .placeholder("Type and press Enter or comma")
            .preferred_width(420.0);
        let out = ui.mineral(f).into_output();
        if !out.added.is_empty() || !out.removed.is_empty() {
            s.tags_last = format!("added {:?}, removed {:?}", out.added, out.removed);
        }
        ui.label(format!("Last change: {}", s.tags_last));
        let f = TagsField::new("ti-tags-max", "Max 3, outlined", &mut s.tags_limited)
            .variant(TextFieldVariant::Outlined)
            .max_tags(3)
            .icon(ICON_SEARCH)
            .supporting("Counter shows used tags")
            .preferred_width(420.0);
        ui.mineral(f);
        let err = s.tags_semi.is_empty().then(|| "Add at least one tag".to_owned());
        let f = TagsField::new("ti-tags-semi", "Semicolon delimiter, required", &mut s.tags_semi)
            .delimiters([';'])
            .maybe_error(err)
            .preferred_width(420.0);
        ui.mineral(f);
        let mut off = vec!["locked".to_owned()];
        ui.mineral(
            TagsField::new("ti-tags-dis", "Disabled", &mut off)
                .disabled(true)
                .preferred_width(420.0),
        );
    });
}

fn inline(ui: &mut Mui<'_>, s: &mut State, shared: &mut Shared) {
    super::section(ui, TOPICS[4], Some(<InlineEdit<'static>>::META), |ui| {
        caption(
            ui,
            TypeScale::LabelMedium,
            "Click, Enter or F2 to edit; Enter submits, Escape cancels",
        );
        let r = ui.mineral(InlineEdit::new("ti-inline", "Title", &mut s.inline_title).preferred_width(320.0));
        if r.output().submitted {
            shared.toast(format!("Renamed to {}", s.inline_title));
        }
        caption(ui, TypeScale::LabelMedium, "Headline style");
        ui.mineral(
            InlineEdit::new("ti-inline-h", "Heading", &mut s.inline_heading)
                .text_style(TypeScale::HeadlineSmall)
                .preferred_width(420.0),
        );
        caption(ui, TypeScale::LabelMedium, "Empty value shows the placeholder");
        ui.mineral(
            InlineEdit::new("ti-inline-e", "Description", &mut s.inline_empty)
                .placeholder("Add a description")
                .preferred_width(320.0),
        );
        caption(ui, TypeScale::LabelMedium, "Disabled");
        let mut fixed = String::from("Read-only text");
        ui.mineral(InlineEdit::new("ti-inline-d", "Locked", &mut fixed).disabled(true));
    });
}

fn dates(ui: &mut Mui<'_>, s: &mut State) {
    super::section(ui, TOPICS[5], Some(<DateField<'static>>::META), |ui| {
        super::top_row(ui, |row| {
            row.mineral(
                DateField::new("ti-date", "Birthday", &mut s.date)
                    .supporting("Digits fill segments")
                    .preferred_width(220.0),
            );
            let mut v = Some(Date::constant(2026, 6, 1));
            row.mineral(
                DateField::new("ti-date-d", "Disabled", &mut v)
                    .disabled(true)
                    .preferred_width(220.0),
            );
        });
        super::top_row(ui, |row| {
            let required = s.date_outlined.is_none().then(|| "Required".to_owned());
            row.mineral(
                DateField::new("ti-date-o", "Outlined", &mut s.date_outlined)
                    .variant(TextFieldVariant::Outlined)
                    .maybe_error(required)
                    .preferred_width(220.0),
            );
            row.mineral(
                DateField::new("ti-date-b", "2026 only", &mut s.date_bounded)
                    .variant(TextFieldVariant::Outlined)
                    .bounds(DateBounds::new(
                        Some(Date::constant(2026, 1, 1)),
                        Some(Date::constant(2026, 12, 31)),
                    ))
                    .preferred_width(220.0),
            );
        });
        ui.label(format!(
            "Values: {} / {} / {}",
            s.date.map_or_else(|| "none".to_owned(), |d| d.to_string()),
            s.date_outlined.map_or_else(|| "none".to_owned(), |d| d.to_string()),
            s.date_bounded.map_or_else(|| "none".to_owned(), |d| d.to_string()),
        ));
    });
}

fn form(ui: &mut Mui<'_>, s: &mut State, shared: &mut Shared) {
    super::section(ui, TOPICS[6], None, |ui| {
        // Narrow form column; the section divider keeps the full width.
        ui.scope(|ui| {
            let width = ui.available_width().min(420.0);
            ui.set_max_width(width);
            let r = Form::new("signup", &mut s.form)
                .field("username", "Username", |f| {
                    f.required(true)
                        .supporting("At least 3 characters")
                        .validator(MinLength(3))
                        .validate_on(ValidateOn::Change)
                })
                .field("email", "Email", |f| f.required(true).validator(EmailValidator))
                .field("password", "Password", |f| {
                    f.required(true)
                        .kind(FieldKind::Password)
                        .supporting("8+ characters, 3 of: lower, upper, digit, symbol")
                        .validator(PasswordPolicy {
                            min_len: 8,
                            min_classes: 3,
                        })
                        .validate_on(ValidateOn::Change)
                })
                .field("confirm", "Confirm password", |f| {
                    f.required(true).kind(FieldKind::Password)
                })
                .field("bio", "About you (optional)", |f| f.kind(FieldKind::Multiline))
                .check(|st| {
                    if st.value("password") == st.value("confirm") {
                        Ok(())
                    } else {
                        Err(CheckError::on_field(
                            "confirm",
                            ValidationError::new("mismatch", "Passwords do not match"),
                        ))
                    }
                })
                .submit_label("Sign up")
                .show(ui);
            if let Some(values) = r.submitted {
                s.form_done = Some(format!("{} <{}>", values["username"], values["email"]));
                s.form.finish_submit(Ok(()));
                shared.toast("Signed up");
            }
            ui.add_space(8.0);
            super::row(ui, |row| {
                if row
                    .mineral(PushButton::new("ti-form-reset", "Reset").variant(ButtonVariant::Outlined))
                    .clicked()
                {
                    s.form.reset();
                    s.form_done = None;
                }
                row.add(|ui| {
                    ui.label(format!(
                        "dirty: {}, valid: {}, submits: {}",
                        s.form.is_dirty(),
                        s.form.is_valid(),
                        s.form.submit_count()
                    ))
                });
            });
            if let Some(done) = &s.form_done {
                ui.label(format!("Last sign-up: {done}"));
            }
        });
    });
}

fn files(ui: &mut Mui<'_>, s: &mut State, shared: &mut Shared) {
    super::section(ui, TOPICS[7], Some(FileDropZone::META), |ui| {
        caption(
            ui,
            TypeScale::LabelMedium,
            "Images only, up to 3 files; drag files from the OS onto the zone",
        );
        let r = ui.mineral(
            FileDropZone::new("ti-drop")
                .accept(FileFilter::new(FileType::new("Images", ["png", "jpg", "jpeg", "gif"])))
                .max_files(3)
                .supporting("PNG, JPG or GIF")
                .preferred_width(420.0)
                .height_range(140.0..),
        );
        for f in &r.output().dropped {
            s.files.push(f.path().display().to_string());
        }
        for f in &r.output().rejected {
            s.rejected.push(f.path().display().to_string());
        }
        if r.output().browse_clicked {
            shared.toast("Browse requested; a real app opens PlatformPickers::pick_files");
        }
        for f in &s.files {
            ui.label(format!("Accepted: {f}"));
        }
        for f in &s.rejected {
            ui.label(format!("Rejected: {f}"));
        }
        caption(ui, TypeScale::LabelMedium, "Disabled, default text");
        ui.mineral(
            FileDropZone::new("ti-drop-dis")
                .disabled(true)
                .preferred_width(420.0)
                .height_range(100.0..),
        );
    });
}

struct EmailValidator;

impl Validator for EmailValidator {
    fn validate(&self, value: &str, _locale: &dyn LocaleFormat) -> Result<(), ValidationError> {
        let err = || Err(ValidationError::new("email", "Enter a valid email address"));
        let Some((local, domain)) = value.split_once('@') else {
            return err();
        };
        let local_ok = !local.is_empty()
            && local
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".!#$%&'*+/=?^_`{|}~-".contains(&b));
        let label_ok = |l: &str| {
            let b = l.as_bytes();
            (1..=63).contains(&b.len())
                && b.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'-')
                && b[0] != b'-'
                && b[b.len() - 1] != b'-'
        };
        if local_ok && domain.split('.').all(label_ok) {
            Ok(())
        } else {
            err()
        }
    }
}

struct PasswordPolicy {
    min_len: usize,
    min_classes: usize,
}

impl Validator for PasswordPolicy {
    fn validate(&self, value: &str, _locale: &dyn LocaleFormat) -> Result<(), ValidationError> {
        let classes = [
            value.chars().any(char::is_lowercase),
            value.chars().any(char::is_uppercase),
            value.chars().any(char::is_numeric),
            value.chars().any(|c| !c.is_alphanumeric()),
        ]
        .into_iter()
        .filter(|b| *b)
        .count();
        if value.chars().count() >= self.min_len && classes >= self.min_classes {
            Ok(())
        } else {
            Err(ValidationError::new(
                "password-policy",
                format!(
                    "Use at least {} characters, mixing {} of: lowercase, uppercase, digits, symbols",
                    self.min_len, self.min_classes
                ),
            ))
        }
    }
}

fn inspector(ui: &mut Mui<'_>, s: &mut State) {
    let rt = ui.rt();
    super::section(ui, TOPICS[8], None, |ui| {
        caption(
            ui,
            TypeScale::LabelMedium,
            "Density -6, flat accordion groups: label column, label-less fields, units as suffixes",
        );
        let [x, y, angle, opacity] = &mut s.inspector;
        let name = &mut s.inspector_name;
        let group = |ui: &mut Mui<'_>, id, rows: &mut dyn FnMut(&mut egui::Ui)| {
            egui::Grid::new(id)
                .num_columns(2)
                .min_col_width(64.0)
                .spacing([16.0, 4.0])
                .show(ui, |g| rows(g));
        };
        ui.with_density(Density::MIN, |ui| {
            Accordion::new("ti-insp-acc", &mut s.inspector_open)
                .variant(AccordionVariant::Flat)
                .show(ui, |a| {
                    let title = |t| AccordionPanel::new(t).title_style(TypeScale::TitleSmall);
                    a.panel(title("Identity"), |ui| {
                        group(ui, "ti-insp-identity", &mut |g| {
                            g.label("Name");
                            rt.ui(g).mineral(
                                TextField::new("ti-insp-name", "Name", name)
                                    .variant(TextFieldVariant::Outlined)
                                    .label_visible(false)
                                    .preferred_width(200.0),
                            );
                            g.end_row();
                        });
                    });
                    a.panel(title("Transform"), |ui| {
                        group(ui, "ti-insp-transform", &mut |g| {
                            for (id, label, unit, v) in [
                                ("x", "X", "px", &mut *x),
                                ("y", "Y", "px", &mut *y),
                                ("angle", "Angle", "deg", &mut *angle),
                            ] {
                                g.label(label);
                                rt.ui(g).mineral(
                                    NumberField::new(
                                        ("ti-insp", id),
                                        label,
                                        RangeModel::new(-360.0, 360.0).step(1.0),
                                        v,
                                    )
                                    .variant(TextFieldVariant::Outlined)
                                    .label_visible(false)
                                    .step_buttons(false)
                                    .scrub(true)
                                    .suffix(unit)
                                    .preferred_width(200.0),
                                );
                                g.end_row();
                            }
                        });
                    });
                    a.panel(title("Appearance"), |ui| {
                        group(ui, "ti-insp-appearance", &mut |g| {
                            g.label("Opacity");
                            rt.ui(g).mineral(
                                ValueSlider::new(
                                    "ti-insp-opacity",
                                    "Opacity",
                                    RangeModel::new(0.0, 100.0).step(1.0),
                                    &mut *opacity,
                                )
                                .value_field(true)
                                .preferred_width(200.0),
                            );
                            g.end_row();
                        });
                    });
                });
        });
    });
}
