use egui::{Align, Rect, vec2};
use egui_mineral::{
    layout::{Axis, Flow, Zones},
    range::RangeModel,
    select::Selection,
    widget::SizeExt,
};
use egui_mineral_fluent::{
    Size,
    actions::{CompoundButton, MenuButton, PushButton, SplitButton, TextLink, ToggleButton},
    data::{
        Avatar, AvatarGroup, AvatarSize, InteractionTag, Persona, PersonaSize, Tag, TagGroup, TagSize, Text, TextSize,
    },
    feedback::{
        Badge, BadgeSize, CounterBadge, PresenceBadge, PresenceStatus, ProgressBar, SkeletonItem, Spinner, SpinnerSize,
    },
    icons::ICON_ADD,
    inputs::{
        Checkbox, Combobox, DatePicker, Dropdown, FieldLabel, InfoLabel, Input, ListOption, Radio, RadioGroup,
        RadioLayout, RadioOption, RangeSlider, RangeValue, Rating, RatingDisplay, RatingSize, SearchBox, Select,
        Slider, SpinButton, Swatch, SwatchPicker, SwatchSize, Switch, TagPicker, TagPickerSize, Textarea, TimePicker,
    },
    navigation::{Breadcrumb, BreadcrumbItem, Tab, TabList, Toolbar, ToolbarButton},
    surfaces::Divider,
};

use crate::support::{FluentUi, harness};

// Visual-box drift a row or column may show: half a point, below one physical pixel at 1x.
const TOLERANCE: f32 = 0.5;

const ITEM_WIDTH: f32 = 240.0;

type Add = Box<dyn Fn(&mut FluentUi<'_>) -> Rect>;

fn leaf(name: impl Into<String>, add: impl Fn(&mut FluentUi<'_>) -> Zones + 'static) -> (String, Add) {
    (name.into(), Box::new(move |ui| add(ui).visual))
}

// Groups report their visual box as the group response.
fn group(name: impl Into<String>, add: impl Fn(&mut FluentUi<'_>) -> egui::Response + 'static) -> (String, Add) {
    (name.into(), Box::new(move |ui| add(ui).rect))
}

fn options() -> Vec<ListOption<&'static str>> {
    vec![ListOption::new("cat", "Cat"), ListOption::new("dog", "Dog")]
}

fn sized() -> Vec<(String, Add)> {
    Size::ALL
        .iter()
        .flat_map(|&size| [actions(size), choices(size), fields(size)])
        .flatten()
        .collect()
}

fn actions(size: Size) -> Vec<(String, Add)> {
    let n = |kind: &str| format!("{kind} {size:?}");
    vec![
        leaf(n("PushButton"), move |ui| {
            ui.mineral(PushButton::new("c", "Go").size(size)).zones()
        }),
        leaf(n("PushButton icon"), move |ui| {
            ui.mineral(PushButton::new("c", "Go").size(size).icon(ICON_ADD)).zones()
        }),
        leaf(n("PushButton icon only"), move |ui| {
            let button = PushButton::new("c", "Add").size(size).icon(ICON_ADD);
            ui.mineral(button.label_visible(false)).zones()
        }),
        leaf(n("ToggleButton"), move |ui| {
            ui.mineral(ToggleButton::new("c", "Bold", &mut true).size(size)).zones()
        }),
        leaf(n("CompoundButton"), move |ui| {
            ui.mineral(CompoundButton::new("c", "Go", "Now").size(size)).zones()
        }),
        leaf(n("MenuButton"), move |ui| {
            ui.mineral(MenuButton::new("c", "Menu").size(size)).zones()
        }),
        group(n("SplitButton"), move |ui| {
            let r = SplitButton::new("c", "Save").size(size).show(ui);
            r.response.union(r.menu_response)
        }),
        leaf(n("FieldLabel"), move |ui| {
            ui.mineral(FieldLabel::new("c", "Label").size(size)).zones()
        }),
        leaf(n("InfoLabel"), move |ui| {
            ui.mineral(InfoLabel::new("c", "Label", "Info").size(size)).zones()
        }),
    ]
}

fn choices(size: Size) -> Vec<(String, Add)> {
    let n = |kind: &str| format!("{kind} {size:?}");
    vec![
        leaf(n("Switch"), move |ui| {
            ui.mineral(Switch::new("c", "On", &mut true).size(size)).zones()
        }),
        leaf(n("Checkbox"), move |ui| {
            ui.mineral(Checkbox::new("c", "Check", &mut true).size(size)).zones()
        }),
        leaf(n("Slider"), move |ui| {
            let mut value = 4.0;
            let slider = Slider::new("c", "Volume", RangeModel::new(0.0, 10.0), &mut value);
            ui.mineral(slider.size(size)).zones()
        }),
        leaf(n("Slider vertical"), move |ui| {
            let mut value = 4.0;
            let slider = Slider::new("c", "Volume", RangeModel::new(0.0, 10.0), &mut value);
            ui.mineral(slider.size(size).axis(Axis::Vertical)).zones()
        }),
        leaf(n("RangeSlider"), move |ui| {
            let mut value = RangeValue::new(2.0, 6.0);
            let slider = RangeSlider::new("c", "Price", RangeModel::new(0.0, 10.0), &mut value);
            ui.mineral(slider.size(size)).zones()
        }),
        leaf(n("SpinButton"), move |ui| {
            let mut value = 4.0;
            let spin = SpinButton::new("c", "Count", RangeModel::new(0.0, 10.0), &mut value);
            ui.mineral(spin.size(size)).zones()
        }),
    ]
}

fn fields(size: Size) -> Vec<(String, Add)> {
    let n = |kind: &str| format!("{kind} {size:?}");
    vec![
        leaf(n("Input"), move |ui| {
            ui.mineral(Input::new("c", "Name", &mut String::new()).size(size))
                .zones()
        }),
        leaf(n("SearchBox"), move |ui| {
            ui.mineral(SearchBox::new("c", "Search", &mut String::new()).size(size))
                .zones()
        }),
        leaf(n("Textarea"), move |ui| {
            ui.mineral(Textarea::new("c", "Notes", &mut String::new()).size(size))
                .zones()
        }),
        leaf(n("Select"), move |ui| {
            ui.mineral(Select::new("c", "Pet", options(), &mut None).size(size))
                .zones()
        }),
        leaf(n("Dropdown"), move |ui| {
            let mut picked = Selection::single();
            ui.mineral(Dropdown::new("c", "Pet", options(), &mut picked).size(size))
                .zones()
        }),
        leaf(n("Combobox"), move |ui| {
            let (mut picked, mut text) = (Selection::single(), String::new());
            ui.mineral(Combobox::new("c", "Pet", options(), &mut picked, &mut text).size(size))
                .zones()
        }),
        leaf(n("TimePicker"), move |ui| {
            ui.mineral(TimePicker::new("c", "Time", &mut None).size(size)).zones()
        }),
        leaf(n("DatePicker"), move |ui| {
            ui.mineral(DatePicker::new("c", "Date", &mut None).size(size)).zones()
        }),
        group(n("TabList"), move |ui| {
            let tabs = [Tab::new("One"), Tab::new("Two")];
            TabList::new("c", "Tabs", &mut 0)
                .tabs(tabs.into_iter().enumerate())
                .size(size)
                .show(ui)
                .response
        }),
        group(n("Toolbar"), move |ui| {
            let bar = Toolbar::new("c", "Tools").size(size);
            bar.action(ToolbarButton::new("Add").icon(ICON_ADD)).show(ui).response
        }),
        group(n("Breadcrumb"), move |ui| {
            let items = [BreadcrumbItem::new("Home"), BreadcrumbItem::new("Here")];
            let crumbs = Breadcrumb::new("c", "Path").items(items);
            crumbs.size(size).show(ui).response
        }),
    ]
}

fn singles() -> Vec<(String, Add)> {
    vec![
        leaf("TextLink", |ui| ui.mineral(TextLink::new("c", "Link")).zones()),
        leaf("Radio", |ui| ui.mineral(Radio::new("c", "Pear")).zones()),
        group("RadioGroup", |ui| {
            let options = [RadioOption::new(0, "Apple"), RadioOption::new(1, "Pear")];
            let mut value = 0;
            let group = RadioGroup::new("c", "Fruit", &mut value).options(options);
            group.layout(RadioLayout::Horizontal).show(ui).response
        }),
        leaf("ProgressBar", |ui| {
            ui.mineral(ProgressBar::determinate("c", 0.4).preferred_width(80.0))
                .zones()
        }),
        leaf("SkeletonItem", |ui| {
            ui.mineral(SkeletonItem::new("c").size(16).preferred_width(80.0))
                .zones()
        }),
        leaf("CounterBadge", |ui| ui.mineral(CounterBadge::new("c", 5)).zones()),
        leaf("Divider vertical", |ui| {
            ui.mineral(Divider::new("c").axis(Axis::Vertical)).zones()
        }),
        group("AvatarGroup", |ui| {
            AvatarGroup::new("c", "People")
                .item(Avatar::new("a", "Ada Lovelace"))
                .item(Avatar::new("b", "Alan Turing"))
                .show(ui)
                .response
        }),
        group("InteractionTag", |ui| {
            InteractionTag::new("c", "Tag").dismissible(true).show(ui).response
        }),
        group("TagGroup", |ui| {
            let group = TagGroup::new("c", "Tags").tag(0, Tag::new("a", "One"));
            group
                .interaction_tag(1, InteractionTag::new("b", "Two"))
                .show(ui)
                .response
        }),
    ]
}

fn families() -> Vec<(String, Add)> {
    let mut out = singles();
    for &size in BadgeSize::ALL {
        out.push(leaf(format!("Badge {size:?}"), move |ui| {
            ui.mineral(Badge::new("c", "1").size(size)).zones()
        }));
        out.push(leaf(format!("PresenceBadge {size:?}"), move |ui| {
            ui.mineral(PresenceBadge::new("c", PresenceStatus::Available).size(size))
                .zones()
        }));
    }
    for &size in SpinnerSize::ALL {
        out.push(leaf(format!("Spinner {size:?}"), move |ui| {
            ui.mineral(Spinner::new("c").size(size)).zones()
        }));
        out.push(leaf(format!("Spinner label {size:?}"), move |ui| {
            ui.mineral(Spinner::new("c").size(size).label("Busy")).zones()
        }));
    }
    for &size in RatingSize::ALL {
        out.push(leaf(format!("Rating {size:?}"), move |ui| {
            ui.mineral(Rating::new("c", "Rating", &mut 3.0).size(size)).zones()
        }));
        out.push(leaf(format!("RatingDisplay {size:?}"), move |ui| {
            ui.mineral(RatingDisplay::new("c", "Reviews", 4.5).count(12).size(size))
                .zones()
        }));
    }
    for &size in TextSize::ALL {
        out.push(leaf(format!("Text {size:?}"), move |ui| {
            ui.mineral(Text::new("c", "Text").size(size)).zones()
        }));
    }
    for &size in AvatarSize::ALL {
        out.push(leaf(format!("Avatar {size:?}"), move |ui| {
            ui.mineral(Avatar::new("c", "Ada Lovelace").size(size)).zones()
        }));
    }
    for &size in PersonaSize::ALL {
        out.push(leaf(format!("Persona {size:?}"), move |ui| {
            ui.mineral(Persona::new("c", "Ada Lovelace").secondary_text("Engineer").size(size))
                .zones()
        }));
    }
    for &size in TagSize::ALL {
        out.push(leaf(format!("Tag {size:?}"), move |ui| {
            ui.mineral(Tag::new("c", "Tag").size(size).dismissible(true)).zones()
        }));
    }
    for &size in TagPickerSize::ALL {
        out.push(leaf(format!("TagPicker {size:?}"), move |ui| {
            let (mut picked, mut text) = (Selection::multiple(), String::new());
            ui.mineral(TagPicker::new("c", "Pets", options(), &mut picked, &mut text).size(size))
                .zones()
        }));
    }
    for &size in SwatchSize::ALL {
        out.push(group(format!("SwatchPicker {size:?}"), move |ui| {
            let swatches = [
                ("red", Swatch::color("Red", egui::Color32::RED)),
                ("blue", Swatch::color("Blue", egui::Color32::BLUE)),
            ];
            let mut value = None;
            let picker = SwatchPicker::new("c", "Color", &mut value).swatches(swatches);
            picker.size(size).show(ui).response
        }));
    }
    out
}

fn controls() -> impl Iterator<Item = (String, Add)> {
    sized().into_iter().chain(families())
}

fn reference(ui: &mut FluentUi<'_>) -> Rect {
    ui.mineral(PushButton::new("reference", "Reference")).zones().visual
}

#[test]
fn controls_in_a_row_share_a_vertical_center() {
    let mut off = Vec::new();
    for (name, add) in controls() {
        let mut h = harness([Rect::NOTHING; 2], move |ui, zones| {
            Flow::new(vec2(8.0, 8.0), Align::Center).show(ui, |row| {
                *zones = [
                    row.add(reference),
                    row.add(|ui| {
                        // Fields fill their constraint; both items stay on one line.
                        ui.set_max_width(ITEM_WIDTH);
                        add(ui)
                    }),
                ];
            });
        });
        h.run();
        let [a, b] = h.state().value;
        let drift = b.center().y - a.center().y;
        // An unbounded item height must not leak into the box: `NaN` and infinite drifts fail too.
        if !(drift.abs() <= TOLERANCE && b.height() > 0.0) {
            off.push(format!("{name}: {drift}"));
        }
    }
    assert!(off.is_empty(), "visual centers off the row center: {off:#?}");
}

#[test]
fn controls_in_a_column_share_a_left_edge() {
    let mut off = Vec::new();
    for (name, add) in controls() {
        let mut h = harness([Rect::NOTHING; 2], move |ui, zones| {
            *zones = [reference(ui), add(ui)];
        });
        h.run();
        let [a, b] = h.state().value;
        let drift = b.left() - a.left();
        if drift.abs() > TOLERANCE {
            off.push(format!("{name}: {drift}"));
        }
    }
    assert!(off.is_empty(), "visual boxes off the column edge: {off:#?}");
}
