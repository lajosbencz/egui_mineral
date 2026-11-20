use egui::{Color32, ColorImage, RichText, Sense, TextureHandle, TextureOptions, Ui, UiBuilder, vec2};
use egui::{ecolor::Hsva, load::SizedTexture};
use egui_mineral::anchor::Align;
use egui_mineral_material::MaterialExt;
use egui_mineral_material::{
    catalog::Component,
    containment::CarouselPadding,
    icons::{
        ICON_ARCHIVE, ICON_DELETE, ICON_DESCRIPTION, ICON_FOLDER, ICON_INFO, ICON_MAIL, ICON_PERSON, ICON_SETTINGS,
        ICON_STAR, ICON_STORAGE, ICON_WORK,
    },
    prelude::*,
};

use super::{Shared, section, sub};

pub(crate) const TOPICS: &[&str] = &[
    "Card",
    "Dialog",
    "Bottom sheet",
    "Side sheet",
    "Divider",
    "Carousel",
    "List item",
    "Swipe actions",
    "Reorderable list",
    "Accordion",
    "Popover",
    "Hover card",
    "Floating panel",
    "Split pane",
    "Image list",
];

const CARD_VARIANTS: [(CardVariant, &str); 3] = [
    (CardVariant::Elevated, "Elevated"),
    (CardVariant::Filled, "Filled"),
    (CardVariant::Outlined, "Outlined"),
];

// Compose Carousel: 8dp spacing, 16dp start/end padding
const CAROUSELS: [(CarouselLayout, &str); 5] = [
    (CarouselLayout::multi_browse(186.0), "Multi-browse"),
    (CarouselLayout::hero(), "Hero"),
    (CarouselLayout::centered_hero(), "Center-aligned hero"),
    (CarouselLayout::uncontained(186.0), "Uncontained"),
    (CarouselLayout::FullScreen, "Full screen (vertical)"),
];

const TEXTURE_COUNT: usize = 8;
const TEX: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, egui_mineral::semantic::KeySalt)]
enum RowAction {
    Top,
    Bottom,
}

pub(crate) struct State {
    card_clicks: u32,
    dialog_basic: bool,
    dialog_icon: bool,
    dialog_full: bool,
    dialog_name: String,
    dialog_notes: String,
    bottom_modal: bool,
    bottom_standard: bool,
    side_modal: bool,
    side_standard: bool,
    carousel_clicked: Option<usize>,
    list_selected: Option<usize>,
    list_checks: [bool; 3],
    list_switch: bool,
    swipe_rows: Vec<&'static str>,
    reorder: Vec<&'static str>,
    accordion_single: Expansion<usize>,
    accordion_multi: Expansion<usize>,
    panel_open: bool,
    panel_fixed_open: bool,
    textures: Vec<TextureHandle>,
    image_clicked: Option<usize>,
    popover_dismissed: Option<Dismiss>,
    // Four sided popovers, the plain one, then the anchored one.
    popovers: [bool; 6],
}

const SWIPE_ROWS: [&str; 4] = ["Invoice March", "Invoice April", "Invoice May", "Invoice June"];
const REORDER_ROWS: [&str; 5] = ["Wake up", "Coffee", "Stand-up", "Deep work", "Lunch"];

impl Default for State {
    fn default() -> Self {
        Self {
            card_clicks: 0,
            dialog_basic: false,
            dialog_icon: false,
            dialog_full: false,
            dialog_name: String::new(),
            dialog_notes: String::new(),
            bottom_modal: false,
            bottom_standard: false,
            side_modal: false,
            side_standard: false,
            carousel_clicked: None,
            list_selected: Some(1),
            list_checks: [true, false, false],
            list_switch: true,
            popover_dismissed: None,
            popovers: [false; 6],
            swipe_rows: SWIPE_ROWS.to_vec(),
            reorder: REORDER_ROWS.to_vec(),
            accordion_single: Expansion::single(),
            accordion_multi: Expansion::default(),
            panel_open: false,
            panel_fixed_open: false,
            textures: Vec::new(),
            image_clicked: None,
        }
    }
}

fn swatch(i: usize) -> Color32 {
    Hsva::new(i as f32 * 0.13 % 1.0, 0.45, 0.85, 1.0).into()
}

fn ensure_textures(ctx: &egui::Context, textures: &mut Vec<TextureHandle>) {
    if !textures.is_empty() {
        return;
    }
    for i in 0..TEXTURE_COUNT {
        let (a, b) = (
            Hsva::new(i as f32 / TEXTURE_COUNT as f32, 0.6, 0.9, 1.0),
            Hsva::new((i as f32 / TEXTURE_COUNT as f32 + 0.25) % 1.0, 0.7, 0.55, 1.0),
        );
        let mut px = Vec::with_capacity(TEX * TEX);
        for y in 0..TEX {
            for x in 0..TEX {
                let k = (x + y) as f32 / (2 * TEX - 2) as f32;
                px.push(Color32::from(Hsva::new(
                    a.h + (b.h - a.h) * k,
                    a.s + (b.s - a.s) * k,
                    a.v + (b.v - a.v) * k,
                    1.0,
                )));
            }
        }
        textures.push(ctx.load_texture(
            format!("showcase-gradient-{i}"),
            ColorImage::new([TEX, TEX], px),
            TextureOptions::LINEAR,
        ));
    }
}

fn boxed(ui: &mut MaterialUi<'_>, height: f32, add: impl FnOnce(&mut MaterialUi<'_>)) {
    let rt = ui.rt();
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let t = rt.tokens();
    ui.painter()
        .rect_filled(rect, f32::from(t.shape.small), t.color.surface_container_low);
    let mut child = ui.new_child(UiBuilder::new().max_rect(rect.shrink(4.0)));
    add(&mut rt.ui(&mut child));
}

fn pane(ui: &mut Ui, text: &str, color: Color32) {
    ui.painter().rect_filled(ui.max_rect(), 0.0, color);
    ui.centered_and_justified(|ui| {
        ui.label(RichText::new(text).color(Color32::BLACK));
    });
}

pub(crate) fn show(ui: &mut MaterialUi<'_>, s: &mut State, shared: &mut Shared) {
    ensure_textures(ui.ctx(), &mut s.textures);
    section(ui, TOPICS[0], Some(Card::META), |ui| cards(ui, s, shared));
    section(ui, TOPICS[1], Some(Dialog::META), |ui| dialogs(ui, s, shared));
    section(ui, TOPICS[2], Some(BottomSheet::META), |ui| {
        bottom_sheets(ui, s, shared);
    });
    section(ui, TOPICS[3], Some(SideSheet::META), |ui| {
        side_sheets(ui, s, shared);
    });
    section(ui, TOPICS[4], Some(Divider::META), dividers);
    section(ui, TOPICS[5], Some(Carousel::META), |ui| carousels(ui, s));
    section(ui, TOPICS[6], Some(ListItem::META), |ui| {
        list_items(ui, s, shared);
    });
    section(ui, TOPICS[7], Some(ListItem::META), |ui| swipe(ui, s, shared));
    section(ui, TOPICS[8], Some(ListItem::META), |ui| reorder(ui, s, shared));
    section(ui, TOPICS[9], Some(<Accordion<'static, ()>>::META), |ui| {
        accordions(ui, s, shared);
    });
    section(ui, TOPICS[10], Some(Popover::META), |ui| popovers(ui, s));
    section(ui, TOPICS[11], Some(HoverCard::META), hover_cards);
    section(ui, TOPICS[12], Some(FloatingPanel::META), |ui| floating(ui, s));
    section(ui, TOPICS[13], Some(SplitPane::META), split_panes);
    section(ui, TOPICS[14], Some(<ImageList<'static>>::META), |ui| {
        image_lists(ui, s, shared);
    });
}

const CARD_WIDTH: f32 = 220.0;

fn card(id: impl Into<KeyedSalt>, label: &str) -> Card {
    Card::new(id, label)
        .width(CARD_WIDTH..=CARD_WIDTH)
        .content_padding(Card::CONTENT_PADDING)
}

fn cards(ui: &mut MaterialUi<'_>, s: &mut State, shared: &mut Shared) {
    let rt = ui.rt();
    sub(ui, "Variants");
    super::row(ui, |row| {
        for (i, (v, name)) in CARD_VARIANTS.into_iter().enumerate() {
            row.add(|ui| {
                card(("card", i), name).variant(v).show(ui, |ui| {
                    ui.label(RichText::new(name).font(TypeScale::TitleMedium.font(rt.ctx())));
                    ui.label("Static container with supporting text.");
                })
            });
        }
    });
    sub(ui, &format!("Clickable ({} clicks) and disabled", s.card_clicks));
    super::row(ui, |row| {
        for (i, (v, name)) in CARD_VARIANTS.into_iter().enumerate() {
            row.add(|ui| {
                let r = card(("card-click", i), name).variant(v).clickable(true).show(ui, |ui| {
                    ui.label(RichText::new(format!("{name} clickable")).font(TypeScale::TitleMedium.font(rt.ctx())));
                    ui.label("Whole surface reacts.");
                });
                if r.clicked {
                    s.card_clicks += 1;
                    shared.toast(format!("{name} card clicked"));
                }
            });
        }
        row.add(|ui| {
            card("card-dis", "Disabled card")
                .clickable(true)
                .disabled(true)
                .show(ui, |ui| {
                    ui.label(RichText::new("Disabled").font(TypeScale::TitleMedium.font(rt.ctx())));
                    ui.label("Not interactive.");
                })
        });
    });
}

fn dialogs(ui: &mut MaterialUi<'_>, s: &mut State, shared: &mut Shared) {
    let rt = ui.rt();
    super::row(ui, |row| {
        s.dialog_basic |= row.mineral(PushButton::new("dlg-open-basic", "Basic dialog")).clicked();
        s.dialog_icon |= row
            .mineral(PushButton::new("dlg-open-icon", "Dialog with icon").variant(ButtonVariant::Tonal))
            .clicked();
        s.dialog_full |= row
            .mineral(PushButton::new("dlg-open-full", "Full-screen dialog").variant(ButtonVariant::Outlined))
            .clicked();
    });
    let r = Dialog::new("dlg-basic", "Discard draft?")
        .supporting("The draft will be lost. This cannot be undone.")
        .action("cancel", DialogAction::new("Cancel").mnemonic('c'))
        .action("discard", DialogAction::new("Discard").mnemonic('d'))
        .default_action("discard")
        .show_modal(rt, &mut s.dialog_basic, |_| {});
    if let Some(a) = r.clicked {
        shared.toast(format!("Dialog action {a}"));
    }
    if let Some(d) = r.dismissed {
        shared.toast(format!("Dialog dismissed: {d:?}"));
    }
    let r = Dialog::new("dlg-icon", "Rename profile")
        .icon(ICON_SETTINGS)
        .resizable(true)
        .supporting("Enter in the name field saves; Enter in the notes field adds a line.")
        .action("cancel", "Cancel")
        .action(
            "save",
            DialogAction::new("Save").disabled(s.dialog_name.trim().is_empty()),
        )
        .default_action("save")
        .show_modal(rt, &mut s.dialog_icon, |ui| {
            ui.mineral(TextField::new("dlg-name", "Name", &mut s.dialog_name).preferred_width(260.0));
            ui.mineral(
                TextField::new("dlg-notes", "Notes", &mut s.dialog_notes)
                    .rows(3)
                    .preferred_width(260.0),
            );
        });
    if let Some(a) = r.clicked {
        shared.toast(format!("Dialog action {a}"));
    }
    let r = Dialog::full_screen("dlg-full", "New event")
        .confirm("save", "Save")
        .enter_confirms(true)
        .show_modal(rt, &mut s.dialog_full, |ui| {
            ui.label("Full-screen content area. The confirm button is in the header.");
        });
    if let Some(a) = r.clicked {
        shared.toast(format!("Full-screen action {a}"));
    }
}

fn sheet_body(ui: &mut MaterialUi<'_>, id: &'static str, shared: &mut Shared) {
    for i in 0..4 {
        let r = ui.mineral(
            ListItem::new((id, i), format!("Sheet row {}", i + 1))
                .leading(ListSlot::new().icon(ICON_FOLDER))
                .clickable(true),
        );
        if r.output().activated {
            shared.toast(format!("Sheet row {} opened", i + 1));
        }
    }
}

fn bottom_sheets(ui: &mut MaterialUi<'_>, s: &mut State, shared: &mut Shared) {
    let rt = ui.rt();
    ui.label("Sheets attach to the window edge.");
    super::row(ui, |row| {
        s.bottom_modal |= row
            .mineral(PushButton::new("bs-open-modal", "Modal bottom sheet"))
            .clicked();
        let r = row.mineral(PushButton::new("bs-open-std", "Standard bottom sheet").variant(ButtonVariant::Tonal));
        s.bottom_standard ^= r.clicked();
    });
    BottomSheet::new("bs-modal")
        .detents([0.4, 0.8])
        .show_modal(rt, &mut s.bottom_modal, |ui| sheet_body(ui, "bs-modal-row", shared));
    BottomSheet::new("bs-std")
        .detents([0.25, 0.5])
        .drag_handle(true)
        .show(rt, &mut s.bottom_standard, |ui| sheet_body(ui, "bs-std-row", shared));
}

fn side_sheets(ui: &mut MaterialUi<'_>, s: &mut State, shared: &mut Shared) {
    let rt = ui.rt();
    super::row(ui, |row| {
        s.side_modal |= row
            .mineral(PushButton::new("ss-open-modal", "Modal side sheet"))
            .clicked();
        let r = row.mineral(PushButton::new("ss-open-std", "Standard side sheet").variant(ButtonVariant::Tonal));
        s.side_standard ^= r.clicked();
    });
    SideSheet::new("ss-modal")
        .headline("Filters")
        .show_modal(rt, &mut s.side_modal, |ui| sheet_body(ui, "ss-modal-row", shared));
    ui.label("The standard sheet docks at the end of this box and shares its width with the content; drag its edge to resize.");
    boxed(ui, 240.0, |ui| {
        SideSheet::new("ss-start")
            .edge(egui_mineral::layout::Edge::Start)
            .headline("Outline")
            .width(200.0..=200.0)
            .scrollable(false)
            .close_button(false)
            .show(ui, &mut true, |ui| {
                ui.label("Start edge, no scroll area: the content owns the full height.");
            });
        SideSheet::new("ss-std")
            .headline("Details")
            .width(256.0..=400.0)
            .resizable(true)
            .close_button(false)
            .show(ui, &mut s.side_standard, |ui| {
                ui.label("Standard sheet without a close button; use the toggle button.");
            });
        let rest = ui.available_rect_before_wrap();
        pane(
            &mut ui.new_child(UiBuilder::new().max_rect(rest)),
            &format!("Content: {:.0} dp wide", rest.width()),
            Color32::from_rgb(0xc8, 0xe6, 0xc9),
        );
    });
}

fn dividers(ui: &mut MaterialUi<'_>) {
    sub(ui, "Full");
    ui.mineral(Divider::new("div-full"));
    sub(ui, "Inset (16 start)");
    ui.mineral(Divider::new("div-inset").inset_start(16.0));
    sub(ui, "Inset both ends");
    ui.mineral(Divider::new("div-inset-both").inset_start(32.0).inset_end(32.0));
    sub(ui, "Vertical");
    super::row(ui, |row| {
        row.add(|ui| ui.label("One"));
        row.add(|ui| ui.mineral(Divider::new("div-v1").axis(Axis::Vertical).preferred_height(24.0)));
        row.add(|ui| ui.label("Two"));
        row.add(|ui| ui.mineral(Divider::new("div-v2").axis(Axis::Vertical).preferred_height(24.0)));
        row.add(|ui| ui.label("Three"));
    });
}

fn carousels(ui: &mut MaterialUi<'_>, s: &mut State) {
    let rt = ui.rt();
    let t = rt.tokens();
    ui.label(format!("Clicked item: {:?}", s.carousel_clicked));
    for (i, (layout, name)) in CAROUSELS.into_iter().enumerate() {
        sub(ui, name);
        let full = layout == CarouselLayout::FullScreen;
        let r = Carousel::new(("carousel", i), layout)
            .height(if full { 400.0 } else { 221.0 })
            .item_spacing(8.0)
            .content_padding(CarouselPadding { start: 16.0, end: 16.0 })
            .item_label(|i| format!("Item {}", i + 1))
            .surround(t.color.surface)
            .show(ui, 8, |ui, idx| pane(ui, &format!("Item {}", idx + 1), swatch(idx)));
        if r.activated.is_some() {
            s.carousel_clicked = r.activated;
        }
    }
    sub(ui, "Texture items (show_images)");
    let textures = &s.textures;
    Carousel::new("carousel-img", CarouselLayout::multi_browse(186.0))
        .height(221.0)
        .item_spacing(8.0)
        .content_padding(CarouselPadding { start: 16.0, end: 16.0 })
        .show_images(ui, textures.len(), |idx| SizedTexture::from_handle(&textures[idx]));
}

fn list_items(ui: &mut MaterialUi<'_>, s: &mut State, shared: &mut Shared) {
    sub(
        ui,
        "One line, leading kinds (click to select; the checkbox and radio toggle on their own)",
    );
    let gradient = LeadingPaint::new(|p, slot, _| {
        let mut mesh = egui::Mesh::default();
        let (from, to) = (Color32::from_rgb(0x67, 0x3a, 0xb7), Color32::from_rgb(0xff, 0x98, 0x00));
        mesh.colored_vertex(slot.left_top(), from);
        mesh.colored_vertex(slot.right_top(), to);
        mesh.colored_vertex(slot.right_bottom(), to);
        mesh.colored_vertex(slot.left_bottom(), from);
        mesh.add_triangle(0, 1, 2);
        mesh.add_triangle(0, 2, 3);
        p.add(mesh);
    });
    let rows: [(&str, ListSlot); 5] = [
        ("Icon", ListSlot::new().icon(ICON_MAIL)),
        ("Avatar", ListSlot::new().avatar('A')),
        (
            "Checkbox + icon",
            ListSlot::new().checkbox(s.list_checks[0]).icon(ICON_MAIL),
        ),
        ("Radio", ListSlot::new().radio(s.list_selected == Some(3))),
        ("Custom paint", ListSlot::new().paint(gradient)),
    ];
    for (i, (name, lead)) in rows.into_iter().enumerate() {
        let r = ui.mineral(
            ListItem::new(("li1", i), name)
                .leading(lead)
                .clickable(true)
                .selected(s.list_selected == Some(i)),
        );
        if r.output().activated {
            s.list_selected = Some(i);
        }
        if r.output().leading_toggled {
            match i {
                2 => s.list_checks[0] ^= true,
                _ => s.list_selected = Some(3),
            }
        }
    }
    sub(ui, "Two lines, trailing kinds");
    let r = ui.mineral(
        ListItem::new("li2-icon", "Trailing icon")
            .supporting("Supporting text")
            .leading(ListSlot::new().avatar('B'))
            .trailing(ListSlot::new().icon(ICON_STAR))
            .clickable(true),
    );
    if r.output().activated {
        shared.toast("Trailing icon item activated");
    }
    let r = ui.mineral(
        ListItem::new("li2-text", "Trailing text")
            .supporting("Supporting text")
            .leading(ListSlot::new().icon(ICON_DESCRIPTION))
            .trailing(ListSlot::new().text("100+"))
            .clickable(true),
    );
    if r.output().activated {
        shared.toast("Trailing text item activated");
    }
    let r = ui.mineral(
        ListItem::new("li2-switch", "Trailing switch")
            .supporting("The switch toggles; the row activates")
            .leading(ListSlot::new().icon(ICON_SETTINGS))
            .trailing(ListSlot::new().switch(s.list_switch))
            .clickable(true),
    );
    s.list_switch ^= r.output().trailing_toggled;
    if r.output().activated {
        shared.toast("Trailing switch item activated");
    }
    sub(ui, "Overline, three lines");
    ui.mineral(
        ListItem::new("li3-overline", "Overline and supporting")
            .overline("Overline")
            .supporting("Supporting text under the headline")
            .leading(ListSlot::new().avatar('C'))
            .trailing(ListSlot::new().text("10:24")),
    );
    ui.mineral(
        ListItem::new("li3-lines", "Three line item")
            .supporting("Two supporting rows are allowed when the item is forced to three lines. The text wraps.")
            .lines(ListLines::Three)
            .leading(ListSlot::new().icon(ICON_PERSON)),
    );
    list_states(ui, s);
}

fn list_states(ui: &mut MaterialUi<'_>, s: &mut State) {
    sub(ui, "Selected, disabled, and selection rows (the whole row toggles)");
    ui.mineral(
        ListItem::new("li-sel", "Selected")
            .leading(ListSlot::new().icon(ICON_STAR))
            .selected(true),
    );
    ui.mineral(
        ListItem::new("li-dis", "Disabled")
            .supporting("Not interactive")
            .leading(ListSlot::new().icon(ICON_WORK))
            .clickable(true)
            .disabled(true),
    );
    for (i, checked) in s.list_checks.iter_mut().enumerate().skip(1) {
        let r = ui.mineral(
            ListItem::new(("li-check", i), format!("Checkbox row {i}")).leading(ListSlot::new().checkbox(*checked)),
        );
        *checked ^= r.output().leading_toggled;
    }
}

fn swipe(ui: &mut MaterialUi<'_>, s: &mut State, shared: &mut Shared) {
    ui.label("Swipe rows: end action archives (tap to confirm), start action deletes (full swipe).");
    let mut removed = None;
    for (i, name) in s.swipe_rows.iter().enumerate() {
        let item = ListItem::new(("swipe", *name), *name)
            .supporting("Swipe left or right")
            .leading(ListSlot::new().icon(ICON_DESCRIPTION))
            .swipe_start(
                SwipeAction::new(ICON_DELETE, "Delete")
                    .removes_row(true)
                    .destructive(true),
            )
            .swipe_end(SwipeAction::new(ICON_ARCHIVE, "Archive"));
        if let Some(side) = ui.mineral(item).output().swiped {
            let verb = if side == Edge::Start { "Deleted" } else { "Archived" };
            shared.toast(format!("{verb} {name}"));
            removed = Some(i);
        }
    }
    if let Some(i) = removed {
        s.swipe_rows.remove(i);
    }
    if s.swipe_rows.len() < SWIPE_ROWS.len()
        && ui
            .mineral(PushButton::new("swipe-reset", "Restore rows").variant(ButtonVariant::Text))
            .clicked()
    {
        s.swipe_rows = SWIPE_ROWS.to_vec();
    }
}

fn reorder(ui: &mut MaterialUi<'_>, s: &mut State, shared: &mut Shared) {
    let rt = ui.rt();
    ui.label("Drag a row, or focus a row and press Alt+Up/Down. Right-click or long-press for a menu.");
    let rows = s.reorder.clone();
    let r = ReorderableList::new("reorder", "Reorder demo", &rows).show(ui, |i, name| {
        ListItem::new(("reorder-row", *name), *name)
            .supporting(format!("Position {}", i + 1))
            .leading(ListSlot::new().avatar(name.chars().next().unwrap_or('?')))
            .clickable(true)
    });
    let mut moved = r.moved;
    for (&i, row) in r.rows.iter() {
        if row.output().activated {
            shared.toast(format!("Activated {}", rows[i]));
        }
        if Menu::context_requested(rt, row) {
            let menu = Menu::new(("reorder-menu", i), "Row actions")
                .item(RowAction::Top, MenuItem::new("Move to top").disabled(i == 0))
                .item(
                    RowAction::Bottom,
                    MenuItem::new("Move to bottom").disabled(i + 1 == rows.len()),
                );
            moved = match menu.show_context(rt, row).clicked {
                Some(RowAction::Top) => Some(Reorder { from: i, to: 0 }),
                Some(RowAction::Bottom) => Some(Reorder {
                    from: i,
                    to: rows.len() - 1,
                }),
                None => moved,
            };
        }
    }
    if let Some(m) = moved {
        m.apply(&mut s.reorder);
    }
}

fn accordions(ui: &mut MaterialUi<'_>, s: &mut State, shared: &mut Shared) {
    let mut density = f64::from(shared.settings.density);
    ui.mineral(
        ValueSlider::new(
            "acc-density",
            "Density (header height)",
            RangeModel::new(f64::from(Density::MIN.step()), 0.0).step(1.0),
            &mut density,
        )
        .ticks(true)
        .format(|v| format!("{v:.0}"))
        .preferred_width(280.0),
    );
    shared.settings.density = density.round() as i8;
    sub(ui, "Single expand");
    Accordion::new("acc-single", &mut s.accordion_single).show(ui, |a| {
        a.panel(
            0,
            AccordionPanel::new("Account")
                .supporting("Profile and security")
                .icon(ICON_PERSON),
            |ui| {
                ui.label("Account settings live here.");
            },
        );
        a.panel(1, AccordionPanel::new("Storage").icon(ICON_STORAGE), |ui| {
            ui.label("2.1 GB of 15 GB used.");
        });
        a.panel(2, AccordionPanel::new("Disabled").disabled(true), |ui| {
            ui.label("Never shown.");
        });
        a.panel(3, "Plain string title", |ui| {
            ui.label("Panels convert from plain strings.");
        });
    });
    sub(ui, "Multiple expand");
    Accordion::new("acc-multi", &mut s.accordion_multi).show(ui, |a| {
        for i in 0..3 {
            let title = AccordionPanel::new(format!("Section {}", i + 1)).title_style(TypeScale::TitleSmall);
            a.panel(i, title, |ui| {
                ui.label("Several panels can stay open together.");
            });
        }
    });
}

fn popovers(ui: &mut MaterialUi<'_>, s: &mut State) {
    let rt = ui.rt();
    ui.label(format!(
        "Click the trigger; Escape or an outside press closes. Last dismiss: {:?}",
        s.popover_dismissed
    ));
    let mut at_rect = None;
    super::row(ui, |row| {
        for (i, (side, name)) in [
            (Side::Bottom, "Bottom"),
            (Side::Top, "Top"),
            (Side::Start, "Start"),
            (Side::End, "End"),
        ]
        .into_iter()
        .enumerate()
        {
            let r = row.mineral(PushButton::new(("pop-btn", i), name).variant(ButtonVariant::Tonal));
            let p = Popover::new(("pop", i), format!("{name} popover")).side(side).show(
                rt,
                r.response(),
                &mut s.popovers[i],
                |ui| {
                    ui.label(format!("Anchored {name}."));
                    ui.label("Arrow points at the trigger.");
                },
            );
            s.popover_dismissed = p.dismissed.or(s.popover_dismissed);
        }
        let r = row.mineral(PushButton::new("pop-noarrow", "No arrow, start-aligned").variant(ButtonVariant::Outlined));
        Popover::new("pop-noarrow-p", "Plain popover")
            .arrow_visible(false)
            .align(Align::Start)
            .max_width(200.0)
            .show(rt, r.response(), &mut s.popovers[4], |ui| {
                ui.label("A narrow popover without arrow.");
            });
        let r = row.mineral(PushButton::new("pop-at", "Opened by code").variant(ButtonVariant::Outlined));
        s.popovers[5] |= r.clicked();
        at_rect = Some(r.response().rect);
    });
    if let Some(rect) = at_rect {
        Popover::new("pop-at-p", "Anchored popover").show_at(ui.rt(), rect, &mut s.popovers[5], |ui| {
            ui.label("Anchored at a rect; the button never toggles it.");
        });
    }
}

fn hover_cards(ui: &mut MaterialUi<'_>) {
    let rt = ui.rt();
    ui.label("Hover or focus the triggers.");
    super::row(ui, |row| {
        let r = row.mineral(PushButton::new("hc-btn", "@mineral").variant(ButtonVariant::Text));
        HoverCard::new("hc", "Profile card").show(rt, r.response(), |ui| {
            ui.label(RichText::new("egui_mineral").font(TypeScale::TitleSmall.font(rt.ctx())));
            ui.label("A layered UI framework on egui.");
        });
        let r = row.mineral(
            PushButton::new("hc-fast", "Fast, on top")
                .variant(ButtonVariant::Text)
                .icon(ICON_INFO),
        );
        HoverCard::new("hc-fast-card", "Quick card")
            .side(Side::Top)
            .open_delay(std::time::Duration::from_millis(100))
            .close_delay(std::time::Duration::from_millis(100))
            .show(rt, r.response(), |ui| {
                ui.label("Opens after 100 ms.");
            });
    });
}

fn floating(ui: &mut MaterialUi<'_>, s: &mut State) {
    let rt = ui.rt();
    super::row(ui, |row| {
        let r = row.mineral(PushButton::new("fp-open", "Resizable panel"));
        s.panel_open ^= r.clicked();
        let r = row.mineral(PushButton::new("fp-open-fixed", "Fixed-size panel").variant(ButtonVariant::Tonal));
        s.panel_fixed_open ^= r.clicked();
    });
    FloatingPanel::new("fp", "Inspector")
        .default_pos([160.0, 160.0])
        .default_size([320.0, 220.0])
        .show(rt, &mut s.panel_open, |ui| {
            ui.label("Drag the header; resize from the edges; Escape closes.");
        });
    FloatingPanel::new("fp-fixed", "Fixed")
        .default_pos([240.0, 240.0])
        .default_size([280.0, 180.0])
        .resizable(false)
        .show(rt, &mut s.panel_fixed_open, |ui| {
            ui.label("Not resizable.");
        });
}

fn split_panes(ui: &mut MaterialUi<'_>) {
    sub(ui, "Horizontal (side by side)");
    boxed(ui, 180.0, |ui| {
        SplitPane::new("split-h", "Resize panes")
            .default_fraction(0.35)
            .min_first(80.0)
            .min_second(80.0)
            .show(
                ui,
                |ui| pane(ui, "First", swatch(1)),
                |ui| pane(ui, "Second", swatch(4)),
            );
    });
    sub(ui, "Vertical (stacked)");
    boxed(ui, 220.0, |ui| {
        SplitPane::new("split-v", "Resize panes")
            .axis(Axis::Vertical)
            .min_first(60.0)
            .min_second(60.0)
            .show(
                ui,
                |ui| pane(ui, "Top", swatch(2)),
                |ui| {
                    SplitPane::new("split-nested", "Resize nested panes").show(
                        ui,
                        |ui| pane(ui, "Nested left", swatch(5)),
                        |ui| pane(ui, "Nested right", swatch(6)),
                    );
                },
            );
    });
}

fn image_lists(ui: &mut MaterialUi<'_>, s: &mut State, shared: &mut Shared) {
    let textures = &s.textures;
    let mut clicked = None;
    sub(ui, "Standard, 4 columns, titled tiles");
    let r = ImageList::new("il-std")
        .columns(4)
        .item_label(|i| format!("Tile {}", i + 1))
        .show(ui, textures.len(), |ui, i| {
            ui.mineral(
                ImageTile::new(("il-std-tile", i), SizedTexture::from_handle(&textures[i]))
                    .title(format!("Tile {}", i + 1)),
            );
        });
    clicked = r.activated.or(clicked);
    sub(ui, "Masonry, 3 columns");
    let r = ImageList::new("il-masonry")
        .columns(3)
        .layout(ImageListLayout::Masonry)
        .aspect(|i| [1.0, 1.4, 0.8, 1.2][i % 4])
        .gap(8.0)
        .show(ui, textures.len(), |ui, i| {
            ui.mineral(ImageTile::new(
                ("il-masonry-tile", i),
                SizedTexture::from_handle(&textures[i]),
            ));
        });
    clicked = r.activated.or(clicked);
    sub(ui, "Standard, wide rows");
    let r = ImageList::new("il-wide")
        .columns(2)
        .row_aspect(0.5)
        .show(ui, 4, |ui, i| {
            ui.mineral(ImageTile::new(("il-wide-tile", i), SizedTexture::from_handle(&textures[i])).title("Wide"));
        });
    clicked = r.activated.or(clicked);
    if let Some(i) = clicked {
        s.image_clicked = Some(i);
        shared.toast(format!("Image tile {i}"));
    }
    ui.label(format!("Last clicked: {:?}", s.image_clicked));
}
