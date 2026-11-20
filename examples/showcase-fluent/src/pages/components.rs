use egui_mineral_fluent::{
    IconPosition, Size,
    actions::{
        ButtonAppearance, ButtonShape, CompoundButton, LinkAppearance, MenuButton, PushButton, SplitButton, TextLink,
        ToggleButton,
    },
    data::{Text, TextSize, TextWeight},
    egui_mineral::layout::Axis,
    icons::{ICON_ADD, ICON_CALENDAR, ICON_MAIL, ICON_TEXT_BOLD},
    inputs::FieldLabel,
    surfaces::{Divider, DividerAlign, DividerAppearance},
};

use super::{heading, row};
use crate::FluentUi;

#[derive(Debug, Default)]
pub(crate) struct State {
    toggles: [bool; 5],
    menu_open: bool,
    split_open: bool,
    clicks: u32,
    inputs: super::inputs::State,
    feedback: super::feedback::State,
    surfaces: super::surfaces::State,
    data: super::data::State,
    navigation: super::navigation::State,
    extras: super::extras::State,
}

pub(crate) fn actions(ui: &mut FluentUi<'_>, s: &mut State) {
    heading(ui, "h-appearance", "Button appearances");
    row(ui, |ui| {
        for (i, a) in ButtonAppearance::ALL.iter().copied().enumerate() {
            let label = format!("{a:?}");
            if ui
                .mineral(PushButton::new(("appearance", i), label).appearance(a))
                .clicked()
            {
                s.clicks += 1;
            }
        }
        ui.mineral(PushButton::new("disabled", "Disabled").disabled(true));
    });
    heading(ui, "h-size", "Sizes, shapes and icons");
    row(ui, |ui| {
        for (i, size) in Size::ALL.iter().copied().enumerate() {
            ui.mineral(
                PushButton::new(("size", i), format!("{size:?}"))
                    .size(size)
                    .icon(ICON_CALENDAR),
            );
        }
        for (i, shape) in ButtonShape::ALL.iter().copied().enumerate() {
            ui.mineral(PushButton::new(("shape", i), format!("{shape:?}")).shape(shape));
        }
        ui.mineral(
            PushButton::new("after", "Icon after")
                .icon(ICON_MAIL)
                .icon_position(IconPosition::After),
        );
        ui.mineral(PushButton::new("icon-only", "Add").icon(ICON_ADD).label_visible(false));
    });
    heading(ui, "h-toggle", "Toggle buttons");
    row(ui, |ui| {
        for (i, (a, on)) in ButtonAppearance::ALL.iter().copied().zip(&mut s.toggles).enumerate() {
            ui.mineral(
                ToggleButton::new(("toggle", i), format!("{a:?}"), on)
                    .appearance(a)
                    .icon(ICON_TEXT_BOLD),
            );
        }
    });
    heading(ui, "h-compound", "Compound, menu and split buttons");
    ui.mineral(CompoundButton::new("compound", "Example", "Secondary").icon(ICON_CALENDAR));
    row(ui, |ui| {
        if ui
            .mineral(MenuButton::new("menu", "Menu").expanded(s.menu_open))
            .clicked()
        {
            s.menu_open = !s.menu_open;
        }
        let split = SplitButton::new("split", "Save")
            .appearance(ButtonAppearance::Primary)
            .expanded(s.split_open);
        let split = ui.add(|ui| split.show(ui));
        if split.menu_clicked {
            s.split_open = !s.split_open;
        }
    });
    heading(ui, "h-link", "Links");
    row(ui, |ui| {
        ui.mineral(TextLink::new("link", "Fluent 2").url("https://fluent2.microsoft.design"));
        ui.mineral(TextLink::new("link-subtle", "Subtle").appearance(LinkAppearance::Subtle));
        ui.mineral(TextLink::new("link-inline", "Inline").inline(true));
        ui.mineral(TextLink::new("link-disabled", "Disabled").disabled(true));
    });
    ui.mineral(Text::new("clicks", format!("Clicked {} times", s.clicks)));
}

pub(crate) fn inputs(ui: &mut FluentUi<'_>, s: &mut State) {
    heading(ui, "h-label", "Labels");
    for (i, size) in Size::ALL.iter().copied().enumerate() {
        ui.mineral(FieldLabel::new(("label", i), format!("{size:?} label")).size(size));
    }
    ui.mineral(FieldLabel::new("label-required", "Required").required(true));
    ui.mineral(FieldLabel::new("label-disabled", "Disabled").disabled(true));
    super::inputs::show(ui, &mut s.inputs);
}

pub(crate) fn feedback(ui: &mut FluentUi<'_>, s: &mut State) {
    super::feedback::show(ui, &mut s.feedback);
}

pub(crate) fn surfaces(ui: &mut FluentUi<'_>, s: &mut State) {
    heading(ui, "h-divider", "Dividers");
    ui.mineral(Divider::new("plain"));
    for (i, align) in [DividerAlign::Start, DividerAlign::Center, DividerAlign::End]
        .into_iter()
        .enumerate()
    {
        ui.mineral(
            Divider::new(("align", i))
                .content(format!("{align:?}"))
                .align_content(align),
        );
    }
    for (i, a) in [
        DividerAppearance::Brand,
        DividerAppearance::Subtle,
        DividerAppearance::Strong,
    ]
    .into_iter()
    .enumerate()
    {
        ui.mineral(
            Divider::new(("appearance", i))
                .content(format!("{a:?}"))
                .appearance(a)
                .inset(true),
        );
    }
    row(ui, |ui| {
        ui.mineral(Text::new("left", "Left"));
        ui.mineral(Divider::new("vertical").axis(Axis::Vertical));
        ui.mineral(Text::new("right", "Right"));
    });
    super::surfaces::show(ui, &mut s.surfaces);
}

pub(crate) fn data(ui: &mut FluentUi<'_>, s: &mut State) {
    super::data::show(ui, &mut s.data);
    heading(ui, "h-text", "Text");
    for (i, size) in TextSize::ALL.iter().copied().take(6).enumerate() {
        ui.mineral(Text::new(("size", i), format!("{size:?}")).size(size));
    }
    ui.mineral(Text::new("bold", "Bold").weight(TextWeight::Bold));
    ui.mineral(Text::new("italic", "Italic underline").italic(true).underline(true));
    ui.mineral(Text::new("strike", "Strikethrough").strikethrough(true));
    ui.mineral(
        Text::new(
            "truncate",
            "A long line that is cut with an ellipsis where it no longer fits",
        )
        .wrap(false)
        .truncate(true),
    );
}

pub(crate) fn extras(ui: &mut FluentUi<'_>, s: &mut State) {
    super::extras::show(ui, &mut s.extras);
}

pub(crate) fn navigation(ui: &mut FluentUi<'_>, s: &mut State) {
    super::navigation::show(ui, &mut s.navigation);
}
