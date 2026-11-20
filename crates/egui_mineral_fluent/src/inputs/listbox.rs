use crate::tokens::ShadowTint;
use egui::{
    CornerRadius, Frame, Id, LayerId, Margin, Rangef, Rect, Response, ScrollArea, Sense, Stroke, StrokeKind, Vec2, vec2,
};
use egui_mineral::{
    anchor::{Align, Anchor, Side},
    overlay::Popup,
    semantic::{self, KeyedSalt, Role, SemanticNode, StableId},
    widget::{Availability, MineralUi},
};

use crate::spec::ColorToken::{
    ColorCompoundBrandBackground, ColorNeutralBackground1, ColorNeutralBackground1Hover,
    ColorNeutralBackground1Pressed, ColorNeutralForeground1, ColorNeutralForeground1Hover,
    ColorNeutralForeground1Pressed, ColorNeutralForeground3, ColorNeutralForegroundDisabled,
    ColorNeutralForegroundInverted, ColorNeutralStroke2, ColorNeutralStrokeAccessible, ColorStrokeFocus2,
    ColorTransparentStroke,
};
use crate::{
    Fluent, FluentExt,
    spec::{
        defaults::combobox as cb,
        tokens::{geometry as g, typography as ty},
    },
    theme::Elevation,
    widget::paint,
};

/// Fluent `Option`: a choice of a [`Dropdown`](super::Dropdown), [`Combobox`](super::Combobox),
/// [`Select`](super::Select) or [`TagPicker`](super::TagPicker); converts from its text. The widget pairs it with
/// a value (`item(value, option)`).
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct ListOption {
    text: String,
    disabled: bool,
    salt: Option<KeyedSalt>,
}

impl ListOption {
    /// `text` is shown and is the accessible name.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            disabled: false,
            salt: None,
        }
    }

    crate::widget::setters! {
        item_salt
    }

    /// Skipped by the keyboard and ignores clicks.
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

impl From<&str> for ListOption {
    fn from(text: &str) -> Self {
        Self::new(text)
    }
}

impl From<String> for ListOption {
    fn from(text: String) -> Self {
        Self::new(text)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Item<K> {
    pub(crate) value: K,
    pub(crate) text: String,
    pub(crate) disabled: bool,
    pub(crate) salt: Option<KeyedSalt>,
}

impl<K> Item<K> {
    pub(crate) fn new(value: K, option: impl Into<ListOption>) -> Self {
        let ListOption { text, disabled, salt } = option.into();
        Self {
            value,
            text,
            disabled,
            salt,
        }
    }
}

/// Fluent `OptionGroup`: options under a heading and a divider.
#[derive(Clone, Debug, PartialEq, Eq)]
#[must_use]
pub struct OptionGroup<K> {
    pub(crate) label: String,
    pub(crate) options: Vec<Item<K>>,
}

impl<K> OptionGroup<K> {
    /// `label` is the group heading.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            options: Vec::new(),
        }
    }

    /// Appends one option carrying `value`.
    pub fn item(mut self, value: K, option: impl Into<ListOption>) -> Self {
        self.options.push(Item::new(value, option));
        self
    }

    /// Appends `(value, option)` pairs.
    pub fn items(mut self, options: impl IntoIterator<Item = (K, impl Into<ListOption>)>) -> Self {
        self.options
            .extend(options.into_iter().map(|(value, option)| Item::new(value, option)));
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Entry<K> {
    Option(Item<K>),
    Group(OptionGroup<K>),
}

/// Option or [`OptionGroup`] of a list, for lists that mix both; converts from a `(value, option)` pair and from
/// an [`OptionGroup`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListEntry<K>(pub(crate) Entry<K>);

impl<K, O: Into<ListOption>> From<(K, O)> for ListEntry<K> {
    fn from((value, option): (K, O)) -> Self {
        Self(Entry::Option(Item::new(value, option)))
    }
}

impl<K> From<OptionGroup<K>> for ListEntry<K> {
    fn from(group: OptionGroup<K>) -> Self {
        Self(Entry::Group(group))
    }
}

pub(crate) trait Entries<K> {
    fn push_entry(&mut self, entry: Entry<K>);
}

impl<K> Entries<K> for Vec<Entry<K>> {
    fn push_entry(&mut self, entry: Entry<K>) {
        self.push(entry);
    }
}

// Unique while the widget is built, so `make_mut` does not clone.
impl<K: Clone> Entries<K> for std::sync::Arc<Vec<Entry<K>>> {
    fn push_entry(&mut self, entry: Entry<K>) {
        std::sync::Arc::make_mut(self).push(entry);
    }
}

// `item`, `items` and `group` of a widget whose `entries` implement `Entries`.
macro_rules! entry_setters {
    () => {
        /// Appends one option carrying `value`.
        pub fn item(mut self, value: K, option: impl Into<$crate::inputs::ListOption>) -> Self {
            $crate::inputs::listbox::Entries::push_entry(
                &mut self.entries,
                $crate::inputs::listbox::Entry::Option($crate::inputs::listbox::Item::new(value, option)),
            );
            self
        }

        /// Appends `(value, option)` pairs and [`OptionGroup`](crate::inputs::OptionGroup)s.
        pub fn items(mut self, entries: impl IntoIterator<Item = impl Into<$crate::inputs::ListEntry<K>>>) -> Self {
            for entry in entries {
                $crate::inputs::listbox::Entries::push_entry(&mut self.entries, entry.into().0);
            }
            self
        }

        /// Appends a group of options.
        pub fn group(mut self, group: $crate::inputs::OptionGroup<K>) -> Self {
            $crate::inputs::listbox::Entries::push_entry(
                &mut self.entries,
                $crate::inputs::listbox::Entry::Group(group),
            );
            self
        }
    };
}
pub(crate) use entry_setters;

#[derive(Clone, Copy, Debug)]
pub(crate) enum Row {
    Group(usize),
    Option(usize),
}

// Options in navigation order and the rows to draw; groups without a visible option are dropped.
// `visible` gets each option's position among all options of the entries.
pub(crate) struct Flat<'a, K> {
    pub options: Vec<&'a Item<K>>,
    // Position of each of `options` among all options of the entries.
    pub positions: Vec<usize>,
    pub rows: Vec<Row>,
    pub groups: Vec<&'a str>,
}

impl<'a, K> Flat<'a, K> {
    pub(crate) fn new(entries: &'a [Entry<K>], visible: &dyn Fn(usize, &Item<K>) -> bool) -> Self {
        let mut flat = Self {
            options: Vec::new(),
            positions: Vec::new(),
            rows: Vec::new(),
            groups: Vec::new(),
        };
        let mut pos = 0usize;
        for entry in entries {
            match entry {
                Entry::Option(o) => {
                    if visible(pos, o) {
                        flat.rows.push(Row::Option(flat.options.len()));
                        flat.options.push(o);
                        flat.positions.push(pos);
                    }
                    pos = pos.saturating_add(1);
                }
                Entry::Group(group) => {
                    let mut heading = false;
                    for o in &group.options {
                        if visible(pos, o) {
                            if !heading {
                                heading = true;
                                flat.rows.push(Row::Group(flat.groups.len()));
                                flat.groups.push(&group.label);
                            }
                            flat.rows.push(Row::Option(flat.options.len()));
                            flat.options.push(o);
                            flat.positions.push(pos);
                        }
                        pos = pos.saturating_add(1);
                    }
                }
            }
        }
        flat
    }

    pub(crate) fn len(&self) -> usize {
        self.options.len()
    }

    pub(crate) fn enabled(&self, i: usize) -> bool {
        self.options.get(i).is_some_and(|o| !o.disabled)
    }

    pub(crate) fn value(&self, i: usize) -> Option<&'a Item<K>> {
        self.options.get(i).copied()
    }
}

pub(crate) struct View<'a, K> {
    pub flat: &'a Flat<'a, K>,
    pub multiselect: bool,
    pub selected: &'a dyn Fn(&Item<K>) -> bool,
    pub active: Option<usize>,
    pub scroll_to_active: bool,
    pub max_height: f32,
}

pub(crate) struct Shown {
    pub clicked: Option<usize>,
    pub closed: bool,
    pub active_id: Option<Id>,
    pub layer: Option<LayerId>,
}

const GROUP_PAD_V: f32 = g::SPACING_HORIZONTAL_S;
const GROUP_PAD_H: f32 = g::SPACING_HORIZONTAL_S_NUDGE;
const OPTION_PAD_V: f32 = g::SPACING_HORIZONTAL_S_NUDGE;
const OPTION_PAD_H: f32 = g::SPACING_HORIZONTAL_S;
const LIST_PAD: f32 = g::SPACING_HORIZONTAL_XS;
const ROW_GAP: f32 = g::SPACING_HORIZONTAL_XXS;
const OPTION_LINE: f32 = ty::LINE_HEIGHT_BASE_300;
const GROUP_LINE: f32 = ty::LINE_HEIGHT_BASE_200;
const OPTION_H: f32 = OPTION_LINE + 2.0 * OPTION_PAD_V;
const GROUP_H: f32 = GROUP_LINE + 2.0 * GROUP_PAD_V;
// useOptionGroupStyles `::after`: `paddingBottom`, 1px border, `marginBottom` of `spacingVerticalXS`.
const DIVIDER_H: f32 = g::SPACING_HORIZONTAL_XS + g::STROKE_WIDTH_THIN + g::SPACING_HORIZONTAL_XS;

impl<K> View<'_, K> {
    fn content_height(&self) -> f32 {
        let f = self.flat;
        let mut h = 0.0;
        let mut gaps = 0.0;
        for (i, row) in f.rows.iter().enumerate() {
            h += match row {
                Row::Group(_) => GROUP_H,
                Row::Option(_) => OPTION_H,
            };
            if i > 0 {
                gaps += ROW_GAP;
            }
        }
        let dividers = f.groups.len().saturating_sub(1) as f32 * DIVIDER_H;
        h + gaps + dividers
    }
}

// react-combobox Listbox / OptionGroup / Option, shown as a popup under `anchor`.
pub(crate) fn show<K: egui_mineral::select::ItemKey>(
    ui: &MineralUi<'_, Fluent>,
    popup: KeyedSalt,
    trigger: &Response,
    anchor: Rect,
    key: &StableId,
    label: &str,
    view: &View<'_, K>,
) -> Shown {
    let rt = ui.rt();
    let ctx = rt.ctx();
    let t = ui.tokens();
    let dir = ui.i18n().direction();
    let width = anchor.width().max(cb::LISTBOX_MIN_WIDTH);
    let screen = ctx.content_rect();
    let frame_pad = 2.0 * (LIST_PAD + g::STROKE_WIDTH_THIN);
    let max_h = (screen.height() * cb::LISTBOX_MAX_HEIGHT)
        .min(view.max_height)
        .max(OPTION_H);
    let height = (view.content_height() + frame_pad).min(max_h);
    let icons = rt.icons();
    let mut clicked = None;
    let mut active_id = None;
    let mut open = true;
    let shown = Popup::new(popup, Anchor::new(Side::Bottom, Align::Start).gap(cb::POPUP_OFFSET))
        .size(vec2(width, height))
        .name(label)
        .anchor_rect(anchor)
        .show(rt, trigger, &mut open, |ui, _| {
            let radius = CornerRadius::from(g::BORDER_RADIUS_MEDIUM);
            let base = ui.unique_id().with("option");
            let panel = Frame::new()
                .fill(t.color(ColorNeutralBackground1))
                .stroke(Stroke::new(g::STROKE_WIDTH_THIN, t.color(ColorTransparentStroke)))
                .corner_radius(radius)
                .inner_margin(Margin::same(LIST_PAD as i8))
                .shadow(t.shadow(Elevation::E16, ShadowTint::Neutral).ambient)
                .show(ui, |ui| {
                    ui.set_width(width - frame_pad);
                    ui.spacing_mut().item_spacing = Vec2::new(0.0, ROW_GAP);
                    ScrollArea::vertical()
                        .max_height(height - frame_pad)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            ui.set_width(width - frame_pad);
                            let rows = Rows {
                                view,
                                base,
                                dir,
                                key,
                                t: &t,
                                icons: &icons,
                            };
                            (clicked, active_id) = rows.draw(ui);
                        });
                });
            semantic::emit(ctx, Some(panel.response.id), panel.response.rect, || {
                let mut n = SemanticNode::new(key.sub("listbox"), Role::ListBox, label);
                n.depth = 1;
                n
            });
        });
    Shown {
        clicked,
        closed: !open,
        active_id,
        layer: shown.response.map(|r| r.layer_id),
    }
}

struct Rows<'a, 'b, K> {
    view: &'a View<'a, K>,
    base: Id,
    dir: egui_mineral::i18n::Direction,
    key: &'a StableId,
    t: &'b crate::theme::Tokens,
    icons: &'b crate::IconSet,
}

impl<K: egui_mineral::select::ItemKey> Rows<'_, '_, K> {
    fn draw(&self, ui: &mut egui::Ui) -> (Option<usize>, Option<Id>) {
        let (view, t, dir, icons) = (self.view, self.t, self.dir, self.icons);
        let ctx = ui.ctx().clone();
        let inner_w = ui.available_width();
        let mut clicked = None;
        let mut active_id = None;
        let mut groups_left = view.flat.groups.len();
        let mut pending_divider = false;
        for row in &view.flat.rows {
            match *row {
                Row::Group(gi) => {
                    if pending_divider {
                        divider(ui, inner_w, t);
                    }
                    groups_left = groups_left.saturating_sub(1);
                    pending_divider = groups_left > 0;
                    if let Some(text) = view.flat.groups.get(gi) {
                        group_heading(ui, text, inner_w, dir, t);
                    }
                }
                Row::Option(i) => {
                    let Some(opt) = view.flat.value(i) else { continue };
                    let selected = (view.selected)(opt);
                    let active = view.active == Some(i);
                    let look = OptionLook {
                        selected,
                        multiselect: view.multiselect,
                        active,
                    };
                    let part = opt.salt.as_ref().map(KeyedSalt::id_salt);
                    let id = part.map_or_else(|| self.base.with(i), |salt| self.base.with(salt));
                    let r = option_row(ui, id, opt, look, inner_w, dir, (t, icons));
                    if active && view.scroll_to_active {
                        ui.scroll_to_rect(r.rect, None);
                    }
                    semantic::emit(&ctx, Some(r.id), r.rect, || {
                        let mut n = SemanticNode::new(option_key(self.key, opt), Role::ListBoxOption, opt.text.clone())
                            .pressable(Availability::disabled_if(opt.disabled));
                        n.state.selected = Some(selected);
                        n.depth = 2;
                        n
                    });
                    if r.clicked() && !opt.disabled {
                        clicked = Some(i);
                    }
                    if active {
                        active_id = Some(r.id);
                    }
                }
            }
        }
        (clicked, active_id)
    }
}

fn option_key<K: egui_mineral::select::ItemKey>(key: &StableId, opt: &Item<K>) -> StableId {
    match &opt.salt {
        Some(salt) => key.sub((salt, "option")),
        None => key.sub((&opt.value, "option")),
    }
}

fn divider(ui: &mut egui::Ui, width: f32, t: &crate::theme::Tokens) {
    let (rect, _) = ui.allocate_exact_size(vec2(width, DIVIDER_H), Sense::hover());
    let y = rect.top() + g::SPACING_HORIZONTAL_XS + g::STROKE_WIDTH_THIN / 2.0;
    let bleed = LIST_PAD;
    ui.painter().hline(
        rect.left() - bleed..=rect.right() + bleed,
        y,
        Stroke::new(g::STROKE_WIDTH_THIN, t.color(ColorNeutralStroke2)),
    );
}

fn group_heading(
    ui: &mut egui::Ui,
    text: &str,
    width: f32,
    dir: egui_mineral::i18n::Direction,
    t: &crate::theme::Tokens,
) {
    let (rect, _) = ui.allocate_exact_size(vec2(width, GROUP_H), Sense::hover());
    let format = crate::typography::format(
        ui.ctx(),
        crate::typography::TypeSpec {
            size: ty::FONT_SIZE_BASE_200,
            line_height: ty::LINE_HEIGHT_BASE_200,
            weight: ty::FONT_WEIGHT_SEMIBOLD,
        },
        t.color(ColorNeutralForeground3),
    );
    let galley = ui.painter().layout(
        text.to_owned(),
        format.font_id.clone(),
        format.color,
        (width - 2.0 * GROUP_PAD_H).max(0.0),
    );
    let x = dir.start_x(rect, Rangef::new(GROUP_PAD_H, GROUP_PAD_H + galley.size().x));
    ui.painter()
        .galley(egui::pos2(x, rect.top() + GROUP_PAD_V), galley, format.color);
}

#[derive(Clone, Copy)]
struct OptionLook {
    selected: bool,
    multiselect: bool,
    active: bool,
}

// react-combobox useOptionStyles.styles.ts
fn option_row<K>(
    ui: &mut egui::Ui,
    id: Id,
    opt: &Item<K>,
    look: OptionLook,
    width: f32,
    dir: egui_mineral::i18n::Direction,
    (t, icons): (&crate::theme::Tokens, &crate::IconSet),
) -> Response {
    let sense = if opt.disabled {
        Sense::hover()
    } else {
        Sense::click() - Sense::FOCUSABLE
    };
    let (rect, _) = ui.allocate_exact_size(vec2(width, OPTION_H), Sense::hover());
    let r = ui.interact(rect, id, sense);
    let radius = CornerRadius::from(g::BORDER_RADIUS_MEDIUM);
    let pressed = r.is_pointer_button_down_on() && !opt.disabled;
    let hovered = r.hovered() && !opt.disabled;
    let (fill, fore) = if opt.disabled {
        (None, ColorNeutralForegroundDisabled)
    } else if pressed {
        (Some(ColorNeutralBackground1Pressed), ColorNeutralForeground1Pressed)
    } else if hovered {
        (Some(ColorNeutralBackground1Hover), ColorNeutralForeground1Hover)
    } else {
        (None, ColorNeutralForeground1)
    };
    let p = ui.painter();
    if let Some(fill) = fill {
        p.rect_filled(rect, radius, t.color(fill));
    }
    let check_x = OPTION_PAD_H + cb::OPTION_CHECK_MARGIN_START;
    let check_box = cb::OPTION_CHECK_BOX;
    let check = Rect::from_center_size(
        egui::pos2(
            dir.start_x(rect, Rangef::new(check_x, check_x + check_box)) + check_box / 2.0,
            rect.center().y,
        ),
        vec2(check_box, check_box),
    );
    paint_check(p, icons, check, opt.disabled, look, fore, t);
    let text_off = check_x + check_box + g::SPACING_HORIZONTAL_XXS + g::SPACING_HORIZONTAL_XS;
    let format = crate::typography::format(
        ui.ctx(),
        crate::typography::TypeSpec {
            size: ty::FONT_SIZE_BASE_300,
            line_height: OPTION_LINE,
            weight: ty::FONT_WEIGHT_REGULAR,
        },
        t.color(fore),
    );
    let max_w = (width - text_off - OPTION_PAD_H).max(1.0);
    let mut job = egui::text::LayoutJob::single_section(opt.text.clone(), format.clone());
    job.wrap = egui::text::TextWrapping::truncate_at_width(max_w);
    let galley = ui.painter().layout_job(job);
    let x = dir.start_x(rect, Rangef::new(text_off, text_off + galley.size().x));
    ui.painter().galley(
        egui::pos2(x, rect.center().y - galley.size().y / 2.0),
        galley,
        format.color,
    );
    if look.active {
        ui.painter().rect_stroke(
            rect,
            radius,
            Stroke::new(cb::OPTION_FOCUS_RING, t.color(ColorStrokeFocus2)),
            StrokeKind::Outside,
        );
    }
    r
}

fn paint_check(
    p: &egui::Painter,
    icons: &crate::IconSet,
    check: Rect,
    disabled: bool,
    look: OptionLook,
    fore: crate::spec::ColorToken,
    t: &crate::theme::Tokens,
) {
    if look.multiselect {
        let (bg, border, mark) = if look.selected {
            let c = if disabled {
                ColorNeutralForegroundDisabled
            } else {
                ColorCompoundBrandBackground
            };
            (Some(c), c, ColorNeutralForegroundInverted)
        } else if disabled {
            (None, ColorNeutralForegroundDisabled, ColorNeutralForegroundDisabled)
        } else {
            (None, ColorNeutralStrokeAccessible, fore)
        };
        if let Some(bg) = bg {
            p.rect_filled(check, CornerRadius::from(g::BORDER_RADIUS_SMALL), t.color(bg));
        }
        p.rect_stroke(
            check,
            CornerRadius::from(g::BORDER_RADIUS_SMALL),
            Stroke::new(g::STROKE_WIDTH_THIN, t.color(border)),
            StrokeKind::Inside,
        );
        if look.selected {
            paint::icon(
                p,
                check.center(),
                icons.checkmark,
                false,
                cb::OPTION_CHECK_MARK,
                t.color(mark),
            );
        }
    } else if look.selected {
        paint::icon(
            p,
            check.center(),
            icons.checkmark,
            false,
            cb::OPTION_CHECK_ICON,
            t.color(fore),
        );
    }
}
