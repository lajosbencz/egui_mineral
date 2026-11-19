use crate::Material;
use crate::MaterialExt;
use crate::MaterialUiExt as _;
use crate::Rt;
use crate::adaptive::Density;
use egui::{
    Color32, Event, Frame, Margin, Rect, Stroke, StrokeKind, TextEdit, UiBuilder, pos2,
    text::{CCursor, CCursorRange},
    text_edit::TextEditState,
    vec2,
};
use egui_mineral::i18n::I18n;
use egui_mineral::semantic::AgentKey;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::{Availability, MineralUi};
use egui_mineral::{
    layout::{Constraints, Measure},
    mask::{CellMask, Charset},
    semantic::{Role, SemanticNode},
    widget::{MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::spec::{TypeScale, tokens::outlined_text_field};
use crate::text_inputs::text_field::{self, FieldState, TextFieldVariant};

// No MD3 value; each cell is an outlined text field holding one large character
const CELL_FONT: TypeScale = TypeScale::HeadlineSmall;
// Bullet of the cell font
const DOT_RADIUS: f32 = CELL_FONT.size() / 4.0;

// Minimum interactive width, text field height
fn cell_size(rt: &Rt, density: Density) -> egui::Vec2 {
    vec2(
        text_field::icon_slot(rt, density),
        text_field::field_h(density, TextFieldVariant::Outlined, false),
    )
}

const CELL_GAP: f32 = crate::adaptive::grid(2);

/// One-time code entry of fixed length; non-MD3 (shadcn `InputOTP` pattern).
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct OtpField<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    value: &'a mut String,
    len: usize,
    charset: Charset,
    masked: bool,
    error: Option<String>,
    disabled: bool,
}

/// Events of [`OtpField`], read from `WidgetResponse::output`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct OtpFieldOutput {
    /// Last empty cell filled this frame.
    pub completed: bool,
}

impl<'a> OtpField<'a> {
    fn mask(&self) -> CellMask {
        CellMask {
            charset: self.charset,
            len: self.len,
        }
    }

    /// `len` single-character cells (minimum 1) bound to `value`. Agent key `<id_salt>-otp`.
    #[track_caller]
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, len: usize, value: &'a mut String) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        let len = egui_mineral::kit::sanitize::at_least(len, 1.., "OtpField len");
        Self {
            key: AgentKey::Kind("otp"),
            salt: id_salt,
            label: label.into(),
            value,
            len,
            charset: Charset::Digits,
            masked: false,
            error: None,
            disabled: false,
        }
    }

    /// Accepted characters; default [`Charset::Digits`](egui_mineral::mask::Charset::Digits).
    pub fn charset(mut self, charset: Charset) -> Self {
        self.charset = charset;
        self
    }

    /// Shows dots instead of the characters.
    pub fn masked(mut self, masked: bool) -> Self {
        self.masked = masked;
        self
    }

    crate::widget::error_setters! {
        /// Error colors; `message` shows below the cells.
        |s| s.error
    }

    crate::widget::setters! {
        /// Greys out the cells; input is ignored.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-otp` agent key.
        agent_key(key)
    }

    fn prepare_edit(&mut self, ctx: &egui::Context, edit_id: egui::Id) {
        let at = cursor_of(ctx, edit_id).map_or(self.value.chars().count(), |r| r.start.0);
        let pasted: String = ctx.input_mut(|i| {
            let mut s = String::new();
            i.events.retain(|e| match e {
                Event::Paste(p) => {
                    s.push_str(p);
                    false
                }
                _ => true,
            });
            s
        });
        if !pasted.is_empty() {
            let next = self.mask().fill(self.value, at, &pasted);
            set_cursor(
                ctx,
                edit_id,
                CCursorRange::one(CCursor::new(next.min(self.value.chars().count()))),
            );
        }
        // Overwrite mode: the char under the cursor is selected so typing replaces it.
        if let Some(r) = cursor_of(ctx, edit_id)
            && r.start == r.end
            && r.start.0 < self.value.chars().count()
        {
            set_cursor(
                ctx,
                edit_id,
                CCursorRange::two(CCursor::new(r.start.0), CCursor::new(r.start.0 + 1)),
            );
        }
    }

    fn paint_cells(&self, ui: &mut MineralUi<'_, Material>, rect: Rect, te: &egui::Response, active: Option<usize>) {
        let rt = ui.rt();
        let ctx = &ui.ctx().clone();
        let t = rt.tokens();
        let p = ui.painter();
        let chars: Vec<char> = self.value.chars().collect();
        let radius = outlined_text_field::CONTAINER_SHAPE.corner_radius(&t.shape, rt.i18n().direction());
        let cursor = ui.visuals().text_cursor.clone();
        let mut supporting = None;
        for i in 0..self.len {
            let cell = Self::cell(rt, ui.density(), rect, i);
            let current = active == Some(i);
            let state = FieldState {
                focused: current,
                hovered: !self.disabled && te.hover_pos().is_some_and(|h| cell.contains(h)),
                error: self.error.is_some(),
                disabled: self.disabled,
            };
            let colors = text_field::colors(TextFieldVariant::Outlined, state, &t);
            supporting.get_or_insert(colors.supporting);
            let w = if current {
                outlined_text_field::FOCUS_OUTLINE_WIDTH
            } else {
                outlined_text_field::OUTLINE_WIDTH
            };
            p.rect_stroke(cell, radius, Stroke::new(w, colors.indicator), StrokeKind::Inside);
            if let Some(ch) = chars.get(i) {
                if self.masked {
                    p.circle_filled(cell.center(), DOT_RADIUS, colors.input);
                } else {
                    let g = text_field::line_galley(p, &ch.to_string(), CELL_FONT, colors.input, cell.width());
                    p.galley(cell.center() - g.size() / 2.0, g, colors.input);
                }
            } else if current {
                let reduced = rt.core().motion().reduced();
                let period = cursor.on_duration + cursor.off_duration;
                let phase = (ctx.input(|i| i.time) % f64::from(period)) as f32;
                if !reduced && cursor.blink {
                    let wake = if phase < cursor.on_duration {
                        cursor.on_duration
                    } else {
                        period
                    } - phase;
                    ctx.request_repaint_after_secs(wake);
                }
                if reduced || !cursor.blink || phase < cursor.on_duration {
                    let h = CELL_FONT.line_height() / 2.0;
                    p.vline(
                        cell.center().x,
                        (cell.center().y - h)..=(cell.center().y + h),
                        Stroke::new(cursor.stroke.width, colors.caret),
                    );
                }
            }
        }
        if let (Some(message), Some(color)) = (&self.error, supporting) {
            text_field::paint_support(ui, rect, Some(message), None, color, text_field::Elide::End);
        }
    }

    fn cell(rt: &Rt, density: Density, rect: Rect, i: usize) -> Rect {
        let size = cell_size(rt, density);
        Rect::from_min_size(pos2(rect.left() + i as f32 * (size.x + CELL_GAP), rect.top()), size)
    }
}

fn cursor_of(ctx: &egui::Context, id: egui::Id) -> Option<std::ops::Range<egui::text::CharIndex>> {
    TextEditState::load(ctx, id)
        .and_then(|s| s.cursor.char_range())
        .map(|r| r.as_sorted_char_range())
}

fn set_cursor(ctx: &egui::Context, id: egui::Id, range: CCursorRange) {
    let mut st = TextEditState::load(ctx, id).unwrap_or_default();
    st.cursor.set_char_range(Some(range));
    st.store(ctx, id);
}

impl MineralWidget for OtpField<'_> {
    type Kit = Material;
    type State = ();
    type Output = OtpFieldOutput;

    egui_mineral::kit::widget_meta!(salt, enabled, no_target, frame_sense);

    fn layout(&self, ui: &MineralUi<'_, Material>, c: Constraints, (): &Self::State) -> Measure {
        let n = self.len as f32;
        let cell = cell_size(ui.rt(), ui.density());
        let error_h = if self.error.is_some() {
            text_field::support_h()
        } else {
            0.0
        };
        c.constrain(vec2(n * cell.x + (n - 1.0) * CELL_GAP, cell.y + error_h))
            .into()
    }

    fn update(
        &mut self,
        ui: &mut MineralUi<'_, Material>,
        frame: &WidgetFrame<'_>,
        (): &mut (),
    ) -> WidgetResponse<OtpFieldOutput> {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let full = frame.zones.visual;
        let cell = cell_size(rt, ui.density());
        let rect = Rect::from_min_size(full.min, vec2(full.width(), cell.y));
        let edit_id = ui.make_persistent_id(self.salt.id_salt()).with("edit");
        let before = self.value.clone();
        *self.value = self.mask().sanitize(self.value);
        let focused = ctx.memory(|m| m.has_focus(edit_id));

        if focused && !self.disabled {
            self.prepare_edit(ctx, edit_id);
        }

        let te = {
            let ui = &mut ui.new_child(UiBuilder::new().id_salt("edit").max_rect(rect));
            {
                let v = ui.visuals_mut();
                v.selection.bg_fill = Color32::TRANSPARENT;
                v.selection.stroke = Stroke::NONE;
                v.text_cursor.stroke = Stroke::NONE;
            }
            ui.add_sized(
                rect.size(),
                TextEdit::singleline(self.value)
                    .id(edit_id)
                    .frame(Frame::NONE)
                    .margin(Margin::ZERO)
                    .text_color(Color32::TRANSPARENT)
                    .char_limit(self.len)
                    .interactive(!self.disabled),
            )
        };
        *self.value = self.mask().sanitize(self.value);
        let n = self.value.chars().count();
        if !self.disabled {
            let press = te.interact_pointer_pos().filter(|_| te.clicked()).or(frame.tapped);
            if let Some(p) = press {
                #[expect(clippy::cast_sign_loss, reason = ">= 0")]
                let at = (((p.x - rect.left()) / (cell.x + CELL_GAP)).floor().max(0.0) as usize).min(n);
                set_cursor(ctx, edit_id, CCursorRange::one(CCursor::new(at)));
                ctx.memory_mut(|m| m.request_focus(edit_id));
            }
        }
        let focused = ctx.memory(|m| m.has_focus(edit_id));
        let active = cursor_of(ctx, edit_id).map_or(n, |r| r.start.0).min(self.len - 1);

        self.paint_cells(ui, rect, &te, focused.then_some(active));
        if te.hovered() && !self.disabled {
            ctx.set_cursor_icon(egui::CursorIcon::Text);
        }

        let changed = *self.value != before;
        WidgetResponse::new(
            te,
            OtpFieldOutput {
                completed: changed && n == self.len,
            },
        )
        .with_changed(changed)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse<OtpFieldOutput>, (): &()) -> Option<SemanticNode> {
        let value = if self.masked {
            "*".repeat(self.value.chars().count())
        } else {
            self.value.clone()
        };
        let role = if self.masked {
            Role::PasswordInput
        } else {
            Role::TextInput
        };
        let mut n = SemanticNode::new(self.key.id(&self.salt), role, &self.label).value(value);
        n.state.invalid = self.error.is_some();
        Some(n.editable(Availability::disabled_if(self.disabled)))
    }
}
