use crate::Material;
use crate::MaterialExt;
use crate::Rt;
use crate::TextAction;
use crate::msg;
use egui_mineral::semantic::AgentKey;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;
use std::sync::Arc;

use egui::{Align, Color32, Galley, Layout, Rangef, Rect, Sense, Stroke, Ui, pos2, text::LayoutJob, vec2};
use egui_mineral::{
    i18n::Direction,
    kit,
    layout::Constraints,
    semantic::{self, Role, SemanticNode, StableId},
    widget::{Composite, WidgetResponse},
};

use crate::actions::{ButtonVariant, PushButton};
use crate::widget::{common, parts, row_parts};
use crate::{
    Icon,
    spec::{
        ColorRole, TypeScale, defaults,
        tokens::{button_small, dialog, divider, filled_icon_button, filled_tonal_icon_button, list},
    },
};

pub(crate) fn pair(rt: &Rt, first: &str, second: &str) -> String {
    rt.i18n().tr(msg::Pair {
        first: first.into(),
        second: second.into(),
    })
}

fn centered(ui: &Ui, text: &str, scale: TypeScale, color: Color32, width: f32) -> Arc<Galley> {
    let mut job = LayoutJob::single_section(text.to_owned(), scale.format(ui.ctx(), color));
    job.wrap.max_width = width.max(0.0);
    job.halign = Align::Center;
    ui.painter().layout_job(job)
}

fn paint_centered(ui: &mut Ui, g: Arc<Galley>) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), g.size().y), Sense::hover());
    ui.painter().galley(rect.center_top(), g, Color32::PLACEHOLDER);
}

// MD3 has no empty state
const EMPTY_ACTION: ButtonVariant = ButtonVariant::Tonal;

/// Empty-state illustration with title, description and optional action; non-MD3.
#[derive(Clone, Debug)]
#[must_use = "call .show(..)"]
pub struct EmptyPlaceholder {
    salt: KeyedSalt,
    icon: Option<Icon>,
    title: String,
    description: Option<String>,
    action: Option<TextAction>,
}

/// Result of [`EmptyPlaceholder::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct EmptyPlaceholderResponse {
    /// Placeholder area response.
    pub response: egui::Response,
    /// Action button clicked this frame.
    pub action_clicked: bool,
}

impl EmptyPlaceholder {
    /// `title` is the headline and accessible name; default icon [`crate::IconSet::inbox`]. Agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>, title: impl Into<String>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            icon: None,
            title: title.into(),
            description: None,
            action: None,
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    /// Large icon above the title; default [`crate::IconSet::inbox`].
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Body text below the title.
    pub fn supporting(mut self, supporting: impl Into<String>) -> Self {
        self.description = Some(supporting.into());
        self
    }

    /// Filled button below the text; reports [`EmptyPlaceholderResponse::action_clicked`].
    pub fn action(mut self, action: impl Into<TextAction>) -> Self {
        self.action = Some(action.into());
        self
    }

    /// Centers the placeholder in the available width.
    pub fn show(self, ui: &mut MineralUi<'_, Material>) -> EmptyPlaceholderResponse {
        let rt = ui.rt();
        let t = rt.tokens();
        let ctx = ui.ctx().clone();
        let id = ui.make_persistent_id(self.salt.id_salt());
        semantic::emit_parent(&ctx, id, || {
            let mut n = SemanticNode::new(StableId::of(&self.salt, "empty"), Role::Group, &self.title);
            n.value.clone_from(&self.description);
            n
        });
        let mut clicked = false;
        let r = ui.with_layout(Layout::top_down(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let width = ui.available_width().min(defaults::dialog::MAX_WIDTH);
            ui.add_space(defaults::dialog::PADDING);
            let (ir, _) = ui.allocate_exact_size(vec2(dialog::ICON_SIZE, dialog::ICON_SIZE), Sense::hover());
            common::icon(
                ui.painter(),
                ir.center(),
                self.icon.unwrap_or(rt.icons().inbox),
                dialog::ICON_SIZE,
                t.role(dialog::ICON_COLOR),
            );
            ui.add_space(defaults::dialog::ICON_BOTTOM_PADDING);
            let title = centered(
                ui,
                &self.title,
                dialog::HEADLINE_FONT,
                t.role(dialog::HEADLINE_COLOR),
                width,
            );
            paint_centered(ui, title);
            if let Some(d) = &self.description {
                ui.add_space(defaults::dialog::TITLE_BOTTOM_PADDING);
                let g = centered(
                    ui,
                    d,
                    dialog::SUPPORTING_TEXT_FONT,
                    t.role(dialog::SUPPORTING_TEXT_COLOR),
                    width,
                );
                paint_centered(ui, g);
            }
            if let Some(a) = &self.action {
                ui.add_space(defaults::dialog::TEXT_BOTTOM_PADDING);
                clicked = rt
                    .ui(ui)
                    .mineral(
                        PushButton::new(self.salt.child("action"), &a.label)
                            .variant(EMPTY_ACTION)
                            .agent_key(StableId::of(&self.salt, "button")),
                    )
                    .clicked();
                if clicked {
                    a.fire(&ctx);
                }
            }
            ui.add_space(defaults::dialog::PADDING);
        });
        kit::semantic::remember_rect(&ctx, id, r.response.rect);
        EmptyPlaceholderResponse {
            response: r.response,
            action_clicked: clicked,
        }
    }
}

/// Change shown on a [`Statistic`], as percent or absolute.
#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub enum Trend {
    /// Change in percent; formatted with a percent sign.
    Percent(f64),
    /// Change in the value's own unit.
    Absolute(f64),
}

impl Trend {
    fn value(self) -> f64 {
        match self {
            Self::Percent(v) | Self::Absolute(v) => v,
        }
    }
}

// No MD3 counterpart
const STAT_LABEL_FONT: TypeScale = TypeScale::LabelLarge;
const STAT_VALUE_FONT: TypeScale = TypeScale::HeadlineMedium;
const STAT_HELP_FONT: TypeScale = TypeScale::BodySmall;

/// Labelled number with unit and trend; non-MD3 (Chakra `Stat` pattern).
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Statistic {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    value: f64,
    fraction_digits: Option<u8>,
    unit: Option<String>,
    trend: Option<Trend>,
    inverted: bool,
    help: Option<String>,
}

impl Statistic {
    /// `label` names the value; `value` is formatted per locale. Agent key `<id_salt>-statistic`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, value: f64) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            key: egui_mineral::agent_key!("statistic"),
            salt: id_salt,
            label: label.into(),
            value,
            fraction_digits: None,
            unit: None,
            trend: None,
            inverted: false,
            help: None,
        }
    }

    /// Fixed fraction digits of the value and absolute trends; default the locale format.
    pub fn fraction_digits(mut self, fraction_digits: u8) -> Self {
        self.fraction_digits = Some(fraction_digits);
        self
    }

    /// Text after the value, e.g. a currency or unit.
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    /// Arrow and change below the value; increases read as good.
    pub fn trend(mut self, trend: Trend) -> Self {
        self.trend = Some(trend);
        self
    }

    /// Decreases read as good, e.g. for costs.
    pub fn inverted(mut self, inverted: bool) -> Self {
        self.inverted = inverted;
        self
    }

    /// Small text below the value.
    pub fn supporting(mut self, supporting: impl Into<String>) -> Self {
        self.help = Some(supporting.into());
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-statistic` agent key.
        agent_key(key)
    }

    fn render(&mut self, ui: &mut MineralUi<'_, Material>) -> egui::Response {
        let rt = ui.rt();
        let t = ui.tokens();
        let ctx = ui.ctx().clone();
        let fmt = rt.i18n().format();
        let dir = rt.i18n().direction();
        let digits = self.fraction_digits.map(i16::from);
        let number = fmt.number_lossy(self.value, digits);
        let value = match &self.unit {
            Some(u) => rt.i18n().tr(msg::StatUnit {
                value: (&number).into(),
                unit: u.into(),
            }),
            None => number,
        };
        let trend = self.trend.map(|tr| {
            let v = tr.value();
            let shown = match tr {
                Trend::Percent(_) => {
                    let mag = fmt.number_lossy(v.abs(), Some(1));
                    rt.i18n().tr(msg::StatPercent { value: (&mag).into() })
                }
                Trend::Absolute(_) => fmt.number_lossy(v.abs(), digits),
            };
            let sign = v.partial_cmp(&0.0);
            let icon = match sign {
                Some(std::cmp::Ordering::Greater) => ui.icons().trending_up,
                Some(std::cmp::Ordering::Less) => ui.icons().trending_down,
                _ => ui.icons().trending_flat,
            };
            let good = (v > 0.0) != self.inverted;
            let color = if v == 0.0 {
                t.color.on_surface_variant
            } else if good {
                t.color.tertiary
            } else {
                t.color.error
            };
            (icon, shown, color, sign)
        });
        let r = ui.with_layout(dir.column(), |ui| {
            ui.spacing_mut().item_spacing.y = crate::adaptive::grid(1);
            parts::type_label(ui, &self.label, STAT_LABEL_FONT, t.color.on_surface_variant, 1);
            parts::type_label(ui, &value, STAT_VALUE_FONT, t.color.on_surface, 1);
            if let Some((icon, shown, color, _)) = &trend {
                parts::row(&mut rt.ui(ui), dir, STAT_LABEL_FONT.line_height(), |ui| {
                    ui.spacing_mut().item_spacing.x = button_small::ICON_LABEL_SPACE;
                    let size = defaults::button::ICON_SIZE;
                    let (ir, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
                    common::icon(ui.painter(), ir.center(), *icon, size, *color);
                    parts::type_label(ui, shown, STAT_LABEL_FONT, *color, 1);
                });
            }
            if let Some(h) = &self.help {
                parts::type_label(ui, h, STAT_HELP_FONT, t.color.on_surface_variant, 0);
            }
        });
        semantic::emit(&ctx, Some(r.response.id), r.response.rect, || {
            let mut n = SemanticNode::new(self.key.id(&self.salt), Role::Group, &self.label).value(&value);
            if let Some((_, shown, _, sign)) = &trend {
                let shown = shown.into();
                let change = match sign {
                    Some(std::cmp::Ordering::Greater) => rt.i18n().tr(msg::StatIncrease { value: shown }),
                    Some(std::cmp::Ordering::Less) => rt.i18n().tr(msg::StatDecrease { value: shown }),
                    _ => rt.i18n().tr(msg::STAT_UNCHANGED),
                };
                n.value = Some(pair(rt, &value, &change));
            }
            n
        });
        r.response
    }
}

impl Composite for Statistic {
    type Kit = Material;
    type Output = ();

    egui_mineral::kit::widget_meta!(salt);

    fn compose(&mut self, ui: &mut MineralUi<'_, Material>, _: Constraints) -> WidgetResponse {
        WidgetResponse::new(self.render(ui), ())
    }
}

/// Label placement of a [`DescriptionList`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum DescriptionLayout {
    #[default]
    /// Term beside its detail.
    Horizontal,
    /// Term above its detail.
    Vertical,
}

// No MD3 counterpart: term and detail pairs in list item supporting/headline roles.
const TERM_FONT: TypeScale = TypeScale::LabelLarge;
const TERM_COLOR: ColorRole = list::ITEM_SUPPORTING_TEXT_COLOR;
const DETAIL_FONT: TypeScale = list::ITEM_SUPPORTING_TEXT_FONT;
const DETAIL_COLOR: ColorRole = list::ITEM_LABEL_TEXT_COLOR;
const TERM_MAX_FRACTION: f32 = 0.4;

/// Term and detail of a [`DescriptionList`].
#[derive(Clone, Debug, Default)]
#[must_use]
pub struct DescriptionItem {
    term: String,
    detail: String,
}

impl DescriptionItem {
    /// Item labeled `term`, with an empty detail.
    pub fn term(term: impl Into<String>) -> Self {
        Self {
            term: term.into(),
            detail: String::new(),
        }
    }

    /// Value shown for the term.
    pub fn detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = detail.into();
        self
    }
}

/// Label and value pairs in columns; non-MD3 (Ant `Descriptions` pattern).
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct DescriptionList {
    salt: KeyedSalt,
    label: Option<String>,
    items: Vec<DescriptionItem>,
    columns: usize,
    layout: DescriptionLayout,
    dividers: bool,
}

impl DescriptionList {
    /// One column, horizontal, no dividers; agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            label: None,
            items: Vec::new(),
            columns: 1,
            layout: DescriptionLayout::default(),
            dividers: false,
        }
    }

    /// Accessible name of the list; default `mineral-description-list`.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Appends an item.
    pub fn item(mut self, item: impl Into<DescriptionItem>) -> Self {
        self.items.push(item.into());
        self
    }

    /// Appends items.
    pub fn items(mut self, items: impl IntoIterator<Item = impl Into<DescriptionItem>>) -> Self {
        self.items.extend(items.into_iter().map(Into::into));
        self
    }

    /// Pairs per row; minimum 1.
    #[track_caller]
    pub fn columns(mut self, columns: usize) -> Self {
        self.columns = egui_mineral::kit::sanitize::at_least(columns, 1.., "DescriptionList columns");
        self
    }

    /// Default [`DescriptionLayout::Horizontal`].
    pub fn layout(mut self, layout: DescriptionLayout) -> Self {
        self.layout = layout;
        self
    }

    /// Divider below each row.
    pub fn dividers(mut self, dividers: bool) -> Self {
        self.dividers = dividers;
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    fn measure_row(
        &self,
        ui: &Ui,
        t: &crate::tokens::Tokens,
        cols: &Cols,
        row_i: usize,
        row: &[DescriptionItem],
    ) -> (f32, Vec<DescCell>) {
        let mut row_h: f32 = 0.0;
        let mut cells = Vec::with_capacity(row.len());
        for (ci, DescriptionItem { term, detail }) in row.iter().enumerate() {
            let off = (cols.col_w + cols.gap) * ci as f32;
            let x0 = cols.dir.span_x(cols.span, Rangef::new(off, off + cols.col_w));
            let (tw, dw) = match self.layout {
                DescriptionLayout::Horizontal => (cols.term_w, cols.col_w - cols.term_w - cols.gap),
                DescriptionLayout::Vertical => (cols.col_w, cols.col_w),
            };
            let p = ui.painter();
            let tg = parts::type_wrapped(p, term, TERM_FONT, t.role(TERM_COLOR), tw, 0);
            let dg = parts::type_wrapped(p, detail, DETAIL_FONT, t.role(DETAIL_COLOR), dw, 0);
            let h = match self.layout {
                DescriptionLayout::Horizontal => tg.size().y.max(dg.size().y),
                DescriptionLayout::Vertical => tg.size().y + crate::adaptive::grid(1) + dg.size().y,
            };
            row_h = row_h.max(h);
            cells.push(DescCell {
                x0,
                tw,
                term: tg,
                detail: dg,
                idx: row_i * self.columns + ci,
            });
        }
        (row_h, cells)
    }

    fn place_cell(&self, ui: &Ui, cols: &Cols, cell: &DescCell, top: f32) {
        let ctx = ui.ctx();
        let (x0, tw, idx) = (cell.x0, cell.tw, cell.idx);
        let cell_span = egui::Rangef::new(x0, x0 + cols.col_w);
        let place = |from: f32, gw: f32| cols.dir.span_x(cell_span, Rangef::new(from, from + gw));
        let tp = pos2(place(0.0, cell.term.size().x), top);
        let dp = match self.layout {
            DescriptionLayout::Horizontal => pos2(place(tw + cols.gap, cell.detail.size().x), top),
            DescriptionLayout::Vertical => pos2(
                place(0.0, cell.detail.size().x),
                top + cell.term.size().y + crate::adaptive::grid(1),
            ),
        };
        let (tr, dr) = (
            Rect::from_min_size(tp, cell.term.size()),
            Rect::from_min_size(dp, cell.detail.size()),
        );
        if ui.is_rect_visible(tr.union(dr)) {
            ui.painter().galley(tp, cell.term.clone(), Color32::PLACEHOLDER);
            ui.painter().galley(dp, cell.detail.clone(), Color32::PLACEHOLDER);
        }
        let DescriptionItem { term, detail } = &self.items[cell.idx];
        semantic::emit(ctx, None, tr, || {
            let mut n = SemanticNode::new(StableId::of(&self.salt, (idx, "term")), Role::Term, term);
            n.depth = 1;
            n
        });
        semantic::emit(ctx, None, dr, || {
            let mut n = SemanticNode::new(StableId::of(&self.salt, (idx, "definition")), Role::Definition, detail);
            n.depth = 1;
            n
        });
    }

    fn render(&mut self, ui: &mut MineralUi<'_, Material>) -> egui::Response {
        let rt = ui.rt();
        let t = ui.tokens();
        let ctx = ui.ctx().clone();
        let dir = rt.i18n().direction();
        let id = ui.make_persistent_id(self.salt.id_salt());
        semantic::emit_parent(&ctx, id, || {
            let name = crate::tr_or(rt.i18n(), self.label.as_deref(), msg::DESCRIPTION_LIST);
            SemanticNode::new(
                StableId::of(&self.salt, "description-list"),
                Role::DescriptionList,
                name,
            )
        });
        let gap = crate::adaptive::grid(4);
        let total = ui.available_width();
        let col_w = ((total - gap * (self.columns - 1) as f32) / self.columns as f32).max(0.0);
        let term_w = self
            .items
            .iter()
            .map(|item| {
                parts::type_wrapped(
                    ui.painter(),
                    &item.term,
                    TERM_FONT,
                    Color32::PLACEHOLDER,
                    f32::INFINITY,
                    1,
                )
                .size()
                .x
            })
            .fold(0.0, f32::max)
            .min(col_w * TERM_MAX_FRACTION);
        let origin = ui.cursor().min;
        let cols = Cols {
            dir,
            span: egui::Rangef::new(origin.x, origin.x + total),
            col_w,
            gap,
            term_w,
        };
        let mut y = origin.y;
        for (row_i, row) in self.items.chunks(self.columns).enumerate() {
            let (row_h, cells) = self.measure_row(ui, &t, &cols, row_i, row);
            let pad = crate::adaptive::grid(2);
            for cell in cells {
                self.place_cell(ui, &cols, &cell, y + pad);
            }
            y += row_h + 2.0 * pad;
            if self.dividers {
                ui.painter().hline(
                    origin.x..=origin.x + total,
                    y,
                    Stroke::new(divider::THICKNESS, t.role(divider::COLOR)),
                );
            }
        }
        let (rect, response) = ui.allocate_exact_size(vec2(total, y - origin.y), Sense::hover());
        kit::semantic::remember_rect(&ctx, id, rect);
        response
    }
}

#[derive(Clone, Copy)]
struct Cols {
    dir: Direction,
    span: egui::Rangef,
    col_w: f32,
    gap: f32,
    term_w: f32,
}

struct DescCell {
    x0: f32,
    tw: f32,
    term: Arc<Galley>,
    detail: Arc<Galley>,
    idx: usize,
}

impl Composite for DescriptionList {
    type Kit = Material;
    type Output = ();

    egui_mineral::kit::widget_meta!(salt);

    fn compose(&mut self, ui: &mut MineralUi<'_, Material>, _: Constraints) -> WidgetResponse {
        WidgetResponse::new(self.render(ui), ())
    }
}

/// Event of a [`Timeline`].
#[derive(Clone, Debug)]
#[must_use]
pub struct TimelineItem {
    title: String,
    time: Option<String>,
    description: Option<String>,
    icon: Option<Icon>,
    active: bool,
}

impl TimelineItem {
    /// `title` is the event headline.
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            time: None,
            description: None,
            icon: None,
            active: false,
        }
    }
    /// Timestamp overline above the title, preformatted.
    pub fn time(mut self, time: impl Into<String>) -> Self {
        self.time = Some(time.into());
        self
    }
    /// Body text below the title.
    pub fn supporting(mut self, supporting: impl Into<String>) -> Self {
        self.description = Some(supporting.into());
        self
    }
    /// Marker icon; without it the marker is a dot.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }
    /// Highlights the marker as current.
    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
}

impl TimelineItem {
    fn galleys(&self, p: &egui::Painter, width: f32, t: &crate::tokens::Tokens) -> Vec<Arc<Galley>> {
        let line = |text: &str, font, color| parts::type_wrapped(p, text, font, t.role(color), width, 0);
        let mut galleys = Vec::with_capacity(3);
        if let Some(tm) = &self.time {
            galleys.push(line(tm, list::ITEM_OVERLINE_FONT, list::ITEM_OVERLINE_COLOR));
        }
        galleys.push(line(
            &self.title,
            list::ITEM_LABEL_TEXT_FONT,
            list::ITEM_LABEL_TEXT_COLOR,
        ));
        if let Some(d) = &self.description {
            galleys.push(line(
                d,
                list::ITEM_SUPPORTING_TEXT_FONT,
                list::ITEM_SUPPORTING_TEXT_COLOR,
            ));
        }
        galleys
    }
}

/// Vertical list of events on a rail; non-MD3 (MUI `Timeline` pattern).
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Timeline {
    salt: KeyedSalt,
    label: Option<String>,
    items: Vec<TimelineItem>,
}

// No MD3 counterpart
const RAIL: f32 = row_parts::MARKER;
const DOT: f32 = RAIL / 2.0;

impl Timeline {
    /// Vertical list of events; agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            label: None,
            items: Vec::new(),
        }
    }

    /// Accessible name of the list; default `mineral-timeline`.
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Appends one event.
    pub fn item(mut self, item: impl Into<TimelineItem>) -> Self {
        self.items.push(item.into());
        self
    }

    /// Appends events.
    pub fn items(mut self, items: impl IntoIterator<Item = impl Into<TimelineItem>>) -> Self {
        self.items.extend(items.into_iter().map(Into::into));
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    fn render(&mut self, ui: &mut MineralUi<'_, Material>) -> egui::Response {
        let rt = ui.rt();
        let t = ui.tokens();
        let ctx = ui.ctx().clone();
        let dir = rt.i18n().direction();
        let id = ui.make_persistent_id(self.salt.id_salt());
        semantic::emit_parent(&ctx, id, || {
            let name = crate::tr_or(rt.i18n(), self.label.as_deref(), msg::TIMELINE);
            SemanticNode::new(StableId::of(&self.salt, "timeline"), Role::List, name)
        });
        let total = ui.available_width();
        let origin = ui.cursor().min;
        let gap = defaults::list::LEADING_CONTENT_END_PADDING;
        let text_w = (total - RAIL - gap).max(0.0);
        let row = Rangef::new(origin.x, origin.x + total);
        let rail_x = dir.span_x(row, Rangef::point(RAIL / 2.0));
        let text_x = dir.span_x(row, Rangef::new(RAIL + gap, RAIL + gap + text_w));
        let line = Stroke::new(divider::THICKNESS, t.role(divider::COLOR));
        let n = self.items.len();
        let mut y = origin.y;
        for (i, item) in self.items.iter().enumerate() {
            let p = ui.painter();
            let galleys = item.galleys(p, text_w, &t);
            let content_h: f32 = galleys.iter().map(|g| g.size().y).sum();
            let marker = if item.icon.is_some() { RAIL } else { DOT };
            let item_h = content_h.max(RAIL) + if i + 1 < n { crate::adaptive::grid(6) } else { 0.0 };
            let rect = Rect::from_min_size(pos2(origin.x, y), vec2(total, item_h));
            if ui.is_rect_visible(rect) {
                let center = pos2(rail_x, y + RAIL / 2.0);
                if i > 0 {
                    p.vline(rail_x, y..=center.y - marker / 2.0, line);
                }
                if i + 1 < n {
                    p.vline(rail_x, center.y + marker / 2.0..=y + item_h, line);
                }
                let (fill, fg) = if item.active {
                    (
                        t.role(filled_icon_button::CONTAINER_COLOR),
                        t.role(filled_icon_button::COLOR),
                    )
                } else {
                    (
                        t.role(filled_tonal_icon_button::CONTAINER_COLOR),
                        t.role(filled_tonal_icon_button::COLOR),
                    )
                };
                row_parts::marker(p, center, marker, fill, item.icon.map(|icon| (icon, fg)));
                let mut gy = y;
                for g in galleys {
                    let h = g.size().y;
                    let gx = dir.span_x(
                        egui::Rangef::new(text_x, text_x + text_w),
                        Rangef::new(0.0, 0.0 + g.size().x),
                    );
                    p.galley(pos2(gx, gy), g, Color32::PLACEHOLDER);
                    gy += h;
                }
            }
            semantic::emit(&ctx, None, rect, || {
                let mut node = SemanticNode::new(StableId::of(&self.salt, (i, "item")), Role::ListItem, &item.title);
                node.value = match (&item.time, &item.description) {
                    (Some(tm), Some(d)) => Some(pair(rt, tm, d)),
                    (Some(s), None) | (None, Some(s)) => Some(s.clone()),
                    (None, None) => None,
                };
                node.depth = 1;
                node
            });
            y += item_h;
        }
        let (rect, response) = ui.allocate_exact_size(vec2(total, y - origin.y), Sense::hover());
        kit::semantic::remember_rect(&ctx, id, rect);
        response
    }
}

impl Composite for Timeline {
    type Kit = Material;
    type Output = ();

    egui_mineral::kit::widget_meta!(salt);

    fn compose(&mut self, ui: &mut MineralUi<'_, Material>, _: Constraints) -> WidgetResponse {
        WidgetResponse::new(self.render(ui), ())
    }
}
