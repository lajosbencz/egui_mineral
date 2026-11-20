use crate::Material;
use crate::MaterialExt;
use crate::MaterialUiExt as _;
use egui::{
    Align, Color32, CursorIcon, Key, Rangef, Rect, pos2,
    text::{CCursor, CCursorRange},
    vec2,
};
use egui_mineral::i18n::I18n;
use egui_mineral::kit::state::TempState;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::Availability;
use egui_mineral::widget::Composite;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    layout::{Constraints, Measure},
    semantic::{Role, SemanticNode, StableId},
    widget::{MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::actions::IconButton;
use crate::text_inputs::text_field::{self, FieldChrome, FieldState, line_galley};
use crate::widget::common;
use crate::{
    spec::{
        TypeScale,
        defaults::{button, text_field as tf},
        tokens::filled_text_field,
    },
    tr,
};

const EDIT_ICON: f32 = button::ICON_SIZE;

fn row_height(style: TypeScale) -> f32 {
    let (container, _) = crate::widget::parts::small_icon_metrics();
    style.line_height().max(container.y)
}

#[derive(Clone, Default)]
struct EditState {
    draft: Option<String>,
    cancel: Option<Rect>,
    focus: bool,
}

impl TempState for EditState {}

/// Events of [`InlineEdit`], read from `WidgetResponse::output`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct InlineEditOutput {
    /// Editor open after this frame.
    pub editing: bool,
    /// Edit confirmed this frame; `value` holds the result.
    pub submitted: bool,
    /// Edit discarded this frame; `value` is unchanged.
    pub cancelled: bool,
}

/// Text that turns into a field on activation; non-MD3 (Chakra `Editable` pattern).
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct InlineEdit<'a> {
    salt: KeyedSalt,
    label: String,
    value: &'a mut String,
    style: TypeScale,
    placeholder: Option<String>,
    disabled: bool,
    editing: Option<&'a mut bool>,
    chrome: FieldChrome,
}

impl<'a> InlineEdit<'a> {
    /// `label` is the accessible name; `value` is the bound text. Agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, value: &'a mut String) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            label: label.into(),
            value,
            style: TypeScale::BodyLarge,
            placeholder: None,
            disabled: false,
            editing: None,
            chrome: FieldChrome::default(),
        }
    }

    crate::text_inputs::text_field::field_chrome_setters!(variant);

    /// Text role of the preview and the editor; default [`TypeScale::BodyLarge`].
    pub fn text_style(mut self, text_style: TypeScale) -> Self {
        self.style = text_style;
        self
    }

    /// Binds the editor's open state: setting it opens the editor focused with the text selected
    /// (an app shortcut such as F2), clearing it discards the draft; the widget writes it back as the
    /// editor opens, submits and cancels.
    pub fn editing(mut self, editing: &'a mut bool) -> Self {
        self.editing = Some(editing);
        self
    }

    /// Preview text while `value` is empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    crate::widget::setters! {
        /// Shows the preview only; it cannot be edited.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }
}

impl InlineEdit<'_> {
    fn edit_row(
        &self,
        ui: &mut MineralUi<'_, Material>,
        draft: &mut String,
        edit_id: egui::Id,
        size: egui::Vec2,
    ) -> EditRow {
        let (width, h) = (size.x, size.y);
        let rt = ui.rt();
        let t = rt.tokens();
        // A focused filled text field without its container.
        let colors = text_field::colors(
            self.chrome.variant,
            FieldState {
                focused: true,
                ..FieldState::default()
            },
            &t,
        );
        let pad = crate::adaptive::grid(1);
        let density = ui.density();
        let row = ui.allocate_ui_with_layout(size, rt.i18n().direction().row(), |ui| {
            let submit = IconButton::new(
                self.salt.child("submit"),
                rt.icons().check,
                tr(rt.i18n(), "mineral-inline-submit"),
            );
            let cancel = IconButton::new(
                self.salt.child("cancel"),
                rt.icons().close,
                tr(rt.i18n(), "mineral-inline-cancel"),
            );
            let button = submit.footprint(rt, density);
            let edit_w = (width - 2.0 * (button.x + ui.spacing().item_spacing.x)).max(button.x);
            let (rect, _) = ui.allocate_exact_size(vec2(edit_w, h), egui::Sense::hover());
            let format = self.style.format(ui.ctx(), colors.input);
            // The child inherits this row layout, which centers the edit vertically.
            let te = text_field::BareEdit::new(edit_id, rect.shrink2(vec2(pad, 0.0)), format, &colors, &t)
                .v_align(Align::Center)
                .show(&mut rt.ui(ui), &mut *draft, |ui, edit| ui.add(edit));
            let line = filled_text_field::FOCUS_ACTIVE_INDICATOR_HEIGHT;
            ui.painter().rect_filled(
                Rect::from_min_max(pos2(rect.left(), rect.bottom() - line), rect.max),
                0,
                colors.indicator,
            );
            let ok = rt.ui(ui).mineral(submit);
            let cancel = rt.ui(ui).mineral(cancel);
            (te, ok, cancel)
        });
        let (te, ok, cancel) = row.inner;
        EditRow {
            response: row.response,
            field: te,
            ok_clicked: ok.clicked(),
            cancel_clicked: cancel.clicked(),
            cancel_rect: cancel.rect,
        }
    }

    fn preview(&self, ui: &mut MineralUi<'_, Material>, st: &mut EditState, size: egui::Vec2) -> egui::Response {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let r = rt.ui(ui).mineral(Preview {
            salt: self.salt.clone(),
            key: StableId::of(&self.salt, "preview"),
            label: &self.label,
            text: self.value,
            placeholder: self.placeholder.as_deref(),
            style: self.style,
            width: size.x,
            height: size.y,
            disabled: self.disabled,
        });
        if *r.output() {
            st.draft = Some(self.value.clone());
            st.focus = true;
            #[expect(clippy::disallowed_methods, reason = "one pass to enter edit mode")]
            ctx.request_repaint();
        }
        r.into_response()
    }
}

struct EditRow {
    response: egui::Response,
    field: egui::Response,
    ok_clicked: bool,
    cancel_clicked: bool,
    cancel_rect: Rect,
}

impl Composite for InlineEdit<'_> {
    type Kit = Material;
    type Output = InlineEditOutput;

    egui_mineral::kit::widget_meta!(salt);

    fn compose(
        &mut self,
        ui: &mut MineralUi<'_, Material>,
        constraints: Constraints,
    ) -> WidgetResponse<InlineEditOutput> {
        let ctx = ui.ctx().clone();
        let base = ui.make_persistent_id(self.salt.id_salt());
        let state_id = base.with("inline");
        let edit_id = base.with("edit");
        let mut st = EditState::load(&ctx, state_id);
        let (mut submitted, mut cancelled) = (false, false);
        match self.editing.as_deref() {
            Some(true) if st.draft.is_none() && !self.disabled => {
                st.draft = Some(self.value.clone());
                st.focus = true;
            }
            Some(false) if st.draft.take().is_some() => {
                st.cancel = None;
                cancelled = true;
            }
            _ => {}
        }
        let h = row_height(self.style);
        let width = constraints.constrain(vec2(tf::MIN_WIDTH, 0.0)).x;

        let response = if let Some(mut draft) = st.draft.take() {
            let escape = egui_mineral::kit::overlay::consume_escape_from(&ctx, edit_id);
            let row = self.edit_row(ui, &mut draft, edit_id, vec2(width, h));
            let te = &row.field;
            if st.focus {
                te.request_focus();
                let mut s = egui::text_edit::TextEditState::load(&ctx, edit_id).unwrap_or_default();
                s.cursor.set_char_range(Some(CCursorRange::two(
                    CCursor::new(0),
                    CCursor::new(draft.chars().count()),
                )));
                s.store(&ctx, edit_id);
                st.focus = false;
            }
            let pressed_cancel = st
                .cancel
                .zip(ctx.input(|i| i.pointer.interact_pos()))
                .is_some_and(|(r, p)| r.contains(p));
            if escape || row.cancel_clicked {
                cancelled = true;
            } else if row.ok_clicked || te.lost_focus() && !pressed_cancel {
                *self.value = draft;
                submitted = true;
            } else {
                st.draft = Some(draft);
                st.cancel = Some(row.cancel_rect);
            }
            if submitted || cancelled {
                st.cancel = None;
            }
            crate::text_inputs::text_field::emit_part(&ctx, te, 0, || {
                SemanticNode::new(StableId::of(&self.salt, "field"), Role::TextInput, &self.label)
                    .editable(Availability::Enabled)
            });
            row.response
        } else {
            self.preview(ui, &mut st, vec2(width, h))
        };
        let editing = st.draft.is_some();
        if let Some(bound) = self.editing.as_deref_mut() {
            *bound = editing;
        }
        st.store(&ctx, state_id);
        let mut response = response;
        if submitted {
            response.mark_changed();
        }
        WidgetResponse::new(
            response,
            InlineEditOutput {
                editing,
                submitted,
                cancelled,
            },
        )
        .with_changed(submitted)
    }
}

struct Preview<'p> {
    salt: KeyedSalt,
    key: StableId,
    label: &'p str,
    text: &'p str,
    placeholder: Option<&'p str>,
    style: TypeScale,
    width: f32,
    height: f32,
    disabled: bool,
}

impl MineralWidget for Preview<'_> {
    type Kit = Material;
    type State = ();
    type Output = bool;

    egui_mineral::kit::widget_meta!(salt, enabled, no_target);

    fn layout(&self, _ui: &MineralUi<'_, Material>, c: Constraints, (): &Self::State) -> Measure {
        c.constrain(vec2(self.width, self.height)).into()
    }

    fn update(
        &mut self,
        ui: &mut MineralUi<'_, Material>,
        frame: &WidgetFrame<'_>,
        (): &mut (),
    ) -> WidgetResponse<bool> {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let t = rt.tokens();
        let c = &t.color;
        let r = &frame.response;
        let rect = frame.zones.visual;
        let activated = r.clicked()
            || frame.tapped.is_some()
            || r.has_focus() && ctx.input(|i| i.key_pressed(Key::Enter) || i.key_pressed(Key::F2));
        if r.hovered() && !self.disabled {
            ctx.set_cursor_icon(CursorIcon::Text);
        }
        let i = common::Interaction::of(r, Availability::disabled_if(self.disabled));
        let p = ui.painter();
        let cr = filled_text_field::CONTAINER_SHAPE.scale().radius(&t.shape);
        crate::widget::indication::paint(ui, r, rect, egui::CornerRadius::same(cr), c.on_surface, i, &t);
        let (text, color): (&str, Color32) = if self.text.is_empty() {
            (self.placeholder.unwrap_or_default(), c.on_surface_variant)
        } else {
            (self.text, c.on_surface)
        };
        let color = if self.disabled {
            common::disabled_content(&t)
        } else {
            color
        };
        let dir = rt.i18n().direction();
        let pad = crate::adaptive::grid(1);
        let icon_room = if i.hovered || i.focused {
            EDIT_ICON + 2.0 * pad
        } else {
            0.0
        };
        let g = line_galley(p, text, self.style, color, rect.width() - 2.0 * pad - icon_room);
        let x = dir.start_x(rect, Rangef::new(pad, pad + g.size().x));
        p.galley(pos2(x, rect.center().y - g.size().y / 2.0), g, color);
        if (i.hovered || i.focused) && !self.disabled {
            let half = pad + EDIT_ICON / 2.0;
            let ix = dir.reversed().start_x(rect, Rangef::point(half));
            common::icon(
                p,
                pos2(ix, rect.center().y),
                ui.icons().edit,
                EDIT_ICON,
                c.on_surface_variant,
            );
        }
        WidgetResponse::new(r.clone(), activated)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse<bool>, (): &()) -> Option<SemanticNode> {
        Some(
            SemanticNode::new(self.key.clone(), Role::Button, self.label)
                .value(self.text)
                .clickable(Availability::disabled_if(self.disabled)),
        )
    }
}
