use egui_mineral_fluent::{
    IconPosition,
    actions::PushButton,
    egui_mineral::toast::ToastQueue,
    feedback::{
        Badge, BadgeAppearance, BadgeColor, BadgeShape, BadgeSize, CounterBadge, CounterBadgeAppearance,
        CounterBadgeColor, MessageBar, MessageBarGroup, MessageBarIntent, MessageBarLayout, MessageBarShape,
        PresenceBadge, PresenceStatus, ProgressBar, ProgressShape, ProgressThickness, Skeleton, SkeletonAnimation,
        SkeletonAppearance, SkeletonItem, SkeletonShape, Spinner, SpinnerAppearance, SpinnerLabelPosition, SpinnerSize,
        Toast, ToastIntent, ToastLink, ToastPosition, Toaster, toast_queue,
    },
    icons::ICON_CHECKMARK_CIRCLE,
    inputs::Validation,
};

use super::{heading, row};
use crate::FluentUi;

#[derive(Debug)]
pub(crate) struct State {
    dismissed: [bool; 4],
    queue: ToastQueue<Toast>,
    pushed: u32,
}

impl Default for State {
    fn default() -> Self {
        Self {
            dismissed: [false; 4],
            queue: toast_queue(),
            pushed: 0,
        }
    }
}

pub(crate) fn show(ui: &mut FluentUi<'_>, s: &mut State) {
    progress(ui);
    spinners(ui);
    skeletons(ui);
    message_bars(ui, s);
    badges(ui);
    toasts(ui, s);
}

fn progress(ui: &mut FluentUi<'_>) {
    heading(ui, "h-progress", "Progress bars");
    ui.mineral(ProgressBar::determinate("progress-value", 0.4));
    ui.mineral(ProgressBar::indeterminate("progress-indeterminate"));
    ui.mineral(ProgressBar::determinate("progress-large", 0.6).thickness(ProgressThickness::Large));
    ui.mineral(ProgressBar::determinate("progress-square", 0.6).shape(ProgressShape::Square));
    for (i, v) in [Validation::Error, Validation::Warning, Validation::Success]
        .into_iter()
        .enumerate()
    {
        ui.mineral(ProgressBar::determinate(("progress-validation", i), 0.7).validation(v));
    }
}

fn spinners(ui: &mut FluentUi<'_>) {
    heading(ui, "h-spinner", "Spinners");
    let rt = ui.rt();
    row(ui, |ui| {
        for (i, size) in SpinnerSize::ALL.iter().copied().enumerate() {
            ui.mineral(Spinner::new(("spinner-size", i)).size(size));
        }
    });
    row(ui, |ui| {
        for (i, position) in SpinnerLabelPosition::ALL.iter().copied().enumerate() {
            ui.mineral(
                Spinner::new(("spinner-label", i))
                    .label(format!("{position:?}"))
                    .label_position(position),
            );
        }
    });
    egui::Frame::NONE
        .fill(egui::Color32::from_rgb(0x0f, 0x6c, 0xbd))
        .inner_margin(8)
        .show(ui, |ui| {
            rt.ui(ui).mineral(
                Spinner::new("spinner-inverted")
                    .label("Inverted")
                    .appearance(SpinnerAppearance::Inverted),
            );
        });
}

fn skeletons(ui: &mut FluentUi<'_>) {
    heading(ui, "h-skeleton", "Skeletons");
    for (i, (animation, appearance)) in [
        (SkeletonAnimation::Wave, SkeletonAppearance::Opaque),
        (SkeletonAnimation::Pulse, SkeletonAppearance::Opaque),
        (SkeletonAnimation::Wave, SkeletonAppearance::Translucent),
    ]
    .into_iter()
    .enumerate()
    {
        Skeleton::new(("skeleton", i))
            .animation(animation)
            .appearance(appearance)
            .show(ui, |ui, cx| {
                row(ui, |ui| {
                    ui.mineral(cx.item(("circle", i)).shape(SkeletonShape::Circle).size(32));
                    ui.mineral(cx.item(("square", i)).shape(SkeletonShape::Square).size(32));
                });
                ui.mineral(cx.item(("line", i)).size(12));
                ui.mineral(cx.item(("line-2", i)).size(12));
            });
    }
    ui.mineral(SkeletonItem::new("standalone").size(20));
}

fn message_bars(ui: &mut FluentUi<'_>, s: &mut State) {
    heading(ui, "h-message-bar", "Message bars");
    for (i, intent) in MessageBarIntent::ALL.iter().copied().enumerate() {
        let bar = MessageBar::new(("bar", i), "Something happened.")
            .title(format!("{intent:?}"))
            .intent(intent);
        bar.show(ui);
    }
    MessageBar::new("bar-square", "A square bar.")
        .shape(MessageBarShape::Square)
        .show(ui);
    for (i, intent) in [MessageBarIntent::Warning, MessageBarIntent::Error]
        .into_iter()
        .enumerate()
    {
        let r = MessageBar::new(
            ("bar-multiline", i),
            "A longer message that wraps over several lines when the available width is small.",
        )
        .title("Multiline")
        .intent(intent)
        .layout(MessageBarLayout::Multiline)
        .dismissible(true)
        .show_with(ui, |ui| {
            ui.mineral(PushButton::new(("bar-action", i), "Action"));
        });
        s.dismissed[i] |= r.dismiss_clicked;
    }
    let group = MessageBarGroup::new("group");
    group.show(ui, |ui| {
        for (i, dismissed) in s.dismissed.iter_mut().enumerate().skip(2) {
            if !*dismissed {
                let r = MessageBar::new(("bar-group", i), "Grouped and dismissible.")
                    .intent(MessageBarIntent::Success)
                    .dismissible(true)
                    .show(ui);
                *dismissed = r.dismiss_clicked;
            }
        }
    });
}

fn badges(ui: &mut FluentUi<'_>) {
    heading(ui, "h-badge", "Badges");
    row(ui, |ui| {
        for (i, size) in BadgeSize::ALL.iter().copied().enumerate() {
            ui.mineral(Badge::new(("badge-size", i), "1").size(size));
        }
        for (i, shape) in BadgeShape::ALL.iter().copied().enumerate() {
            ui.mineral(Badge::new(("badge-shape", i), "New").shape(shape));
        }
        ui.mineral(Badge::new("badge-icon", "Done").icon(ICON_CHECKMARK_CIRCLE));
        ui.mineral(
            Badge::new("badge-icon-after", "Done")
                .icon(ICON_CHECKMARK_CIRCLE)
                .icon_position(IconPosition::After),
        );
    });
    for (a, appearance) in BadgeAppearance::ALL.iter().copied().enumerate() {
        row(ui, |ui| {
            for (c, color) in BadgeColor::ALL.iter().copied().enumerate() {
                ui.mineral(
                    Badge::new(("badge", a, c), format!("{color:?}"))
                        .appearance(appearance)
                        .color(color),
                );
            }
        });
    }
    row(ui, |ui| {
        ui.mineral(CounterBadge::new("counter", 5));
        ui.mineral(CounterBadge::new("counter-overflow", 120));
        ui.mineral(CounterBadge::new("counter-zero", 0).zero_visible(true));
        ui.mineral(CounterBadge::new("counter-dot", 1).dot(true));
        for (i, color) in CounterBadgeColor::ALL.iter().copied().enumerate() {
            ui.mineral(CounterBadge::new(("counter-color", i), 7).color(color));
        }
        ui.mineral(CounterBadge::new("counter-ghost", 7).appearance(CounterBadgeAppearance::Ghost));
    });
    for (o, out_of_office) in [false, true].into_iter().enumerate() {
        row(ui, |ui| {
            for (i, status) in PresenceStatus::ALL.iter().copied().enumerate() {
                ui.mineral(
                    PresenceBadge::new(("presence", o, i), status)
                        .out_of_office(out_of_office)
                        .size(BadgeSize::Large),
                );
            }
        });
    }
    row(ui, |ui| {
        for (i, size) in BadgeSize::ALL.iter().copied().enumerate() {
            ui.mineral(PresenceBadge::new(("presence-size", i), PresenceStatus::Available).size(size));
        }
    });
}

fn toasts(ui: &mut FluentUi<'_>, s: &mut State) {
    heading(ui, "h-toast", "Toasts");
    let rt = ui.rt();
    row(ui, |ui| {
        for (i, intent) in ToastIntent::ALL.iter().copied().enumerate() {
            if ui
                .mineral(PushButton::new(("toast-push", i), format!("{intent:?} toast")))
                .response()
                .clicked()
            {
                s.pushed += 1;
                let position = ToastPosition::ALL[usize::try_from(s.pushed).unwrap_or(0) % ToastPosition::ALL.len()];
                s.queue.push(
                    Toast::new(format!("Toast {}", s.pushed))
                        .body("Something happened.")
                        .intent(intent)
                        .position(position)
                        .footer_action(ToastLink::new("Undo").key("undo"))
                        .dismissible(true),
                );
            }
        }
    });
    Toaster::new(&mut s.queue).pause_on_hover(true).show(rt);
}
