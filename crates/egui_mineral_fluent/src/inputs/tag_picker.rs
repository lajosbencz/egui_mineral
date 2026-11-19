use std::{collections::HashSet, sync::Arc};

use egui::{CornerRadius, Key, Modifiers, Pos2, Rangef, Rect, Sense, Stroke, StrokeKind, TextFormat, vec2};
use egui_mineral::{
    combobox::{ComboItem, ComboKind, ComboList, combo_items},
    i18n::I18n,
    kit,
    layout::{Constraints, Measure, MinTarget},
    select::Selection,
    semantic::{self, AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{Availability, Interaction, MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use super::combobox::{Click, toggle};
use super::field::Validation;
use super::listbox::{self, Flat, ListEntry, ListOption, View};
use super::textbox::{Chrome, Edit, InputAppearance, Live, paint_frame};
use super::value::{self, ListState, Value};
use crate::ColorToken::{
    ColorCompoundBrandForeground1Hover, ColorCompoundBrandForeground1Pressed, ColorNeutralBackground3,
    ColorNeutralBackgroundDisabled, ColorNeutralForeground2, ColorNeutralForegroundDisabled,
    ColorNeutralStrokeAccessible, ColorTransparentStroke, ColorTransparentStrokeDisabled,
};
use crate::{
    Fluent, FluentExt, Size,
    spec::{
        defaults::tag_picker as tp,
        tokens::{geometry as g, typography as ty},
    },
    widget::paint,
};

/// Control of a [`TagPicker`] next to its tags.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TagPickerControl {
    /// `TagPickerInput`: a text input filtering the list.
    #[default]
    Input,
    /// `TagPickerButton`: opens the list and cannot be typed into.
    Button,
}

/// `size` of a [`TagPicker`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TagPickerSize {
    /// `medium`, 32pt tall.
    #[default]
    Medium,
    /// `large`, 40pt tall.
    Large,
    /// `extra-large`, 44pt tall.
    ExtraLarge,
}

impl TagPickerSize {
    /// Every variant, smallest first.
    pub const ALL: &'static [Self] = &[Self::Medium, Self::Large, Self::ExtraLarge];

    const fn min_height(self) -> f32 {
        match self {
            Self::Medium => tp::MIN_HEIGHT_MEDIUM,
            Self::Large => tp::MIN_HEIGHT_LARGE,
            Self::ExtraLarge => tp::MIN_HEIGHT_EXTRA_LARGE,
        }
    }

    const fn icon(self) -> f32 {
        match self {
            Self::Medium => tp::ICON_MEDIUM,
            Self::Large => tp::ICON_LARGE,
            Self::ExtraLarge => tp::ICON_EXTRA_LARGE,
        }
    }

    const fn icon_margin(self) -> f32 {
        match self {
            Self::Medium | Self::Large => g::SPACING_HORIZONTAL_XXS,
            Self::ExtraLarge => g::SPACING_HORIZONTAL_S_NUDGE,
        }
    }

    // useTagPickerGroupStyles padding
    const fn group_pad(self) -> f32 {
        match self {
            Self::Medium => g::SPACING_VERTICAL_S_NUDGE,
            Self::Large | Self::ExtraLarge => g::SPACING_VERTICAL_S,
        }
    }

    // useTagPickerGroupStyles gap
    const fn gap(self) -> f32 {
        match self {
            Self::Medium => g::SPACING_HORIZONTAL_XS,
            Self::Large | Self::ExtraLarge => g::SPACING_HORIZONTAL_S_NUDGE,
        }
    }

    // Sized so group padding plus tag height reaches the control's minHeight.
    const fn tag(self) -> TagSize {
        match self {
            Self::Medium => TagSize::ExtraSmall,
            Self::Large => TagSize::Small,
            Self::ExtraLarge => TagSize::Medium,
        }
    }

    // useTagPickerButtonStyles size variants
    const fn button(self) -> (f32, f32) {
        match self {
            Self::Medium => (ty::FONT_SIZE_BASE_200, ty::LINE_HEIGHT_BASE_200),
            Self::Large => (ty::FONT_SIZE_BASE_300, ty::LINE_HEIGHT_BASE_300),
            Self::ExtraLarge => (ty::FONT_SIZE_BASE_400, ty::LINE_HEIGHT_BASE_400),
        }
    }
}

#[derive(Clone, Copy)]
enum TagSize {
    ExtraSmall,
    Small,
    Medium,
}

impl TagSize {
    const fn height(self) -> f32 {
        match self {
            Self::ExtraSmall => tp::TAG_HEIGHT_EXTRA_SMALL,
            Self::Small => tp::TAG_HEIGHT_SMALL,
            Self::Medium => tp::TAG_HEIGHT_MEDIUM,
        }
    }

    const fn spacing(self) -> f32 {
        match self {
            Self::ExtraSmall | Self::Small => tp::TAG_SPACING_SMALL,
            Self::Medium => tp::TAG_SPACING_MEDIUM,
        }
    }

    const fn icon(self) -> f32 {
        match self {
            Self::ExtraSmall => tp::TAG_ICON_EXTRA_SMALL,
            Self::Small => tp::TAG_ICON_SMALL,
            Self::Medium => tp::TAG_ICON_MEDIUM,
        }
    }

    const fn dismiss_pad(self) -> f32 {
        match self {
            Self::ExtraSmall | Self::Small => g::SPACING_HORIZONTAL_XXS,
            Self::Medium => g::SPACING_HORIZONTAL_XS,
        }
    }

    const fn text(self) -> (f32, f32) {
        match self {
            Self::ExtraSmall | Self::Small => (ty::FONT_SIZE_BASE_200, ty::LINE_HEIGHT_BASE_200),
            Self::Medium => (ty::FONT_SIZE_BASE_300, ty::LINE_HEIGHT_BASE_300),
        }
    }

    // usePrimaryTextStyles `base` padding
    const TEXT_PAD: f32 = g::SPACING_HORIZONTAL_XXS;

    fn width(self, text: f32) -> f32 {
        self.spacing() + Self::TEXT_PAD + text + Self::TEXT_PAD + self.dismiss_pad() + self.icon() + self.spacing()
    }
}

/// Fluent `TagPicker`: selected options as dismissible tags in front of a filtering input, with the unselected
/// options listed in a popup. [`Self::control`] swaps the input for a `TagPickerButton`.
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct TagPicker<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    entries: Arc<[ListEntry]>,
    selection: &'a mut Selection<String>,
    text: &'a mut String,
    appearance: InputAppearance,
    size: TagPickerSize,
    placeholder: Option<String>,
    button: bool,
    invalid: bool,
    disabled: bool,
}

impl<'a> TagPicker<'a> {
    /// `label` is the accessible name; `text` is the typed filter. Agent key `<id_salt>-tag-picker`.
    pub fn new(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        entries: impl IntoIterator<Item = impl Into<ListEntry>>,
        selection: &'a mut Selection<String>,
        text: &'a mut String,
    ) -> Self {
        Self {
            salt: id_salt.into(),
            key: AgentKey::Kind("tag-picker"),
            label: label.into(),
            entries: listbox::entries(entries).into(),
            selection,
            text,
            appearance: InputAppearance::Outline,
            size: TagPickerSize::Medium,
            placeholder: None,
            button: false,
            invalid: false,
            disabled: false,
        }
    }

    /// Default [`InputAppearance::Outline`].
    pub fn appearance(mut self, appearance: InputAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// Default [`TagPickerSize::Medium`].
    pub fn size(mut self, size: TagPickerSize) -> Self {
        self.size = size;
        self
    }

    /// Hint shown in the input, or in the button while nothing is selected.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Default [`TagPickerControl::Input`].
    pub fn control(mut self, control: TagPickerControl) -> Self {
        self.button = control == TagPickerControl::Button;
        self
    }

    /// Validation state, as [`FieldContext::validation`](super::FieldContext::validation) hands it over;
    /// [`Validation::Error`] is `aria-invalid`: red border until focused.
    pub fn validation(mut self, validation: Option<Validation>) -> Self {
        self.invalid = validation == Some(Validation::Error);
        self
    }

    crate::widget::setters! {
        /// Greys out the field; tags cannot be removed and the list cannot open.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-tag-picker` agent key.
        agent_key(key)
    }

    fn chrome(&self) -> Chrome {
        Chrome {
            appearance: self.appearance,
            size: Size::Medium,
            invalid: self.invalid,
            disabled: self.disabled,
        }
    }

    // Selected options as (value, text), in list order.
    fn tags(&self) -> Vec<(String, String)> {
        let all = Flat::new(&self.entries, &|_| true);
        all.options
            .iter()
            .filter(|o| self.selection.is_selected(&o.value))
            .map(|o| (o.value.clone(), o.text.clone()))
            .collect()
    }

    // Unselected options matching the typed text, as the `combobox` model ranks them.
    fn visible(&self, filtering: bool) -> HashSet<String> {
        let all = Flat::new(&self.entries, &|o| !self.selection.is_selected(&o.value));
        let labels: Vec<&str> = all.options.iter().map(|o| o.text.as_str()).collect();
        let query = if filtering && !self.button {
            self.text.as_str()
        } else {
            ""
        };
        combo_items(query, labels)
            .into_iter()
            .filter_map(|item| match item {
                ComboItem::Option { index, .. } => all.options.get(index).map(|o| o.value.clone()),
                _ => None,
            })
            .collect()
    }

    fn tag_format(&self, ui: &MineralUi<'_, Fluent>, color: egui::Color32) -> TextFormat {
        let (font, line) = self.size.tag().text();
        crate::typography::format(
            ui.ctx(),
            crate::typography::TypeSpec {
                size: font,
                line_height: line,
                weight: ty::FONT_WEIGHT_REGULAR,
            },
            color,
        )
    }

    fn widths(&self, ui: &MineralUi<'_, Fluent>, tags: &[(String, String)]) -> Vec<f32> {
        let format = self.tag_format(ui, egui::Color32::WHITE);
        tags.iter()
            .map(|(_, text)| {
                let galley = ui
                    .painter()
                    .layout_no_wrap(text.clone(), format.font_id.clone(), format.color);
                self.size.tag().width(galley.size().x)
            })
            .collect()
    }

    fn flow(&self, widths: &[f32], width: f32) -> Flow {
        let pad = g::SPACING_HORIZONTAL_M;
        let aside = self.size.icon() + self.size.icon_margin();
        let inner = width - 2.0 * g::STROKE_WIDTH_THIN;
        flow(widths, (inner - 2.0 * pad - aside).max(0.0), self.size.gap())
    }

    fn height(&self, flow: &Flow, tags: usize) -> f32 {
        let tag = self.size.tag().height();
        let rows = flow.rows as f32;
        let group = 2.0 * self.size.group_pad() + rows * tag + (rows - 1.0) * self.size.gap();
        if tags == 0 {
            self.size.min_height()
        } else {
            self.size.min_height().max(2.0 * g::STROKE_WIDTH_THIN + group)
        }
    }
}

struct Flow {
    tags: Vec<(usize, f32)>,
    input: (usize, f32, f32),
    rows: usize,
}

// Wraps the tags, then the input (at least `INPUT_MIN_WIDTH` wide, filling the rest of its row).
fn flow(widths: &[f32], content: f32, gap: f32) -> Flow {
    let (mut row, mut x) = (0, 0.0_f32);
    let mut tags = Vec::with_capacity(widths.len());
    for w in widths {
        if x > 0.0 && x + w > content {
            row += 1;
            x = 0.0;
        }
        tags.push((row, x));
        x += w + gap;
    }
    if x > 0.0 && x + tp::INPUT_MIN_WIDTH > content {
        row += 1;
        x = 0.0;
    }
    Flow {
        tags,
        input: (row, x, (content - x).max(tp::INPUT_MIN_WIDTH)),
        rows: row + 1,
    }
}

struct Geometry {
    inner: Rect,
    flow: Flow,
    row_top: f32,
    row_height: f32,
    row_step: f32,
}

impl Geometry {
    fn new(p: &TagPicker<'_>, visual: Rect, flow: Flow, tags: usize) -> Self {
        let inner = visual.shrink(g::STROKE_WIDTH_THIN);
        let tag = p.size.tag().height();
        let (row_top, row_height) = if tags == 0 {
            (inner.top(), inner.height())
        } else {
            (inner.top() + p.size.group_pad(), tag)
        };
        Self {
            inner,
            flow,
            row_top,
            row_height,
            row_step: tag + p.size.gap(),
        }
    }

    fn rect(&self, dir: egui_mineral::i18n::Direction, row: usize, x: f32, w: f32) -> Rect {
        let off = g::SPACING_HORIZONTAL_M + x;
        let left = dir.start_x(self.inner, Rangef::new(off, off + w));
        let top = self.row_top + row as f32 * self.row_step;
        Rect::from_min_size(Pos2::new(left, top), vec2(w, self.row_height))
    }
}

impl TagPicker<'_> {
    fn paint_tag(
        &self,
        ui: &mut MineralUi<'_, Fluent>,
        rect: Rect,
        index: usize,
        (value_text, base): (&str, egui::Id),
        dir: egui_mineral::i18n::Direction,
    ) -> bool {
        let t = ui.tokens();
        let tag = self.size.tag();
        let (bg, fg, border) = if self.disabled {
            (
                ColorNeutralBackgroundDisabled,
                ColorNeutralForegroundDisabled,
                ColorTransparentStrokeDisabled,
            )
        } else {
            (ColorNeutralBackground3, ColorNeutralForeground2, ColorTransparentStroke)
        };
        let p = ui.painter().clone();
        let radius = CornerRadius::from(g::BORDER_RADIUS_MEDIUM);
        p.rect_filled(rect, radius, t.color(bg));
        p.rect_stroke(
            rect,
            radius,
            Stroke::new(g::STROKE_WIDTH_THIN, t.color(border)),
            StrokeKind::Inside,
        );
        let format = self.tag_format(ui, t.color(fg));
        let galley = p.layout_no_wrap(value_text.to_owned(), format.font_id.clone(), format.color);
        let text_off = tag.spacing() + TagSize::TEXT_PAD;
        let x = dir.start_x(rect, Rangef::new(text_off, text_off + galley.size().x));
        p.galley(
            Pos2::new(x, rect.center().y - galley.size().y / 2.0),
            galley.clone(),
            format.color,
        );
        let dismiss_w = tag.dismiss_pad() + tag.icon() + tag.spacing();
        let off = rect.width() - dismiss_w;
        let dx = dir.start_x(rect, Rangef::new(off, off + dismiss_w));
        let hit = Rect::from_min_size(Pos2::new(dx, rect.top()), vec2(dismiss_w, rect.height()));
        let resp = kit::input::interact(
            ui,
            hit,
            base.with(("dismiss", index)),
            Sense::click() - Sense::FOCUSABLE,
            Availability::disabled_if(self.disabled),
        );
        let hot = Interaction::of(&resp, Availability::disabled_if(self.disabled));
        let color = if self.disabled {
            ColorNeutralForegroundDisabled
        } else if hot.pressed {
            ColorCompoundBrandForeground1Pressed
        } else if hot.hovered {
            ColorCompoundBrandForeground1Hover
        } else {
            fg
        };
        let icon_x = dir.start_x(
            rect,
            Rangef::new(off + tag.dismiss_pad(), off + tag.dismiss_pad() + tag.icon()),
        );
        paint::icon(
            &p,
            Pos2::new(icon_x + tag.icon() / 2.0, rect.center().y),
            ui.icons().dismiss,
            false,
            tag.icon(),
            t.color(color),
        );
        let rt = ui.rt();
        semantic::emit(rt.ctx(), Some(resp.id), hit, || {
            let name = rt
                .i18n()
                .tr("mineral-tag-picker-remove", &[("label", value_text.into())]);
            let mut n = SemanticNode::new(self.key.id(&self.salt).sub((index, "dismiss")), Role::Button, &name)
                .clickable(Availability::disabled_if(self.disabled));
            n.depth = 1;
            n
        });
        resp.clicked() && !self.disabled
    }

    fn chevron(&self, ui: &mut MineralUi<'_, Fluent>, base: egui::Id, visual: Rect) -> egui::Response {
        let t = ui.tokens();
        let dir = ui.i18n().direction();
        let inner = visual.shrink(g::STROKE_WIDTH_THIN);
        let icon = self.size.icon();
        let pad = g::SPACING_HORIZONTAL_M;
        let x = dir.start_x(inner, Rangef::new(inner.width() - pad - icon, inner.width() - pad));
        let center = Pos2::new(x + icon / 2.0, visual.top() + self.size.min_height() / 2.0);
        let hit = Rect::from_center_size(center, vec2(icon, self.size.min_height()));
        let resp = kit::input::interact(
            ui,
            hit,
            base.with("chevron"),
            if self.disabled {
                Sense::hover()
            } else {
                Sense::click() - Sense::FOCUSABLE
            },
            Availability::disabled_if(self.disabled),
        );
        paint::icon(
            ui.painter(),
            center,
            ui.icons().chevron_down,
            false,
            icon,
            t.color(if self.disabled {
                ColorNeutralForegroundDisabled
            } else {
                ColorNeutralStrokeAccessible
            }),
        );
        resp
    }
}

impl MineralWidget for TagPicker<'_> {
    type Kit = Fluent;
    type State = ListState;
    type Output = ();

    fn id_salt(&self) -> egui::IdSalt {
        self.salt.id_salt()
    }

    fn enabled(&self) -> bool {
        !self.disabled
    }

    fn sense(&self) -> Sense {
        if self.button {
            Sense::click()
        } else {
            kit::edit::frame_sense()
        }
    }

    fn min_target(&self) -> MinTarget {
        MinTarget::None
    }

    fn layout(&self, ui: &MineralUi<'_, Fluent>, c: Constraints, _: &ListState) -> Measure {
        let width = if c.max.x.is_finite() { c.max.x } else { tp::MIN_WIDTH };
        let tags = self.tags();
        let flow = self.flow(&self.widths(ui, &tags), width);
        c.constrain(vec2(width, self.height(&flow, tags.len()))).into()
    }

    fn update(
        &mut self,
        ui: &mut MineralUi<'_, Fluent>,
        frame: &WidgetFrame<'_>,
        st: &mut ListState,
    ) -> WidgetResponse<()> {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let base = ui.make_persistent_id(self.salt.id_salt());
        let focus_id = if self.button {
            frame.response.id
        } else {
            base.with("edit")
        };
        let focused = ctx.memory(|m| m.has_focus(focus_id));
        let tags = self.tags();
        let mut changed = self.backspace(ctx, focused, &tags);

        let keep = self.visible(st.filtering);
        let visible = |o: &ListOption| keep.contains(&o.value);
        let entries = Arc::clone(&self.entries);
        let flat = Flat::new(&entries, &visible);
        let enabled = |i: usize| flat.enabled(i);
        let list = ComboList {
            kind: if self.button {
                ComboKind::SelectOnly
            } else {
                ComboKind::Editable
            },
            selected: None,
            len: flat.len(),
            enabled: &enabled,
        };
        let (mut chosen, mut moved) = (None, false);
        if focused && !self.disabled {
            (chosen, moved) = value::drive(ctx, &mut st.combo, list);
        }

        let (shown, chevron, removed) = self.paint_field(ui, frame, (base, focused), &tags);
        changed |= removed;
        let edited = shown.as_ref().is_some_and(egui::Response::changed);
        if !self.disabled {
            let click = Click {
                edited,
                chevron: chevron.clicked(),
                field: frame.response.clicked() || shown.as_ref().is_some_and(egui::Response::clicked),
                focused,
                pointer_down: value::pointer_busy(ctx),
            };
            moved |= toggle(st, flat.len(), None, &click);
            if !focused && (click.field || click.chevron) {
                ctx.memory_mut(|m| m.request_focus(focus_id));
            }
        }
        st.combo.retain_active(flat.len());

        if st.combo.is_open() && !self.disabled && flat.len() > 0 {
            if let Some(i) = self.popup(ui, frame, &flat, st, moved) {
                chosen = Some(i);
                ctx.memory_mut(|m| m.request_focus(focus_id));
            }
        } else {
            st.popup = None;
            st.active_id = None;
        }

        if let Some(i) = chosen
            && let Some(o) = flat.value(i)
        {
            changed |= Value::Many(self.selection).pick(&o.value);
            st.filtering = false;
            self.text.clear();
            st.combo.close();
        } else if edited {
            st.filtering = true;
        }
        if shown.as_ref().is_some_and(egui::Response::lost_focus) && !value::pointer_busy(ctx) {
            self.text.clear();
            st.filtering = false;
        }

        let response = shown.unwrap_or_else(|| frame.response.clone());
        WidgetResponse::new(response, ()).with_changed(changed)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse<()>, st: &ListState) -> Option<SemanticNode> {
        let texts: Vec<String> = self.tags().into_iter().map(|(_, t)| t).collect();
        let mut n = SemanticNode::new(self.key.id(&self.salt), Role::ComboBox, &self.label).value(texts.join(", "));
        n = if self.button {
            n.clickable(Availability::disabled_if(self.disabled))
        } else {
            n.editable(Availability::disabled_if(self.disabled))
        };
        n.state.expanded = Some(st.combo.is_open());
        n.state.invalid = self.invalid;
        n.active_descendant = st.active_id.filter(|_| st.combo.is_open());
        Some(n)
    }
}

impl TagPicker<'_> {
    // Backspace in the empty input (or the button) drops the last tag.
    fn backspace(&mut self, ctx: &egui::Context, focused: bool, tags: &[(String, String)]) -> bool {
        let typed = ctx.input(|i| {
            i.events
                .iter()
                .any(|e| matches!(e, egui::Event::Text(_) | egui::Event::Paste(_)))
        });
        if focused
            && !self.disabled
            && !typed
            && (self.button || self.text.is_empty())
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Backspace))
            && let Some((v, _)) = tags.last()
        {
            return self.selection.deselect(v);
        }
        false
    }

    // Frame, tags, hint, chevron and input; the input response, the chevron and whether a tag was dismissed.
    fn paint_field(
        &mut self,
        ui: &mut MineralUi<'_, Fluent>,
        frame: &WidgetFrame<'_>,
        (base, focused): (egui::Id, bool),
        tags: &[(String, String)],
    ) -> (Option<egui::Response>, egui::Response, bool) {
        let ctx = ui.ctx().clone();
        let t = ui.tokens();
        let dir = ui.i18n().direction();
        let visual = frame.zones.visual;
        let chrome = self.chrome();
        let widths = self.widths(ui, tags);
        let geo = Geometry::new(self, visual, self.flow(&widths, visual.width()), tags.len());
        let live = Live {
            hovered: frame.response.contains_pointer(),
            pressed: frame.response.contains_pointer() && ctx.input(|i| i.pointer.primary_down()),
            focused,
        };
        paint_frame(ui, base, visual, chrome, live, &t);

        let mut removed = None;
        for (i, ((v, text), (row, x))) in tags.iter().zip(&geo.flow.tags).enumerate() {
            let rect = geo.rect(dir, *row, *x, widths.get(i).copied().unwrap_or_default());
            if self.paint_tag(ui, rect, i, (text, base), dir) {
                removed = Some(v.clone());
            }
        }
        let dismissed = removed.is_some_and(|v| self.selection.deselect(&v));

        let (row, x, w) = geo.flow.input;
        let slot = geo.rect(dir, row, x, w);
        let empty = if self.button {
            tags.is_empty()
        } else {
            self.text.is_empty()
        };
        if empty && let Some(hint) = self.placeholder.as_deref() {
            let (font, height) = self.input_font();
            let color = chrome.placeholder_color(&t);
            let format = crate::typography::format(
                &ctx,
                crate::typography::TypeSpec {
                    size: font,
                    line_height: height,
                    weight: ty::FONT_WEIGHT_REGULAR,
                },
                color,
            );
            let galley = ui.painter().layout_no_wrap(hint.to_owned(), format.font_id, color);
            let hx = dir.start_x(slot, Rangef::new(0.0, galley.size().x));
            let at = Pos2::new(hx, slot.center().y - galley.size().y / 2.0);
            ui.painter().galley(at, galley, color);
        }
        let chevron = self.chevron(ui, base, visual);
        let shown = (!self.button).then(|| {
            Edit {
                id: base.with("edit"),
                rect: slot,
                format: chrome.text_format(&ctx, &t),
                t: &t,
                mask: false,
                rows: None,
            }
            .show(ui, self.text, |e| e.interactive(!self.disabled))
        });
        (shown, chevron, dismissed)
    }

    // The clicked row, if any.
    fn popup(
        &self,
        ui: &mut MineralUi<'_, Fluent>,
        frame: &WidgetFrame<'_>,
        flat: &Flat<'_>,
        st: &mut ListState,
        moved: bool,
    ) -> Option<usize> {
        let selected = |_: &ListOption| false;
        let popup = listbox::show(
            ui,
            self.salt.child("popup"),
            &frame.response,
            frame.zones.visual,
            &self.key.id(&self.salt),
            &self.label,
            &View {
                flat,
                multiselect: false,
                selected: &selected,
                active: st.combo.active(),
                scroll_to_active: moved,
                max_height: f32::INFINITY,
            },
        );
        st.popup = popup.layer;
        st.active_id = popup.active_id;
        if popup.clicked.is_none() && popup.closed {
            st.combo.close();
        }
        popup.clicked
    }

    // useTagPickerButtonStyles for the button, `body1` for the input.
    fn input_font(&self) -> (f32, f32) {
        if self.button {
            self.size.button()
        } else {
            (ty::FONT_SIZE_BASE_300, ty::LINE_HEIGHT_BASE_300)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flow_wraps_tags_and_keeps_the_input_a_minimum_wide() {
        let f = flow(&[60.0, 60.0, 60.0], 150.0, 4.0);
        assert_eq!(f.tags, [(0, 0.0), (0, 64.0), (1, 0.0)]);
        assert_eq!(f.input, (1, 64.0, 86.0));
        let tight = flow(&[100.0], 110.0, 4.0);
        assert_eq!(tight.input.0, 1);
        assert_eq!(tight.rows, 2);
        let empty = flow(&[], 200.0, 4.0);
        assert_eq!(empty.input, (0, 0.0, 200.0));
    }
}
