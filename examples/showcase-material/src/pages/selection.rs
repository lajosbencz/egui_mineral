use crate::Mui;
use egui::{Align, Color32, Layout, vec2};
use egui_mineral::i18n::HourCycle;
use egui_mineral_material::{
    catalog::Component,
    icons::{
        ICON_EVENT, ICON_FAVORITE, ICON_LOCAL_CAFE, ICON_MUSIC_NOTE, ICON_PUBLIC, ICON_SEARCH, ICON_STAR,
        ICON_VOLUME_DOWN, ICON_VOLUME_UP, ICON_WIFI,
    },
    prelude::*,
};
use std::num::NonZero;

use super::{Shared, caption};

pub(crate) const TOPICS: &[&str] = &[
    "Checkbox",
    "Radio",
    "Switch",
    "Slider",
    "Chip",
    "Rating",
    "Select",
    "Combobox",
    "Transfer list",
    "Color picker",
    "Date picker",
    "Time picker",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Portion {
    Small,
    Medium,
    Large,
}

pub(crate) struct State {
    check: bool,
    check_off: bool,
    check_long: bool,
    check_err: bool,
    children: [bool; 3],
    radio: Portion,
    radio_row: Portion,
    radio_single: bool,
    sw: bool,
    sw_icons: bool,
    slider: f64,
    slider_step: f64,
    slider_ticks: f64,
    slider_range: [f64; 2],
    slider_range_step: [f64; 2],
    slider_center: f64,
    slider_log: f64,
    slider_vert: f64,
    slider_icons: f64,
    slider_quiet: f64,
    slider_sizes: [f64; 5],
    chip_filters: [bool; 3],
    chip_elev_filter: bool,
    chip_inputs: Vec<String>,
    chip_input_selected: Option<usize>,
    chip_assist: u32,
    rating: f64,
    rating_half: f64,
    rating_hearts: f64,
    rating_clear: f64,
    rating_ro: f64,
    select_single: Option<usize>,
    select_color: Option<usize>,
    select_codec: Option<usize>,
    select_outlined: Option<usize>,
    select_err: Option<usize>,
    select_multi: Selection<usize>,
    combo_text: String,
    combo_free: String,
    combo_loading: String,
    combo_created: Vec<String>,
    transfer: Vec<usize>,
    color: Color32,
    color_hct: Color32,
    date_docked: Option<Date>,
    date_bounded: Option<Date>,
    date_modal: Option<Date>,
    date_modal_open: bool,
    date_input: Option<Date>,
    date_input_open: bool,
    date_range: DateRange,
    date_range_open: bool,
    date_inline: Option<Date>,
    date_inline_range: DateRange,
    time_inline: Time,
    time_24: Time,
    time_h: Time,
    time_input: Time,
    time_modal: Time,
    time_modal_open: bool,
}

impl Default for State {
    fn default() -> Self {
        Self {
            check: true,
            check_off: false,
            check_long: false,
            check_err: false,
            children: [true, false, false],
            radio: Portion::Medium,
            radio_row: Portion::Small,
            radio_single: true,
            sw: true,
            sw_icons: true,
            slider: 40.0,
            slider_step: 30.0,
            slider_ticks: 50.0,
            slider_range: [20.0, 70.0],
            slider_range_step: [20.0, 60.0],
            slider_center: 0.0,
            slider_log: 1000.0,
            slider_vert: 60.0,
            slider_icons: 30.0,
            slider_quiet: 25.0,
            slider_sizes: [30.0, 40.0, 50.0, 60.0, 70.0],
            chip_filters: [true, false, false],
            chip_elev_filter: false,
            chip_inputs: ["Alice", "Bob", "Carol"].map(str::to_owned).to_vec(),
            chip_input_selected: None,
            chip_assist: 0,
            rating: 3.0,
            rating_half: 2.5,
            rating_hearts: 4.0,
            rating_clear: 2.0,
            rating_ro: 4.5,
            select_single: Some(1),
            select_color: None,
            select_codec: Some(0),
            select_outlined: None,
            select_err: None,
            select_multi: Selection::multiple().with([0, 2]),
            combo_text: String::new(),
            combo_free: String::new(),
            combo_loading: String::new(),
            combo_created: Vec::new(),
            transfer: vec![1],
            color: Color32::from_rgb(103, 80, 164),
            color_hct: Color32::from_rgb(0, 150, 136),
            date_docked: None,
            date_bounded: None,
            date_modal: Some(Date::constant(2026, 3, 14)),
            date_modal_open: false,
            date_input: None,
            date_input_open: false,
            date_range: DateRange::default(),
            date_range_open: false,
            date_inline: None,
            date_inline_range: DateRange::default(),
            time_inline: Time::constant(9, 30, 0, 0),
            time_24: Time::constant(17, 45, 0, 0),
            time_h: Time::constant(6, 15, 0, 0),
            time_input: Time::constant(14, 5, 0, 0),
            time_modal: Time::constant(12, 0, 0, 0),
            time_modal_open: false,
        }
    }
}

const FRUITS: [&str; 8] = [
    "Apple",
    "Banana",
    "Cherry",
    "Dragonfruit",
    "Elderberry",
    "Fig",
    "Grape",
    "Honeydew",
];

fn fmt_date(d: Option<Date>) -> String {
    d.map_or_else(|| "none".to_owned(), |d| d.to_string())
}

pub(crate) fn show(ui: &mut Mui<'_>, s: &mut State, shared: &mut Shared) {
    checkbox(ui, s);
    radio(ui, s);
    switch(ui, s);
    slider(ui, s);
    chips(ui, s, shared);
    rating(ui, s);
    selects(ui, s, shared);
    transfer(ui, s);
    color(ui, s);
    dates(ui, s);
    times(ui, s);
}

fn checkbox(ui: &mut Mui<'_>, s: &mut State) {
    let rt = ui.rt();
    super::section(ui, TOPICS[0], Some(<CheckButton<'static>>::META), |ui| {
        // Top-aligned: the error line makes one item taller.
        ui.with_layout(Layout::left_to_right(Align::Min).with_main_wrap(true), |ui| {
            rt.ui(ui)
                .mineral(CheckButton::new("sel-check", "Checked", &mut s.check));
            rt.ui(ui)
                .mineral(CheckButton::new("sel-check-off", "Unchecked", &mut s.check_off));
            rt.ui(ui)
                .mineral(CheckButton::new("sel-check-err", "Error", &mut s.check_err).error("Required"));
            let mut on = true;
            rt.ui(ui)
                .mineral(CheckButton::new("sel-check-dis-on", "Disabled on", &mut on).disabled(true));
            let mut off = false;
            rt.ui(ui)
                .mineral(CheckButton::new("sel-check-dis-off", "Disabled off", &mut off).disabled(true));
            let mut ind = false;
            rt.ui(ui)
                .mineral(CheckButton::new("sel-check-ind", "Indeterminate", &mut ind).indeterminate(true));
        });
        caption(ui, TypeScale::LabelMedium, "Long label wraps at the available width");
        ui.scope(|ui| {
            ui.set_max_width(280.0);
            rt.ui(ui).mineral(CheckButton::new(
                "sel-check-long",
                "Export every selected layer, including hidden ones, as separate files",
                &mut s.check_long,
            ));
        });
        caption(ui, TypeScale::LabelMedium, "Parent with indeterminate state");
        let all = s.children.iter().all(|c| *c);
        let any = s.children.iter().any(|c| *c);
        let mut parent = all;
        if rt
            .ui(ui)
            .mineral(CheckButton::new("sel-check-parent", "Select all", &mut parent).indeterminate(any && !all))
            .changed()
        {
            s.children = [parent; 3];
        }
        ui.indent("sel-check-children", |ui| {
            for (i, c) in s.children.iter_mut().enumerate() {
                rt.ui(ui)
                    .mineral(CheckButton::new(("sel-check-child", i), format!("Option {}", i + 1), c));
            }
        });
    });
}

fn radio(ui: &mut Mui<'_>, s: &mut State) {
    super::section(ui, TOPICS[1], Some(Radio::META), |ui| {
        caption(ui, TypeScale::LabelMedium, "RadioGroup, vertical");
        let opts = [
            (Portion::Small, "Small"),
            (Portion::Medium, "Medium"),
            (Portion::Large, "Large"),
        ];
        RadioGroup::new("sel-radio-v", "Size", &mut s.radio)
            .options(opts.map(|(v, l)| RadioOption::new(v, l)))
            .axis(Axis::Vertical)
            .show(ui);
        caption(ui, TypeScale::LabelMedium, "RadioGroup, horizontal");
        RadioGroup::new("sel-radio-h", "Size, horizontal", &mut s.radio_row)
            .options(opts.map(|(v, l)| RadioOption::new(v, l)))
            .axis(Axis::Horizontal)
            .show(ui);
        caption(ui, TypeScale::LabelMedium, "Standalone, error-free states");
        super::row(ui, |row| {
            if row
                .mineral(Radio::new("sel-radio-solo", "Standalone").selected(s.radio_single))
                .changed()
            {
                s.radio_single = !s.radio_single;
            }
            row.mineral(
                Radio::new("sel-radio-dis-on", "Disabled on")
                    .selected(true)
                    .disabled(true),
            );
            row.mineral(Radio::new("sel-radio-dis-off", "Disabled off").disabled(true));
        });
        ui.label(format!(
            "Selected: {} / {}",
            opts.iter().find(|(v, _)| *v == s.radio).map_or("", |o| o.1),
            opts.iter().find(|(v, _)| *v == s.radio_row).map_or("", |o| o.1)
        ));
    });
}

fn switch(ui: &mut Mui<'_>, s: &mut State) {
    super::section(ui, TOPICS[2], Some(<Switch<'static>>::META), |ui| {
        super::row(ui, |row| {
            row.mineral(Switch::new("sel-sw", "Default", &mut s.sw));
            row.mineral(Switch::new("sel-sw-icons", "With icons", &mut s.sw_icons).icons(true));
            let mut a = true;
            row.mineral(Switch::new("sel-sw-dis-on", "Disabled on", &mut a).disabled(true));
            let mut b = false;
            row.mineral(Switch::new("sel-sw-dis-off", "Disabled off", &mut b).disabled(true));
            let mut c = true;
            row.mineral(
                Switch::new("sel-sw-dis-icons", "Disabled icons", &mut c)
                    .icons(true)
                    .disabled(true),
            );
        });
    });
}

fn slider(ui: &mut Mui<'_>, s: &mut State) {
    let rt = ui.rt();
    super::section(ui, TOPICS[3], Some(<ValueSlider<'static>>::META), |ui| {
        let r100 = || RangeModel::new(0.0, 100.0);
        caption(ui, TypeScale::LabelMedium, "Continuous");
        rt.ui(ui)
            .mineral(ValueSlider::new("sel-slider", "Continuous", r100(), &mut s.slider).preferred_width(360.0));
        caption(ui, TypeScale::LabelMedium, "Stepped (10)");
        rt.ui(ui).mineral(
            ValueSlider::new("sel-slider-step", "Stepped", r100().step(10.0), &mut s.slider_step)
                .preferred_width(360.0),
        );
        caption(ui, TypeScale::LabelMedium, "Stepped with ticks, formatted");
        rt.ui(ui).mineral(
            ValueSlider::new("sel-slider-ticks", "Ticks", r100().step(10.0), &mut s.slider_ticks)
                .ticks(true)
                .format(|v| format!("{v:.0}%"))
                .preferred_width(360.0),
        );
        caption(ui, TypeScale::LabelMedium, "Range");
        rt.ui(ui).mineral(
            ValueSlider::range("sel-slider-range", "Range", r100(), &mut s.slider_range).preferred_width(360.0),
        );
        caption(ui, TypeScale::LabelMedium, "Range, stepped with ticks");
        rt.ui(ui).mineral(
            ValueSlider::range(
                "sel-slider-range-step",
                "Stepped range",
                r100().step(20.0),
                &mut s.slider_range_step,
            )
            .ticks(true)
            .preferred_width(360.0),
        );
        caption(ui, TypeScale::LabelMedium, "Centered (-50..50)");
        rt.ui(ui).mineral(
            ValueSlider::new("sel-slider-center", "Centered", -50.0..=50.0, &mut s.slider_center)
                .centered(true)
                .preferred_width(360.0),
        );
        caption(
            ui,
            TypeScale::LabelMedium,
            "Log scale (20 Hz..20 kHz), 3 significant digits",
        );
        rt.ui(ui).mineral(
            ValueSlider::new(
                "sel-slider-log",
                "Frequency",
                RangeModel::new(20.0, 20_000.0).scale(Scale::Log).significant(3),
                &mut s.slider_log,
            )
            .format(|v| format!("{v} Hz"))
            .preferred_width(360.0),
        );
        caption(ui, TypeScale::LabelMedium, "End icons");
        rt.ui(ui).mineral(
            ValueSlider::new("sel-slider-icons", "Volume", r100(), &mut s.slider_icons)
                .icon(ICON_VOLUME_DOWN)
                .trailing_icon(ICON_VOLUME_UP)
                .preferred_width(360.0),
        );
        caption(ui, TypeScale::LabelMedium, "Disabled");
        let mut dis = 50.0;
        rt.ui(ui).mineral(
            ValueSlider::new("sel-slider-dis", "Disabled", r100(), &mut dis)
                .disabled(true)
                .preferred_width(360.0),
        );
        caption(ui, TypeScale::LabelMedium, "No value indicator");
        rt.ui(ui).mineral(
            ValueSlider::new("sel-slider-quiet", "Quiet", r100(), &mut s.slider_quiet)
                .indicator(false)
                .preferred_width(360.0),
        );
        caption(ui, TypeScale::LabelMedium, "Expressive sizes XS, S, M, L, XL");
        let sizes = [Size::XSmall, Size::Small, Size::Medium, Size::Large, Size::XLarge];
        for (i, (size, v)) in sizes.into_iter().zip(s.slider_sizes.iter_mut()).enumerate() {
            rt.ui(ui).mineral(
                ValueSlider::new(("sel-slider-size", i), "Sized", r100(), v)
                    .size(size)
                    .icon(ICON_VOLUME_DOWN)
                    .trailing_icon(ICON_VOLUME_UP)
                    .preferred_width(360.0),
            );
        }
        caption(ui, TypeScale::LabelMedium, "Vertical");
        rt.ui(ui).mineral(
            ValueSlider::new("sel-slider-vert", "Vertical", r100(), &mut s.slider_vert)
                .axis(Axis::Vertical)
                .preferred_height(180.0),
        );
    });
}

fn chips(ui: &mut Mui<'_>, s: &mut State, shared: &mut Shared) {
    super::section(ui, TOPICS[4], Some(Chip::META), |ui| {
        caption(ui, TypeScale::LabelMedium, "Assist (click counter), with icon");
        super::row(ui, |row| {
            if row
                .mineral(Chip::assist("sel-chip-assist", format!("Clicked {}", s.chip_assist)).icon(ICON_EVENT))
                .clicked()
            {
                s.chip_assist += 1;
            }
            if row
                .mineral(
                    Chip::assist("sel-chip-assist-e", "Elevated")
                        .icon(ICON_LOCAL_CAFE)
                        .elevated(true),
                )
                .clicked()
            {
                shared.toast("Elevated assist chip");
            }
            row.mineral(
                Chip::assist("sel-chip-assist-d", "Disabled")
                    .icon(ICON_WIFI)
                    .disabled(true),
            );
        });
        caption(ui, TypeScale::LabelMedium, "Filter (toggle), elevated, disabled");
        super::row(ui, |row| {
            for (i, name) in ["Wi-Fi", "Music", "Public"].into_iter().enumerate() {
                let icon = [ICON_WIFI, ICON_MUSIC_NOTE, ICON_PUBLIC][i];
                row.mineral(Chip::filter(("sel-chip-filter", i), name, &mut s.chip_filters[i]).icon(icon));
            }
            row.mineral(Chip::filter("sel-chip-filter-e", "Elevated", &mut s.chip_elev_filter).elevated(true));
            row.mineral(Chip::filter("sel-chip-filter-d", "Disabled", &mut true).disabled(true));
        });
        caption(ui, TypeScale::LabelMedium, "Input (removable, selectable)");
        super::row(ui, |row| {
            let mut remove = None;
            for (i, name) in s.chip_inputs.iter().enumerate() {
                let chip = Chip::input(("sel-chip-input", i), name.as_str()).selected(s.chip_input_selected == Some(i));
                let r = row.mineral(chip);
                if r.output().removed {
                    remove = Some(i);
                } else if r.changed() {
                    s.chip_input_selected = if s.chip_input_selected == Some(i) {
                        None
                    } else {
                        Some(i)
                    };
                }
            }
            if let Some(i) = remove {
                s.chip_inputs.remove(i);
                s.chip_input_selected = None;
            }
            if row
                .mineral(PushButton::new("sel-chip-input-reset", "Reset").variant(ButtonVariant::Text))
                .clicked()
            {
                s.chip_inputs = ["Alice", "Bob", "Carol"].map(str::to_owned).to_vec();
            }
        });
        suggestion_chips(ui, shared);
    });
}

fn suggestion_chips(ui: &mut Mui<'_>, shared: &mut Shared) {
    caption(ui, TypeScale::LabelMedium, "Suggestion");
    super::row(ui, |row| {
        for (i, name) in ["Dinner nearby", "Open now", "Top rated"].into_iter().enumerate() {
            if row.mineral(Chip::suggestion(("sel-chip-sug", i), name)).clicked() {
                shared.toast(format!("Suggestion: {name}"));
            }
        }
        row.mineral(Chip::suggestion("sel-chip-sug-d", "Disabled").disabled(true));
        if row
            .mineral(Chip::suggestion("sel-chip-new", "Elevated").elevated(true))
            .clicked()
        {
            shared.toast("Elevated suggestion chosen");
        }
    });
}

fn rating(ui: &mut Mui<'_>, s: &mut State) {
    let rt = ui.rt();
    super::section(ui, TOPICS[5], Some(<Rating<'static>>::META), |ui| {
        caption(ui, TypeScale::LabelMedium, "Whole stars");
        rt.ui(ui).mineral(Rating::new("sel-rate", "Rating", &mut s.rating));
        caption(ui, TypeScale::LabelMedium, "Half precision, 10 stars");
        rt.ui(ui).mineral(
            Rating::new("sel-rate-half", "Half rating", &mut s.rating_half)
                .half(true)
                .count(NonZero::new(10).expect("non-zero")),
        );
        caption(ui, TypeScale::LabelMedium, "Custom icon, medium icon button size");
        rt.ui(ui).mineral(
            Rating::new("sel-rate-heart", "Hearts", &mut s.rating_hearts)
                .icon(ICON_FAVORITE)
                .size(Size::Medium),
        );
        caption(ui, TypeScale::LabelMedium, "Clearable (click the value again)");
        rt.ui(ui).mineral(
            Rating::new("sel-rate-clear", "Clearable", &mut s.rating_clear)
                .clearable(true)
                .icon(ICON_STAR),
        );
        caption(ui, TypeScale::LabelMedium, "Read only and disabled");
        rt.ui(ui).mineral(
            Rating::new("sel-rate-ro", "Read only", &mut s.rating_ro)
                .half(true)
                .read_only(true),
        );
        let mut dis = 3.0;
        rt.ui(ui)
            .mineral(Rating::new("sel-rate-dis", "Disabled", &mut dis).disabled(true));
        ui.label(format!(
            "Values: {} / {} / {} / {}",
            s.rating, s.rating_half, s.rating_hearts, s.rating_clear
        ));
    });
}

fn selects(ui: &mut Mui<'_>, s: &mut State, shared: &mut Shared) {
    let rt = ui.rt();
    super::section(ui, TOPICS[6], Some(<Select<'static>>::META), |ui| {
        // Rows by box top: outlined fields with a label reserve its overhang above the box.
        super::top_row(ui, |row| {
            row.mineral(Select::new("sel-select", "Fruit", FRUITS, &mut s.select_single).preferred_width(240.0));
            row.add(|ui| color_select(ui, s));
            // No label, so no overhang: its box lines up with the filled ones.
            row.add(|ui| codec_select(ui, s));
            let mut dis = Some(0);
            row.mineral(
                Select::new("sel-select-dis", "Disabled", FRUITS, &mut dis)
                    .disabled(true)
                    .preferred_width(240.0),
            );
        });
        super::top_row(ui, |row| {
            row.mineral(
                Select::new("sel-select-out", "Outlined", FRUITS, &mut s.select_outlined)
                    .variant(TextFieldVariant::Outlined)
                    .icon(ICON_SEARCH)
                    .supporting("With leading icon")
                    .preferred_width(240.0),
            );
            let err = s.select_err.is_none().then(|| "Pick a fruit".to_owned());
            row.mineral(
                Select::new("sel-select-err", "Required", FRUITS, &mut s.select_err)
                    .variant(TextFieldVariant::Outlined)
                    .maybe_error(err)
                    .preferred_width(240.0),
            );
        });
        caption(ui, TypeScale::LabelMedium, "Multiple (input chips)");
        rt.ui(ui).mineral(
            Select::multi("sel-select-multi", "Fruits", FRUITS, &mut s.select_multi)
                .variant(TextFieldVariant::Outlined)
                .supporting("Toggling keeps the menu open")
                .preferred_width(420.0),
        );
        let picked: Vec<&str> = s.select_multi.selected().map(|i| FRUITS[*i]).collect();
        ui.label(format!(
            "Single: {} / multi: {}",
            s.select_single.map_or("none", |i| FRUITS[i]),
            picked.join(", ")
        ));
        if rt
            .ui(ui)
            .mineral(PushButton::new("sel-toast-selection", "Toast selection").variant(ButtonVariant::Tonal))
            .clicked()
        {
            shared.toast(format!("{} selected", picked.len()));
        }
    });
    comboboxes(ui, s);
}

fn comboboxes(ui: &mut Mui<'_>, s: &mut State) {
    super::section(ui, TOPICS[7], Some(<Combobox<'static>>::META), |ui| {
        super::top_row(ui, |row| {
            let r = row.add(|ui| {
                ui.mineral(
                    Combobox::new("sel-combo", "Filter fruit", FRUITS, &mut s.combo_text)
                        .supporting("Type to filter")
                        .preferred_width(260.0),
                )
            });
            if let Some(i) = r.output().selected {
                s.combo_created.push(format!("picked {}", FRUITS[i]));
            }
            let mut dis = String::from("Fig");
            row.add(|ui| {
                ui.mineral(
                    Combobox::new("sel-combo-dis", "Disabled", FRUITS, &mut dis)
                        .disabled(true)
                        .preferred_width(260.0),
                )
            });
        });
        // One row per variant, as in `selects`.
        super::top_row(ui, |row| {
            let r = row.add(|ui| {
                ui.mineral(
                    Combobox::new("sel-combo-free", "Free text", FRUITS, &mut s.combo_free)
                        .variant(TextFieldVariant::Outlined)
                        .free_text(true)
                        .icon(ICON_SEARCH)
                        .supporting("Enter creates a value")
                        .preferred_width(260.0),
                )
            });
            if let Some(text) = r.into_output().created {
                s.combo_created.push(format!("created {text}"));
            }
            let loading = !s.combo_loading.is_empty() && s.combo_loading.len() < 3;
            row.add(|ui| {
                ui.mineral(
                    Combobox::new(
                        "sel-combo-load",
                        "Async (loading under 3 chars)",
                        FRUITS,
                        &mut s.combo_loading,
                    )
                    .variant(TextFieldVariant::Outlined)
                    .loading(loading)
                    .preferred_width(260.0),
                )
            });
        });
        if let Some(last) = s.combo_created.last() {
            ui.label(format!("Last event: {last}"));
        }
    });
}

fn color_select(ui: &mut Mui<'_>, s: &mut State) {
    let colors = [
        ("Crimson", [0xdc, 0x14, 0x3c]),
        ("Teal", [0x00, 0x80, 0x80]),
        ("Amber", [0xff, 0xbf, 0x00]),
    ]
    .map(|(name, [r, g, b])| {
        SelectOption::new(name).leading(LeadingPaint::new(move |p, slot, _| {
            p.rect_filled(slot.shrink(2.0), 4, Color32::from_rgb(r, g, b));
        }))
    });
    ui.mineral(
        Select::new("sel-select-color", "Colour", colors, &mut s.select_color)
            .supporting("Painted option leading")
            .preferred_width(240.0),
    );
}

fn codec_select(ui: &mut Mui<'_>, s: &mut State) {
    let codecs = [
        SelectOption::new("FLAC"),
        SelectOption::new("MP3 (no encoder)").disabled(true),
        SelectOption::new("WAV"),
    ];
    ui.mineral(
        Select::new("sel-select-codec", "Codec", codecs, &mut s.select_codec)
            .variant(TextFieldVariant::Outlined)
            .label_visible(false)
            .supporting("Hidden label, disabled option")
            .preferred_width(240.0),
    );
}

fn transfer(ui: &mut Mui<'_>, s: &mut State) {
    let rt = ui.rt();
    super::section(ui, TOPICS[8], Some(<TransferList<'static>>::META), |ui| {
        rt.ui(ui).mineral(
            TransferList::new("sel-transfer", FRUITS, &mut s.transfer)
                .source_title("Available")
                .target_title("Basket")
                .preferred_size(vec2(500.0, 300.0)),
        );
        let names: Vec<&str> = s.transfer.iter().map(|i| FRUITS[*i]).collect();
        ui.label(format!("Basket: {}", names.join(", ")));
        let mut fixed = vec![0];
        ui.add_space(8.0);
        caption(ui, TypeScale::LabelMedium, "Disabled");
        rt.ui(ui).mineral(
            TransferList::new("sel-transfer-dis", FRUITS, &mut fixed)
                .disabled(true)
                .preferred_size(vec2(500.0, 200.0)),
        );
    });
}

fn color(ui: &mut Mui<'_>, s: &mut State) {
    let rt = ui.rt();
    super::section(ui, TOPICS[9], Some(<ColorPicker<'static>>::META), |ui| {
        let swatches = [
            Color32::from_rgb(244, 67, 54),
            Color32::from_rgb(255, 152, 0),
            Color32::from_rgb(76, 175, 80),
            Color32::from_rgb(33, 150, 243),
            Color32::from_rgb(156, 39, 176),
        ];
        super::top_row(ui, |row| {
            row.add(|ui| {
                caption(
                    &mut rt.ui(ui),
                    TypeScale::LabelMedium,
                    "HSV with swatches and RGB fields",
                );
                rt.ui(ui).mineral(
                    ColorPicker::new("sel-color", "Color", &mut s.color)
                        .space(ColorSpace::Hsv)
                        .swatches(swatches)
                        .rgb_fields(true)
                        .preferred_width(300.0),
                );
            });
            row.add(|ui| {
                caption(&mut rt.ui(ui), TypeScale::LabelMedium, "HCT (perceptual)");
                rt.ui(ui).mineral(
                    ColorPicker::new("sel-color-hct", "HCT color", &mut s.color_hct)
                        .space(ColorSpace::Hct)
                        .preferred_width(300.0),
                );
            });
            row.add(|ui| {
                caption(&mut rt.ui(ui), TypeScale::LabelMedium, "Disabled");
                let mut c = Color32::from_rgb(0, 100, 200);
                rt.ui(ui).mineral(
                    ColorPicker::new("sel-color-dis", "Disabled color", &mut c)
                        .disabled(true)
                        .preferred_width(300.0),
                );
            });
        });
        let (rect, _) = ui.allocate_exact_size(vec2(120.0, 24.0), egui::Sense::hover());
        ui.painter().rect_filled(rect, 8, s.color);
    });
}

fn dates(ui: &mut Mui<'_>, s: &mut State) {
    let rt = ui.rt();
    super::section(ui, TOPICS[10], Some(<DatePicker<'static>>::META), |ui| {
        caption(ui, TypeScale::LabelMedium, "Docked (typed or picked)");
        super::row(ui, |row| {
            row.add(|ui| {
                DatePicker::new("sel-date", &mut s.date_docked)
                    .input_mode(DisplayMode::Input)
                    .label("Departure")
                    .show(ui)
            });
            row.add(|ui| {
                DatePicker::new("sel-date-bounded", &mut s.date_bounded)
                    .bounds(DateBounds::new(
                        Some(Date::constant(2026, 1, 1)),
                        Some(Date::constant(2026, 12, 31)),
                    ))
                    .input_mode(DisplayMode::Input)
                    .label("2026 only")
                    .show(ui)
            });
        });
        ui.label(format!(
            "Docked: {} / bounded: {}",
            fmt_date(s.date_docked),
            fmt_date(s.date_bounded)
        ));
        caption(ui, TypeScale::LabelMedium, "Modal");
        super::row(ui, |row| {
            if row
                .mineral(PushButton::new("sel-date-modal-btn", "Calendar dialog"))
                .clicked()
            {
                s.date_modal_open = true;
            }
            if row
                .mineral(PushButton::new("sel-date-input-btn", "Input dialog").variant(ButtonVariant::Tonal))
                .clicked()
            {
                s.date_input_open = true;
            }
            if row
                .mineral(PushButton::new("sel-date-range-btn", "Range dialog").variant(ButtonVariant::Outlined))
                .clicked()
            {
                s.date_range_open = true;
            }
        });
        DatePicker::new("sel-date-m", &mut s.date_modal).show_modal(rt, &mut s.date_modal_open);
        DatePicker::new("sel-date-mi", &mut s.date_input)
            .input_mode(DisplayMode::Input)
            .show_modal(rt, &mut s.date_input_open);
        DateRangePicker::new("sel-date-mr", &mut s.date_range).show_modal(rt, &mut s.date_range_open);
        ui.label(format!(
            "Modal: {} / input: {} / range: {} .. {}",
            fmt_date(s.date_modal),
            fmt_date(s.date_input),
            fmt_date(s.date_range.start()),
            fmt_date(s.date_range.end())
        ));
        super::top_row(ui, |row| {
            row.add(|ui| {
                caption(&mut rt.ui(ui), TypeScale::LabelMedium, "Inline month grid");
                DatePicker::new("sel-date-inline", &mut s.date_inline).show(&mut rt.ui(ui));
                ui.label(fmt_date(s.date_inline));
            });
            row.add(|ui| {
                caption(
                    &mut rt.ui(ui),
                    TypeScale::LabelMedium,
                    "Inline range (scrolling months)",
                );
                DateRangePicker::new("sel-date-inline-r", &mut s.date_inline_range).show(&mut rt.ui(ui));
                ui.label(format!(
                    "{} .. {}",
                    fmt_date(s.date_inline_range.start()),
                    fmt_date(s.date_inline_range.end())
                ));
            });
        });
    });
}

fn times(ui: &mut Mui<'_>, s: &mut State) {
    let rt = ui.rt();
    super::section(ui, TOPICS[11], Some(<TimePicker<'static>>::META), |ui| {
        super::top_row(ui, |row| {
            row.add(|ui| {
                caption(&mut rt.ui(ui), TypeScale::LabelMedium, "Inline, locale clock");
                TimePicker::new("sel-time", &mut s.time_inline).show(&mut rt.ui(ui));
                ui.label(s.time_inline.to_string());
            });
            row.add(|ui| {
                caption(&mut rt.ui(ui), TypeScale::LabelMedium, "Inline, 24-hour");
                TimePicker::new("sel-time-24", &mut s.time_24)
                    .hour_cycle(HourCycle::H23)
                    .show(&mut rt.ui(ui));
                ui.label(s.time_24.to_string());
            });
        });
        super::top_row(ui, |row| {
            row.add(|ui| {
                caption(&mut rt.ui(ui), TypeScale::LabelMedium, "Horizontal layout");
                TimePicker::new("sel-time-h", &mut s.time_h)
                    .axis(Axis::Horizontal)
                    .show(&mut rt.ui(ui));
            });
            row.add(|ui| {
                caption(&mut rt.ui(ui), TypeScale::LabelMedium, "Time input");
                TimePicker::new("sel-time-in", &mut s.time_input)
                    .input_mode(DisplayMode::Input)
                    .show(&mut rt.ui(ui));
                ui.label(s.time_input.to_string());
            });
        });
        caption(ui, TypeScale::LabelMedium, "Modal");
        if rt
            .ui(ui)
            .mineral(PushButton::new("sel-time-modal-btn", "Time dialog"))
            .clicked()
        {
            s.time_modal_open = true;
        }
        TimePicker::new("sel-time-m", &mut s.time_modal).show_modal(rt, &mut s.time_modal_open);
        ui.label(format!("Modal time: {}", s.time_modal));
    });
}
