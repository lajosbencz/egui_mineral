use egui::Color32;

use egui_mineral_fluent::{
    Size,
    egui_mineral::{layout::Axis, range::RangeModel, select::Selection},
    icons::{ICON_MAIL, ICON_PERSON},
    inputs::{
        Calendar, Checkbox, CheckboxLabelPosition, CheckboxShape, ColorPicker, Combobox, DatePicker, Dropdown, Field,
        InfoLabel, Input, InputAppearance, ListEntry, ListOption, OptionGroup, RadioGroup, RadioLayout, RadioOption,
        RangeSlider, Rating, RatingAppearance, RatingColor, RatingDisplay, RatingSize, SearchBox, Select, Slider,
        SpinButton, Swatch, SwatchLayout, SwatchPicker, SwatchShape, SwatchSize, Switch, SwitchLabelPosition,
        TagPicker, TagPickerControl, TagPickerSize, Textarea, TimePicker, Validation,
    },
};

use super::heading;
use crate::FluentUi;

#[derive(Debug)]
pub(crate) struct State {
    checks: [bool; 4],
    radio: usize,
    stacked: usize,
    switches: [bool; 3],
    slider: f64,
    vertical: f64,
    range: [f64; 2],
    spin: f64,
    rating: f64,
    half: f64,
    text: [String; 3],
    search: String,
    area: String,
    password: String,
    pick: Selection<Animal>,
    multi: Selection<Animal>,
    combo: Selection<Animal>,
    combo_text: String,
    native: Option<Animal>,
    time: Option<egui_mineral_fluent::egui_mineral::date::Time>,
    tags: Selection<Animal>,
    tags_text: String,
    tag_button: Selection<Animal>,
    tag_button_text: String,
    date: Option<egui_mineral_fluent::egui_mineral::date::Date>,
    day: Option<egui_mineral_fluent::egui_mineral::date::Date>,
    swatch: Option<Paint>,
    color: egui::ecolor::Hsva,
}

impl Default for State {
    fn default() -> Self {
        Self {
            checks: [true, false, false, true],
            radio: 1,
            stacked: 0,
            switches: [true, false, true],
            slider: 40.0,
            vertical: 25.0,
            range: [20.0, 70.0],
            spin: 10.0,
            rating: 3.0,
            half: 2.5,
            text: [String::from("Hello"), String::new(), String::new()],
            search: String::from("fluent"),
            area: String::from("Line one\nLine two"),
            password: String::from("secret"),
            pick: Selection::single().with([Animal::Cat]),
            multi: Selection::multiple().with([Animal::Dog]),
            combo: Selection::single(),
            combo_text: String::new(),
            native: None,
            time: None,
            tags: Selection::multiple().with([Animal::Dog]),
            tags_text: String::new(),
            tag_button: Selection::multiple(),
            tag_button_text: String::new(),
            date: None,
            day: None,
            swatch: Some(Paint::Blue),
            color: egui::ecolor::Hsva::new(0.58, 0.8, 0.9, 1.0),
        }
    }
}

pub(crate) fn show(ui: &mut FluentUi<'_>, s: &mut State) {
    choices(ui, s);
    sliders(ui, s);
    fields(ui, s);
    pickers(ui, s);
    tags_and_dates(ui, s);
    swatches(ui, s);
    ui.mineral(ColorPicker::new("cp-1", "Color", &mut s.color).alpha(true));
    composed(ui, s);
}

fn choices(ui: &mut FluentUi<'_>, s: &mut State) {
    heading(ui, "h-checkbox", "Checkbox");
    let [a, b, c, d] = &mut s.checks;
    ui.mineral(Checkbox::new("cb-1", "Checked", a));
    ui.mineral(Checkbox::new("cb-2", "Mixed", b).mixed(true));
    ui.mineral(
        Checkbox::new("cb-3", "Large circular", c)
            .size(Size::Large)
            .shape(CheckboxShape::Circular),
    );
    ui.mineral(Checkbox::new("cb-4", "Label before", d).label_position(CheckboxLabelPosition::Before));
    let mut off = false;
    ui.mineral(Checkbox::new("cb-5", "Disabled", &mut off).disabled(true));
    heading(ui, "h-radio", "Radio");
    let options: Vec<_> = ["Apple", "Pear", "Cherry"]
        .into_iter()
        .enumerate()
        .map(|(i, l)| (i, RadioOption::new(l)))
        .collect();
    RadioGroup::new("rg-1", "Fruit", &mut s.radio)
        .items(options.clone())
        .show(ui);
    RadioGroup::new("rg-2", "Stacked", &mut s.stacked)
        .items(options)
        .layout(RadioLayout::HorizontalStacked)
        .show(ui);
    heading(ui, "h-switch", "Switch");
    let [a, b, c] = &mut s.switches;
    ui.mineral(Switch::new("sw-1", "On", a));
    ui.mineral(
        Switch::new("sw-2", "Small, label before", b)
            .size(Size::Small)
            .label_position(SwitchLabelPosition::Before),
    );
    ui.mineral(Switch::new("sw-3", "Label above", c).label_position(SwitchLabelPosition::Above));
    let mut off = false;
    ui.mineral(Switch::new("sw-4", "Disabled", &mut off).disabled(true));
}

fn sliders(ui: &mut FluentUi<'_>, s: &mut State) {
    heading(ui, "h-slider", "Slider and RangeSlider");
    let range = RangeModel::new(0.0, 100.0).step(10.0);
    ui.mineral(Slider::new("sl-1", "Volume", range, &mut s.slider));
    let mut small = 60.0;
    ui.mineral(Slider::new("sl-2", "Small", RangeModel::new(0.0, 100.0), &mut small).size(Size::Small));
    let mut off = 30.0;
    ui.mineral(Slider::new("sl-3", "Disabled", RangeModel::new(0.0, 100.0), &mut off).disabled(true));
    ui.mineral(Slider::new("sl-4", "Vertical", RangeModel::new(0.0, 100.0), &mut s.vertical).axis(Axis::Vertical));
    ui.mineral(RangeSlider::new("rs-1", "Price", range, &mut s.range));
    heading(ui, "h-spin", "SpinButton");
    ui.mineral(SpinButton::new(
        "sp-1",
        "Quantity",
        RangeModel::new(0.0, 100.0).step(1.0),
        &mut s.spin,
    ));
    let mut small = 5.0;
    ui.mineral(
        SpinButton::new(
            "sp-2",
            "Small underline",
            RangeModel::new(0.0, 10.0).step(0.5),
            &mut small,
        )
        .size(Size::Small)
        .appearance(InputAppearance::Underline),
    );
    heading(ui, "h-rating", "Rating");
    ui.mineral(
        Rating::new("rt-1", "Rating", &mut s.rating)
            .size(RatingSize::Large)
            .color(RatingColor::Brand),
    );
    ui.mineral(
        Rating::new("rt-2", "Half rating", &mut s.half)
            .half(true)
            .color(RatingColor::Marigold)
            .appearance(RatingAppearance::Filled),
    );
    ui.mineral(
        RatingDisplay::new("rd-1", "Reviews", 4.5)
            .count(1234)
            .size(RatingSize::Large)
            .color(RatingColor::Marigold),
    );
    ui.mineral(RatingDisplay::new("rd-2", "Compact", 4.2).compact(true).max(5));
}

fn fields(ui: &mut FluentUi<'_>, s: &mut State) {
    heading(ui, "h-input", "Input, Textarea and SearchBox");
    for (i, a) in InputAppearance::ALL.iter().copied().enumerate() {
        ui.mineral(
            Input::new(("in", i), format!("{a:?}"), &mut s.text[0])
                .appearance(a)
                .placeholder("Placeholder"),
        );
    }
    ui.mineral(
        Input::new("in-icons", "With content", &mut s.text[1])
            .icon_before(ICON_PERSON)
            .icon_after(ICON_MAIL)
            .size(Size::Large),
    );
    ui.mineral(
        Input::new("in-small", "Small", &mut s.text[2])
            .size(Size::Small)
            .placeholder("Small"),
    );
    ui.mineral(Input::new("in-password", "Password", &mut s.password).password(true));
    let mut text = String::from("Invalid");
    ui.mineral(Input::new("in-invalid", "Invalid", &mut text).validation(Some(Validation::Error)));
    let mut off = String::from("Disabled");
    ui.mineral(Input::new("in-disabled", "Disabled", &mut off).disabled(true));
    ui.mineral(Textarea::new("ta-1", "Notes", &mut s.area).placeholder("Notes"));
    ui.mineral(SearchBox::new("sb-1", "Search", &mut s.search).placeholder("Search"));
}

fn composed(ui: &mut FluentUi<'_>, s: &mut State) {
    heading(ui, "h-field", "Field and InfoLabel");
    Field::new("fd-1")
        .label("Name")
        .required(true)
        .hint("As on your passport")
        .show(ui, |ui, cx| {
            ui.mineral(Input::new("fd-1-in", "Name", &mut s.text[0]).validation(cx.validation))
        });
    Field::new("fd-2")
        .label("Email")
        .validation(Validation::Error, "Enter a valid email")
        .show(ui, |ui, cx| {
            ui.mineral(Input::new("fd-2-in", "Email", &mut s.text[1]).validation(cx.validation))
        });
    Field::new("fd-3")
        .label("Code")
        .validation(Validation::Success, "Looks good")
        .axis(Axis::Horizontal)
        .show(ui, |ui, cx| {
            ui.mineral(Input::new("fd-3-in", "Code", &mut s.text[2]).validation(cx.validation))
        });
    ui.mineral(InfoLabel::new(
        "il-1",
        "Label with info",
        "Additional guidance for the field.",
    ));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, egui_mineral_fluent::prelude::KeySalt)]
pub(crate) enum Animal {
    Cat,
    Dog,
    Fish,
    Eel,
    Bird,
}

fn animals() -> Vec<ListEntry<Animal>> {
    vec![
        OptionGroup::new("Land")
            .items([
                (Animal::Cat, ListOption::new("Cat")),
                (Animal::Dog, ListOption::new("Dog")),
            ])
            .into(),
        OptionGroup::new("Water")
            .items([
                (Animal::Fish, ListOption::new("Fish")),
                (Animal::Eel, ListOption::new("Eel").disabled(true)),
            ])
            .into(),
        (Animal::Bird, ListOption::new("Bird")).into(),
    ]
}

fn pickers(ui: &mut FluentUi<'_>, s: &mut State) {
    heading(ui, "h-pick", "Select, Dropdown, Combobox and TimePicker");
    ui.mineral(
        Select::optional("sel-1", "Pet", &mut s.native)
            .items(animals())
            .placeholder("Select an animal"),
    );
    ui.mineral(
        Dropdown::multi("dd-1", "Dropdown", &mut s.pick)
            .items(animals())
            .placeholder("Pick one"),
    );
    ui.mineral(
        Dropdown::multi("dd-2", "Multiselect", &mut s.multi)
            .items(animals())
            .appearance(InputAppearance::FilledDarker)
            .placeholder("Pick several"),
    );
    ui.mineral(
        Combobox::multi("combo-1", "Combobox", &mut s.combo, &mut s.combo_text)
            .items(animals())
            .size(Size::Large)
            .placeholder("Type to filter"),
    );
    ui.mineral(
        TimePicker::new("tp-1", "Time", &mut s.time)
            .freeform(true)
            .placeholder("Select a time"),
    );
    let mut none = Selection::single();
    ui.mineral(
        Dropdown::multi("dd-3", "Disabled", &mut none)
            .items(animals())
            .disabled(true),
    );
}

fn tags_and_dates(ui: &mut FluentUi<'_>, s: &mut State) {
    heading(ui, "h-tags", "TagPicker, Calendar and DatePicker");
    ui.mineral(
        TagPicker::new("tag-1", "Pets", &mut s.tags, &mut s.tags_text)
            .items(animals())
            .placeholder("Pick pets"),
    );
    ui.mineral(
        TagPicker::new("tag-2", "Pets button", &mut s.tag_button, &mut s.tag_button_text)
            .items(animals())
            .control(TagPickerControl::Button)
            .size(TagPickerSize::Large)
            .placeholder("Pick pets"),
    );
    ui.mineral(Calendar::new("cal-1", "Day", &mut s.day).week_numbers(true));
    ui.mineral(DatePicker::new("date-1", "Date", &mut s.date).placeholder("Select a date"));
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, egui_mineral_fluent::prelude::KeySalt)]
pub(crate) enum Paint {
    Red,
    Green,
    Blue,
    Gold,
    Plum,
    Gray,
}

fn swatches(ui: &mut FluentUi<'_>, s: &mut State) {
    heading(ui, "h-swatch", "SwatchPicker");
    let colors = [
        (Paint::Red, "Red", Color32::from_rgb(0xd1, 0x34, 0x38)),
        (Paint::Green, "Green", Color32::from_rgb(0x10, 0x7c, 0x10)),
        (Paint::Blue, "Blue", Color32::from_rgb(0x00, 0x78, 0xd4)),
        (Paint::Gold, "Gold", Color32::from_rgb(0xff, 0xb9, 0x00)),
        (Paint::Plum, "Plum", Color32::from_rgb(0x88, 0x17, 0x98)),
        (Paint::Gray, "Gray", Color32::from_rgb(0x7a, 0x7a, 0x7a)),
    ];
    let all = || {
        colors
            .iter()
            .map(|&(v, l, c)| (v, Swatch::color(l, c).disabled(v == Paint::Gray)))
    };
    SwatchPicker::optional("sw-1", "Color", &mut s.swatch)
        .items(all())
        .size(SwatchSize::Large)
        .show(ui);
    SwatchPicker::optional("sw-2", "Shape", &mut s.swatch)
        .items(all())
        .shape(SwatchShape::Circular)
        .size(SwatchSize::Small)
        .show(ui);
    SwatchPicker::optional("sw-3", "Grid", &mut s.swatch)
        .items(all())
        .shape(SwatchShape::Rounded)
        .layout(SwatchLayout::Grid { columns: 3 })
        .show(ui);
}
