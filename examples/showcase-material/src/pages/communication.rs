use egui::{Rect, vec2};
use egui_mineral_material::{
    catalog::Component,
    icons::{ICON_ADD, ICON_FAVORITE, ICON_MAIL, ICON_NOTIFICATIONS, ICON_SETTINGS, ICON_SHARE},
    prelude::*,
};

use super::{Shared, section, snack, sub};

pub(crate) const TOPICS: &[&str] = &[
    "Badge",
    "Linear progress",
    "Circular progress",
    "Loading indicator",
    "Snackbar",
    "Tooltips",
    "Alert",
    "Skeleton",
    "Tour",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, egui_mineral::semantic::KeySalt)]
enum TipAction {
    Learn,
    Archive,
}

const SEVERITIES: [(Severity, &str); 4] = [
    (Severity::Info, "Info"),
    (Severity::Success, "Success"),
    (Severity::Warning, "Warning"),
    (Severity::Error, "Error"),
];

const ALERT_VARIANTS: [(AlertVariant, &str); 3] = [
    (AlertVariant::Standard, "Standard"),
    (AlertVariant::Outlined, "Outlined"),
    (AlertVariant::Banner, "Banner"),
];

pub(crate) struct State {
    badge_count: u32,
    progress: f64,
    dismissed: [bool; 4],
    tour_step: Option<usize>,
    tour_rects: [Rect; 3],
}

impl Default for State {
    fn default() -> Self {
        Self {
            badge_count: 7,
            progress: 40.0,
            dismissed: [false; 4],
            tour_step: None,
            tour_rects: [Rect::NOTHING; 3],
        }
    }
}

pub(crate) fn show(ui: &mut MaterialUi<'_>, s: &mut State, shared: &mut Shared) {
    section(ui, TOPICS[0], Some(Badge::META), |ui| badges(ui, s));
    section(ui, TOPICS[1], Some(LinearProgressIndicator::META), |ui| linear(ui, s));
    let circular_meta = CircularProgressIndicator::META;
    section(ui, TOPICS[2], Some(circular_meta), |ui| circular(ui, s));
    section(ui, TOPICS[3], Some(LoadingIndicator::META), loading);
    section(ui, TOPICS[4], Some(Snackbar::META), |ui| snackbars(ui, shared));
    section(ui, TOPICS[5], Some(RichTooltip::META), |ui| tooltips(ui, shared));
    section(ui, TOPICS[6], Some(Alert::META), |ui| alerts(ui, s, shared));
    section(ui, TOPICS[7], Some(Skeleton::META), skeletons);
    section(ui, TOPICS[8], Some(<Tour<'static>>::META), |ui| {
        tour(ui, s, shared);
    });
}

fn badges(ui: &mut MaterialUi<'_>, s: &mut State) {
    sub(ui, "Count badge on icons; dot and overflow (max 99)");
    super::row(ui, |row| {
        row.mineral(IconButton::new("badge-mail", ICON_MAIL, "Mail").badge(Badge::count(s.badge_count).max(99)));
        row.space(16.0);
        row.mineral(IconButton::new("badge-bell", ICON_NOTIFICATIONS, "Notifications").badge(Badge::dot()));
        row.space(16.0);
        row.mineral(IconButton::new("badge-fav", ICON_FAVORITE, "Favorites").badge(Badge::count(1234).max(999)));
    });
    super::row(ui, |row| {
        if row
            .mineral(PushButton::new("badge-dec", "-").variant(ButtonVariant::Tonal))
            .clicked()
        {
            s.badge_count = s.badge_count.saturating_sub(1);
        }
        if row
            .mineral(PushButton::new("badge-inc", "+").variant(ButtonVariant::Tonal))
            .clicked()
        {
            s.badge_count += 1;
        }
        if row
            .mineral(PushButton::new("badge-big", "+50").variant(ButtonVariant::Text))
            .clicked()
        {
            s.badge_count += 50;
        }
        row.add(|ui| ui.label(format!("count = {}", s.badge_count)));
    });
}

fn linear(ui: &mut MaterialUi<'_>, s: &mut State) {
    sub(ui, "Driven by slider");
    ui.mineral(ValueSlider::new(
        "lin-slider",
        "Progress",
        RangeModel::new(0.0, 100.0).step(1.0),
        &mut s.progress,
    ));
    let v = (s.progress / 100.0) as f32;
    ui.mineral(LinearProgressIndicator::determinate("lin-det", v).label("Determinate"));
    sub(ui, "Wavy");
    ui.mineral(
        LinearProgressIndicator::determinate("lin-wavy", v)
            .label("Wavy")
            .variant(ProgressVariant::Wavy),
    );
    sub(ui, "Indeterminate");
    ui.mineral(LinearProgressIndicator::indeterminate("lin-ind").label("Indeterminate"));
    sub(ui, "Indeterminate, wavy, fixed width");
    ui.mineral(
        LinearProgressIndicator::indeterminate("lin-ind-wavy")
            .label("Wavy indeterminate")
            .variant(ProgressVariant::Wavy)
            .preferred_width(240.0),
    );
}

fn circular(ui: &mut MaterialUi<'_>, s: &mut State) {
    let v = (s.progress / 100.0) as f32;
    sub(ui, "Determinate (value from the slider above), sizes 24/48/72");
    super::row(ui, |row| {
        for (i, size) in [24.0, 48.0, 72.0].into_iter().enumerate() {
            row.add(|ui| {
                ui.mineral(
                    CircularProgressIndicator::determinate(("circ-det", i), v)
                        .label("Determinate")
                        .preferred_size(egui::Vec2::splat(size)),
                )
            });
        }
        row.add(|ui| {
            ui.mineral(
                CircularProgressIndicator::determinate("circ-wavy", v)
                    .label("Wavy")
                    .variant(ProgressVariant::Wavy),
            )
        });
    });
    sub(ui, "Indeterminate");
    super::row(ui, |row| {
        for (i, size) in [24.0, 48.0, 72.0].into_iter().enumerate() {
            row.add(|ui| {
                ui.mineral(
                    CircularProgressIndicator::indeterminate(("circ-ind", i))
                        .label("Indeterminate")
                        .preferred_size(egui::Vec2::splat(size)),
                )
            });
        }
        row.add(|ui| {
            ui.mineral(
                CircularProgressIndicator::indeterminate("circ-ind-wavy")
                    .label("Wavy indeterminate")
                    .variant(ProgressVariant::Wavy),
            )
        });
    });
}

fn loading(ui: &mut MaterialUi<'_>) {
    sub(ui, "Uncontained and contained, sizes 32/48/96");
    super::row(ui, |row| {
        for (i, size) in [32.0, 48.0, 96.0].into_iter().enumerate() {
            row.add(|ui| ui.mineral(LoadingIndicator::new(("load", i)).preferred_size(egui::Vec2::splat(size))));
        }
        for (i, size) in [32.0, 48.0, 96.0].into_iter().enumerate() {
            row.add(|ui| {
                ui.mineral(
                    LoadingIndicator::new(("load-c", i))
                        .variant(LoadingIndicatorVariant::Contained)
                        .preferred_size(egui::Vec2::splat(size)),
                )
            });
        }
    });
}

fn snackbars(ui: &mut MaterialUi<'_>, shared: &mut Shared) {
    ui.label("The shell hosts the queue at the bottom of the window.");
    super::row(ui, |row| {
        if row.mineral(PushButton::new("snack-plain", "Single line")).clicked() {
            shared.toasts.push(Snackbar::new("Message archived"));
        }
        if row.mineral(PushButton::new("snack-action", "With action")).clicked() {
            shared
                .toasts
                .push(Snackbar::new("Conversation deleted").action("undo", TextAction::new("Undo").command(snack::UNDO)));
        }
        if row.mineral(PushButton::new("snack-close", "With close")).clicked() {
            shared.toasts.push(Snackbar::new("Connection lost").dismissible(true));
        }
        if row.mineral(PushButton::new("snack-both", "Action and close")).clicked() {
            shared.toasts.push(
                Snackbar::new("Draft saved to the cloud")
                    .action("open", TextAction::new("Open").command(snack::OPEN))
                    .dismissible(true),
            );
        }
        if row.mineral(PushButton::new("snack-long", "Long text")).clicked() {
            shared.toasts.push(
                Snackbar::new(
                    "This snackbar carries a longer supporting text that wraps onto a second line to show the \
                     two-line layout of the component.",
                )
                .action("retry", TextAction::new("Retry").command(snack::RETRY))
                .dismissible(true),
            );
        }
        if row
            .mineral(PushButton::new("snack-burst", "Queue burst").variant(ButtonVariant::Tonal))
            .clicked()
        {
            for i in 1..=4 {
                shared.toast(format!("Queued message {i}"));
            }
        }
    });
}

fn tooltips(ui: &mut MaterialUi<'_>, shared: &mut Shared) {
    let rt = ui.rt();
    sub(ui, "Plain tooltip: hover");
    super::row(ui, |row| {
        let r = row.mineral(IconButton::new("tip-plain-ib", ICON_SHARE, "Share"));
        PlainTooltip::new("Share").show(rt, r.response());
        let r = row.mineral(PushButton::new("tip-plain-btn", "Hover me").variant(ButtonVariant::Outlined));
        PlainTooltip::new("A plain tooltip holds a short single-line label.").show(rt, r.response());
    });
    sub(ui, "Rich tooltip: supporting text, subhead, actions");
    super::row(ui, |row| {
        let r = row.mineral(PushButton::new("tip-rich-1", "Supporting only").variant(ButtonVariant::Tonal));
        RichTooltip::new(
            "tip-rich-1-tooltip",
            "Rich tooltips explain a feature in a few sentences.",
        )
        .show(rt, r.response());
        let r = row.mineral(PushButton::new("tip-rich-2", "With subhead").variant(ButtonVariant::Tonal));
        RichTooltip::new("tip-rich-2-tooltip", "Grants read access to everyone in the workspace.")
            .title("Sharing")
            .show(rt, r.response());
        let r = row.mineral(PushButton::new("tip-rich-3", "With actions").variant(ButtonVariant::Tonal));
        let tip = RichTooltip::new("tip-rich-3-tooltip", "Move the file to the archive folder.")
            .title("Archive")
            .action(TipAction::Learn, "Learn more")
            .action(TipAction::Archive, "Archive now")
            .show(rt, r.response());
        match tip.clicked {
            Some(TipAction::Learn) => shared.toast("Tooltip action: Learn more"),
            Some(TipAction::Archive) => shared.toast("Tooltip action: Archive now"),
            None => {}
        }
    });
}

fn alerts(ui: &mut MaterialUi<'_>, s: &mut State, shared: &mut Shared) {
    for (vi, (variant, vname)) in ALERT_VARIANTS.into_iter().enumerate() {
        sub(ui, vname);
        for (si, (sev, name)) in SEVERITIES.into_iter().enumerate() {
            Alert::new(("alert", vi, si), format!("{name} message in the {vname} variant."))
                .severity(sev)
                .variant(variant)
                .show(ui);
        }
    }
    sub(ui, "Title, actions and dismissible");
    for (i, (sev, name)) in SEVERITIES.into_iter().enumerate() {
        if s.dismissed[i] {
            continue;
        }
        let r = Alert::new(("alert-dis", i), "Details about what happened and what to do next.")
            .severity(sev)
            .title(name)
            .action("details", "Details")
            .action("ignore", "Ignore")
            .dismissible(true)
            .show(ui);
        if r.dismissed.is_some() {
            s.dismissed[i] = true;
        }
        if let Some(a) = r.clicked {
            shared.toast(format!("{name} alert action {a}"));
        }
    }
    if s.dismissed.iter().any(|d| *d)
        && ui
            .mineral(PushButton::new("alert-reset", "Restore alerts").variant(ButtonVariant::Text))
            .clicked()
    {
        s.dismissed = [false; 4];
    }
}

fn skeletons(ui: &mut MaterialUi<'_>) {
    sub(ui, "Text lines");
    let width = 480.0_f32.min(ui.available_width());
    ui.allocate_ui(vec2(width, 0.0), |ui| {
        ui.mineral(Skeleton::new(
            "skel-text",
            SkeletonShape::text(3, TypeScale::BodyMedium),
        ));
    });
    sub(ui, "Title scale text");
    let width = 480.0_f32.min(ui.available_width());
    ui.allocate_ui(vec2(width, 0.0), |ui| {
        ui.mineral(Skeleton::new(
            "skel-title",
            SkeletonShape::text(2, TypeScale::TitleLarge),
        ));
    });
    sub(ui, "Rect and circle");
    super::row(ui, |row| {
        row.add(|ui| ui.mineral(Skeleton::new("skel-circle", SkeletonShape::Circle(48.0))));
        row.add(|ui| ui.mineral(Skeleton::new("skel-rect", SkeletonShape::Rect(vec2(200.0, 96.0)))));
    });
}

fn tour(ui: &mut MaterialUi<'_>, s: &mut State, shared: &mut Shared) {
    ui.label("Each step highlights one of the widgets below.");
    if ui
        .mineral(PushButton::new("tour-start", "Start tour").icon(ICON_ADD))
        .clicked()
    {
        s.tour_step = Some(0);
    }
    ui.add_space(8.0);
    super::row(ui, |row| {
        s.tour_rects[0] = row
            .mineral(PushButton::new("tour-t0", "Create").variant(ButtonVariant::Filled))
            .rect;
        row.space(24.0);
        s.tour_rects[1] = row
            .mineral(IconButton::new("tour-t1", ICON_SETTINGS, "Settings").variant(IconButtonVariant::Tonal))
            .rect;
        row.space(24.0);
        s.tour_rects[2] = row.mineral(Fab::new("tour-t2", ICON_ADD, "Quick add")).rect;
    });
    ui.add_space(120.0);
    let r = Tour::new("tour", &mut s.tour_step)
        .step(
            TourStep::new(s.tour_rects[0], "Create")
                .supporting("Starts a new document.")
                .side(Side::Bottom),
        )
        .step(
            TourStep::new(s.tour_rects[1], "Settings")
                .supporting("Tweaks how the app behaves.")
                .side(Side::Bottom),
        )
        .step(
            TourStep::new(s.tour_rects[2], "Quick add")
                .supporting("A shortcut for the most common action.")
                .side(Side::Top),
        )
        .show_modal(ui.rt());
    if r.finished {
        shared.toast("Tour finished");
    }
    if r.dismissed.is_some() {
        shared.toast("Tour skipped");
    }
}
