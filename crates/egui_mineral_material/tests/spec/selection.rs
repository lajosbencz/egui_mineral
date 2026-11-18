use egui::{Vec2, vec2};
use egui_mineral::{
    date::{Date, DateRange, Time},
    range::RangeModel,
    widget::SizeExt,
};
use egui_mineral_material::{
    selection::{CheckButton, DatePicker, DateRangePicker, DisplayMode, Switch, TimePicker, ValueSlider},
    spec::{
        defaults::{date_picker, time_picker},
        tokens::{date_picker_modal, slider, switch, time_input, time_picker as time_tokens},
    },
};

use crate::support::{WIDE, min_interactive, render, render_on};

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

#[test]
fn unlabeled_checkbox_and_switch_fill_the_interactive_minimum() {
    let mut on = false;
    let p = render(|ui| {
        ui.mineral(CheckButton::new("cb", "Check", &mut on).label_visible(false));
    });
    assert_eq!(p.outer.size(), Vec2::splat(min_interactive()), "checkbox target");
    assert_eq!(p.rect("cb-checkbox"), p.outer, "the whole target is hit");
    let mut on = true;
    let p = render(|ui| {
        ui.mineral(Switch::new("sw", "Switch", &mut on).label_visible(false));
    });
    assert_eq!(
        p.outer.size(),
        vec2(switch::TRACK_WIDTH, min_interactive()),
        "switch target"
    );
    let track = p.filled(switch::SELECTED_TRACK_COLOR);
    assert!(
        track
            .iter()
            .any(|r| r.size() == vec2(switch::TRACK_WIDTH, switch::TRACK_HEIGHT)),
        "track in {track:?}"
    );
}

#[test]
fn slider_defaults_to_the_xs_track_and_a_4dp_handle() {
    let mut v = 50.0;
    let p = render(|ui| {
        ui.mineral(ValueSlider::new("sl", "Slider", RangeModel::new(0.0, 100.0), &mut v).fixed_width(300.0));
    });
    assert_eq!(p.outer.height(), min_interactive(), "interactive minimum");
    let primary = p.filled(slider::ACTIVE_TRACK_COLOR);
    let handle = vec2(slider::HANDLE_WIDTH, slider::HANDLE_HEIGHT);
    assert!(primary.iter().any(|r| r.size() == handle), "handle in {primary:?}");
    let inactive = p.filled(slider::INACTIVE_TRACK_COLOR);
    assert!(
        inactive.iter().all(|r| r.height() == slider::INACTIVE_TRACK_HEIGHT),
        "inactive track height"
    );
    // Gap: half the handle plus `ActiveHandleLeadingSpace` on each side.
    let gap = slider::HANDLE_WIDTH / 2.0 + slider::ACTIVE_HANDLE_LEADING_SPACE;
    let active = primary
        .iter()
        .find(|r| r.height() == slider::ACTIVE_TRACK_HEIGHT)
        .expect("active track");
    assert_eq!(inactive[0].left() - active.right(), 2.0 * gap, "thumb gap");
}

#[test]
fn date_picker_day_grid() {
    let mut day = Some(Date::constant(2024, 5, 15));
    let p = render_on(WIDE, |ui| {
        DatePicker::new("dp", &mut day).show(ui);
    });
    let cell = p.rect("dp-2024-05-15-cell");
    assert_eq!(cell.size(), Vec2::splat(date_picker::CELL_SIZE));
    let selected = p.circles(date_picker_modal::DATE_SELECTED_CONTAINER_COLOR);
    assert!(
        selected
            .iter()
            .any(|&(c, d)| c == cell.center() && near(d, date_picker_modal::DATE_CONTAINER_WIDTH)),
        "selected day container centered in its cell"
    );
    assert_eq!(
        p.rect("dp-grid").width(),
        date_picker_modal::CONTAINER_WIDTH - 2.0 * date_picker::HORIZONTAL_PADDING
    );
}

#[test]
fn date_picker_modal_width() {
    let mut open = true;
    let mut day = None;
    let p = render_on(WIDE, |ui| {
        DatePicker::new("dm", &mut day).show_modal(ui.rt(), &mut open);
    });
    let dialog = p.rect("dm-dialog");
    assert_eq!(dialog.width(), date_picker_modal::CONTAINER_WIDTH);
    let ok = p.rect("dm-confirm-button");
    assert_eq!(
        dialog.bottom() - ok.bottom(),
        date_picker::DIALOG_BUTTONS_BOTTOM_PADDING,
        "buttons below the calendar"
    );
    assert_eq!(dialog.right() - ok.right(), date_picker::DIALOG_BUTTONS_END_PADDING);
}

#[test]
fn date_range_months_list() {
    let dp = date_picker::CELL_SIZE;
    let mut open = true;
    let mut range = DateRange::new(Date::constant(2024, 5, 10), Date::constant(2024, 5, 20));
    let p = render_on(WIDE, |ui| {
        DateRangePicker::new("dr", &mut range).show_modal(ui.rt(), &mut open);
    });
    assert_eq!(
        p.rect("dr-dialog").height(),
        date_picker_modal::CONTAINER_HEIGHT,
        "the months list fills the dialog"
    );
    let list = p.rect("dr-grid");
    let may = p.rect("dr-2024-05-01-cell");
    let subhead = date_picker::MONTH_SUBHEAD_TOP_PADDING
        + date_picker_modal::RANGE_SELECTION_MONTH_SUBHEAD_FONT.line_height()
        + date_picker::MONTH_SUBHEAD_BOTTOM_PADDING;
    assert_eq!(may.top() - list.top(), subhead, "scrolled to the start month");
    let june = p.rect("dr-2024-06-01-cell");
    assert_eq!(
        june.top() - may.top(),
        subhead + dp * date_picker::MAX_CALENDAR_ROWS as f32,
        "every month is six rows high"
    );
    let p = render_on(WIDE, |ui| {
        DateRangePicker::new("di", &mut range).show(ui);
    });
    let mid = p.rect("di-2024-05-15-cell");
    let strip = date_picker_modal::RANGE_SELECTION_ACTIVE_INDICATOR_CONTAINER_COLOR;
    assert!(
        p.filled(strip)
            .iter()
            .any(|r| r.center() == mid.center() && r.size() == vec2(dp, date_picker_modal::DATE_STATE_LAYER_HEIGHT)),
        "range strip spans the in-range cell"
    );
}

#[test]
fn time_picker_geometry() {
    let mut time = Time::constant(9, 41, 0, 0);
    let p = render_on(WIDE, |ui| {
        TimePicker::new("tp", &mut time).show(ui);
    });
    let hour = p.rect("tp-hour-selector");
    assert_eq!(
        hour.size(),
        vec2(
            time_tokens::TIME_SELECTOR_CONTAINER_WIDTH,
            time_tokens::TIME_SELECTOR_CONTAINER_HEIGHT
        )
    );
    assert_eq!(
        p.rect("tp-minute-selector").left() - hour.right(),
        time_picker::SEPARATOR_WIDTH
    );
    assert!(
        p.tree.find_key("tp-mode-button").is_none(),
        "the toggle is the dialog's"
    );
    let dial = p.rect("tp-hour-dial");
    assert_eq!(dial.size(), Vec2::splat(time_tokens::CLOCK_DIAL_CONTAINER_SIZE));
    assert_eq!(dial.top() - hour.bottom(), time_picker::CLOCK_DISPLAY_MARGIN);
    let handle = time_tokens::CLOCK_DIAL_SELECTOR_HANDLE_CONTAINER_SIZE;
    assert!(
        p.circles(time_tokens::CLOCK_DIAL_SELECTOR_HANDLE_CONTAINER_COLOR)
            .iter()
            .any(|&(c, d)| near(d, handle) && near(c.distance(dial.center()), time_picker::OUTER_RADIUS)),
        "selector handle on the outer ring"
    );
}

#[test]
fn time_input_fields() {
    let mut time = Time::constant(9, 41, 0, 0);
    let p = render_on(WIDE, |ui| {
        TimePicker::new("ti", &mut time).input_mode(DisplayMode::Input).show(ui);
    });
    let (hour, minute) = (p.rect("ti-hour-field"), p.rect("ti-minute-field"));
    assert_eq!(
        hour.size(),
        vec2(
            time_input::TIME_FIELD_CONTAINER_WIDTH,
            time_input::TIME_FIELD_CONTAINER_HEIGHT
        )
    );
    assert_eq!(minute.left() - hour.right(), time_picker::SEPARATOR_WIDTH);
}
