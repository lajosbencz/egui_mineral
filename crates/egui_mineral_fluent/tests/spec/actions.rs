use egui_mineral_fluent::{
    FluentExt, Size,
    actions::{ButtonAppearance, CompoundButton, MenuButton, PushButton, SplitButton},
    icons::ICON_ADD,
    spec::ColorToken,
    spec::{
        defaults::{button, compound_button, split_button},
        tokens::{geometry, typography},
    },
};

use crate::support::{FluentHarness, harness, render};

// Border box: `strokeWidthThin` border, padding, line height.
fn height(pad: f32, line: f32) -> f32 {
    2.0 * geometry::STROKE_WIDTH_THIN + 2.0 * pad + line
}

#[test]
fn button_sizes_follow_the_styles() {
    let cases = [
        (
            Size::Small,
            button::SPACING_SMALL,
            typography::LINE_HEIGHT_BASE_200,
            button::MIN_WIDTH_SMALL,
        ),
        (
            Size::Medium,
            button::SPACING_MEDIUM,
            typography::LINE_HEIGHT_BASE_300,
            button::MIN_WIDTH,
        ),
        (
            Size::Large,
            button::SPACING_LARGE,
            typography::LINE_HEIGHT_BASE_400,
            button::MIN_WIDTH,
        ),
    ];
    for (size, pad, line, min_width) in cases {
        let h = harness((), move |ui, ()| {
            ui.mineral(PushButton::new("b", "Go").size(size));
        });
        let r = h.rect("b-button");
        assert_eq!(r.height(), height(pad, line), "{size:?} height");
        assert_eq!(r.width(), min_width, "{size:?} min width");
    }
}

#[test]
fn button_with_icon_keeps_the_height_of_its_size() {
    for (size, pad, icon) in [
        (Size::Small, button::SPACING_SMALL_WITH_ICON, button::ICON_SIZE),
        (Size::Large, button::SPACING_LARGE_WITH_ICON, button::ICON_SIZE_LARGE),
    ] {
        let h = harness((), move |ui, ()| {
            ui.mineral(PushButton::new("b", "Go").size(size).icon(ICON_ADD));
        });
        assert_eq!(h.rect("b-button").height(), height(pad, icon), "{size:?}");
    }
}

#[test]
fn icon_only_buttons_are_square() {
    for (size, edge) in [
        (Size::Small, button::ICON_ONLY_SMALL),
        (Size::Medium, button::ICON_ONLY_MEDIUM),
        (Size::Large, button::ICON_ONLY_LARGE),
    ] {
        let h = harness((), move |ui, ()| {
            ui.mineral(
                PushButton::new("b", "Add")
                    .icon(ICON_ADD)
                    .label_visible(false)
                    .size(size),
            );
        });
        let r = h.rect("b-button");
        assert_eq!((r.width(), r.height()), (edge, edge), "{size:?}");
    }
}

#[test]
fn primary_button_fills_with_the_brand_background() {
    let p = render(|ui| {
        ui.mineral(PushButton::new("b", "Go").appearance(ButtonAppearance::Primary));
    });
    let t = p.rt.tokens();
    let fill = p.one(t.color(ColorToken::ColorBrandBackground));
    assert_eq!(
        fill.height(),
        height(button::SPACING_MEDIUM, typography::LINE_HEIGHT_BASE_300)
    );
    assert_eq!(p.text_color("Go"), t.color(ColorToken::ColorNeutralForegroundOnBrand));
}

#[test]
fn compound_button_pads_the_label_and_secondary_column() {
    let h = harness((), |ui, ()| {
        ui.mineral(CompoundButton::new("c", "Title", "Secondary").icon(ICON_ADD));
    });
    let r = h.rect("c-button");
    let min = 2.0 * geometry::STROKE_WIDTH_THIN
        + compound_button::PADDING_TOP_MEDIUM
        + geometry::SPACING_HORIZONTAL_L
        + compound_button::ICON_SIZE;
    assert_eq!(r.height(), min, "the 40pt icon sets the content height");
    assert_eq!(h.node("c-button").value.as_deref(), Some("Secondary"));
}

#[test]
fn menu_button_reports_expanded() {
    let h = harness(true, |ui, open| {
        ui.mineral(MenuButton::new("m", "Menu").expanded(*open));
    });
    assert_eq!(h.node("m-button").state.expanded, Some(true));
}

#[test]
fn split_button_menu_part_is_the_minimum_target_wide() {
    let h = harness((), |ui, ()| {
        SplitButton::new("s", "Save").show(ui);
    });
    let lead = h.rect("s-button");
    let menu = h.rect("s-split-menu-button");
    assert_eq!(menu.width(), split_button::MIN_TARGET_SIZE);
    assert_eq!(menu.left(), lead.right(), "parts are joined");
    assert_eq!(menu.height(), lead.height());
}
