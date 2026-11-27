use crate::Material;
use crate::MaterialExt;
use crate::MaterialUiExt as _;
use crate::text_inputs::text_field::FieldChrome;
use egui::{Key, Modifiers, Rangef, Rect, Ui, pos2, vec2};
use egui_mineral::i18n::I18n;
use egui_mineral::kit::state::TempState;
use egui_mineral::semantic::AgentKey;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::{Availability, MineralUi};
use egui_mineral::{
    i18n::Direction,
    layout::{Constraints, Measure},
    semantic::{Role, SemanticNode},
    tags,
    widget::{MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::selection::{
    Chip,
    select::{CHIP_GAP, ChipRows, chip_flow, chip_size, place_chip},
};
use crate::spec::defaults::text_field as tf;
use crate::text_inputs::text_field::{self, AffixCx, Decor, INPUT_FONT, content_inset, line_galley, line_h};

// No MD3 value; five bodyLarge digits
const INPUT_MIN: f32 = 80.0;

/// Field that turns typed text into removable tags; non-MD3.
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct TagsField<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    tags: &'a mut Vec<String>,
    placeholder: Option<String>,
    delimiters: Vec<char>,
    max: Option<usize>,
    disabled: bool,
    chrome: FieldChrome,
}

/// Events of [`TagsField`], read from `WidgetResponse::output`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct TagsFieldOutput {
    /// Tags appended to the bound list this frame.
    pub added: Vec<String>,
    /// Tags removed from the bound list this frame.
    pub removed: Vec<String>,
}

impl<'a> TagsField<'a> {
    /// `label` is the field label; `tags` is the bound list. Agent key `<id_salt>-tags`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, tags: &'a mut Vec<String>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            chrome: FieldChrome::default(),
            key: egui_mineral::agent_key!("tags"),
            salt: id_salt,
            label: label.into(),
            tags,
            placeholder: None,
            delimiters: vec![','],
            max: None,
            disabled: false,
        }
    }

    crate::text_inputs::text_field::field_chrome_setters!();
    crate::text_inputs::text_field::field_chrome_setters!(affix);

    /// Hint shown in the empty input.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Characters that commit the typed tag, besides Enter; default `,`.
    pub fn delimiters(mut self, delimiters: impl IntoIterator<Item = char>) -> Self {
        self.delimiters = delimiters.into_iter().collect();
        self
    }

    /// Tag limit, with a counter below the field; input stops at it.
    pub fn max_tags(mut self, max_tags: usize) -> Self {
        self.max = Some(max_tags);
        self
    }

    crate::widget::setters! {
        /// Greys out the field; tags cannot be added or removed.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-tags` agent key.
        agent_key(key)
    }

    fn labeled(&self) -> bool {
        !self.chrome.label_hidden && !self.label.is_empty()
    }

    fn content_width(&self, ui: &MineralUi<'_, Material>, w: f32) -> f32 {
        let density = ui.density();
        let span = w - content_inset(density, self.chrome.leading.is_some()) - content_inset(density, false);
        span - text_field::affix_room(ui, &self.chrome, span)
    }

    fn paint_chips(
        &mut self,
        ui: &mut MineralUi<'_, Material>,
        rows: &ChipRows,
        field_top: f32,
        x_of: &dyn Fn(f32, f32) -> f32,
        widths: &[f32],
        pos: &[(usize, f32)],
    ) -> Option<String> {
        let mut remove = None;
        for (i, ((tag, (row, x)), w)) in self.tags.iter().zip(pos).zip(widths).enumerate() {
            let crect = Rect::from_min_size(
                pos2(x_of(*x, *w), field_top + rows.row_top(*row)),
                vec2(*w, rows.chip_h),
            );
            let chip = Chip::input(self.salt.child(("tag", i)), tag)
                .disabled(self.disabled)
                .agent_key(self.key.id(&self.salt).sub((i, "chip")));
            if place_chip(ui, ("tag", i), crect, chip) {
                remove = Some(i);
            }
        }
        remove.map(|i| self.tags.remove(i))
    }

    fn flow(&self, ui: &MineralUi<'_, Material>, w: f32) -> (Vec<f32>, Vec<(usize, f32)>, usize) {
        let widths: Vec<f32> = self.tags.iter().map(|t| chip_size(ui, t).x).collect();
        let (pos, rows) = chip_flow(&widths, self.content_width(ui, w), INPUT_MIN);
        (widths, pos, rows)
    }

    fn has_support(&self) -> bool {
        self.chrome.supporting.is_some() || self.chrome.error.is_some() || self.max.is_some()
    }

    fn full(&self) -> bool {
        self.max.is_some_and(|m| self.tags.len() >= m)
    }
}

impl MineralWidget for TagsField<'_> {
    type Kit = Material;
    type State = ();
    type Output = TagsFieldOutput;

    egui_mineral::kit::widget_meta!(salt, enabled, no_target, frame_sense);

    fn layout(&self, ui: &MineralUi<'_, Material>, c: Constraints, (): &Self::State) -> Measure {
        let w = c.constrain(vec2(tf::MIN_WIDTH, 0.0)).x;
        let rows = ChipRows::new(ui, self.chrome.variant, self.labeled(), !self.tags.is_empty());
        let h = rows.field_height(ui.density(), self.chrome.variant, self.labeled(), self.flow(ui, w).2);
        text_field::measure(
            c.constrain(vec2(w, h)),
            self.chrome.variant,
            self.labeled(),
            self.has_support(),
        )
    }

    fn update(
        &mut self,
        ui: &mut MineralUi<'_, Material>,
        frame: &WidgetFrame<'_>,
        (): &mut (),
    ) -> WidgetResponse<TagsFieldOutput> {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let t = rt.tokens();
        let base = ui.make_persistent_id(self.salt.id_salt());
        let edit_id = base.with("edit");
        let input_id = base.with("input");
        let field = frame.zones.visual;
        let mut input = String::load(ctx, input_id);
        let focused = ctx.memory(|m| m.has_focus(edit_id));
        let dir = rt.i18n().direction();
        let before = self.tags.len();
        let mut out = TagsFieldOutput::default();

        if focused && !self.disabled && input.is_empty() && backspace(ctx) {
            out.removed.extend(self.tags.pop());
        }
        let (widths, pos, _) = self.flow(ui, field.width());

        let decor = Decor {
            id: base,
            variant: self.chrome.variant,
            label: if self.labeled() { &self.label } else { "" },
            focused,
            floated: focused || !self.tags.is_empty() || !input.is_empty() || !self.labeled(),
            error: self.chrome.error.is_some(),
            disabled: self.disabled,
            leading: self.chrome.leading,
            trailing: false,
        }
        .paint(ui, field, &t);
        let colors = decor.colors;
        let affixes = AffixCx {
            chrome: &self.chrome,
            labeled: self.labeled(),
            base,
            focused,
            empty: self.tags.is_empty() && input.is_empty(),
            lines: 1,
        };
        let decor = decor.within(text_field::paint_affixes(ui, &decor, &affixes));
        let span = decor.span.span();
        let rows = ChipRows::new(ui, self.chrome.variant, self.labeled(), !self.tags.is_empty());
        let x_of = |x: f32, w: f32| dir.span_x(decor.span, Rangef::new(x, x + w));

        let removed = self.paint_chips(ui, &rows, field.top(), &x_of, &widths, &pos);
        out.removed.extend(removed);

        let (last_row, tail) = pos
            .last()
            .zip(widths.last())
            .map_or((0, 0.0), |((r, x), w)| (*r, x + w + CHIP_GAP));
        let (row, x) = if tail + INPUT_MIN > span && !pos.is_empty() {
            (last_row + 1, 0.0)
        } else {
            (last_row, tail)
        };
        // The input line centers on its chip row.
        let row_top = field.top() + rows.row_top(row) + (rows.chip_h - line_h()) / 2.0;
        let w = (span - x).max(1.0);
        let input_rect = Rect::from_min_size(pos2(x_of(x, w), row_top), vec2(w, line_h()));
        if let (Some(ph), true) = (
            &self.placeholder,
            input.is_empty() && self.tags.is_empty() && (focused || !self.labeled()),
        ) {
            paint_placeholder(ui, ph, input_rect, row_top, colors.placeholder, dir);
        }
        let interactive = !self.disabled && !self.full();
        let te = text_field::bare_edit(
            edit_id,
            input_rect,
            self.chrome.text_style.unwrap_or(INPUT_FONT).format(ctx, colors.input),
            &colors,
            &t,
        )
        .show(ui, &mut input, |edit| edit.interactive(interactive));
        if !focused && frame.response.clicked() {
            ctx.memory_mut(|m| m.request_focus(edit_id));
        }
        let tags::Split {
            tags: mut done, rest, ..
        } = tags::split(&input, &self.delimiters);
        input = rest;
        // After the editor, so text typed in the same frame as Enter is part of the tag.
        if text_field::submitted(&te) {
            ctx.input_mut(|i| i.consume_key(i.modifiers, Key::Enter));
            done.push(input.trim().to_owned());
            input.clear();
            ctx.memory_mut(|m| m.request_focus(edit_id));
        }
        let start = self.tags.len();
        tags::add(self.tags, done, self.max);
        out.added.extend(self.tags[start..].iter().cloned());
        if self.has_support() {
            let s = self.chrome.error.as_deref().or(self.chrome.supporting.as_deref());
            let count = self.max.map(|m| text_field::counter(rt, self.tags.len(), m));
            text_field::paint_support(ui, field, s, count, colors.supporting, text_field::Elide::End);
        }
        input.store(ctx, input_id);
        let changed = self.tags.len() != before || !out.added.is_empty();
        self.chrome.show_tooltip(rt, &te);
        WidgetResponse::new(te, out).with_changed(changed)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse<TagsFieldOutput>, (): &()) -> Option<SemanticNode> {
        let mut n =
            SemanticNode::new(self.key.id(&self.salt), Role::TextInput, &self.label).value(self.tags.join(", "));
        n.state.invalid = self.chrome.error.is_some();
        Some(n.editable(Availability::disabled_if(self.disabled)))
    }
}

// Text in the same frame belongs to the editor; Backspace then edits it instead.
fn backspace(ctx: &egui::Context) -> bool {
    let typed = ctx.input(|i| {
        i.events
            .iter()
            .any(|e| matches!(e, egui::Event::Text(_) | egui::Event::Paste(_)))
    });
    !typed && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Backspace))
}

fn paint_placeholder(ui: &Ui, text: &str, rect: Rect, top: f32, color: egui::Color32, dir: Direction) {
    let g = line_galley(ui.painter(), text, INPUT_FONT, color, rect.width());
    let x = dir.start_x(rect, Rangef::new(0.0, 0.0 + g.size().x));
    ui.painter().galley(pos2(x, top), g, color);
}
