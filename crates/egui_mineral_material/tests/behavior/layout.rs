use std::collections::HashMap;

use egui::{Rect, Vec2, vec2};
use egui_mineral::widget::SizeExt;
use egui_mineral_material::{
    actions::{IconButton, PushButton},
    adaptive,
    communication::{CircularProgressIndicator, LinearProgressIndicator},
    extras::{DescriptionItem, DescriptionLayout, DescriptionList, Timeline, TimelineItem},
    icons::ICON_MAIL,
    scroll,
    spec::{defaults::text_field, tokens::linear_progress_indicator},
    text_inputs::{TextField, TextFieldVariant},
};

use crate::support::{MineralHarness, Mui, SCREEN, build, builder, harness, min_interactive, still};

const KEYS: [&str; 4] = ["f-field", "o-field", "ib-button", "b-button"];

fn rows(locale: &'static str) -> (HashMap<&'static str, Rect>, Rect, u64) {
    let setup = move |rt: &egui_mineral_material::Rt| rt.i18n().set_locale(locale).expect("locale");
    let long = "A long label that wraps inside its flex item at the row width. ".repeat(4);
    let mut h = build(
        builder(SCREEN),
        Some(Box::new(setup)),
        (String::from("text"), String::from("text"), Rect::NOTHING),
        move |ui, (a, b, label)| {
            adaptive::wrapping_row_top(ui, |row| {
                row.mineral(TextField::new("f", "Filled", a).fixed_width(150.0));
                row.mineral(
                    TextField::new("o", "Outlined", b)
                        .variant(TextFieldVariant::Outlined)
                        .fixed_width(150.0),
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
        ui.mineral(TextField::new("tf-fixed", "Label", b).fixed_width(200.0));
        ui.mineral(LinearProgressIndicator::determinate("lp", 0.5));
        ui.mineral(LinearProgressIndicator::determinate("lp-fixed", 0.5).fixed_width(120.0));
        ui.mineral(CircularProgressIndicator::determinate("cp", 0.5));
        ui.mineral(CircularProgressIndicator::determinate("cp-fixed", 0.5).fixed_size(Vec2::splat(64.0)));
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
    for width in [0.0, 1.0, 2.0, 3.0] {
        harness(SCREEN, (), move |ui, ()| {
            for (i, wavy) in [false, true].into_iter().enumerate() {
                ui.mineral(
                    LinearProgressIndicator::determinate(("d", i), 0.5)
                        .wavy(wavy)
                        .fixed_width(width),
                );
                ui.mineral(
                    LinearProgressIndicator::indeterminate(("i", i))
                        .wavy(wavy)
                        .fixed_width(width),
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
    let rows = std::num::NonZero::new(4).expect("non-zero");
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
fn full_width_composites_never_widen_their_page() {
    type Add = fn(&mut Mui<'_>);
    let cases: [(&str, Add); 4] = [
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
    for width in [360.0, 673.0, 1024.0] {
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
