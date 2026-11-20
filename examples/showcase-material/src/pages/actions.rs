use egui::vec2;
use egui_mineral_material::MaterialExt;
use egui_mineral_material::{
    catalog::Component,
    icons::{
        ICON_ADD, ICON_ARCHIVE, ICON_CAMERA, ICON_CHAT, ICON_CONTENT_COPY, ICON_DELETE, ICON_EDIT, ICON_FAVORITE,
        ICON_FORMAT_BOLD, ICON_FORMAT_ITALIC, ICON_FORMAT_UNDERLINED, ICON_IMAGE, ICON_SAVE, ICON_SEND, ICON_SHARE,
        ICON_STAR, ICON_VIEW_LIST, ICON_VIEW_MODULE,
    },
    prelude::*,
};

use super::{Shared, section, sub};

pub(crate) const TOPICS: &[&str] = &[
    "Button",
    "Icon button",
    "FAB",
    "Extended FAB",
    "FAB menu",
    "Button group",
    "Segmented button",
    "Split button",
];

const VARIANTS: [(ButtonVariant, &str); 5] = [
    (ButtonVariant::Elevated, "Elevated"),
    (ButtonVariant::Filled, "Filled"),
    (ButtonVariant::Tonal, "Tonal"),
    (ButtonVariant::Outlined, "Outlined"),
    (ButtonVariant::Text, "Text"),
];

const TOGGLE_VARIANTS: [(ContainerVariant, &str); 4] = [
    (ContainerVariant::Elevated, "Elevated"),
    (ContainerVariant::Filled, "Filled"),
    (ContainerVariant::Tonal, "Tonal"),
    (ContainerVariant::Outlined, "Outlined"),
];

const SPLIT_VARIANTS: [(ContainerVariant, &str); 4] = [
    (ContainerVariant::Elevated, "Elevated"),
    (ContainerVariant::Filled, "Filled"),
    (ContainerVariant::Tonal, "Tonal"),
    (ContainerVariant::Outlined, "Outlined"),
];

const SIZES: [(Size, &str); 5] = [
    (Size::XSmall, "XSmall"),
    (Size::Small, "Small"),
    (Size::Medium, "Medium"),
    (Size::Large, "Large"),
    (Size::XLarge, "XLarge"),
];

const FAB_COLORS: [(FabColors, &str); 4] = [
    (FabColors::Surface, "Surface"),
    (FabColors::PrimaryContainer, "Primary"),
    (FabColors::SecondaryContainer, "Secondary"),
    (FabColors::TertiaryContainer, "Tertiary"),
];

pub(crate) struct State {
    shape_round: bool,
    shape_square: bool,
    toggle_icon: bool,
    toggle_text: bool,
    toggle_group: [bool; 4],
    icon_toggles: [bool; 4],
    ext_expanded: bool,
    group_single: Selection<usize>,
    group_multi: Selection<usize>,
    group_connected: Selection<usize>,
    segment_period: usize,
    segment_view: usize,
    segment_layer: Option<usize>,
    segment_days: Selection<usize>,
    segment_sizes: Selection<usize>,
    last_menu: String,
}

impl Default for State {
    fn default() -> Self {
        Self {
            shape_round: false,
            shape_square: true,
            toggle_icon: false,
            toggle_text: true,
            toggle_group: [false; 4],
            icon_toggles: [false, true, false, true],
            ext_expanded: true,
            group_single: Selection::single().required(true).with([0]),
            group_multi: Selection::multiple().with([0, 2]),
            group_connected: Selection::single().required(true).with([1]),
            segment_period: 0,
            segment_view: 1,
            segment_layer: None,
            segment_days: Selection::multiple().with([0, 2]),
            segment_sizes: Selection::multiple().with([1]),
            last_menu: "none".to_owned(),
        }
    }
}

pub(crate) fn show(ui: &mut MaterialUi<'_>, s: &mut State, shared: &mut Shared) {
    button(ui, s, shared);
    icon_button(ui, s, shared);
    fab(ui, shared);
    extended_fab(ui, s, shared);
    fab_menu(ui, shared);
    button_group(ui, s, shared);
    segmented_button(ui, s);
    split_button(ui, s);
}

fn button(ui: &mut MaterialUi<'_>, s: &mut State, shared: &mut Shared) {
    section(ui, TOPICS[0], Some(PushButton::META), |ui| {
        sub(ui, "Variants");
        super::row(ui, |row| {
            for (i, (v, name)) in VARIANTS.into_iter().enumerate() {
                if row.mineral(PushButton::new(("btn-var", i), name).variant(v)).clicked() {
                    shared.toast(format!("{name} button pressed"));
                }
            }
        });
        sub(ui, "With leading icon");
        super::row(ui, |row| {
            for (i, (v, name)) in VARIANTS.into_iter().enumerate() {
                if row
                    .mineral(PushButton::new(("btn-ico", i), name).variant(v).icon(ICON_ADD))
                    .clicked()
                {
                    shared.toast(format!("{name} button with icon pressed"));
                }
            }
        });
        sub(ui, "Sizes");
        super::row(ui, |row| {
            for (i, (sz, name)) in SIZES.into_iter().enumerate() {
                if row
                    .mineral(
                        PushButton::new(("btn-sz", i), name)
                            .size(sz)
                            .variant(ButtonVariant::Filled),
                    )
                    .clicked()
                {
                    shared.toast(format!("{name} button pressed"));
                }
            }
        });
        sub(ui, "Sizes with icon, tonal");
        super::row(ui, |row| {
            for (i, (sz, name)) in SIZES.into_iter().enumerate() {
                if row
                    .mineral(
                        PushButton::new(("btn-szi", i), name)
                            .size(sz)
                            .variant(ButtonVariant::Tonal)
                            .icon(ICON_SEND),
                    )
                    .clicked()
                {
                    shared.toast(format!("{name} button sent"));
                }
            }
        });
        sub(ui, "Shapes (round, square) and toggle morph; click to toggle");
        super::row(ui, |row| {
            row.mineral(ToggleButton::new("btn-shape-round", "Round", &mut s.shape_round).shape(ButtonShape::Round));
            row.mineral(
                ToggleButton::new("btn-shape-square", "Square", &mut s.shape_square).shape(ButtonShape::Square),
            );
            row.mineral(
                ToggleButton::new("btn-toggle-icon", "Favorite", &mut s.toggle_icon)
                    .variant(ContainerVariant::Tonal)
                    .icon(ICON_FAVORITE),
            );
            row.mineral(
                ToggleButton::new("btn-toggle-text", "Outlined toggle", &mut s.toggle_text)
                    .variant(ContainerVariant::Outlined),
            );
        });
        sub(ui, "Toggle in every variant");
        super::row(ui, |row| {
            for (i, (v, name)) in TOGGLE_VARIANTS.into_iter().enumerate() {
                row.mineral(ToggleButton::new(("btn-tg", i), name, &mut s.toggle_group[i]).variant(v));
            }
        });
        sub(ui, "Disabled");
        super::row(ui, |row| {
            for (i, (v, name)) in VARIANTS.into_iter().enumerate() {
                row.mineral(
                    PushButton::new(("btn-dis", i), name)
                        .variant(v)
                        .icon(ICON_SAVE)
                        .disabled(true),
                );
            }
        });
    });
}

fn icon_button(ui: &mut MaterialUi<'_>, s: &mut State, shared: &mut Shared) {
    section(
        ui,
        TOPICS[1],
        Some(<egui_mineral_material::actions::IconButton>::META),
        |ui| {
            let variants = [
                (IconButtonVariant::Standard, "Standard"),
                (IconButtonVariant::Filled, "Filled"),
                (IconButtonVariant::Tonal, "Tonal"),
                (IconButtonVariant::Outlined, "Outlined"),
            ];
            sub(ui, "Variants");
            super::row(ui, |row| {
                for (i, (v, name)) in variants.into_iter().enumerate() {
                    if row
                        .mineral(IconButton::new(("ib-var", i), ICON_SHARE, name).variant(v))
                        .clicked()
                    {
                        shared.toast(format!("{name} icon button"));
                    }
                }
            });
            sub(ui, "Toggle (click to flip)");
            super::row(ui, |row| {
                for (i, (v, name)) in variants.into_iter().enumerate() {
                    row.mineral(IconButton::toggle(("ib-tg", i), ICON_STAR, name, &mut s.icon_toggles[i]).variant(v));
                }
            });
            sub(ui, "Widths (narrow, default, wide)");
            super::row(ui, |row| {
                for (i, w) in [IconButtonWidth::Narrow, IconButtonWidth::Default, IconButtonWidth::Wide]
                    .into_iter()
                    .enumerate()
                {
                    if row
                        .mineral(
                            IconButton::new(("ib-w", i), ICON_EDIT, "Edit")
                                .variant(IconButtonVariant::Tonal)
                                .width(w),
                        )
                        .clicked()
                    {
                        shared.toast(format!("Edit icon button, width {w:?}"));
                    }
                }
            });
            sub(ui, "Sizes");
            super::row(ui, |row| {
                for (i, (sz, name)) in SIZES.into_iter().enumerate() {
                    if row
                        .mineral(
                            IconButton::new(("ib-sz", i), ICON_CAMERA, name)
                                .variant(IconButtonVariant::Filled)
                                .size(sz),
                        )
                        .clicked()
                    {
                        shared.toast(format!("{name} camera icon button"));
                    }
                }
            });
            sub(ui, "Square shape");
            super::row(ui, |row| {
                for (i, (v, name)) in variants.into_iter().enumerate() {
                    if row
                        .mineral(
                            IconButton::new(("ib-sq", i), ICON_DELETE, name)
                                .variant(v)
                                .shape(ButtonShape::Square),
                        )
                        .clicked()
                    {
                        shared.toast(format!("{name} square icon button"));
                    }
                }
            });
            sub(ui, "Disabled");
            super::row(ui, |row| {
                for (i, (v, name)) in variants.into_iter().enumerate() {
                    row.mineral(
                        IconButton::new(("ib-dis", i), ICON_ARCHIVE, name)
                            .variant(v)
                            .disabled(true),
                    );
                }
            });
        },
    );
}

fn fab(ui: &mut MaterialUi<'_>, shared: &mut Shared) {
    section(ui, TOPICS[2], Some(Fab::META), |ui| {
        sub(ui, "Colors (regular)");
        super::row(ui, |row| {
            for (i, (c, name)) in FAB_COLORS.into_iter().enumerate() {
                if row.mineral(Fab::new(("fab-c", i), ICON_ADD, name).colors(c)).clicked() {
                    shared.toast(format!("{name} FAB"));
                }
            }
        });
        sub(ui, "Sizes (small, regular, medium, large)");
        super::row(ui, |row| {
            for (i, sz) in [FabSize::Small, FabSize::Baseline, FabSize::Medium, FabSize::Large]
                .into_iter()
                .enumerate()
            {
                if row
                    .mineral(
                        Fab::new(("fab-s", i), ICON_EDIT, "Edit")
                            .size(sz)
                            .colors(FabColors::PrimaryContainer),
                    )
                    .clicked()
                {
                    shared.toast(format!("FAB size {sz:?}"));
                }
            }
        });
        sub(ui, "Disabled");
        ui.mineral(Fab::new("fab-dis", ICON_ADD, "Add").disabled(true));
    });
}

fn extended_fab(ui: &mut MaterialUi<'_>, s: &mut State, shared: &mut Shared) {
    section(ui, TOPICS[3], Some(ExtendedFab::META), |ui| {
        sub(ui, "Sizes and colors");
        super::row(ui, |row| {
            for (i, sz) in [ExtendedFabSize::Small, ExtendedFabSize::Medium, ExtendedFabSize::Large]
                .into_iter()
                .enumerate()
            {
                let (c, name) = FAB_COLORS[i + 1];
                if row
                    .mineral(
                        ExtendedFab::new(("efab-s", i), ICON_EDIT, format!("Compose ({name})"))
                            .size(sz)
                            .colors(c),
                    )
                    .clicked()
                {
                    shared.toast(format!("Extended FAB ({name}) pressed"));
                }
            }
        });
        sub(ui, "Collapses to a FAB");
        super::row(ui, |row| {
            if row
                .mineral(
                    ExtendedFab::new("efab-exp", ICON_CHAT, "New chat")
                        .expanded(s.ext_expanded)
                        .colors(FabColors::TertiaryContainer),
                )
                .clicked()
            {
                shared.toast("New chat started");
            }
            let r = row.mineral(
                PushButton::new("efab-toggle", if s.ext_expanded { "Collapse" } else { "Expand" })
                    .variant(ButtonVariant::Text),
            );
            s.ext_expanded ^= r.clicked();
        });
        sub(ui, "Disabled");
        ui.mineral(ExtendedFab::new("efab-dis", ICON_ADD, "Create").disabled(true));
    });
}

fn fab_menu(ui: &mut MaterialUi<'_>, shared: &mut Shared) {
    let rt = ui.rt();
    section(ui, TOPICS[4], Some(FabMenu::META), |ui| {
        ui.label("Opens upward from the bottom end of the box.");
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(vec2(width, 300.0), egui::Sense::hover());
        let t = rt.tokens();
        ui.painter()
            .rect_filled(rect, f32::from(t.shape.medium), t.color.surface_container_low);
        let mut inner = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect)
                .layout(egui::Layout::bottom_up(egui::Align::Max)),
        );
        let r = FabMenu::new("fab-menu", ICON_ADD, "Actions")
            .colors(FabMenuColors::PrimaryContainer)
            .items([
                FabMenuItem::new(ICON_CAMERA, "Camera"),
                FabMenuItem::new(ICON_IMAGE, "Gallery"),
                FabMenuItem::new(ICON_CONTENT_COPY, "Duplicate"),
            ])
            .show(&mut rt.ui(&mut inner));
        if let Some(i) = r.clicked {
            shared.toast(format!("FAB menu item {}", i.index()));
        }
    });
}

fn button_group(ui: &mut MaterialUi<'_>, s: &mut State, shared: &mut Shared) {
    section(ui, TOPICS[5], Some(<ButtonGroup<'static, ()>>::META), |ui| {
        sub(ui, "Standard, plain actions");
        let r = ButtonGroup::new("grp-std", "Actions")
            .items([
                ButtonGroupItem::new("Copy"),
                ButtonGroupItem::new("Paste"),
                ButtonGroupItem::new("Cut"),
            ])
            .button_variant(ContainerVariant::Tonal)
            .show(ui);
        if let Some(i) = r.clicked {
            shared.toast(format!("Group item {}", i.index()));
        }
        sub(ui, "Standard, single selection (mixed text/icon)");
        ButtonGroup::selection("grp-single", "Alignment", &mut s.group_single)
            .toggles(
                [
                    ButtonGroupItem::new("Left").icon(ICON_EDIT),
                    ButtonGroupItem::new("Star").icon(ICON_STAR).label_visible(false),
                    ButtonGroupItem::new("Disabled").disabled(true),
                ]
                .into_iter()
                .enumerate(),
            )
            .button_variant(ContainerVariant::Outlined)
            .show(ui);
        sub(ui, "Standard, multi selection, sizes");
        for (i, (sz, name)) in SIZES.into_iter().enumerate().take(3) {
            ButtonGroup::selection(("grp-multi", i), name, &mut s.group_multi)
                .toggles(
                    [
                        ButtonGroupItem::new("Bold").icon(ICON_FORMAT_BOLD).label_visible(false),
                        ButtonGroupItem::new("Italic")
                            .icon(ICON_FORMAT_ITALIC)
                            .label_visible(false),
                        ButtonGroupItem::new("Underline")
                            .icon(ICON_FORMAT_UNDERLINED)
                            .label_visible(false),
                    ]
                    .into_iter()
                    .enumerate(),
                )
                .button_variant(ContainerVariant::Filled)
                .size(sz)
                .show(ui);
        }
        sub(ui, "Connected, single selection");
        ButtonGroup::selection("grp-conn", "Period", &mut s.group_connected)
            .variant(ButtonGroupVariant::Connected)
            .toggles(
                [
                    ButtonGroupItem::new("Day"),
                    ButtonGroupItem::new("Week"),
                    ButtonGroupItem::new("Month"),
                ]
                .into_iter()
                .enumerate(),
            )
            .button_variant(ContainerVariant::Tonal)
            .shape(ButtonShape::Square)
            .show(ui);
        sub(ui, "Disabled");
        ButtonGroup::new("grp-dis", "Disabled")
            .variant(ButtonGroupVariant::Connected)
            .items([ButtonGroupItem::new("One"), ButtonGroupItem::new("Two")])
            .disabled(true)
            .show(ui);
    });
}

fn segmented_button(ui: &mut MaterialUi<'_>, s: &mut State) {
    section(ui, TOPICS[6], Some(<SegmentedButton<'static, ()>>::META), |ui| {
        sub(ui, "Single choice");
        SegmentedButton::single("seg-period", "Period", &mut s.segment_period)
            .items(["Day", "Week", "Month", "Year"].into_iter().enumerate())
            .show(ui);
        sub(ui, "Single choice, icons, one disabled");
        SegmentedButton::single("seg-view", "View", &mut s.segment_view)
            .item(0, Segment::new("List").icon(ICON_VIEW_LIST))
            .item(1, Segment::new("Grid").icon(ICON_VIEW_MODULE))
            .item(2, Segment::new("Gallery").icon(ICON_IMAGE).disabled(true))
            .show(ui);
        sub(ui, "Content-width segments, nothing selected until a click");
        SegmentedButton::optional("seg-layer", "Layer", &mut s.segment_layer)
            .items(["Field", "Both", "Intensity"].into_iter().enumerate())
            .equal_width(false)
            .show(ui);
        sub(ui, "Multiple choice");
        SegmentedButton::multi("seg-days", "Days", &mut s.segment_days)
            .items(["Mon", "Tue", "Wed", "Thu", "Fri"].into_iter().enumerate())
            .show(ui);
        sub(ui, "Disabled");
        SegmentedButton::multi("seg-sizes", "Sizes", &mut s.segment_sizes)
            .items(["S", "M", "L"].into_iter().enumerate())
            .disabled(true)
            .show(ui);
    });
}

fn split_button(ui: &mut MaterialUi<'_>, s: &mut State) {
    section(ui, TOPICS[7], Some(SplitButton::META), |ui| {
        let menu = || {
            Menu::new("split-menu", "More save options")
                .item(MenuItem::new("Save as").leading(ICON_SAVE))
                .item(MenuItem::new("Duplicate").leading(ICON_CONTENT_COPY))
                .item(MenuItem::new("Archive").leading(ICON_ARCHIVE).disabled(true))
        };
        sub(ui, "Variants");
        super::row(ui, |row| {
            for (i, (v, name)) in SPLIT_VARIANTS.into_iter().enumerate() {
                let r = row.add(|ui| {
                    SplitButton::new(("split-v", i), name, menu())
                        .variant(v)
                        .icon(ICON_SAVE)
                        .show(ui)
                });
                if r.leading_clicked {
                    s.last_menu = format!("{name} main action");
                }
                if let Some(m) = r.clicked {
                    s.last_menu = format!("{name} menu item {}", m.index());
                }
            }
        });
        sub(ui, "Sizes");
        super::row(ui, |row| {
            for (i, (sz, name)) in SIZES.into_iter().enumerate() {
                let r = row.add(|ui| SplitButton::new(("split-s", i), name, menu()).size(sz).show(ui));
                if r.leading_clicked {
                    s.last_menu = format!("{name} size main action");
                }
                if let Some(m) = r.clicked {
                    s.last_menu = format!("{name} size menu item {}", m.index());
                }
            }
        });
        ui.label(format!("Last: {}", s.last_menu));
        sub(ui, "Disabled");
        SplitButton::new("split-dis", "Save", menu()).disabled(true).show(ui);
    });
}
