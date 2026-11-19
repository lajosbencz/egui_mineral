use egui::vec2;
use egui_mineral::steps::Steps;
use egui_mineral_material::{
    extras::{DescriptionItem, DescriptionLayout, DescriptionList, StepLabel, Stepper, Timeline, TimelineItem},
    scroll,
};

use crate::support::{Mui, runtime, still};

fn description_list(layout: DescriptionLayout, columns: usize) -> DescriptionList {
    DescriptionList::new("dl")
        .layout(layout)
        .columns(columns)
        .dividers(true)
        .items(DL_ITEMS.map(|(term, detail)| DescriptionItem::term(term).detail(detail)))
}

const COMPACT_MIN: f32 = 320.0;

const DL_ITEMS: [(&str, &str); 6] = [
    ("Name", "egui_mineral"),
    ("License", "GPL-3.0-or-later"),
    ("Toolkit", "egui 0.37"),
    ("Layout", "Taffy"),
    ("Locale data", "ICU4X / CLDR"),
    ("Targets", "Linux, macOS, Windows, wasm32"),
];

#[test]
fn full_width_composites_fit_every_window_class() {
    type Add = fn(&mut Mui<'_>);
    let cases: [(&str, Add); 5] = [
        ("stepper", |ui| {
            let mut steps = Steps::new(4);
            Stepper::new("st", &mut steps)
                .step("Account")
                .step(StepLabel::new("Profile").supporting("Name and photo"))
                .step("Preferences")
                .step("Review")
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
