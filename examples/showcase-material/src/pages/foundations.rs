use crate::Mui;
use egui::{Align2, Color32, CornerRadius, Frame, Margin, RichText, Sense, Stroke, StrokeKind, vec2};
use egui_mineral::{
    i18n::{self, DateFields, DateLength, HourCycle, ListKind, NameWidth},
    layout,
    motion::{CubicBezier, Spring},
};
use egui_mineral_material::MaterialExt;
use egui_mineral_material::{
    adaptive::{self, WindowClass},
    prelude::*,
    spec::{ElevationLevel, tokens::motion as m},
    typography::TypeSpec,
};

use super::{LOCALES, Shared, THEMES, VARIANTS, caption, section};

pub(crate) const TOPICS: &[&str] = &[
    "Theme",
    "Color roles",
    "Typography",
    "Shape",
    "Elevation",
    "Spacing",
    "Motion",
    "Locale formatting",
    "Layout environment",
];

const SCALES: [(TypeScale, &str); 15] = [
    (TypeScale::DisplayLarge, "Display large"),
    (TypeScale::DisplayMedium, "Display medium"),
    (TypeScale::DisplaySmall, "Display small"),
    (TypeScale::HeadlineLarge, "Headline large"),
    (TypeScale::HeadlineMedium, "Headline medium"),
    (TypeScale::HeadlineSmall, "Headline small"),
    (TypeScale::TitleLarge, "Title large"),
    (TypeScale::TitleMedium, "Title medium"),
    (TypeScale::TitleSmall, "Title small"),
    (TypeScale::BodyLarge, "Body large"),
    (TypeScale::BodyMedium, "Body medium"),
    (TypeScale::BodySmall, "Body small"),
    (TypeScale::LabelLarge, "Label large"),
    (TypeScale::LabelMedium, "Label medium"),
    (TypeScale::LabelSmall, "Label small"),
];

const EASINGS: [(&str, CubicBezier); 7] = [
    ("linear", m::EASING_LINEAR_CUBIC_BEZIER),
    ("standard", m::EASING_STANDARD_CUBIC_BEZIER),
    ("standard decelerate", m::EASING_STANDARD_DECELERATE_CUBIC_BEZIER),
    ("standard accelerate", m::EASING_STANDARD_ACCELERATE_CUBIC_BEZIER),
    ("emphasized", m::EASING_EMPHASIZED_CUBIC_BEZIER),
    ("emphasized decelerate", m::EASING_EMPHASIZED_DECELERATE_CUBIC_BEZIER),
    ("emphasized accelerate", m::EASING_EMPHASIZED_ACCELERATE_CUBIC_BEZIER),
];

const DURATION_GROUPS: [&str; 4] = ["short", "medium", "long", "extra long"];
const SPRING: Spring = Spring {
    damping_ratio: 0.5,
    stiffness: 400.0,
    rest_threshold: 0.001,
};

pub(crate) struct State {
    group: Selection<usize>,
    step: f64,
    forward: bool,
    spring_forward: bool,
    number: f64,
    number_input: String,
    date_input: String,
}

impl Default for State {
    fn default() -> Self {
        let mut group = Selection::single().required(true);
        group.select(1);
        Self {
            group,
            step: 1.0,
            forward: false,
            spring_forward: false,
            number: 1_234_567.891,
            number_input: String::new(),
            date_input: String::new(),
        }
    }
}

pub(crate) fn show(ui: &mut Mui<'_>, s: &mut State, shared: &mut Shared) {
    section(ui, TOPICS[0], None, |ui| theme(ui, shared));
    section(ui, TOPICS[1], None, colors);
    section(ui, TOPICS[2], None, typography);
    section(ui, TOPICS[3], None, shapes);
    section(ui, TOPICS[4], None, elevation);
    section(ui, TOPICS[5], None, spacing);
    section(ui, TOPICS[6], None, |ui| motion_demo(ui, s));
    section(ui, TOPICS[7], None, |ui| locale(ui, s));
    section(ui, TOPICS[8], None, environment);
}

fn environment(ui: &mut Mui<'_>) {
    let rt = ui.rt();
    let ctx = ui.ctx().clone();
    let width = ctx.content_rect().width();
    SettingRow::new("env-class", "Window class")
        .supporting(format!("Window width {width:.0} dp"))
        .show_value(
            ui,
            format!(
                "breakpoint {} ({:?})",
                layout::breakpoint(rt.core(), width),
                WindowClass::of(rt.core())
            ),
        );
    let i = layout::safe_area(&ctx);
    ui.label(format!(
        "Safe area insets: top {} / right {} / bottom {} / left {}",
        i.top, i.right, i.bottom, i.left
    ));
    caption(
        ui,
        TypeScale::BodySmall,
        "Resize the window to cross breakpoints; insets are nonzero on notched devices. Setting rows stack \
         their label above the control below the first breakpoint.",
    );
}

fn contrast(bg: Color32) -> Color32 {
    let lum = 0.2126 * f32::from(bg.r()) + 0.7152 * f32::from(bg.g()) + 0.0722 * f32::from(bg.b());
    if lum > 140.0 { Color32::BLACK } else { Color32::WHITE }
}

fn theme(ui: &mut Mui<'_>, shared: &mut Shared) {
    let rt = ui.rt();
    let s = &mut shared.settings;
    let mut theme = Some(s.theme);
    rt.ui(ui)
        .mineral(Select::new("set-theme", "Theme", THEMES.map(|(_, n)| n), &mut theme).preferred_width(240.0));
    if let Some(t) = theme {
        s.theme = t;
    }
    ui.add_space(8.0);
    SettingRow::new("set-global-row", "Style egui globals")
        .supporting("Tooltips, popups and window decorations follow the kit")
        .show(ui, |ui, label| {
            ui.mineral(Switch::new("set-global", label, &mut s.global_kit).label_visible(false))
        });
    ui.add_space(8.0);
    SettingRow::new("set-contrast-row", "High contrast")
        .supporting("Maximum contrast between roles")
        .show(ui, |ui, label| {
            ui.mineral(Switch::new("set-contrast", label, &mut s.high_contrast).label_visible(false))
        });
    ui.add_space(8.0);
    let mut seed = Color32::from_rgb(s.seed[0], s.seed[1], s.seed[2]);
    rt.ui(ui).mineral(ColorPicker::new("set-seed", "Seed color", &mut seed));
    s.seed = [seed.r(), seed.g(), seed.b()];
    ui.add_space(8.0);
    let mut variant = Some(s.variant);
    rt.ui(ui).mineral(
        Select::new("set-variant", "Scheme variant", VARIANTS.map(|(_, n)| n), &mut variant).preferred_width(240.0),
    );
    if let Some(v) = variant {
        s.variant = v;
    }
    ui.add_space(8.0);
    let mut locale = Some(s.locale);
    rt.ui(ui)
        .mineral(Select::new("set-locale", "Locale", LOCALES, &mut locale).preferred_width(240.0));
    if let Some(l) = locale {
        s.locale = l;
    }
    ui.add_space(8.0);
    let mut density = f64::from(s.density);
    rt.ui(ui).mineral(
        ValueSlider::new(
            "set-density",
            "Density",
            RangeModel::new(f64::from(Density::MIN.step()), 0.0).step(1.0),
            &mut density,
        )
        .ticks(true)
        .format(|v| format!("{v:.0}"))
        .preferred_width(280.0),
    );
    s.density = density.round() as i8;
    caption(
        ui,
        TypeScale::BodySmall,
        "Density -6..0 shrinks the visual size of components by 4dp per step, down to their content. Hit targets stay at 48dp: \
         the visual box is smaller, the touch zone is not.",
    );
}

fn colors(ui: &mut Mui<'_>) {
    let rt = ui.rt();
    let t = rt.tokens();
    let c = &t.color;
    macro_rules! roles {
        ($($name:ident $(: $on:ident)?),* $(,)?) => {
            [$((stringify!($name), c.$name, roles!(@on c, c.$name $(, $on)?))),*]
        };
        (@on $c:ident, $bg:expr) => { contrast($bg) };
        (@on $c:ident, $bg:expr, $on:ident) => { $c.$on };
    }
    let all: [(&str, Color32, Color32); 49] = roles![
        primary: on_primary,
        on_primary,
        primary_container: on_primary_container,
        on_primary_container,
        inverse_primary,
        primary_fixed: on_primary_fixed,
        primary_fixed_dim: on_primary_fixed,
        on_primary_fixed,
        on_primary_fixed_variant,
        secondary: on_secondary,
        on_secondary,
        secondary_container: on_secondary_container,
        on_secondary_container,
        secondary_fixed: on_secondary_fixed,
        secondary_fixed_dim: on_secondary_fixed,
        on_secondary_fixed,
        on_secondary_fixed_variant,
        tertiary: on_tertiary,
        on_tertiary,
        tertiary_container: on_tertiary_container,
        on_tertiary_container,
        tertiary_fixed: on_tertiary_fixed,
        tertiary_fixed_dim: on_tertiary_fixed,
        on_tertiary_fixed,
        on_tertiary_fixed_variant,
        error: on_error,
        on_error,
        error_container: on_error_container,
        on_error_container,
        surface_dim: on_surface,
        surface: on_surface,
        surface_tint,
        surface_bright: on_surface,
        surface_container_lowest: on_surface,
        surface_container_low: on_surface,
        surface_container: on_surface,
        surface_container_high: on_surface,
        surface_container_highest: on_surface,
        on_surface,
        on_surface_variant,
        outline,
        outline_variant,
        inverse_surface: inverse_on_surface,
        inverse_on_surface,
        surface_variant: on_surface_variant,
        background: on_background,
        on_background,
        shadow,
        scrim,
    ];
    let cols = 4;
    let gap = adaptive::grid(2);
    // Flex items take whole points.
    let w = ((ui.available_width() - gap * (cols - 1) as f32) / cols as f32).floor();
    let font = TypeScale::LabelMedium.font(ui.ctx());
    for chunk in all.chunks(cols) {
        super::row(ui, |row| {
            for (name, bg, fg) in chunk {
                row.add(|ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(w, 56.0), Sense::hover());
                    let p = ui.painter();
                    p.rect_filled(rect, 8, *bg);
                    p.rect_stroke(rect, 8, Stroke::new(1.0, c.outline_variant), StrokeKind::Inside);
                    p.text(
                        rect.left_center() + vec2(10.0, 0.0),
                        Align2::LEFT_CENTER,
                        name,
                        font.clone(),
                        *fg,
                    );
                });
            }
        });
    }
    caption(
        ui,
        TypeScale::BodySmall,
        "Text color is the paired on-color where the role has one.",
    );
}

fn typography(ui: &mut Mui<'_>) {
    let rt = ui.rt();
    let t = rt.tokens();
    for (scale, name) in SCALES {
        let TypeSpec {
            size,
            line_height: line,
            weight,
            tracking,
            ..
        } = scale.spec();
        super::row(ui, |row| {
            row.add(|ui| ui.label(RichText::new(name).font(scale.font(rt.ctx()))));
            row.add(|ui| {
                ui.label(
                    RichText::new(format!("{size}/{line} w{weight} t{tracking}"))
                        .font(TypeScale::LabelSmall.font(rt.ctx()))
                        .color(t.color.on_surface_variant),
                )
            });
        });
    }
}

fn shapes(ui: &mut Mui<'_>) {
    let rt = ui.rt();
    let t = rt.tokens();
    let sh = t.shape;
    let items = [
        ("none", sh.none),
        ("extra small", sh.extra_small),
        ("small", sh.small),
        ("medium", sh.medium),
        ("large", sh.large),
        ("large increased", sh.large_increased),
        ("extra large", sh.extra_large),
        ("extra large increased", sh.extra_large_increased),
        ("extra extra large", sh.extra_extra_large),
        ("full", sh.full),
    ];
    super::row(ui, |row| {
        for (name, r) in items {
            row.add(|ui| {
                ui.vertical(|ui| {
                    let (rect, _) = ui.allocate_exact_size(vec2(96.0, 72.0), Sense::hover());
                    ui.painter()
                        .rect_filled(rect, CornerRadius::same(r), t.color.primary_container);
                    caption(&mut rt.ui(ui), TypeScale::BodySmall, name);
                    caption(
                        &mut rt.ui(ui),
                        TypeScale::BodySmall,
                        &if r == u8::MAX {
                            "full".to_owned()
                        } else {
                            format!("{r} dp")
                        },
                    );
                })
            });
        }
    });
}

fn elevation(ui: &mut Mui<'_>) {
    let rt = ui.rt();
    let t = rt.tokens();
    ui.add_space(8.0);
    layout::Flow::new(vec2(28.0, adaptive::grid(2)), egui::Align::Center).show(ui, |row| {
        for (i, &level) in ElevationLevel::ALL.iter().enumerate() {
            row.add(|ui| {
                ui.vertical(|ui| {
                    Frame::new()
                        .fill(t.color.surface_container_low)
                        .corner_radius(12)
                        .shadow(t.elevation_shadow(level))
                        .inner_margin(Margin::same(8))
                        .show(ui, |ui| {
                            ui.set_min_size(vec2(80.0, 56.0));
                            ui.label(format!("Level {i}"));
                        });
                    ui.add_space(12.0);
                    caption(&mut rt.ui(ui), TypeScale::BodySmall, &format!("{} dp", level.dp()));
                })
            });
        }
    });
    ui.add_space(12.0);
}

fn spacing(ui: &mut Mui<'_>) {
    let rt = ui.rt();
    let t = rt.tokens();
    for steps in [1, 2, 4, 6, 8] {
        let v = adaptive::grid(steps);
        super::row(ui, |row| {
            row.add(|ui| ui.label(RichText::new(format!("grid({steps})")).font(TypeScale::LabelLarge.font(rt.ctx()))));
            row.add(|ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(v * 4.0, 16.0), Sense::hover());
                ui.painter().rect_filled(rect, 2, t.color.tertiary);
            });
            row.add(|ui| caption(ui, TypeScale::BodySmall, &format!("{v} dp (bar drawn at 4x)")));
        });
    }
}

fn motion_demo(ui: &mut Mui<'_>, s: &mut State) {
    let rt = ui.rt();
    let ctx = ui.ctx().clone();
    let t = rt.tokens();
    let mut reduced = rt.core().motion().reduced();
    if ui
        .mineral(Switch::new("motion-reduced", "Reduced motion", &mut reduced))
        .changed()
    {
        rt.core().motion().set_reduced(reduced);
    }
    ui.add_space(8.0);
    ui.label("Duration token");
    rt.ui(ui).mineral(
        ValueSlider::new(
            "motion-step",
            "Duration step",
            RangeModel::new(0.0, 3.0).step(1.0),
            &mut s.step,
        )
        .ticks(true)
        .preferred_width(200.0),
    );
    let mut sel = s.group.clone();
    ButtonGroup::new("motion-group", "Duration group")
        .variant(ButtonGroupVariant::Connected)
        .items(DURATION_GROUPS.iter().map(|g| ButtonGroupItem::new(*g)))
        .selection(&mut sel)
        .show(ui);
    s.group = sel;
    let g = s.group.first().copied().unwrap_or(1);
    let d = &t.duration;
    #[expect(clippy::cast_sign_loss, reason = "the slider range is 0..=3")]
    let step = s.step.round() as usize;
    let duration = [d.short, d.medium, d.long, d.extra_long][g][step];
    super::row(ui, |row| {
        if row
            .mineral(
                PushButton::new("motion-play", if s.forward { "Reverse" } else { "Play" })
                    .variant(ButtonVariant::Tonal),
            )
            .clicked()
        {
            s.forward = !s.forward;
        }
        row.add(|ui| {
            caption(
                ui,
                TypeScale::BodySmall,
                &format!(
                    "{} {} = {} ms; animates only while a value is moving",
                    DURATION_GROUPS[g],
                    step + 1,
                    duration.as_millis()
                ),
            );
        });
    });
    let time = rt.core().motion().duration(duration).as_secs_f32();
    // Easing name column, then the track; both shrink on narrow windows.
    let label_w = (ui.available_width() / 2.0).min(180.0);
    let track_w = (ui.available_width() - label_w - 20.0).max(0.0);
    let font = TypeScale::LabelMedium.font(&ctx);
    for (i, (name, ease)) in EASINGS.iter().enumerate() {
        let id = ui.make_persistent_id(("motion-ease", i));
        let v = ease.ease(ctx.animate_bool_with_time(id, s.forward, time));
        let (rect, _) = ui.allocate_exact_size(vec2(label_w + track_w + 10.0, 24.0), Sense::hover());
        let p = ui.painter();
        p.text(
            rect.left_center(),
            Align2::LEFT_CENTER,
            name,
            font.clone(),
            t.color.on_surface,
        );
        let track = egui::Rect::from_min_size(rect.min + vec2(label_w, 0.0), vec2(track_w, 24.0));
        p.line_segment(
            [track.left_center(), track.right_center()],
            Stroke::new(2.0, t.color.outline_variant),
        );
        p.circle_filled(track.left_center() + vec2(v * track_w, 0.0), 9.0, t.color.primary);
    }
    spring_demo(ui, s, label_w, track_w);
}

fn spring_demo(ui: &mut Mui<'_>, s: &mut State, label_w: f32, track_w: f32) {
    let rt = ui.rt();
    let ctx = ui.ctx().clone();
    let t = rt.tokens();
    ui.add_space(12.0);
    ui.label("Core spring (damping 0.5, stiffness 400)");
    super::row(ui, |row| {
        if row
            .mineral(
                PushButton::new("motion-spring-play", if s.spring_forward { "Reverse" } else { "Play" })
                    .variant(ButtonVariant::Tonal),
            )
            .clicked()
        {
            s.spring_forward = !s.spring_forward;
        }
    });
    let id = ui.make_persistent_id("motion-spring");
    let p = ctx.animate_bool_with_time(
        id,
        s.spring_forward,
        rt.core()
            .motion()
            .duration(std::time::Duration::from_secs(1))
            .as_secs_f32(),
    );
    let (rect, _) = ui.allocate_exact_size(vec2(label_w + track_w + 10.0, 24.0), Sense::hover());
    let track = egui::Rect::from_min_size(rect.min + vec2(label_w, 0.0), vec2(track_w, 24.0));
    // Spring time runs forward in both directions: mirror the reverse leg.
    let x = if s.spring_forward {
        SPRING.at(p)
    } else {
        1.0 - SPRING.at(1.0 - p)
    };
    let painter = ui.painter();
    painter.line_segment(
        [track.left_center(), track.right_center()],
        Stroke::new(2.0, t.color.outline_variant),
    );
    painter.circle_filled(
        track.left_center() + vec2(x * track_w * 0.8, 0.0),
        9.0,
        t.color.tertiary,
    );
}

fn locale(ui: &mut Mui<'_>, s: &mut State) {
    let rt = ui.rt();
    let f = rt.i18n().format();
    let sample = Date::constant(2026, 11, 15);
    let afternoon = Time::constant(14, 5, 0, 0);
    let first = f.first_weekday();
    let rtl = rt.i18n().direction() == i18n::Direction::Rtl;
    let words = ["apple", "Zebra", "\u{c4}pfel", "zebra", "eclair", "\u{e9}clair"];
    let mut sorted = words;
    sorted.sort_by(|a, b| f.compare(a, b));
    let weekdays: Vec<String> = (0..7)
        .map(|i| f.weekday_name(first.wrapping_add(i), NameWidth::Abbreviated))
        .collect();
    let months: Vec<String> = (1..=12).map(|m| f.month_name(m, NameWidth::Wide)).collect();
    let rows: Vec<(&str, String)> = vec![
        ("Tag", f.tag().to_owned()),
        (
            "Direction",
            format!("{:?}{}", f.direction(), if rtl { " (UI mirrored)" } else { "" }),
        ),
        ("First weekday", f.weekday_name(first, NameWidth::Wide)),
        ("Hour cycle", format!("{:?}", f.hour_cycle())),
        (
            "Date numeric",
            f.date(sample, DateFields::YearMonthDay, DateLength::Numeric),
        ),
        (
            "Date medium",
            f.date(sample, DateFields::YearMonthDay, DateLength::Medium),
        ),
        ("Date long", f.date(sample, DateFields::YearMonthDay, DateLength::Long)),
        (
            "Date headline",
            f.date(sample, DateFields::MonthDayWeekday, DateLength::Medium),
        ),
        ("Month and year", f.month_year(2026, 11)),
        ("Time 12-hour", f.time(afternoon, HourCycle::H12)),
        ("Time 24-hour", f.time(afternoon, HourCycle::H23)),
        ("Number", f.number(s.number, None)),
        ("Number, 0 digits", f.number(s.number, Some(0))),
        ("Number, 2 digits", f.number(s.number, Some(2))),
        ("Integer", f.integer(-1_234_567)),
        ("List and", f.list(&["red", "green", "blue"], ListKind::And)),
        ("List or", f.list(&["red", "green", "blue"], ListKind::Or)),
        ("List unit", f.list(&["3 ft", "7 in"], ListKind::Unit)),
        ("Weekdays", weekdays.join(" ")),
        ("Months", months.join(", ")),
        ("Collation", sorted.join(" < ")),
    ];
    ui.mineral(
        DescriptionList::new("locale-values")
            .columns(1)
            .dividers(true)
            .items(rows.into_iter().map(|(k, v)| DescriptionItem::term(k).detail(v))),
    );
    ui.add_space(8.0);
    super::row(ui, |row| {
        row.mineral(
            TextField::new("fd-parse-number", "Parse number", &mut s.number_input)
                .placeholder("type a localized number")
                .preferred_width(240.0),
        );
        row.add(|ui| {
            ui.label(match f.parse_number(&s.number_input) {
                Some(v) => format!("= {v}"),
                None if s.number_input.is_empty() => String::new(),
                None => "not a number".to_owned(),
            })
        });
    });
    super::row(ui, |row| {
        row.mineral(
            TextField::new("fd-parse-date", "Parse date", &mut s.date_input)
                .placeholder(f.date(sample, DateFields::YearMonthDay, DateLength::Numeric))
                .preferred_width(240.0),
        );
        row.add(|ui| {
            ui.label(match f.parse_date(&s.date_input) {
                Some(d) => format!("= {d}"),
                None if s.date_input.is_empty() => String::new(),
                None => "not a date".to_owned(),
            })
        });
    });
}
