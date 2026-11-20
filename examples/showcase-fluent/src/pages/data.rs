use egui::{Color32, ColorImage, TextureHandle, TextureOptions, vec2};
use egui_mineral_fluent::{
    data::{
        Avatar, AvatarActive, AvatarActiveAppearance, AvatarColor, AvatarGroup, AvatarGroupLayout, AvatarShape,
        AvatarSize, DataGrid, Image, ImageFit, ImageShape, InteractionTag, List, ListItem, ListNavigation, Persona,
        PersonaSize, PersonaTextAlignment, PersonaTextPosition, Table, TableCellLayout, TableColumn,
        TableRowAppearance, TableSize, Tag, TagAppearance, TagGroup, TagShape, TagSize, Text,
    },
    egui_mineral::{
        select::Selection,
        table::{ColumnSize, ColumnWidths, SortState, sort_order},
    },
    feedback::PresenceStatus,
    icons::ICON_PERSON,
};

use super::{heading, row, row_spaced};
use crate::FluentUi;

#[derive(Default)]
pub(crate) struct State {
    table_sort: SortState,
    table_selection: Option<Selection<usize>>,
    grid_sort: SortState,
    grid_selection: Option<Selection<usize>>,
    grid_widths: Option<ColumnWidths>,
    tags: Option<Vec<String>>,
    tag_selection: Option<Selection<String>>,
    texture: Option<TextureHandle>,
    single: Selection<usize>,
    multiple: Option<Selection<usize>>,
}

egui_mineral_fluent::egui_mineral::kit::opaque_debug!(State);

fn gradient() -> ColorImage {
    let (w, h) = (64, 32);
    let pixels = (0..w * h)
        .map(|i| {
            let (x, y) = (i % w, i / w);
            Color32::from_rgb((x * 4) as u8, (y * 8) as u8, 160)
        })
        .collect();
    ColorImage::new([w, h], pixels)
}

pub(crate) fn show(ui: &mut FluentUi<'_>, s: &mut State) {
    avatars(ui);
    personas(ui);
    tags(ui, s);
    tables(ui, s);
    images(ui, s);
    lists(ui, s);
}

const PEOPLE: [&str; 7] = [
    "Kat Larrson",
    "Mauricio August",
    "Robert Tolbert",
    "Cameron Evans",
    "Carole Poland",
    "Ashley McCarthy",
    "Tim Deboer",
];

fn avatars(ui: &mut FluentUi<'_>) {
    heading(ui, "h-avatar", "Avatars");
    row(ui, |ui| {
        for (i, size) in AvatarSize::ALL.iter().copied().enumerate() {
            ui.mineral(Avatar::new(("size", i), PEOPLE[0]).size(size));
        }
    });
    row(ui, |ui| {
        ui.mineral(
            Avatar::new("square", PEOPLE[1])
                .shape(AvatarShape::Square)
                .size(AvatarSize::S48),
        );
        ui.mineral(Avatar::new("icon", "").size(AvatarSize::S48));
        ui.mineral(
            Avatar::new("custom-icon", "")
                .icon(ICON_PERSON)
                .color(AvatarColor::Brand)
                .size(AvatarSize::S48),
        );
        for (i, name) in PEOPLE.iter().enumerate() {
            ui.mineral(
                Avatar::new(("colorful", i), *name)
                    .color(AvatarColor::Colorful)
                    .size(AvatarSize::S48),
            );
        }
    });
    row_spaced(ui, 16.0, |ui| {
        for (i, status) in PresenceStatus::ALL.iter().copied().enumerate() {
            ui.mineral(
                Avatar::new(("presence", i), PEOPLE[2])
                    .color(AvatarColor::Colorful)
                    .size(AvatarSize::S48)
                    .presence(status),
            );
        }
        for (i, active) in [AvatarActive::Active, AvatarActive::Inactive].into_iter().enumerate() {
            for (j, appearance) in AvatarActiveAppearance::ALL.iter().copied().enumerate() {
                ui.mineral(
                    Avatar::new(("active", i * 3 + j), PEOPLE[3])
                        .color(AvatarColor::Brand)
                        .size(AvatarSize::S48)
                        .active(active)
                        .active_appearance(appearance),
                );
            }
        }
    });
    for (i, layout) in AvatarGroupLayout::ALL.iter().copied().enumerate() {
        ui.add_space(8.0);
        let mut group = AvatarGroup::new(("group", i), format!("{layout:?} group"))
            .layout(layout)
            .size(AvatarSize::S32)
            .max_inline_items(3);
        for (j, name) in PEOPLE.iter().enumerate() {
            group = group.item(Avatar::new((("group", i), j), *name).color(AvatarColor::Colorful));
        }
        group.show(ui);
    }
}

fn personas(ui: &mut FluentUi<'_>) {
    heading(ui, "h-persona", "Personas");
    row_spaced(ui, 24.0, |ui| {
        for (i, size) in PersonaSize::ALL.iter().copied().enumerate() {
            ui.mineral(
                Persona::new(("size", i), PEOPLE[i])
                    .size(size)
                    .secondary_text("Software engineer")
                    .avatar(Avatar::new("a", PEOPLE[i]).color(AvatarColor::Colorful))
                    .presence(PresenceStatus::Available),
            );
        }
    });
    row_spaced(ui, 24.0, |ui| {
        for (i, position) in PersonaTextPosition::ALL.iter().copied().enumerate() {
            for (j, alignment) in PersonaTextAlignment::ALL.iter().copied().enumerate() {
                ui.mineral(
                    Persona::new(("pos", i * 2 + j), PEOPLE[3])
                        .text_position(position)
                        .text_alignment(alignment)
                        .secondary_text("Designer")
                        .tertiary_text("Seattle")
                        .quaternary_text("Available"),
                );
            }
        }
        ui.mineral(
            Persona::new("presence-only", PEOPLE[4])
                .presence(PresenceStatus::Busy)
                .presence_only(true)
                .secondary_text("In a meeting"),
        );
    });
}

fn tags(ui: &mut FluentUi<'_>, s: &mut State) {
    heading(ui, "h-tag", "Tags");
    for (i, appearance) in TagAppearance::ALL.iter().copied().enumerate() {
        row(ui, |ui| {
            for (j, size) in TagSize::ALL.iter().copied().enumerate() {
                let shape = if j == 1 { TagShape::Circular } else { TagShape::Rounded };
                ui.mineral(
                    Tag::new(format!("tag-{i}-{j}"), "Tag")
                        .appearance(appearance)
                        .size(size)
                        .shape(shape)
                        .icon(ICON_PERSON),
                );
                ui.mineral(
                    Tag::new(format!("dismiss-{i}-{j}"), "Dismissible")
                        .appearance(appearance)
                        .size(size)
                        .shape(shape)
                        .dismissible(true),
                );
            }
            ui.mineral(
                Tag::new(format!("media-{i}"), "Kat Larrson")
                    .appearance(appearance)
                    .secondary_text("Designer")
                    .media(Avatar::new("a", PEOPLE[0]).color(AvatarColor::Colorful)),
            );
            ui.mineral(
                Tag::new(format!("off-{i}"), "Disabled")
                    .appearance(appearance)
                    .disabled(true),
            );
        });
    }
    let values = s
        .tags
        .get_or_insert_with(|| ["Design", "Code", "Review", "Ship"].map(str::to_owned).to_vec());
    let selection = s.tag_selection.get_or_insert_with(Selection::multiple);
    let mut group = TagGroup::new("tag-group", "Workflow")
        .appearance(TagAppearance::Brand)
        .dismissible(true)
        .selection(selection);
    for v in values.iter() {
        group = group.interaction_tag(v.clone(), InteractionTag::new(v.clone(), v.clone()));
    }
    // Upstream TagGroup does not wrap; narrow windows scroll it.
    let rt = ui.rt();
    let shown = egui::ScrollArea::horizontal()
        .id_salt("tag-group-scroll")
        .show(ui, |ui| group.show(&mut rt.ui(ui)))
        .inner;
    if let Some(gone) = shown.removed {
        values.retain(|v| *v != gone);
    }
}

const ROWS: [(&str, &str, u32); 4] = [
    ("Kat Larrson", "Designer", 4),
    ("Ashley McCarthy", "Engineer", 2),
    ("Tim Deboer", "Researcher", 5),
    ("Cameron Evans", "Manager", 3),
];

fn tables(ui: &mut FluentUi<'_>, s: &mut State) {
    heading(ui, "h-table", "Tables");
    let columns = || {
        [
            TableColumn::new("Person").sortable(true),
            TableColumn::new("Role").sortable(true),
            TableColumn::new("Rank"),
        ]
    };
    let cell = |size: TableSize| {
        move |ui: &mut FluentUi<'_>, row: usize, col: usize| {
            let (name, role, rank) = ROWS[row];
            match col {
                0 => {
                    ui.mineral(
                        TableCellLayout::new((("person", row), size as usize), name)
                            .size(size)
                            .media(Avatar::new((("p", row), size as usize), name).color(AvatarColor::Colorful))
                            .secondary_text(role),
                    );
                }
                1 => {
                    ui.mineral(Text::new((("role", row), size as usize), role));
                }
                _ => {
                    ui.mineral(Text::new((("rank", row), size as usize), rank.to_string()));
                }
            }
        }
    };
    let order = |sort: SortState| {
        sort_order(ROWS.len(), sort, |c| match c.column {
            0 => ROWS[c.row].0,
            _ => ROWS[c.row].1,
        })
    };
    let table_order = order(s.table_sort);
    let selection = s.table_selection.get_or_insert_with(Selection::multiple);
    Table::new("table", "People", columns())
        .size(TableSize::Small)
        .row_appearance(TableRowAppearance::Brand)
        .sort(&mut s.table_sort)
        .selection(selection)
        .row_order(table_order)
        .show(ui, ROWS.len(), cell(TableSize::Small));
    ui.add_space(12.0);
    let widths = s
        .grid_widths
        .get_or_insert_with(|| ColumnWidths::new((0..3).map(|_| ColumnSize { width: 70.0, min: 48.0 })));
    let grid_order = order(s.grid_sort);
    let grid_selection = s.grid_selection.get_or_insert_with(Selection::single);
    DataGrid::new("grid", "People grid", columns())
        .size(TableSize::ExtraSmall)
        .sort(&mut s.grid_sort)
        .selection(grid_selection)
        .resizable(widths)
        .row_order(grid_order)
        .show(ui, ROWS.len(), cell(TableSize::ExtraSmall));
}

fn images(ui: &mut FluentUi<'_>, s: &mut State) {
    heading(ui, "h-image", "Images");
    let texture = s
        .texture
        .get_or_insert_with(|| {
            ui.ctx()
                .load_texture("showcase-gradient", gradient(), TextureOptions::LINEAR)
        })
        .clone();
    row(ui, |ui| {
        for (i, fit) in ImageFit::ALL.iter().copied().enumerate() {
            ui.mineral(
                Image::new(("fit", i), &texture)
                    .alt(format!("{fit:?}"))
                    .fit(fit)
                    .size(vec2(64.0, 64.0))
                    .bordered(true),
            );
        }
        for (i, shape) in ImageShape::ALL.iter().copied().enumerate() {
            ui.mineral(
                Image::new(("shape", i), &texture)
                    .alt(format!("{shape:?}"))
                    .shape(shape)
                    .size(vec2(64.0, 64.0))
                    .fit(ImageFit::Cover)
                    .shadow(true),
            );
        }
    });
}

fn lists(ui: &mut FluentUi<'_>, s: &mut State) {
    heading(ui, "h-list", "Lists");
    List::new("list-single")
        .navigation(Some(ListNavigation::Items))
        .selection(&mut s.single)
        .show(ui, |l| {
            for i in 0..3 {
                l.item(i, ListItem::new(format!("Single {i}")).disabled(i == 2), |ui| {
                    ui.label(format!("Single selection {i}"));
                });
            }
        });
    let multiple = s.multiple.get_or_insert_with(Selection::multiple);
    List::new("list-multiple")
        .navigation(Some(ListNavigation::Composite))
        .selection(multiple)
        .show(ui, |l| {
            for i in 0..3 {
                l.item(i, format!("Multiple {i}"), |ui| {
                    ui.label(format!("Multiple selection {i}"));
                });
            }
        });
}
