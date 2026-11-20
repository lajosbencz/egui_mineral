use crate::Material;
use crate::MaterialExt;
use crate::MaterialUiExt as _;
use crate::Rt;
use crate::adaptive::Density;
use crate::text_inputs::text_field::FieldChrome;
use crate::widget::ValueBinding;
use egui::{
    Color32, Frame, Id, Key, LayerId, Rangef, Rect, Response, UiBuilder, Vec2, pos2,
    text::{LayoutJob, TextWrapping},
    vec2,
};
use egui_mineral::i18n::I18n;
use egui_mineral::semantic::AgentKey;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::Availability;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    anchor::{Align, Anchor, Side},
    combobox::{ComboAction, ComboKind, ComboList, ComboState},
    focus::{self, Typeahead},
    kit,
    layout::{Constraints, Measure, MinTarget, Zones},
    overlay::Popup,
    perf::VirtualList,
    select::Selection,
    semantic::{Role, SemanticNode, StableId},
    widget::{MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::selection::{Chip, InputChip};
use crate::spec::{
    TypeScale,
    defaults::{menu as menu_defaults, text_field as tf},
    tokens::menu,
};
use crate::text_inputs::text_field::{self, Decor, INPUT_FONT, TextFieldVariant, content_inset, field_h, line_galley};
use crate::widget::{
    common::{self},
    leading::Leading,
    parts,
    row_parts::{OptionRow, RowMetrics},
};

pub(crate) const CHIP_GAP: f32 = 8.0;

pub(crate) fn chip_size(ui: &MineralUi<'_, Material>, label: &str) -> Vec2 {
    Chip::input("measure", label)
        .layout(ui, Constraints::loose(Vec2::INFINITY), &())
        .visual
}

pub(crate) fn chip_flow(widths: &[f32], avail: f32, tail: f32) -> (Vec<(usize, f32)>, usize) {
    let mut out = Vec::with_capacity(widths.len());
    let (mut row, mut x) = (0, 0.0);
    for w in widths {
        if x > 0.0 && x + w > avail {
            row += 1;
            x = 0.0;
        }
        out.push((row, x));
        x += w + CHIP_GAP;
    }
    if tail > 0.0 && x > 0.0 && x + tail > avail {
        row += 1;
    }
    (out, row + 1)
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ChipRows {
    /// First row top, from the container top.
    pub top: f32,
    pub chip_h: f32,
    /// Container growth of the first row.
    pub grow: f32,
}

impl ChipRows {
    pub(crate) fn new(ui: &MineralUi<'_, Material>, variant: TextFieldVariant, labeled: bool, chips: bool) -> Self {
        let chip_h = chip_size(ui, "").y;
        let text_top = text_field::text_top(ui.density(), variant, labeled);
        let spill = chip_h - text_field::line_h();
        let (top, grow) = if chips {
            (text_top, spill.max(0.0))
        } else {
            (text_top - spill / 2.0, 0.0)
        };
        Self { top, chip_h, grow }
    }

    pub(crate) fn row_top(self, row: usize) -> f32 {
        self.top + row as f32 * (self.chip_h + CHIP_GAP)
    }

    pub(crate) fn field_height(self, density: Density, variant: TextFieldVariant, labeled: bool, rows: usize) -> f32 {
        field_h(density, variant, labeled) + self.grow + rows.saturating_sub(1) as f32 * (self.chip_h + CHIP_GAP)
    }
}

pub(crate) fn place_chip(
    ui: &mut MineralUi<'_, Material>,
    salt: impl egui::AsIdSalt,
    rect: Rect,
    chip: InputChip,
) -> bool {
    let rt = ui.rt();
    let chip = chip.target(MinTarget::Auto);
    let zones = Zones::new(rect.size().into(), ui.target(chip.min_target(ui)));
    let outer = zones.translate(rect.min - zones.visual.min).outer;
    rt.ui(&mut ui.new_child(UiBuilder::new().id_salt(salt).max_rect(outer)))
        .mineral(chip)
        .into_output()
        .removed
}

// Compose ExposedDropdownMenu
pub(crate) struct Listbox<'a> {
    pub popup: KeyedSalt,
    pub trigger: &'a Response,
    pub anchor: Rect,
    pub key: &'a StableId,
    pub label: &'a str,
    pub len: usize,
    /// Built for visible rows only.
    pub row: &'a dyn Fn(usize) -> OptionRow<'a>,
    pub active: Option<usize>,
    pub scroll_to_active: bool,
}

pub(crate) struct ListboxOut {
    pub clicked: Option<usize>,
    /// Dismissed this pass.
    pub closed: bool,
    pub active_id: Option<Id>,
    /// Popup layer; `None` when not shown.
    pub layer: Option<LayerId>,
}

pub(crate) fn focus_within(ctx: &egui::Context, layer: Option<LayerId>) -> bool {
    layer.is_some_and(|l| {
        ctx.memory(egui::Memory::focused)
            .and_then(|f| ctx.read_response(f))
            .is_some_and(|r| r.layer_id == l)
    })
}

impl Listbox<'_> {
    pub(crate) fn show(self, ui: &MineralUi<'_, Material>) -> ListboxOut {
        let Self {
            popup,
            trigger,
            anchor,
            key,
            label,
            len,
            row: row_at,
            active,
            scroll_to_active,
        } = self;
        let rt = ui.rt();
        let ctx = rt.ctx();
        let t = rt.tokens();
        let mut clicked = None;
        let mut active_id = None;
        let width = anchor.width();
        let screen = ctx.content_rect();
        // `calculateMaxHeight`: the larger space above or below, less the margin.
        let room =
            (anchor.top() - screen.top()).max(screen.bottom() - anchor.bottom()) - menu_defaults::VERTICAL_MARGIN;
        let pad = menu_defaults::VERTICAL_PADDING;
        let m = RowMetrics::MENU;
        let list_h = (len as f32 * m.height).min((room - 2.0 * pad).max(m.height));
        let mut open = true;
        let rows_salt = popup.child("rows");
        let shown = Popup::new(
            popup,
            Anchor::new(Side::Bottom, Align::Start).padding(menu_defaults::VERTICAL_MARGIN),
        )
        .size(vec2(width, list_h + 2.0 * pad))
        .name(label)
        .anchor_rect(anchor)
        .show(rt, trigger, &mut open, |ui, _| {
            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
            let r = menu::CONTAINER_SHAPE.corner_radius(&t.shape, rt.i18n().direction());
            let base = ui.unique_id().with("option");
            let panel = common::elevated_frame(
                ui,
                menu::CONTAINER_ELEVATION,
                &t,
                Frame::new()
                    .fill(t.role(menu::CONTAINER_COLOR))
                    .corner_radius(r)
                    .inner_margin(vec2(0.0, pad)),
                |ui| {
                    ui.set_width(width);
                    ui.set_max_height(list_h);
                    ui.set_min_height(list_h);
                    let list = VirtualList::uniform(rows_salt, m.height);
                    let list = match active.filter(|_| scroll_to_active) {
                        Some(i) => list.scroll_to(i),
                        None => list,
                    };
                    list.show(ui, len, |ui, i| {
                        let row = OptionRow {
                            active: active == Some(i),
                            ..row_at(i)
                        };
                        let resp = row.show(&mut rt.ui(ui), base.with(i), width, &m, &t);
                        text_field::emit_part(ctx, &resp, 2, || row.value_node(key.sub((i, "option"))));
                        if resp.clicked() {
                            clicked = Some(i);
                        }
                        if row.active {
                            active_id = Some(resp.id);
                        }
                    });
                },
            );
            text_field::emit_part(ctx, &panel.response, 1, || {
                SemanticNode::new(key.sub("listbox"), Role::ListBox, label)
            });
        });
        ListboxOut {
            clicked,
            closed: !open,
            active_id,
            layer: shown.response.map(|r| r.layer_id),
        }
    }
}

pub(crate) fn highlighted(
    ctx: &egui::Context,
    text: &str,
    hits: &[usize],
    scale: TypeScale,
    normal: Color32,
    hit: Color32,
    max_w: f32,
) -> LayoutJob {
    let mut job = LayoutJob {
        wrap: TextWrapping::truncate_at_width(max_w.max(1.0)),
        ..LayoutJob::default()
    };
    let mut run = String::new();
    let mut run_hit = false;
    let mut flush = |run: &mut String, is_hit: bool| {
        if !run.is_empty() {
            job.append(run, 0.0, scale.format(ctx, if is_hit { hit } else { normal }));
            run.clear();
        }
    };
    for (i, ch) in text.chars().enumerate() {
        let h = hits.contains(&i);
        if h != run_hit {
            flush(&mut run, run_hit);
            run_hit = h;
        }
        run.push(ch);
    }
    flush(&mut run, run_hit);
    job
}

enum Value<'a> {
    Single(&'a mut Option<usize>),
    Bound(ValueBinding<'a>),
    Multi(&'a mut Selection<usize>),
}

/// Option of a [`Select`]; converts from its label.
#[derive(Clone, Debug)]
#[must_use]
pub struct SelectOption {
    label: String,
    leading: Option<Leading>,
    disabled: bool,
}

impl SelectOption {
    /// `label` is the visible text and accessible name.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            leading: None,
            disabled: false,
        }
    }

    crate::widget::setters! {
        /// Not selectable or deselectable from the UI and skipped by keyboard navigation. A bound
        /// selection may still hold it (availability can change at runtime); it shows as selected.
        disabled
    }

    /// Leading icon or app paint.
    pub fn leading(mut self, leading: impl Into<Leading>) -> Self {
        self.leading = Some(leading.into());
        self
    }
}

impl From<&str> for SelectOption {
    fn from(label: &str) -> Self {
        Self::new(label)
    }
}

impl From<String> for SelectOption {
    fn from(label: String) -> Self {
        Self::new(label)
    }
}

/// Option of a multi-select [`Select`]; converts from its label. Check boxes hold the leading
/// slot, so unlike [`SelectOption`] it has none.
#[derive(Clone, Debug)]
#[must_use]
pub struct MultiOption {
    label: String,
    disabled: bool,
}

impl MultiOption {
    /// `label` is the visible text and accessible name.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            disabled: false,
        }
    }

    crate::widget::setters! {
        /// As [`SelectOption::disabled`].
        disabled
    }
}

impl From<&str> for MultiOption {
    fn from(label: &str) -> Self {
        Self::new(label)
    }
}

impl From<String> for MultiOption {
    fn from(label: String) -> Self {
        Self::new(label)
    }
}

impl From<MultiOption> for SelectOption {
    fn from(option: MultiOption) -> Self {
        Self {
            label: option.label,
            leading: None,
            disabled: option.disabled,
        }
    }
}

#[doc(hidden)]
#[derive(Clone, Debug, Default)]
pub struct SelectState {
    combo: ComboState,
    typeahead: Typeahead,
    active_id: Option<Id>,
    popup: Option<LayerId>,
}

/// MD3 exposed dropdown menu (Compose `ExposedDropdownMenuBox`), select-only or multi.
#[must_use = "show with ui.mineral(..)"]
pub struct Select<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    options: Vec<SelectOption>,
    value: Value<'a>,
    disabled: bool,
    label_visible: bool,
    chrome: FieldChrome,
}

egui_mineral::kit::opaque_debug!(Select<'_>);

impl<'a> Select<'a> {
    /// Single select; `selected` is the bound option index, `None` before a pick. Agent key `<id_salt>-select`.
    pub fn new(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        options: impl IntoIterator<Item = impl Into<SelectOption>>,
        selected: &'a mut Option<usize>,
    ) -> Self {
        Self::with(id_salt.into(), label, options, Value::Single(selected))
    }

    /// Single select over options that carry values, like [`RadioGroup`](crate::selection::RadioGroup): shows
    /// the option whose value equals `value` (none when no option matches) and a pick writes that option's
    /// value. Bind an `Option<T>` with `Some(..)` option values to start empty.
    pub fn bound<T: PartialEq + Clone + 'a>(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        options: impl IntoIterator<Item = (T, impl Into<SelectOption>)>,
        value: &'a mut T,
    ) -> Self {
        let (values, options): (Vec<T>, Vec<SelectOption>) = options.into_iter().map(|(v, o)| (v, o.into())).unzip();
        Self::with(
            id_salt.into(),
            label,
            options,
            Value::Bound(ValueBinding::new(values, value)),
        )
    }

    /// Multi-select; `selection` holds the chosen option indices.
    pub fn multi(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        options: impl IntoIterator<Item = impl Into<MultiOption>>,
        selection: &'a mut Selection<usize>,
    ) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self::with(
            id_salt,
            label,
            options.into_iter().map(|o| SelectOption::from(o.into())),
            Value::Multi(selection),
        )
    }

    fn with(
        id_salt: KeyedSalt,
        label: impl Into<String>,
        options: impl IntoIterator<Item = impl Into<SelectOption>>,
        value: Value<'a>,
    ) -> Self {
        Self {
            chrome: FieldChrome::default(),
            key: egui_mineral::agent_key!("select"),
            salt: id_salt,
            label: label.into(),
            options: options.into_iter().map(Into::into).collect(),
            value,
            disabled: false,
            label_visible: true,
        }
    }

    crate::text_inputs::text_field::field_chrome_setters!();

    crate::widget::setters! {
        /// Greys out the field; the menu cannot open.
        disabled
    }

    /// `false` keeps the label for assistive technology only.
    pub fn label_visible(mut self, label_visible: bool) -> Self {
        self.label_visible = label_visible;
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-select` agent key.
        agent_key(key)
    }

    fn option_enabled(&self, i: usize) -> bool {
        self.options.get(i).is_some_and(|o| !o.disabled)
    }

    fn is_selected(&self, i: usize) -> bool {
        match &self.value {
            Value::Single(s) => **s == Some(i),
            Value::Bound(b) => b.index() == Some(i),
            Value::Multi(s) => s.is_selected(&i),
        }
    }

    fn first_selected(&self) -> Option<usize> {
        match &self.value {
            Value::Single(s) => **s,
            Value::Bound(b) => b.index(),
            Value::Multi(s) => s.first().copied(),
        }
    }

    fn chips(&self) -> Vec<usize> {
        match &self.value {
            Value::Single(_) | Value::Bound(_) => Vec::new(),
            Value::Multi(s) => s.selected().copied().filter(|i| *i < self.options.len()).collect(),
        }
    }

    fn label_of(&self, i: usize) -> &str {
        &self.options[i].label
    }

    fn content_width(&self, density: Density, w: f32) -> f32 {
        w - content_inset(density, self.chrome.leading.is_some()) - content_inset(density, true)
    }

    fn chip_widths(&self, ui: &MineralUi<'_, Material>, chips: &[usize]) -> Vec<f32> {
        chips.iter().map(|i| chip_size(ui, self.label_of(*i)).x).collect()
    }

    fn field_height(&self, ui: &MineralUi<'_, Material>, w: f32) -> f32 {
        let chips = self.chips();
        if chips.is_empty() {
            return field_h(ui.density(), self.chrome.variant, self.shows_label());
        }
        let rows = chip_flow(&self.chip_widths(ui, &chips), self.content_width(ui.density(), w), 0.0).1;
        ChipRows::new(ui, self.chrome.variant, self.shows_label(), true).field_height(
            ui.density(),
            self.chrome.variant,
            self.shows_label(),
            rows,
        )
    }

    fn shows_label(&self) -> bool {
        self.label_visible && !self.label.is_empty()
    }

    fn commit(&mut self, i: usize) -> bool {
        match &mut self.value {
            Value::Single(s) => {
                let changed = **s != Some(i);
                **s = Some(i);
                changed
            }
            Value::Bound(b) => b.pick(i),
            Value::Multi(s) => s.toggle(i),
        }
    }

    fn summary(&self) -> String {
        match &self.value {
            Value::Single(_) | Value::Bound(_) => self
                .first_selected()
                .and_then(|i| self.options.get(i))
                .map_or_default(|o| o.label.clone()),
            Value::Multi(s) => s
                .selected()
                .filter_map(|i| self.options.get(*i).map(|o| o.label.as_str()))
                .collect::<Vec<_>>()
                .join(", "),
        }
    }
}

impl Select<'_> {
    fn drive_combo(&mut self, rt: &Rt, r: &egui::Response, state: &mut SelectState, pressed: bool) -> (bool, bool) {
        let ctx = rt.ctx();
        let len = self.options.len();
        let multi = matches!(self.value, Value::Multi(_));
        let mut changed = false;
        let mut moved = false;
        if self.disabled {
            return (changed, moved);
        }
        let enabled = |i: usize| self.option_enabled(i);
        let list = ComboList {
            kind: ComboKind::SelectOnly,
            selected: self.first_selected(),
            len,
            enabled: &enabled,
        };
        // egui turns Enter/Space on the focused field into a click; the combo keys own them.
        let key_click = r.has_focus() && ctx.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::Space));
        if pressed && !key_click {
            if state.combo.is_open() {
                state.combo.close();
            } else {
                state.combo.open_selected(list);
            }
            r.request_focus();
        }
        if r.has_focus() {
            // Up/Down drive the list, not egui's directional focus navigation.
            kit::focus::lock_arrows(
                ctx,
                r.id,
                kit::focus::Arrows::Axis(egui_mineral::layout::Axis::Vertical),
            );
            for a in state.combo.update(ctx, list) {
                match a {
                    ComboAction::Commit(i) => {
                        changed |= self.commit(i);
                        if multi {
                            state.combo.open(Some(i), len);
                        }
                    }
                    ComboAction::Moved(_) | ComboAction::Opened => moved = true,
                    _ => {}
                }
            }
            // Disabled options read as empty labels, which typeahead never matches.
            let labels = self
                .options
                .iter()
                .map(|o| if o.disabled { "" } else { o.label.as_str() });
            if let Some(i) = state.typeahead.update(rt.core(), state.combo.active(), labels) {
                if !state.combo.is_open() {
                    state.combo.open(Some(i), len);
                }
                state.combo.set_active(Some(i), len);
                moved = true;
            }
        } else if !state.combo.is_open() {
            state.combo.set_active(None, len);
        } else if ctx.memory(|m| m.focused().is_some()) && !focus_within(ctx, state.popup) {
            // Focus moved to another widget (Tab, programmatic).
            state.combo.close();
        }
        (changed, moved)
    }

    fn paint_value(&mut self, ui: &mut MineralUi<'_, Material>, frame: &WidgetFrame<'_>, state: &SelectState) -> bool {
        let rt = ui.rt();
        let t = rt.tokens();
        let r = &frame.response;
        let base = ui.make_persistent_id(self.salt.id_salt());
        let field = frame.zones.visual;
        let multi = matches!(self.value, Value::Multi(_));
        let mut changed = false;
        let chips = self.chips();
        let summary = (!multi).then(|| self.summary());
        let focused = focus::visible(&r.ctx, r.id) || state.combo.is_open();
        let labeled = self.shows_label();
        let decor = Decor {
            id: base,
            variant: self.chrome.variant,
            label: if labeled { &self.label } else { "" },
            focused,
            floated: focused || summary.as_ref().is_some_and(|s| !s.is_empty()) || !chips.is_empty(),
            error: self.chrome.error.is_some(),
            disabled: self.disabled,
            leading: self.chrome.leading,
            trailing: true,
        }
        .paint(ui, field, &t);
        let colors = decor.colors;
        let dir = rt.i18n().direction();
        // `ExposedDropdownMenuDefaults.TrailingIcon`: the drop-down arrow turned over while expanded.
        let slot = text_field::icon_slot(ui.density()) / 2.0;
        let ax = dir.reversed().start_x(field, Rangef::point(slot));
        let turn = if state.combo.is_open() {
            std::f32::consts::PI
        } else {
            0.0
        };
        parts::icon_rotated(
            ui.painter(),
            pos2(ax, field.center().y),
            ui.icons().arrow_drop_down,
            text_field::trailing_icon_size(self.chrome.variant),
            colors.trailing,
            turn,
        );

        if let Some(s) = summary {
            let g = line_galley(ui.painter(), &s, INPUT_FONT, colors.input, decor.span.span());
            let x = dir.span_x(decor.span, Rangef::new(0.0, 0.0 + g.size().x));
            ui.painter().galley(pos2(x, decor.text_top), g, colors.input);
        } else if !chips.is_empty() {
            let widths = self.chip_widths(ui, &chips);
            let (pos, _) = chip_flow(&widths, decor.span.span(), 0.0);
            let rows = ChipRows::new(ui, self.chrome.variant, labeled, true);
            let mut removed = None;
            for (n, (&i, (row, x))) in chips.iter().zip(pos).enumerate() {
                let x0 = dir.span_x(decor.span, Rangef::new(x, x + widths[n]));
                let crect =
                    Rect::from_min_size(pos2(x0, field.top() + rows.row_top(row)), vec2(widths[n], rows.chip_h));
                let chip = Chip::input(self.salt.child(("chip", i)), self.label_of(i))
                    .disabled(self.disabled || !self.option_enabled(i))
                    .agent_key(self.key.id(&self.salt).sub((i, "chip")));
                if place_chip(ui, ("chip", i), crect, chip) {
                    removed = Some(i);
                }
            }
            if let Some(i) = removed {
                changed |= self.commit(i);
            }
        }
        if self.chrome.error.is_some() || self.chrome.supporting.is_some() {
            let s = self.chrome.error.as_deref().or(self.chrome.supporting.as_deref());
            text_field::paint_support(ui, field, s, None, colors.supporting, text_field::Elide::End);
        }
        changed
    }

    fn show_popup(
        &mut self,
        ui: &mut MineralUi<'_, Material>,
        frame: &WidgetFrame<'_>,
        state: &mut SelectState,
        moved: bool,
        was_open: bool,
    ) -> bool {
        let rt = ui.rt();
        if !state.combo.is_open() || self.disabled {
            state.popup = None;
            return false;
        }
        let r = &frame.response;
        let len = self.options.len();
        let multi = matches!(self.value, Value::Multi(_));
        let mut changed = false;
        let this = &*self;
        let row = |i: usize| {
            this.options.get(i).map_or_default(|o| OptionRow {
                label: &o.label,
                selected: this.is_selected(i),
                disabled: o.disabled,
                check: multi.then(|| this.is_selected(i)),
                leading: o.leading.clone(),
                ..OptionRow::default()
            })
        };
        let key = self.key.id(&self.salt);
        let ListboxOut {
            clicked,
            closed: close,
            active_id,
            layer,
        } = Listbox {
            popup: self.salt.child("popup"),
            trigger: r,
            anchor: frame.zones.outer,
            key: &key,
            label: &self.label,
            len,
            row: &row,
            active: state.combo.active(),
            scroll_to_active: moved || !was_open,
        }
        .show(&rt.ui(ui));
        state.active_id = active_id;
        state.popup = layer;
        if let Some(i) = clicked {
            changed |= self.commit(i);
            if multi {
                state.combo.set_active(Some(i), len);
            } else {
                state.combo.close();
            }
            r.request_focus();
        } else if close {
            state.combo.close();
        }
        changed
    }
}

impl MineralWidget for Select<'_> {
    type Kit = Material;
    type State = SelectState;
    type Output = ();

    egui_mineral::kit::widget_meta!(salt, enabled, no_target);

    fn layout(&self, ui: &MineralUi<'_, Material>, c: Constraints, _: &Self::State) -> Measure {
        let w = c.constrain(vec2(tf::MIN_WIDTH, 0.0)).x;
        let support = self.chrome.supporting.is_some() || self.chrome.error.is_some();
        text_field::measure(
            c.constrain(vec2(w, self.field_height(ui, w))),
            self.chrome.variant,
            self.shows_label(),
            support,
        )
    }

    fn update(
        &mut self,
        ui: &mut MineralUi<'_, Material>,
        frame: &WidgetFrame<'_>,
        state: &mut SelectState,
    ) -> WidgetResponse {
        let r = frame.response.clone();
        let was_open = state.combo.is_open();
        let pressed = parts::press(ui, frame, Availability::disabled_if(self.disabled)).clicked;
        let (mut changed, moved) = self.drive_combo(ui.rt(), &r, state, pressed);
        changed |= self.paint_value(ui, frame, state);
        changed |= self.show_popup(ui, frame, state, moved, was_open);
        self.chrome.show_tooltip(ui.rt(), &r);
        WidgetResponse::new(r, ()).with_changed(changed)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse, state: &SelectState) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(self.key.id(&self.salt), Role::ComboBox, &self.label)
            .value(self.summary())
            .clickable(Availability::disabled_if(self.disabled));
        n.state.invalid = self.chrome.error.is_some();
        n.state.expanded = Some(state.combo.is_open());
        n.active_descendant = state.active_id.filter(|_| state.combo.is_open());
        Some(n)
    }
}
