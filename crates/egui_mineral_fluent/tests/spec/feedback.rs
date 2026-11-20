use egui_mineral_fluent::{
    FluentExt,
    feedback::{
        Badge, BadgeSize, CounterBadge, MessageBar, MessageBarIntent, MessageBarLayout, PresenceBadge, PresenceStatus,
        ProgressBar, ProgressThickness, Skeleton, SkeletonItem, SkeletonShape, Spinner, SpinnerLabelPosition,
        SpinnerSize, Toast, Toaster, toast_queue,
    },
    inputs::Validation,
    spec::ColorToken,
    spec::defaults::{badge, counter_badge, message_bar, presence_badge, progress_bar, skeleton, spinner, toast},
};

use crate::support::{FluentHarness, SCREEN, harness, render};

#[test]
fn determinate_bar_fills_its_fraction_of_the_track() {
    let p = render(|ui| {
        ui.mineral(ProgressBar::determinate("p", 0.4));
    });
    let t = p.rt.tokens();
    let track = p.one(t.color(ColorToken::ColorNeutralBackground6));
    let fill = p.one(t.color(ColorToken::ColorCompoundBrandBackground));
    assert_eq!(track.width(), SCREEN.x);
    assert_eq!(track.height(), progress_bar::THICKNESS_MEDIUM);
    assert!((fill.width() - SCREEN.x * 0.4).abs() < 0.5, "{fill:?}");
}

#[test]
fn bar_thickness_and_validation_color() {
    let p = render(|ui| {
        ui.mineral(
            ProgressBar::determinate("p", 1.0)
                .thickness(ProgressThickness::Large)
                .validation(Validation::Error),
        );
    });
    let fill = p.one(p.rt.tokens().color(ColorToken::ColorPaletteRedBackground3));
    assert_eq!(fill.height(), progress_bar::THICKNESS_LARGE);
}

#[test]
fn indeterminate_bar_without_motion_spans_the_track() {
    let p = render(|ui| {
        ui.mineral(ProgressBar::indeterminate("p"));
    });
    let fill = p.one(p.rt.tokens().color(ColorToken::ColorCompoundBrandBackground));
    assert_eq!(fill.width(), SCREEN.x);
}

#[test]
fn spinner_is_a_square_of_its_size() {
    for (size, ring) in [
        (SpinnerSize::ExtraTiny, spinner::EXTRA_TINY),
        (SpinnerSize::Medium, spinner::MEDIUM),
        (SpinnerSize::Huge, spinner::HUGE),
    ] {
        let h = harness((), move |ui, ()| {
            ui.mineral(Spinner::new("s").size(size));
        });
        let r = h.rect("s-spinner");
        assert_eq!((r.width(), r.height()), (ring, ring), "{size:?}");
    }
}

#[test]
fn spinner_label_sits_beside_or_above_the_ring() {
    let beside = harness((), |ui, ()| {
        ui.mineral(Spinner::new("s").label("Wait"));
    });
    let above = harness((), |ui, ()| {
        ui.mineral(
            Spinner::new("s")
                .label("Wait")
                .label_position(SpinnerLabelPosition::Above),
        );
    });
    let text = render(|ui| {
        ui.mineral(Spinner::new("s").label("Wait"));
    })
    .text("Wait");
    let (b, a) = (beside.rect("s-spinner"), above.rect("s-spinner"));
    assert_eq!(b.width(), spinner::MEDIUM + spinner::GAP + text.width());
    assert_eq!(b.height(), spinner::MEDIUM.max(text.height()));
    assert_eq!(a.height(), spinner::MEDIUM + spinner::GAP + text.height());
}

#[test]
fn skeleton_items_take_their_shape_and_size() {
    let h = harness((), |ui, ()| {
        Skeleton::new("sk").show(ui, |ui, cx| {
            ui.mineral(cx.item("r").size(12));
            ui.mineral(SkeletonItem::new("c").shape(SkeletonShape::Circle).size(40));
        });
    });
    h.node("sk-skeleton");
    let p = render(|ui| {
        ui.mineral(SkeletonItem::new("r").size(13));
    });
    let fill = p.one(p.rt.tokens().color(ColorToken::ColorNeutralStencil1));
    assert_eq!(fill.height(), 12.0, "snaps to the nearest of {:?}", skeleton::SIZES);
    assert_eq!(fill.width(), SCREEN.x);
}

#[test]
fn message_bar_has_its_minimum_height_and_intent_fill() {
    let p = render(|ui| {
        MessageBar::new("m", "Body").intent(MessageBarIntent::Error).show(ui);
    });
    let fill = p.one(p.rt.tokens().color(ColorToken::ColorStatusDangerBackground1));
    assert_eq!(fill.height(), message_bar::MIN_HEIGHT);
    assert_eq!(fill.width(), SCREEN.x);
}

#[test]
fn multiline_message_bar_grows_with_wrapped_text() {
    let long = "word ".repeat(80);
    let p = render(|ui| {
        MessageBar::new("m", long.clone())
            .layout(MessageBarLayout::Multiline)
            .show(ui);
    });
    let fill = p.one(p.rt.tokens().color(ColorToken::ColorNeutralBackground3));
    assert!(fill.height() > message_bar::MIN_HEIGHT, "{fill:?}");
}

#[test]
fn badge_sizes() {
    for (size, h) in [
        (BadgeSize::Tiny, badge::TINY),
        (BadgeSize::ExtraSmall, badge::EXTRA_SMALL),
        (BadgeSize::Small, badge::SMALL),
        (BadgeSize::Medium, badge::MEDIUM),
        (BadgeSize::Large, badge::LARGE),
        (BadgeSize::ExtraLarge, badge::EXTRA_LARGE),
    ] {
        let p = render(|ui| {
            ui.mineral(Badge::new("b", "").size(size));
        });
        let fill = p.one(p.rt.tokens().color(ColorToken::ColorBrandBackground));
        assert_eq!((fill.height(), fill.width()), (h, h), "{size:?}");
    }
}

#[test]
fn counter_badge_caps_at_the_overflow_count_and_hides_zero() {
    let p = render(|ui| {
        ui.mineral(CounterBadge::new("c", 120));
    });
    p.text("99+");
    let h = harness((), |ui, ()| {
        ui.mineral(CounterBadge::new("z", 0));
        ui.mineral(CounterBadge::new("d", 0).dot(true));
    });
    assert!(h.tree().find_key("z-counter-badge").is_none());
    assert_eq!(h.node("d-counter-badge").rect.width(), counter_badge::DOT);
}

#[test]
fn presence_badge_is_a_square_of_its_size() {
    let h = harness((), |ui, ()| {
        ui.mineral(PresenceBadge::new("p", PresenceStatus::Busy).size(BadgeSize::Large));
    });
    let r = h.rect("p-presence-badge");
    assert_eq!((r.width(), r.height()), (presence_badge::LARGE, presence_badge::LARGE));
}

#[test]
fn toast_is_as_wide_as_the_toaster_and_shows_its_text() {
    let mut queue = toast_queue::<egui_mineral::action::NoAction>();
    queue.push(Toast::new("Saved").body("All changes saved."));
    let p = render(|ui| {
        Toaster::new(&mut queue).show(ui.rt());
    });
    p.text("Saved");
    p.text("All changes saved.");
    let fill = p.one(p.rt.tokens().color(ColorToken::ColorNeutralBackground1));
    assert_eq!(fill.width(), toast::WIDTH);
}
