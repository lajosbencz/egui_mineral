//! Text editing shared by stylekits: the frameless editor inside a kit's text input chrome and the edit session
//! of its bound value.

use std::sync::Arc;

use egui::{
    Align, Align2, Color32, Context, EventFilter, Frame, Galley, Id, Margin, Rect, Response, ScrollArea, TextBuffer,
    TextEdit, TextFormat, Ui, UiBuilder, text::LayoutJob,
};

use crate::{
    edit::{EditBuffer, EditEvent, FieldFocus},
    layout::Edge,
    widget::MineralUi,
};

/// Frameless `TextEdit` filling `rect` in a child `Ui`, for text inputs whose kit paints the chrome (label,
/// outline, affixes): text in `format`, masked with egui's password character, aligned to `h_align` in the text
/// direction, and with `rows` a wrapping multiline edit in a vertical `ScrollArea` that
/// [`chain`](super::scroll::chain)s its overscroll.
#[derive(Clone, Debug)]
#[must_use]
pub struct BareEdit {
    id: Id,
    rect: Rect,
    format: TextFormat,
    caret: Color32,
    selection: Color32,
    v_align: Align,
    h_align: Edge,
    mask: bool,
    rows: Option<usize>,
    stick_to_bottom: bool,
    keep_escape: bool,
}

impl BareEdit {
    /// `id` is the edit's widget id; the caret takes `format.color`.
    pub fn new(id: Id, rect: Rect, format: TextFormat, selection: Color32) -> Self {
        Self {
            id,
            rect,
            caret: format.color,
            format,
            selection,
            v_align: Align::Min,
            h_align: Edge::Start,
            mask: false,
            rows: None,
            stick_to_bottom: false,
            keep_escape: false,
        }
    }

    /// Default the text color.
    pub const fn caret(mut self, caret: Color32) -> Self {
        self.caret = caret;
        self
    }

    /// Default [`Align::Min`].
    pub const fn v_align(mut self, v_align: Align) -> Self {
        self.v_align = v_align;
        self
    }

    /// Default [`Edge::Start`].
    pub const fn h_align(mut self, h_align: Edge) -> Self {
        self.h_align = h_align;
        self
    }

    /// Shows the text masked; default `false`.
    pub const fn mask(mut self, mask: bool) -> Self {
        self.mask = mask;
        self
    }

    /// `Some(rows)`: wrapping multiline edit of `rows` visible lines; default single line.
    pub const fn rows(mut self, rows: Option<usize>) -> Self {
        self.rows = rows;
        self
    }

    /// Multiline scroll follows the end of the text, e.g. while focused; default `false`.
    pub const fn stick_to_bottom(mut self, stick_to_bottom: bool) -> Self {
        self.stick_to_bottom = stick_to_bottom;
        self
    }

    /// Edits for `session`: a buffered one keeps focus on Escape, which cancels its edit.
    pub const fn session(mut self, session: &TextSession) -> Self {
        self.keep_escape = session.buffered;
        self
    }

    /// `add` sets the remaining `TextEdit` options (interactivity, char limit, event filter).
    pub fn show<K>(
        self,
        ui: &mut MineralUi<'_, K>,
        text: &mut dyn TextBuffer,
        add: impl FnOnce(TextEdit<'_>) -> TextEdit<'_>,
    ) -> Response {
        let dir = ui.i18n().direction().from_edge(self.h_align);
        let mut layouter = layouter(self.format, self.mask, self.rows.is_some());
        let edit = match self.rows {
            Some(rows) => TextEdit::multiline(text).desired_rows(rows),
            None => TextEdit::singleline(text),
        }
        .id(self.id)
        .frame(Frame::NONE)
        .margin(Margin::ZERO)
        .layouter(&mut layouter)
        // egui otherwise takes the parent layout's alignment.
        .align(Align2([dir.start_align(), self.v_align]))
        .password(self.mask)
        .desired_width(self.rect.width());
        let mut edit = add(edit);
        if self.keep_escape {
            let f = edit.get_event_filter();
            edit = edit.event_filter(EventFilter { escape: true, ..f });
        }
        // A child Ui leaves the parent cursor alone (`put`/`scope_builder` would move it back).
        let mut child = ui.new_child(UiBuilder::new().id_salt(self.id).max_rect(self.rect));
        let v = child.visuals_mut();
        v.text_cursor.stroke.color = self.caret;
        v.selection.bg_fill = self.selection;
        if self.rows.is_none() {
            return child.add(edit);
        }
        let out = ScrollArea::vertical()
            .id_salt("scroll")
            .max_height(self.rect.height())
            .auto_shrink([false; 2])
            .stick_to_bottom(self.stick_to_bottom)
            .show(&mut child, |ui| ui.add(edit));
        super::scroll::chain(&child, &out);
        out.inner
    }
}

/// Edit session of a text field over its bound value. Live, typing writes the value. Buffered, typing edits the
/// [`EditBuffer`] draft, which Enter, focus loss and [`commit_pending`](crate::edit::commit_pending) write to the
/// value and Escape discards. [`Self::begin`] before painting, the editor shows [`Self::text`] through
/// [`BareEdit::session`], then [`Self::end`].
#[derive(Clone, Copy, Debug)]
#[must_use = "end the session with .end(..)"]
pub struct TextSession {
    id: Id,
    buffered: bool,
    edit: Option<EditEvent>,
}

/// What a [`TextSession`] did this pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct SessionOutcome {
    /// Live edit, or buffered commit of a draft that differs from the value.
    pub changed: bool,
    /// `Some` on the pass a buffered edit ends; `None` while live.
    pub edit: Option<EditEvent>,
    /// Enter pressed as a single-line edit lost focus ([`submitted`]).
    pub submitted: bool,
    /// Focus left the edit.
    pub blurred: bool,
}

impl TextSession {
    /// Session of the editor `id` (the [`BareEdit`] id); a disabled one ignores Escape.
    pub fn begin(
        ctx: &Context,
        id: Id,
        buffered: bool,
        enabled: bool,
        value: &String,
        buffer: &mut EditBuffer<String>,
    ) -> Self {
        let mut edit = None;
        if buffered {
            let focus = FieldFocus::of(ctx.memory(|m| m.has_focus(id)));
            buffer.sync(value, focus);
            if enabled && super::layer::consume_escape_from(ctx, id) {
                edit = buffer.cancel();
                ctx.memory_mut(|m| m.surrender_focus(id));
            }
        }
        Self { id, buffered, edit }
    }

    /// Text the editor edits.
    pub fn text<'s>(&self, value: &'s mut String, buffer: &'s mut EditBuffer<String>) -> &'s mut String {
        if self.buffered { buffer.draft_mut() } else { value }
    }

    /// Settles the pass after the editor `edit` showed: a buffered commit writes the draft to `value`, and
    /// `edit` reports `changed` only when `value` did.
    pub fn end(
        self,
        edit: &mut Response,
        multiline: bool,
        value: &mut String,
        buffer: &mut EditBuffer<String>,
    ) -> SessionOutcome {
        let mut out = SessionOutcome {
            edit: self.edit,
            submitted: !multiline && submitted(edit),
            blurred: edit.lost_focus(),
            changed: false,
        };
        if !self.buffered {
            out.changed = edit.changed();
            return out;
        }
        let ctx = edit.ctx.clone();
        if edit.changed() {
            buffer.start(value);
        }
        let flush = buffer.commit_requested(&ctx);
        if out.edit.is_none() && (flush || !ctx.memory(|m| m.has_focus(self.id))) {
            out.edit = buffer.commit();
        }
        // The draft changed, not the bound value.
        edit.flags.remove(egui::response::Flags::CHANGED);
        out.changed = out.edit == Some(EditEvent::Committed) && buffer.draft() != value;
        if out.changed {
            value.clone_from(buffer.draft());
            buffer.wrote(value);
            edit.mark_changed();
        }
        out
    }
}

/// Single-line submit: Enter pressed as the edit lost focus.
pub fn submitted(edit: &Response) -> bool {
    edit.lost_focus() && edit.ctx.input(|i| i.key_pressed(egui::Key::Enter))
}

// Masked or plain text in `format`; single-line text never breaks on newlines.
fn layouter(format: TextFormat, mask: bool, multiline: bool) -> impl FnMut(&Ui, &dyn TextBuffer, f32) -> Arc<Galley> {
    move |ui, buf, wrap_width| {
        let text = if mask {
            std::iter::repeat_n(
                egui::epaint::text::PASSWORD_REPLACEMENT_CHAR,
                buf.as_str().chars().count(),
            )
            .collect()
        } else {
            buf.as_str().to_owned()
        };
        let mut job = LayoutJob::single_section(text, format.clone());
        job.keep_trailing_whitespace = true;
        if multiline {
            job.wrap.max_width = wrap_width;
        } else {
            job.break_on_newline = false;
        }
        ui.fonts_mut(|f| f.layout_job(job))
    }
}

#[cfg(test)]
mod tests {
    use egui::{Color32, Event, FontId, Id, Key, Modifiers, Rect, TextFormat, pos2};

    use super::{BareEdit, submitted};
    use crate::test_support::{ctx, input, run, runtime, tap};

    #[test]
    fn edits_and_submits_in_rect() {
        let ctx = ctx();
        let rt = runtime(&ctx);
        let id = Id::unique("edit");
        let rect = Rect::from_min_max(pos2(10.0, 10.0), pos2(110.0, 30.0));
        let mut text = String::new();
        let frame = |events: Vec<Event>, text: &mut String| {
            let mut shown = None;
            run(&ctx, input(events), |ui| {
                let format = TextFormat::simple(FontId::proportional(14.0), Color32::WHITE);
                let edit = BareEdit::new(id, rect, format, Color32::BLUE).mask(true);
                shown = Some(edit.show(&mut rt.ui(ui), text, |e| e));
            });
            shown
        };
        ctx.memory_mut(|m| m.request_focus(id));
        frame(Vec::new(), &mut text);
        frame(vec![Event::Text("pw".into())], &mut text);
        assert_eq!(text, "pw");
        let te = frame(tap(Key::Enter, Modifiers::NONE), &mut text).unwrap();
        assert!(rect.contains_rect(te.rect), "{:?}", te.rect);
        assert!(submitted(&te));
    }
}
