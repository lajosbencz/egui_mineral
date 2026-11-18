use egui::vec2;
use egui_mineral_material::{
    Size,
    actions::{Fab, FabMenu, FabMenuItem, IconButton, IconButtonVariant, PushButton, SegmentedButton},
    communication::Badge,
    icons::{ICON_ADD, ICON_CHECK, ICON_MAIL},
    spec::{
        ColorRole, TypeScale,
        defaults::{button, segmented_button},
        tokens::{
            baseline_button, button_large, button_medium, button_small, button_x_large, button_x_small, fab_baseline,
            outlined_segmented_button as sb, small_icon_button,
        },
    },
};

use crate::support::{min_interactive, render};

#[test]
fn baseline_button_is_40dp_with_a_centered_line_box() {
    let p = render(|ui| {
        ui.mineral(PushButton::new("b", "Go"));
    });
    let c = p.one(ColorRole::Primary);
    assert_eq!(c.height(), baseline_button::CONTAINER_HEIGHT, "container height");
    assert_eq!(p.outer.height(), min_interactive(), "interactive minimum");
    let text = p.text("Go");
    assert_eq!(text.left() - c.left(), baseline_button::LEADING_SPACE, "leading space");
    assert_eq!(
        c.right() - text.right(),
        baseline_button::TRAILING_SPACE,
        "trailing space"
    );
    assert_eq!(
        text.height(),
        TypeScale::LabelLarge.line_height(),
        "galley is one line box"
    );
    assert_eq!(text.center().y, c.center().y, "line box centered");
}

#[test]
fn short_button_keeps_the_minimum_width() {
    let p = render(|ui| {
        ui.mineral(PushButton::new("b", ""));
    });
    assert_eq!(p.one(ColorRole::Primary).width(), button::MIN_WIDTH);
}

#[test]
fn sized_buttons_follow_their_tokens() {
    // Wider than every size's minimum width, so the paddings show.
    const LABEL: &str = "Wide label";
    let sizes = [
        // Compose pads XSmall with `ButtonDefaults.ExtraSmallContentPadding`, not its tokens.
        (
            Size::XSmall,
            button_x_small::CONTAINER_HEIGHT,
            button::X_SMALL_SPACE,
            button::X_SMALL_SPACE,
        ),
        (
            Size::Small,
            button_small::CONTAINER_HEIGHT,
            button_small::LEADING_SPACE,
            button_small::TRAILING_SPACE,
        ),
        (
            Size::Medium,
            button_medium::CONTAINER_HEIGHT,
            button_medium::LEADING_SPACE,
            button_medium::TRAILING_SPACE,
        ),
        (
            Size::Large,
            button_large::CONTAINER_HEIGHT,
            button_large::LEADING_SPACE,
            button_large::TRAILING_SPACE,
        ),
        (
            Size::XLarge,
            button_x_large::CONTAINER_HEIGHT,
            button_x_large::LEADING_SPACE,
            button_x_large::TRAILING_SPACE,
        ),
    ];
    for (size, height, leading, trailing) in sizes {
        let p = render(|ui| {
            ui.mineral(PushButton::new("b", LABEL).size(size));
        });
        let c = p.one(ColorRole::Primary);
        let text = p.text(LABEL);
        assert_eq!(c.height(), height, "{size:?} height");
        assert_eq!(text.center().y, c.center().y, "{size:?} label centered");
        assert_eq!(text.left() - c.left(), leading, "{size:?} leading space");
        assert_eq!(c.right() - text.right(), trailing, "{size:?} trailing space");
    }
}

#[test]
fn icon_button_container_sits_in_the_interactive_minimum() {
    let p = render(|ui| {
        ui.mineral(IconButton::new("m", ICON_MAIL, "Mail").variant(IconButtonVariant::Filled));
    });
    let width = small_icon_button::DEFAULT_LEADING_SPACE
        + small_icon_button::ICON_SIZE
        + small_icon_button::DEFAULT_TRAILING_SPACE;
    let c = p.one(ColorRole::Primary);
    assert_eq!(c.size(), vec2(width, small_icon_button::CONTAINER_HEIGHT), "container");
    assert_eq!(
        p.outer.size(),
        egui::Vec2::splat(min_interactive()),
        "interactive minimum"
    );
    assert_eq!(c.center(), p.outer.center(), "centered in its target");
    assert_eq!(p.rect("m-button"), p.outer, "the whole target is hit");
}

#[test]
fn icon_button_badge_overflows_without_layout() {
    let mail = |badge: Option<Badge>| {
        render(|ui| {
            let mut b = IconButton::new("mail", ICON_MAIL, "Mail");
            if let Some(badge) = badge {
                b = b.badge(badge);
            }
            ui.mineral(b);
        })
    };
    let p = mail(Some(Badge::count(1234).max(999)));
    assert_eq!(p.outer, mail(None).outer);
}

#[test]
fn fab_is_56dp() {
    let p = render(|ui| {
        ui.mineral(Fab::new("f", ICON_MAIL, "Mail"));
    });
    assert_eq!(
        p.one(ColorRole::PrimaryContainer).size(),
        vec2(fab_baseline::CONTAINER_WIDTH, fab_baseline::CONTAINER_HEIGHT)
    );
}

#[test]
fn fab_menu_button_keeps_the_menu_padding() {
    use egui_mineral_material::spec::defaults::fab_menu;
    let p = render(|ui| {
        FabMenu::new("m", ICON_ADD, "Actions")
            .item(FabMenuItem::new(ICON_MAIL, "Mail"))
            .show(ui);
    });
    let fab = p.one(ColorRole::PrimaryContainer);
    let side = fab_baseline::CONTAINER_HEIGHT;
    assert_eq!(fab.size(), vec2(side, side), "closed toggle is the FAB");
    assert_eq!(
        fab.left() - p.outer.left(),
        fab_menu::PADDING_HORIZONTAL,
        "start padding"
    );
    assert_eq!(
        p.outer.right() - fab.right(),
        fab_menu::PADDING_HORIZONTAL,
        "end padding"
    );
    assert_eq!(
        p.outer.bottom() - fab.bottom(),
        fab_menu::BUTTON_PADDING_BOTTOM,
        "bottom padding"
    );
}

#[test]
fn segmented_button_joins_equal_outlined_segments() {
    let p = render(|ui| {
        let mut selected = 1;
        SegmentedButton::single("seg", "Period", &mut selected)
            .items(["Day", "Week", "Month"])
            .show(ui);
    });
    let segments = p.stroked(ColorRole::Outline);
    assert_eq!(segments.len(), 3, "one outline per segment");
    let (first, selected, last) = (segments[0], segments[1], segments[2]);
    let month = p.text("Month").width();
    let natural = 2.0 * segmented_button::CONTENT_SPACE + sb::ICON_SIZE + segmented_button::ICON_SPACING + month;
    for s in &segments {
        assert_eq!(s.size(), vec2(natural.max(button::MIN_WIDTH), sb::CONTAINER_HEIGHT));
    }
    assert_eq!(
        first.right() - selected.left(),
        sb::OUTLINE_WIDTH,
        "neighbours share the outline"
    );
    assert_eq!(
        selected.right() - last.left(),
        sb::OUTLINE_WIDTH,
        "neighbours share the outline"
    );
    assert_eq!(
        p.one(ColorRole::SecondaryContainer),
        selected,
        "the selected segment is filled"
    );

    let slot = sb::ICON_SIZE + segmented_button::ICON_SPACING;
    let shift = |label: &str, segment: egui::Rect| p.text(label).center().x - segment.center().x;
    assert!(shift("Day", first).abs() < 0.5, "unselected label is centered");
    assert!(
        (shift("Week", selected) - slot / 2.0).abs() < 0.5,
        "the check pushes the label"
    );
    let check = p.text(<&str>::from(ICON_CHECK));
    assert!(check.right() <= p.text("Week").left(), "check before the label");
    assert!(selected.contains_rect(check), "check inside the selected segment");
}
