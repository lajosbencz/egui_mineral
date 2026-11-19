//! Showcase pages: foundations, then one page per stylekit category.

mod components;
mod data;
mod extras;
mod feedback;
mod foundations;
mod inputs;
mod navigation;
mod surfaces;

use egui::{Align, vec2};
use egui_mineral_fluent::{
    Fluent,
    catalog::Category,
    data::Text,
    egui_mineral::{
        i18n::I18n,
        layout::{Flow, FlowItems},
    },
    typography::TypeRamp,
};

use crate::Fui;

// The pages workflow bakes ICU4X data for these only.
pub(crate) const LOCALES: [&str; 5] = ["en-US", "de-DE", "hu-HU", "ja-JP", "ar-EG"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Page {
    Foundations,
    Category(Category),
}

impl Page {
    pub(crate) fn all() -> impl Iterator<Item = Self> {
        std::iter::once(Self::Foundations).chain(Category::ALL.iter().copied().map(Self::Category))
    }

    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::Foundations => "foundations",
            Self::Category(Category::Actions) => "actions",
            Self::Category(Category::Inputs) => "inputs",
            Self::Category(Category::Feedback) => "feedback",
            Self::Category(Category::Surfaces) => "surfaces",
            Self::Category(Category::Navigation) => "navigation",
            Self::Category(Category::Data) => "data",
            Self::Category(Category::Extras) => "extras",
        }
    }

    pub(crate) fn title(self, i18n: &I18n) -> String {
        match self {
            Self::Foundations => "Foundations".to_owned(),
            Self::Category(c) => c.title(i18n),
        }
    }
}

// Wrapping row with its items centered on each line: `Ui::horizontal_wrapped` centers an item in the line
// height so far, so mixed sizes end up on different centers.
#[track_caller]
pub(crate) fn row<R>(ui: &mut Fui<'_>, add: impl FnOnce(&mut FlowItems<'_, Fluent>) -> R) -> R {
    let gap = ui.spacing().item_spacing.x;
    row_spaced(ui, gap, add)
}

#[track_caller]
pub(crate) fn row_spaced<R>(ui: &mut Fui<'_>, gap: f32, add: impl FnOnce(&mut FlowItems<'_, Fluent>) -> R) -> R {
    let line_gap = ui.spacing().item_spacing.y;
    Flow::new(vec2(gap, line_gap), Align::Center).show(ui, add)
}

pub(crate) fn heading(ui: &mut Fui<'_>, salt: &'static str, text: &str) {
    ui.mineral(Text::new(salt, text).ramp(TypeRamp::Subtitle1));
}

pub(crate) fn show(page: Page, ui: &mut Fui<'_>, state: &mut components::State) {
    let title = page.title(ui.i18n());
    ui.mineral(Text::new("page-title", title).ramp(TypeRamp::Title2));
    match page {
        Page::Foundations => foundations::show(ui),
        Page::Category(Category::Actions) => components::actions(ui, state),
        Page::Category(Category::Inputs) => components::inputs(ui, state),
        Page::Category(Category::Surfaces) => components::surfaces(ui, state),
        Page::Category(Category::Data) => components::data(ui, state),
        Page::Category(Category::Feedback) => components::feedback(ui, state),
        Page::Category(Category::Navigation) => components::navigation(ui, state),
        Page::Category(Category::Extras) => components::extras(ui, state),
    }
}

pub(crate) use components::State;
