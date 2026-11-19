use egui_mineral_fluent::inputs::{Swatch, SwatchPicker, SwatchSize};
use egui_mineral_fluent::{
    ColorToken, FluentExt, Size,
    egui_mineral::range::RangeModel,
    inputs::{
        Checkbox, Combobox, Dropdown, Input, InputAppearance, ListOption, Rating, RatingSize, Slider, SpinButton,
        Switch, Textarea,
    },
    spec::{
        defaults::{checkbox, combobox, input, rating, slider, spin_button, swatch_picker, switch, textarea},
        tokens::geometry,
    },
};
use egui_mineral_fluent::{inputs::ColorPicker, spec::defaults::color_picker};

use egui_mineral_fluent::{
    egui_mineral::{date::Date, select::Selection, semantic::Role},
    inputs::{Calendar, DatePicker, TagPicker, TagPickerSize},
    spec::defaults::{calendar, tag_picker},
};

use crate::support::{FluentHarness, harness, render};

// Indicator with its `spacingVerticalS` / `spacingHorizontalS` margins.
fn cell(indicator: f32) -> f32 {
    indicator + 2.0 * geometry::SPACING_VERTICAL_S
}

#[test]
fn checkbox_cell_is_the_indicator_with_its_margins() {
    for (size, indicator) in [
        (Size::Medium, checkbox::INDICATOR_MEDIUM),
        (Size::Large, checkbox::INDICATOR_LARGE),
    ] {
        let h = harness(false, move |ui, on| {
            ui.mineral(Checkbox::new("c", "Check", on).label_visible(false).size(size));
        });
        let r = h.rect("c-checkbox");
        assert_eq!((r.width(), r.height()), (cell(indicator), cell(indicator)), "{size:?}");
    }
}

#[test]
fn checked_checkbox_fills_the_indicator_with_the_compound_brand_background() {
    let p = render(|ui| {
        let mut on = true;
        ui.mineral(Checkbox::new("c", "Check", &mut on));
    });
    let fill = p.one(p.rt.tokens().color(ColorToken::ColorCompoundBrandBackground));
    assert_eq!(
        (fill.width(), fill.height()),
        (checkbox::INDICATOR_MEDIUM, checkbox::INDICATOR_MEDIUM)
    );
}

#[test]
fn switch_cell_is_the_track_with_its_margins() {
    for (size, w, h_) in [
        (Size::Small, switch::TRACK_WIDTH_SMALL, switch::TRACK_HEIGHT_SMALL),
        (Size::Medium, switch::TRACK_WIDTH_MEDIUM, switch::TRACK_HEIGHT_MEDIUM),
    ] {
        let h = harness(false, move |ui, on| {
            ui.mineral(Switch::new("s", "Switch", on).label_visible(false).size(size));
        });
        let r = h.rect("s-switch");
        let margin = 2.0 * geometry::SPACING_HORIZONTAL_S;
        assert_eq!((r.width(), r.height()), (w + margin, h_ + margin), "{size:?}");
    }
}

#[test]
fn input_heights_follow_the_field_heights() {
    for (size, height) in [
        (Size::Small, input::HEIGHT_SMALL),
        (Size::Medium, input::HEIGHT_MEDIUM),
        (Size::Large, input::HEIGHT_LARGE),
    ] {
        let h = harness(String::new(), move |ui, text| {
            ui.mineral(Input::new("i", "Name", text).size(size));
        });
        assert_eq!(h.rect("i-input").height(), height, "{size:?}");
    }
}

#[test]
fn filled_darker_input_fills_with_background3() {
    let p = render(|ui| {
        let mut text = String::new();
        ui.mineral(Input::new("i", "Name", &mut text).appearance(InputAppearance::FilledDarker));
    });
    let fill = p.one(p.rt.tokens().color(ColorToken::ColorNeutralBackground3));
    assert_eq!(fill.height(), input::HEIGHT_MEDIUM);
}

#[test]
fn textarea_two_rows_reach_the_minimum_height_plus_underline_room() {
    let h = harness(String::new(), |ui, text| {
        ui.mineral(Textarea::new("t", "Notes", text));
    });
    let expected = textarea::MIN_HEIGHT_MEDIUM + textarea::FOCUS_UNDERLINE + 2.0 * geometry::STROKE_WIDTH_THIN;
    assert_eq!(h.rect("t-textarea").height(), expected);
}

#[test]
fn slider_cross_extent_follows_the_size() {
    for (size, cross) in [
        (Size::Small, slider::MIN_CROSS_SMALL),
        (Size::Medium, slider::MIN_CROSS_MEDIUM),
    ] {
        let h = harness(30.0, move |ui, v| {
            ui.mineral(Slider::new("s", "Volume", RangeModel::new(0.0, 100.0), v).size(size));
        });
        let r = h.rect("s-slider");
        assert_eq!(r.height(), cross, "{size:?}");
        assert!(r.width() >= slider::MIN_LENGTH);
    }
}

#[test]
fn spin_button_height_follows_the_field_heights() {
    for (size, height) in [
        (Size::Small, spin_button::HEIGHT_SMALL),
        (Size::Medium, spin_button::HEIGHT_MEDIUM),
    ] {
        let h = harness(1.0, move |ui, v| {
            ui.mineral(SpinButton::new("s", "Qty", RangeModel::new(0.0, 9.0), v).size(size));
        });
        assert_eq!(h.rect("s-spin-button").height(), height, "{size:?}");
    }
}

#[test]
fn rating_width_is_the_icon_count_times_the_size() {
    for (size, icon) in [
        (RatingSize::Small, rating::ITEM_SMALL),
        (RatingSize::ExtraLarge, rating::ITEM_EXTRA_LARGE),
    ] {
        let h = harness(2.0, move |ui, v| {
            ui.mineral(Rating::new("r", "Rating", v).size(size));
        });
        let r = h.rect("r-rating");
        assert_eq!((r.width(), r.height()), (5.0 * icon, icon), "{size:?}");
    }
}

#[test]
fn dropdown_and_combobox_share_the_input_heights() {
    for (size, height) in [
        (Size::Small, input::HEIGHT_SMALL),
        (Size::Medium, input::HEIGHT_MEDIUM),
        (Size::Large, input::HEIGHT_LARGE),
    ] {
        let h = harness(
            egui_mineral_fluent::egui_mineral::select::Selection::<String>::single(),
            move |ui, sel| {
                ui.mineral(Dropdown::new("d", "Pet", [ListOption::new("a", "A")], sel).size(size));
            },
        );
        assert_eq!(h.rect("d-dropdown").height(), height, "{size:?}");
        let c = harness(
            (
                egui_mineral_fluent::egui_mineral::select::Selection::<String>::single(),
                String::new(),
            ),
            move |ui, (sel, text)| {
                ui.mineral(Combobox::new("c", "Pet", [ListOption::new("a", "A")], sel, text).size(size));
            },
        );
        assert_eq!(c.rect("c-combobox").height(), height, "{size:?}");
    }
}

#[test]
fn listbox_options_are_a_text_line_with_snudge_padding() {
    let mut h = harness(
        egui_mineral_fluent::egui_mineral::select::Selection::<String>::single(),
        |ui, sel| {
            ui.mineral(Dropdown::new("d", "Pet", [ListOption::new("a", "A")], sel));
        },
    );
    h.run();
    h.click("d-dropdown");
    h.run();
    let option = h.rect("d-dropdown-0-option");
    let line = egui_mineral_fluent::spec::tokens::typography::LINE_HEIGHT_BASE_300;
    assert_eq!(option.height(), line + 2.0 * geometry::SPACING_HORIZONTAL_S_NUDGE);
    let field = h.rect("d-dropdown");
    assert!(option.width() >= combobox::LISTBOX_MIN_WIDTH.min(field.width()));
}

#[test]
fn swatches_are_squares_of_their_size_with_the_gap_between() {
    for (size, side) in [
        (SwatchSize::ExtraSmall, swatch_picker::SIZE_EXTRA_SMALL),
        (SwatchSize::Small, swatch_picker::SIZE_SMALL),
        (SwatchSize::Medium, swatch_picker::SIZE_MEDIUM),
        (SwatchSize::Large, swatch_picker::SIZE_LARGE),
    ] {
        let h = harness(None::<String>, move |ui, v| {
            SwatchPicker::new("s", "Color", v)
                .swatches([
                    Swatch::color("a", "A", egui::Color32::RED),
                    Swatch::color("b", "B", egui::Color32::BLUE),
                ])
                .size(size)
                .show(ui);
        });
        let (a, b) = (h.rect("s-0-swatch"), h.rect("s-1-swatch"));
        assert_eq!((a.width(), a.height()), (side, side), "{size:?}");
        assert_eq!(b.left() - a.right(), swatch_picker::SPACING_MEDIUM, "{size:?}");
    }
}

#[test]
fn color_picker_parts_are_the_area_and_slider_sizes_stacked_with_xs_gaps() {
    let h = harness(egui::ecolor::Hsva::new(0.5, 0.5, 0.5, 1.0), |ui, v| {
        ui.mineral(ColorPicker::new("c", "Color", v).alpha(true));
    });
    let (area, hue, alpha) = (h.rect("c-color-area"), h.rect("c-hue-slider"), h.rect("c-alpha-slider"));
    assert_eq!(
        (area.width(), area.height()),
        (color_picker::AREA_MIN, color_picker::AREA_MIN)
    );
    assert_eq!(hue.height(), color_picker::SLIDER_MIN_HEIGHT);
    assert_eq!(alpha.top() - hue.bottom(), geometry::SPACING_VERTICAL_XS);
}

fn animals() -> Vec<ListOption> {
    vec![ListOption::new("a", "Ant"), ListOption::new("b", "Bee")]
}

fn picker(size: TagPickerSize, selected: &[&str]) -> f32 {
    let sel: Selection<String> = Selection::multiple().with(selected.iter().map(|s| (*s).to_owned()));
    let h = harness((sel, String::new()), move |ui, (sel, text)| {
        ui.mineral(TagPicker::new("t", "Pets", animals(), sel, text).size(size));
    });
    h.rect("t-tag-picker").height()
}

#[test]
fn tag_picker_height_follows_the_min_height_until_tags_add_group_padding() {
    for (size, min) in [
        (TagPickerSize::Medium, tag_picker::MIN_HEIGHT_MEDIUM),
        (TagPickerSize::Large, tag_picker::MIN_HEIGHT_LARGE),
        (TagPickerSize::ExtraLarge, tag_picker::MIN_HEIGHT_EXTRA_LARGE),
    ] {
        assert_eq!(picker(size, &[]), min, "{size:?}");
    }
    // Borders, the medium group padding and an extra-small tag.
    let with_tag = 2.0 * geometry::STROKE_WIDTH_THIN
        + 2.0 * geometry::SPACING_VERTICAL_S_NUDGE
        + tag_picker::TAG_HEIGHT_EXTRA_SMALL;
    assert_eq!(picker(TagPickerSize::Medium, &["a"]), with_tag);
}

#[test]
fn tag_dismiss_hit_target_is_the_tag_height() {
    let h = harness(
        (Selection::<String>::multiple().with(["a".to_owned()]), String::new()),
        |ui, (sel, text)| {
            ui.mineral(TagPicker::new("t", "Pets", animals(), sel, text));
        },
    );
    assert_eq!(
        h.rect("t-tag-picker-0-dismiss").height(),
        tag_picker::TAG_HEIGHT_EXTRA_SMALL
    );
}

fn day(d: i8) -> Date {
    Date::new(2026, 11, d).expect("valid date")
}

#[test]
fn calendar_is_the_spec_width_with_six_week_rows_of_day_cells() {
    for (numbers, width) in [(false, calendar::WIDTH), (true, calendar::WIDTH_WEEK_NUMBERS)] {
        let h = harness(None::<Date>, move |ui, v| {
            ui.mineral(Calendar::new("c", "Day", v).today(day(18)).week_numbers(numbers));
        });
        assert_eq!(h.rect("c-calendar").width(), width, "{numbers}");
        let cells: Vec<_> = h
            .tree()
            .nodes
            .iter()
            .filter(|n| n.role == Role::GridCell)
            .map(|n| n.rect)
            .collect();
        assert_eq!(cells.len(), calendar::WEEKS * 7);
        assert!(
            cells
                .iter()
                .all(|r| r.width() == calendar::DAY_CELL && r.height() == calendar::DAY_CELL)
        );
    }
}

#[test]
fn date_picker_field_is_an_input_and_the_calendar_stays_closed() {
    let h = harness(None::<Date>, |ui, v| {
        ui.mineral(DatePicker::new("d", "Date", v));
    });
    assert_eq!(h.rect("d-date-picker").height(), input::HEIGHT_MEDIUM);
    assert!(h.tree().nodes.iter().all(|n| n.role != Role::GridCell));
}

#[test]
fn spin_button_paints_its_unit_after_the_value() {
    let p = render(|ui| {
        let mut v = 12.0;
        ui.mineral(SpinButton::new("s", "X", RangeModel::new(0.0, 99.0).step(1.0), &mut v).suffix("mm"));
    });
    let (unit, value) = (p.text("mm"), p.text("12"));
    assert!(unit.left() >= value.right(), "{unit:?} {value:?}");
    assert!(
        unit.left() - value.right() < 8.0,
        "unit hugs the value: {unit:?} {value:?}"
    );
}

#[test]
fn field_label_width_sets_the_horizontal_label_column() {
    use egui_mineral_fluent::{egui_mineral::layout::Axis, inputs::Field};
    let h = harness(String::new(), |ui, text| {
        Field::new("f")
            .label("Name")
            .axis(Axis::Horizontal)
            .label_width(100.0)
            .show(ui, |ui, _| ui.mineral(Input::new("f-in", "Name", text)));
    });
    assert_eq!(h.rect("f-in-input").left() - h.rect("f-label").left(), 100.0);
}
