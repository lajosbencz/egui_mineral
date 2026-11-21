use std::collections::HashMap;

use egui::{Rect, Vec2, vec2};
use egui_mineral::{steps::Steps, widget::SizeExt};
use egui_mineral_material::{
    Size,
    actions::{IconButton, PushButton},
    adaptive,
    communication::{CircularProgressIndicator, LinearProgressIndicator, ProgressVariant},
    extras::{DescriptionItem, DescriptionLayout, DescriptionList, StepLabel, Stepper, Timeline, TimelineItem},
    icons::ICON_MAIL,
    scroll,
    spec::{defaults::text_field, tokens::linear_progress_indicator},
    text_inputs::{TextField, TextFieldVariant},
};

use crate::support::{MineralHarness, Mui, SCREEN, build, builder, harness, min_interactive, runtime, still};

const KEYS: [&str; 4] = ["f-field", "o-field", "ib-button", "b-button"];

fn rows(locale: &'static str) -> (HashMap<&'static str, Rect>, Rect, u64) {
    let setup =
        move |rt: &egui_mineral_material::Rt| rt.i18n().set_locale(&locale.parse().expect("tag")).expect("locale");
    let long = "A long label that wraps inside its flex item at the row width. ".repeat(4);
    let mut h = build(
        builder(SCREEN),
        Some(Box::new(setup)),
        (String::from("text"), String::from("text"), Rect::NOTHING),
        move |ui, (a, b, label)| {
            adaptive::wrapping_row_top(ui, |row| {
                row.mineral(TextField::new("f", "Filled", a).preferred_width(150.0));
                row.mineral(
                    TextField::new("o", "Outlined", b)
                        .variant(TextFieldVariant::Outlined)
                        .preferred_width(150.0),
                );
            });
            adaptive::wrapping_row(ui, |row| {
                row.mineral(IconButton::new("ib", ICON_MAIL, "Mail"));
                row.mineral(PushButton::new("b", "Button"));
                *label = row.add(|ui| ui.label(long.as_str())).rect;
            });
        },
    );
    let before = h.ctx.cumulative_pass_nr();
    h.step();
    let passes = h.ctx.cumulative_pass_nr() - before;
    let rects = KEYS.into_iter().map(|k| (k, h.rect(k))).collect();
    (rects, h.state().value.2, passes)
}

#[test]
fn flow_rows_settle_and_align_like_compose() {
    let (r, label, passes) = rows("en-US");
    assert_eq!(passes, 1, "a settled frame takes one pass");
    // Top-aligned: the outlined container sits lower by its label cutout, as in Compose.
    assert_eq!(
        r["o-field"].top() - r["f-field"].top(),
        text_field::MIN_FOCUSED_LABEL_LINE_HEIGHT / 2.0
    );
    assert!(r["f-field"].right() < r["o-field"].left());
    assert_eq!(r["ib-button"].height(), min_interactive());
    assert_eq!(r["ib-button"].center().y, r["b-button"].center().y);
    assert!(label.width() <= SCREEN.x, "label {label:?} wraps inside the row");
    assert!(label.height() > r["b-button"].height(), "label {label:?} wraps");
}

#[test]
fn flow_rows_start_at_the_rtl_edge() {
    let (r, _, passes) = rows("ar-EG");
    assert_eq!(passes, 1, "a settled frame takes one pass");
    assert!(r["f-field"].left() > r["o-field"].right());
    assert!(r["ib-button"].left() > r["b-button"].right());
}

#[test]
fn size_overrides_win_over_token_defaults() {
    let h = harness(SCREEN, (String::new(), String::new()), |ui, (a, b)| {
        ui.mineral(TextField::new("tf", "Label", a));
        ui.mineral(TextField::new("tf-fixed", "Label", b).preferred_width(200.0));
        ui.mineral(LinearProgressIndicator::determinate("lp", 0.5));
        ui.mineral(LinearProgressIndicator::determinate("lp-fixed", 0.5).preferred_width(120.0));
        ui.mineral(CircularProgressIndicator::determinate("cp", 0.5));
        ui.mineral(CircularProgressIndicator::determinate("cp-fixed", 0.5).preferred_size(Vec2::splat(64.0)));
    });
    let (field, fixed) = (h.rect("tf-field"), h.rect("tf-fixed-field"));
    assert_eq!(field.width(), text_field::MIN_WIDTH);
    assert_eq!(fixed.width(), 200.0);
    assert_eq!(fixed.height(), field.height(), "the height keeps its token");
    assert_eq!(
        h.rect("lp-fixed-progress").size(),
        vec2(120.0, linear_progress_indicator::HEIGHT)
    );
    assert_eq!(h.rect("cp-fixed-progress").size(), Vec2::splat(64.0));
}

#[test]
fn linear_progress_narrower_than_its_caps_paints() {
    for width in [0.0, 3.0] {
        harness(SCREEN, (), move |ui, ()| {
            for (i, variant) in [ProgressVariant::Flat, ProgressVariant::Wavy].into_iter().enumerate() {
                ui.mineral(
                    LinearProgressIndicator::determinate(("d", i), 0.5)
                        .variant(variant)
                        .preferred_width(width),
                );
                ui.mineral(
                    LinearProgressIndicator::indeterminate(("i", i))
                        .variant(variant)
                        .preferred_width(width),
                );
            }
        });
    }
}

#[test]
fn scoped_density_shrinks_only_its_subtree() {
    use egui_mineral_material::{MaterialUiExt as _, adaptive::Density};
    let h = harness(SCREEN, [String::new(), String::new()], |ui, [a, b]| {
        let field = |id, value| TextField::new(id, "Label", value).variant(TextFieldVariant::Outlined);
        ui.with_density(Density::new(-2), |ui| ui.mineral(field("dense", a)));
        ui.mineral(field("after", b));
    });
    let (dense, after) = (h.rect("dense-field").height(), h.rect("after-field").height());
    assert_eq!(after - dense, 8.0, "two steps of 4dp, none after the scope");
}

#[test]
fn text_style_sets_the_line_spacing_of_extra_rows() {
    use egui_mineral_material::spec::TypeScale;
    let rows = 4;
    let h = harness(SCREEN, [String::new(), String::new()], move |ui, [a, b]| {
        ui.mineral(TextField::new("body", "Body", a).rows(rows));
        ui.mineral(
            TextField::new("small", "Small", b)
                .rows(rows)
                .text_style(TypeScale::BodyMedium),
        );
    });
    let gap = TypeScale::BodyLarge.line_height() - TypeScale::BodyMedium.line_height();
    let (body, small) = (h.rect("body-field").height(), h.rect("small-field").height());
    assert_eq!(body - small, 3.0 * gap);
}

// Narrowest phone in portrait (dp); the other window classes start at their breakpoints.
const COMPACT_MIN: f32 = 320.0;

const DL_ITEMS: [(&str, &str); 6] = [
    ("Name", "egui_mineral"),
    ("License", "GPL-3.0-or-later"),
    ("Toolkit", "egui 0.37"),
    ("Layout", "Taffy"),
    ("Locale data", "ICU4X / CLDR"),
    ("Targets", "Linux, macOS, Windows, wasm32"),
];

fn description_list(layout: DescriptionLayout, columns: usize) -> DescriptionList {
    DescriptionList::new("dl")
        .layout(layout)
        .columns(columns)
        .dividers(true)
        .items(DL_ITEMS.map(|(term, detail)| DescriptionItem::term(term).detail(detail)))
}

#[test]
fn full_width_composites_fit_every_window_class() {
    type Add = fn(&mut Mui<'_>);
    let cases: [(&str, Add); 5] = [
        ("stepper", |ui| {
            let mut steps = Steps::new(0..4);
            Stepper::new("st", &mut steps)
                .step(0, "Account")
                .step(1, StepLabel::new("Profile").supporting("Name and photo"))
                .step(2, "Preferences")
                .step(3, "Review")
                .show(ui, |ui, i| {
                    ui.label(format!("Content of step {}", i + 1));
                });
        }),
        ("timeline", |ui| {
            ui.mineral(
                Timeline::new("tl")
                    .item(
                        TimelineItem::new("Release")
                            .time("Nov 14")
                            .supporting("Tagged.")
                            .active(true),
                    )
                    .item(TimelineItem::new("Kickoff").time("Nov 1")),
            );
        }),
        ("description list, horizontal", |ui| {
            ui.mineral(description_list(DescriptionLayout::Horizontal, 2));
        }),
        ("description list, vertical", |ui| {
            ui.mineral(description_list(DescriptionLayout::Vertical, 3));
        }),
        ("description list, one column", |ui| {
            ui.mineral(description_list(DescriptionLayout::Horizontal, 1));
        }),
    ];
    let rt = runtime(&egui::Context::default(), egui::Theme::Light);
    let breakpoints = rt.core().policies().breakpoints.clone();
    for width in std::iter::once(COMPACT_MIN).chain(breakpoints.iter().copied()) {
        for (name, add) in cases {
            // Settled overflow: a page scroll widens to its content, so excess compounds across passes.
            let mut h = still(vec2(width, 600.0), 0.0_f32, move |ui, over| {
                // Taller than the window: the page bar appears after the first pass and narrows the content.
                let out = scroll::page(ui, "page", |ui| {
                    add(ui);
                    ui.add_space(2000.0);
                });
                *over = out.content_size.x - out.inner_rect.width();
            });
            h.run_steps(8);
            let over = h.state().value;
            assert!(over <= 0.5, "{name} at {width}: {over:.1} px wider than its page");
        }
    }
}

#[test]
fn dense_controls_shrink_to_their_content_and_no_further() {
    use egui::Rect;
    use egui_mineral_material::{
        MaterialUiExt as _,
        actions::SegmentedButton,
        adaptive::Density,
        navigation::{Tab, Tabs},
        selection::Chip,
        spec::TypeScale,
    };
    const PAD: f32 = 4.0;
    type Add = fn(&mut Mui<'_>) -> Rect;
    let line = TypeScale::LabelLarge.line_height();
    let cases: [(&str, Add, f32); 5] = [
        (
            "button",
            |ui| ui.mineral(PushButton::new("b", "Go")).zones().visual,
            line,
        ),
        (
            "icon button",
            |ui| {
                ui.mineral(IconButton::new("i", ICON_MAIL, "Mail").size(Size::Small))
                    .zones()
                    .visual
            },
            24.0,
        ),
        (
            "segmented",
            |ui| {
                let mut s = 0;
                SegmentedButton::new("s", "Seg", &mut s)
                    .items(["A", "B"].into_iter().enumerate())
                    .show(ui)
                    .response
                    .rect
            },
            line,
        ),
        (
            "tabs",
            |ui| {
                let mut s = 0;
                Tabs::new("t", "Tabs", &mut s)
                    .item(0, Tab::new("A"))
                    .item(1, Tab::new("B"))
                    .show(ui)
                    .response
                    .rect
            },
            line,
        ),
        ("chip", |ui| ui.mineral(Chip::assist("c", "Chip")).zones().visual, line),
    ];
    // The segmented row reserves the hit target; the segments are what shrinks.
    let height = |name: &str, add: Add, step: i8| {
        let mut out = 0.0;
        let p = crate::support::render_on(SCREEN, |ui| {
            out = ui.with_density(Density::new(step), add).height();
        });
        if name == "segmented" {
            (
                p.stroked(egui_mineral_material::spec::ColorRole::Outline)[0].height(),
                p.text("A").height(),
            )
        } else {
            (out, f32::NAN)
        }
    };
    for (name, add, content) in cases {
        let (full, mid, min) = (height(name, add, 0).0, height(name, add, -3).0, height(name, add, -6));
        let (min, text) = min;
        let content = if text.is_nan() { content } else { text };
        assert!(min >= content + PAD, "{name}: {min} below content");
        assert!(min <= mid && min < full, "{name}: {min} vs {mid} vs {full}");
    }
}
