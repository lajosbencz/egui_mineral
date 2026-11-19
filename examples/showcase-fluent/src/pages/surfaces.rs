use egui_mineral_fluent::{
    actions::{ButtonAppearance, PushButton},
    data::Text,
    egui_mineral::{anchor::Side, expand::Expansion, layout::Axis},
    icons::ICON_PERSON,
    surfaces::{
        Accordion, AccordionItem, AccordionSize, Card, CardAppearance, CardHeader, Dialog, DialogAction,
        DialogActionsPosition, DialogKind, Drawer, DrawerPosition, DrawerSize, Popover, PopoverAppearance, PopoverSize,
        TeachingPopover, TeachingStep, Tooltip, TooltipAppearance, TooltipRelationship,
    },
};

use super::{heading, row};
use crate::Fui;
use egui_mineral_fluent::egui_mineral::layout::FlowItems;

#[derive(Debug, Default)]
pub(crate) struct State {
    popovers: [bool; 3],
    teaching: bool,
    dialogs: [bool; 4],
    drawers: [bool; 4],
    cards: [bool; 2],
    accordion: Expansion<usize>,
    accordion_multiple: Expansion<usize>,
    inline: bool,
    last: String,
}

pub(crate) fn show(ui: &mut Fui<'_>, s: &mut State) {
    tooltips(ui);
    popovers(ui, s);
    dialogs(ui, s);
    drawers(ui, s);
    cards(ui, s);
    accordions(ui, s);
    ui.mineral(Text::new("surfaces-last", format!("Last event: {}", s.last)));
}

fn trigger(
    ui: &mut FlowItems<'_, egui_mineral_fluent::Fluent>,
    salt: (&'static str, usize),
    label: &str,
) -> egui::Response {
    ui.mineral(PushButton::new(salt, label)).response().clone()
}

fn tooltips(ui: &mut Fui<'_>) {
    heading(ui, "h-tooltip", "Tooltips");
    let rt = ui.rt();
    row(ui, |ui| {
        let a = trigger(ui, ("tip-trigger", 0), "Description");
        Tooltip::new(("tip", 0), "Describes the button").show(rt, &a);
        let b = trigger(ui, ("tip-trigger", 1), "Inverted arrow");
        Tooltip::new(("tip", 1), "Inverted with an arrow")
            .appearance(TooltipAppearance::Inverted)
            .arrow(true)
            .side(Side::Bottom)
            .show(rt, &b);
        let c = trigger(ui, ("tip-trigger", 2), "Label");
        Tooltip::new(("tip", 2), "Names the button")
            .relationship(TooltipRelationship::Label)
            .show(rt, &c);
    });
}

fn popovers(ui: &mut Fui<'_>, s: &mut State) {
    heading(ui, "h-popover", "Popovers");
    let rt = ui.rt();
    row(ui, |ui| {
        for (i, (size, appearance)) in [
            (PopoverSize::Small, PopoverAppearance::Default),
            (PopoverSize::Medium, PopoverAppearance::Brand),
            (PopoverSize::Large, PopoverAppearance::Inverted),
        ]
        .into_iter()
        .enumerate()
        {
            let t = trigger(ui, ("popover-trigger", i), &format!("{size:?} {appearance:?}"));
            Popover::new(("popover", i), "Popover")
                .size(size)
                .appearance(appearance)
                .arrow(true)
                .side(Side::Bottom)
                .trap_focus(i == 1)
                .show(rt, &t, &mut s.popovers[i], |ui| {
                    ui.mineral(Text::new(("popover-text", i), "Popover content"));
                    ui.mineral(PushButton::new(("popover-button", i), "Action"));
                });
        }
        let t = trigger(ui, ("teaching-trigger", 0), "Teaching");
        let r = TeachingPopover::new("teaching", "Tour")
            .step(TeachingStep::new("Start here.").header("Tip").title("Welcome"))
            .step(TeachingStep::new("Then continue.").header("Tip").title("Next"))
            .side(Side::Bottom)
            .primary("Got it")
            .show(rt, &t, &mut s.teaching);
        if r.primary_clicked {
            "teaching done".clone_into(&mut s.last);
        }
    });
}

fn dialogs(ui: &mut Fui<'_>, s: &mut State) {
    heading(ui, "h-dialog", "Dialogs");
    let rt = ui.rt();
    row(ui, |ui| {
        let kinds = [
            ("Modal", DialogKind::Modal, false),
            ("Non-modal", DialogKind::NonModal, false),
            ("Alert", DialogKind::Alert, false),
            ("Fluid actions", DialogKind::Modal, true),
        ];
        for (i, (label, ..)) in kinds.iter().enumerate() {
            if trigger(ui, ("dialog-trigger", i), label).clicked() {
                s.dialogs[i] = true;
            }
        }
        for (i, (label, kind, fluid)) in kinds.into_iter().enumerate() {
            let r = Dialog::new(("dialog", i), label)
                .kind(kind)
                .close_button(kind != DialogKind::Alert)
                .fluid_actions(fluid)
                .actions_position(DialogActionsPosition::End)
                .action(DialogAction::new("Cancel"))
                .action(DialogAction::new("Confirm").appearance(ButtonAppearance::Primary))
                .show_modal(rt, &mut s.dialogs[i], |ui| {
                    ui.mineral(Text::new(("dialog-text", i), "Dialog content."));
                });
            if let Some(c) = r.clicked {
                s.last = format!("{label} action {c}");
            }
        }
    });
}

fn drawers(ui: &mut Fui<'_>, s: &mut State) {
    heading(ui, "h-drawer", "Drawers");
    let rt = ui.rt();
    row(ui, |ui| {
        for (i, p) in DrawerPosition::ALL.iter().copied().enumerate() {
            if trigger(ui, ("drawer-trigger", i), &format!("Overlay {p:?}")).clicked() {
                s.drawers[i] = true;
            }
        }
        let label = if s.inline { "Hide inline" } else { "Show inline" };
        if trigger(ui, ("drawer-inline", 0), label).clicked() {
            s.inline = !s.inline;
        }
        for (i, p) in DrawerPosition::ALL.iter().copied().enumerate() {
            Drawer::new(("drawer", i), format!("{p:?} drawer"))
                .title(format!("{p:?}"))
                .close_button(true)
                .position(p)
                .size(DrawerSize::Small)
                .footer_action("Save")
                .show_modal(rt, &mut s.drawers[i], |ui| {
                    ui.mineral(Text::new(("drawer-text", i), "Drawer body."));
                });
        }
    });
    if s.inline {
        let width = ui.available_width();
        let rt = ui.rt();
        ui.allocate_ui(egui::vec2(width, 200.0), |ui| {
            let ui = &mut rt.ui(ui);
            Drawer::new("inline-drawer", "Inline")
                .title("Inline")
                .close_button(true)
                .size(DrawerSize::Small)
                .show(ui, &mut s.inline, |ui| {
                    ui.mineral(Text::new("inline-text", "Inline body."));
                });
        });
    }
}

fn cards(ui: &mut Fui<'_>, s: &mut State) {
    heading(ui, "h-card", "Cards");
    for (i, appearance) in CardAppearance::ALL.iter().copied().enumerate() {
        let mut card = Card::new(("card", i)).appearance(appearance).interactive(true);
        if i < 2 {
            card = card.selectable(&mut s.cards[i]).checkbox(true);
        }
        card.show(ui, |c| {
            c.header(
                &CardHeader::new(format!("{appearance:?}"))
                    .secondary_text("Card header")
                    .image(ICON_PERSON),
                |_| (),
            );
            c.body(|ui| {
                ui.label("Card body text.");
            });
            c.footer(|ui| {
                ui.mineral(PushButton::new(("card-action", i), "Open"));
            });
        });
        ui.add_space(8.0);
    }
    Card::new("card-horizontal").axis(Axis::Horizontal).show(ui, |c| {
        c.body(|ui| {
            ui.label("Horizontal");
        });
        c.body(|ui| {
            ui.label("Card");
        });
    });
}

fn accordions(ui: &mut Fui<'_>, s: &mut State) {
    heading(ui, "h-accordion", "Accordion");
    Accordion::new("accordion", &mut s.accordion)
        .collapsible(true)
        .show(ui, |a| {
            for i in 0..3 {
                a.item(AccordionItem::new(format!("Item {i}")), |ui| {
                    ui.label("Panel content.");
                });
            }
        });
    ui.add_space(8.0);
    Accordion::new("accordion-multiple", &mut s.accordion_multiple)
        .size(AccordionSize::Large)
        .expand_icon_position(egui_mineral_fluent::surfaces::ExpandIconPosition::End)
        .show(ui, |a| {
            for i in 0..2 {
                a.item(AccordionItem::new(format!("Large {i}")).icon(ICON_PERSON), |ui| {
                    ui.label("Panel content.");
                });
            }
        });
}
