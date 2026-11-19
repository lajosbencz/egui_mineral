pub(crate) mod actions;
pub(crate) mod communication;
pub(crate) mod containment;
pub(crate) mod data;
pub(crate) mod foundations;
pub(crate) mod interop;
pub(crate) mod navigation;
pub(crate) mod selection;
pub(crate) mod system;
pub(crate) mod text_inputs;

pub(crate) use egui_mineral_material::adaptive::{wrapping_row as row, wrapping_row_top as top_row};

use crate::Mui;
use egui::RichText;
use egui_mineral_material::MaterialExt;
use std::sync::Arc;

use egui_mineral::bus::{Bus, Spawner};
use egui_mineral_material::{
    adaptive,
    catalog::{ComponentMeta, Origin},
    color::SchemeVariant,
    prelude::*,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, egui_mineral::nav::Route)]
pub(crate) enum Page {
    #[route("/")]
    Foundations,
    #[route("/actions")]
    Actions,
    #[route("/communication")]
    Communication,
    #[route("/containment")]
    Containment,
    #[route("/navigation")]
    Navigation,
    #[route("/selection")]
    Selection,
    #[route("/text-inputs")]
    TextInputs,
    #[route("/data")]
    Data,
    #[route("/system")]
    System,
    #[route("/interop")]
    Interop,
}

impl Page {
    pub(crate) const ALL: [Self; 10] = [
        Self::Foundations,
        Self::Actions,
        Self::Communication,
        Self::Containment,
        Self::Navigation,
        Self::Selection,
        Self::TextInputs,
        Self::Data,
        Self::System,
        Self::Interop,
    ];

    pub(crate) fn index(self) -> usize {
        Self::ALL.iter().position(|p| *p == self).expect("ALL lists every page")
    }

    pub(crate) fn title(self) -> &'static str {
        match self {
            Self::Foundations => "Foundations",
            Self::Actions => "Actions",
            Self::Communication => "Communication",
            Self::Containment => "Containment",
            Self::Navigation => "Navigation",
            Self::Selection => "Selection",
            Self::TextInputs => "Text inputs",
            Self::Data => "Data display",
            Self::System => "Core runtime",
            Self::Interop => "egui interop",
        }
    }

    pub(crate) fn icon(self) -> MaterialIcon {
        match self {
            Self::Foundations => icons::ICON_PALETTE,
            Self::Actions => icons::ICON_TOUCH_APP,
            Self::Communication => icons::ICON_NOTIFICATIONS,
            Self::Containment => icons::ICON_WIDGETS,
            Self::Navigation => icons::ICON_EXPLORE,
            Self::Selection => icons::ICON_CHECK_BOX,
            Self::TextInputs => icons::ICON_KEYBOARD,
            Self::Data => icons::ICON_TABLE_CHART,
            Self::System => icons::ICON_MEMORY,
            Self::Interop => icons::ICON_EXTENSION,
        }
    }

    pub(crate) fn topics(self) -> &'static [&'static str] {
        match self {
            Self::Foundations => foundations::TOPICS,
            Self::Actions => actions::TOPICS,
            Self::Communication => communication::TOPICS,
            Self::Containment => containment::TOPICS,
            Self::Navigation => navigation::TOPICS,
            Self::Selection => selection::TOPICS,
            Self::TextInputs => text_inputs::TOPICS,
            Self::Data => data::TOPICS,
            Self::System => system::TOPICS,
            Self::Interop => interop::TOPICS,
        }
    }
}

pub(crate) const VARIANTS: [(SchemeVariant, &str); 9] = [
    (SchemeVariant::TonalSpot, "Tonal spot"),
    (SchemeVariant::Neutral, "Neutral"),
    (SchemeVariant::Vibrant, "Vibrant"),
    (SchemeVariant::Expressive, "Expressive"),
    (SchemeVariant::Fidelity, "Fidelity"),
    (SchemeVariant::Content, "Content"),
    (SchemeVariant::Rainbow, "Rainbow"),
    (SchemeVariant::FruitSalad, "Fruit salad"),
    (SchemeVariant::Monochrome, "Monochrome"),
];

pub(crate) const THEMES: [(egui::ThemePreference, &str); 3] = [
    (egui::ThemePreference::System, "System"),
    (egui::ThemePreference::Light, "Light"),
    (egui::ThemePreference::Dark, "Dark"),
];

pub(crate) const LOCALES: [&str; 5] = ["en-US", "de-DE", "hu-HU", "ja-JP", "ar-EG"];

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "RawSettings")]
pub(crate) struct Settings {
    pub(crate) theme: usize,
    pub(crate) high_contrast: bool,
    pub(crate) global_kit: bool,
    pub(crate) seed: [u8; 3],
    pub(crate) variant: usize,
    pub(crate) density: i8,
    pub(crate) locale: usize,
}

#[derive(Deserialize)]
struct RawSettings {
    theme: usize,
    high_contrast: bool,
    #[serde(default)]
    global_kit: bool,
    seed: [u8; 3],
    variant: usize,
    density: i8,
    locale: usize,
}

impl TryFrom<RawSettings> for Settings {
    type Error = String;

    fn try_from(r: RawSettings) -> Result<Self, String> {
        if r.theme >= THEMES.len() {
            return Err(format!("theme must be < {}", THEMES.len()));
        }
        if r.variant >= VARIANTS.len() {
            return Err(format!("variant must be < {}", VARIANTS.len()));
        }
        if r.locale >= LOCALES.len() {
            return Err(format!("locale must be < {}", LOCALES.len()));
        }
        if !(Density::MIN.step()..=0).contains(&r.density) {
            return Err(format!("density must be in {}..=0", Density::MIN.step()));
        }
        Ok(Self {
            theme: r.theme,
            high_contrast: r.high_contrast,
            global_kit: r.global_kit,
            seed: r.seed,
            variant: r.variant,
            density: r.density,
            locale: r.locale,
        })
    }
}

impl Default for Settings {
    fn default() -> Self {
        let s = egui_mineral_material::color::BASELINE_SEED;
        Self {
            theme: 0,
            high_contrast: false,
            global_kit: false,
            seed: [s.r(), s.g(), s.b()],
            variant: 0,
            density: 0,
            locale: 0,
        }
    }
}

pub(crate) enum Cmd {
    Work { id: u32, millis: u64 },
}

pub(crate) enum Evt {
    Done { id: u32 },
    Platform(Result<String, String>),
}

pub(crate) struct Shared {
    pub(crate) settings: Settings,
    pub(crate) toasts: ToastQueue<Snackbar>,
    pub(crate) bus: Bus<Cmd, Evt>,
    pub(crate) done: Vec<u32>,
    pub(crate) platform: Vec<Result<String, String>>,
    pub(crate) spawner: Arc<dyn Spawner>,
}

pub(crate) mod snack {
    use egui_mineral::shortcut::ActionId;

    pub(crate) const UNDO: ActionId = ActionId::from_static("snack.undo");
    pub(crate) const OPEN: ActionId = ActionId::from_static("snack.open");
    pub(crate) const RETRY: ActionId = ActionId::from_static("snack.retry");
}

impl Shared {
    pub(crate) fn toast(&mut self, text: impl Into<String>) {
        self.toasts.push(Snackbar::new(text));
    }
}

#[derive(Default)]
pub(crate) struct States {
    pub(crate) foundations: foundations::State,
    pub(crate) actions: actions::State,
    pub(crate) communication: communication::State,
    pub(crate) containment: containment::State,
    pub(crate) navigation: navigation::State,
    pub(crate) selection: selection::State,
    pub(crate) text_inputs: text_inputs::State,
    pub(crate) data: data::State,
    pub(crate) system: system::State,
    pub(crate) interop: interop::State,
}

pub(crate) fn show(page: Page, ui: &mut Mui<'_>, s: &mut States, shared: &mut Shared) {
    match page {
        Page::Foundations => foundations::show(ui, &mut s.foundations, shared),
        Page::Actions => actions::show(ui, &mut s.actions, shared),
        Page::Communication => communication::show(ui, &mut s.communication, shared),
        Page::Containment => containment::show(ui, &mut s.containment, shared),
        Page::Navigation => navigation::show(ui, &mut s.navigation, shared),
        Page::Selection => selection::show(ui, &mut s.selection, shared),
        Page::TextInputs => text_inputs::show(ui, &mut s.text_inputs, shared),
        Page::Data => data::show(ui, &mut s.data, shared),
        Page::System => system::show(ui, &mut s.system, shared),
        Page::Interop => interop::show_page(ui, &mut s.interop, shared),
    }
}

pub(crate) fn caption(ui: &mut Mui<'_>, scale: TypeScale, text: &str) {
    let rt = ui.rt();
    let t = rt.tokens();
    ui.label(
        RichText::new(text)
            .font(scale.font(rt.ctx()))
            .color(t.color.on_surface_variant),
    );
}

pub(crate) fn sub(ui: &mut Mui<'_>, text: &str) {
    let rt = ui.rt();
    let t = rt.tokens();
    ui.add_space(adaptive::grid(2));
    ui.label(
        RichText::new(text)
            .font(TypeScale::LabelLarge.font(rt.ctx()))
            .color(t.color.on_surface_variant),
    );
}

pub(crate) fn choice(ui: &mut Mui<'_>, id: &'static str, label: &str, labels: &[&str], current: usize) -> usize {
    let mut sel = Selection::single().required(true).with([current]);
    ButtonGroup::new(id, label)
        .variant(ButtonGroupVariant::Connected)
        .items(labels.iter().map(|l| ButtonGroupItem::new(*l)))
        .selection(&mut sel)
        .show(ui);
    sel.first().copied().unwrap_or(current)
}

pub(crate) fn count(ui: &mut Mui<'_>, id: &'static str, label: &str, value: &mut usize, max: usize) {
    let rt = ui.rt();
    let mut v = *value as f64;
    rt.ui(ui).mineral(
        NumberField::new(id, label, RangeModel::new(0.0, max as f64).step(1.0), &mut v).preferred_width(140.0),
    );
    #[expect(clippy::cast_sign_loss, reason = "the range starts at 0")]
    let rounded = v.round() as usize;
    *value = rounded;
}

pub(crate) fn section(
    ui: &mut Mui<'_>,
    title: &'static str,
    meta: Option<ComponentMeta>,
    add: impl FnOnce(&mut Mui<'_>),
) {
    let rt = ui.rt();
    ui.push_id(title, |ui| section_body(&mut rt.ui(ui), title, meta, add));
}

fn section_body(ui: &mut Mui<'_>, title: &'static str, meta: Option<ComponentMeta>, add: impl FnOnce(&mut Mui<'_>)) {
    let rt = ui.rt();
    let t = rt.tokens();
    ui.add_space(adaptive::grid(6));
    ui.label(RichText::new(title).font(TypeScale::TitleLarge.font(rt.ctx())));
    if let Some(m) = meta {
        row(ui, |row| {
            let origin = match m.origin {
                Origin::Md3 => "Material 3".to_owned(),
                Origin::Md3Expressive => "Material 3 Expressive".to_owned(),
                Origin::Other(kits) => format!("After {kits:?}"),
            };
            row.add(|ui| {
                ui.label(
                    RichText::new(format!("{} - {} - {origin}", m.name, m.category.title(rt.i18n())))
                        .font(TypeScale::BodySmall.font(rt.ctx()))
                        .color(t.color.on_surface_variant),
                )
            });
            row.mineral(
                TextLink::new((title, "spec"), "Spec")
                    .url(m.spec)
                    .new_tab(true)
                    .scale(TypeScale::LabelMedium),
            );
        });
    }
    ui.add_space(adaptive::grid(2));
    add(ui);
    ui.add_space(adaptive::grid(2));
    rt.ui(ui).mineral(Divider::new("section-divider"));
}
