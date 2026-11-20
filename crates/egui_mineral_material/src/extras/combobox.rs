use crate::Material;
use crate::MaterialExt;
use crate::Rt;
use crate::msg;
use crate::text_inputs::text_field::FieldChrome;
use egui_mineral::kit::state::TempState;
use egui_mineral::semantic::AgentKey;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::Composite;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    combobox::{ComboItem, ComboKind, ComboState, combo_items, creatable_combo_items},
    layout::Constraints,
    widget::{SizeExt, WidgetResponse},
};

use crate::Icon;
use crate::text_inputs::{
    TextField,
    text_field::{self},
};
use crate::widget::leading::Leading;
use crate::{
    selection::select::{self, Listbox},
    widget::row_parts::{self, OptionRow},
};

#[derive(Clone, Copy, Default)]
struct ComboboxState {
    combo: ComboState,
    active_id: Option<egui::Id>,
    popup: Option<egui::LayerId>,
}

impl TempState for ComboboxState {}

/// Events of [`Combobox`], read from `WidgetResponse::output`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct ComboboxOutput {
    /// Option picked this frame; `text` holds its label.
    pub selected: Option<usize>,
    /// Typed text submitted as a new entry; only with [`Combobox::free_text`].
    pub created: Option<String>,
}

/// Options are supplied every pass; async sources pass the latest results plus [`Self::loading`].
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Combobox<'a> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    text: &'a mut String,
    options: Vec<String>,
    free_text: bool,
    loading: bool,
    disabled: bool,
    chrome: FieldChrome,
}

impl<'a> Combobox<'a> {
    /// `label` is the field label; `options` filter by `text`, the bound field value. Agent key `<id_salt>-combobox`.
    pub fn new(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        options: impl IntoIterator<Item = impl Into<String>>,
        text: &'a mut String,
    ) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            chrome: FieldChrome::default(),
            key: AgentKey::Kind("combobox"),
            salt: id_salt,
            label: label.into(),
            text,
            options: options.into_iter().map(Into::into).collect(),
            free_text: false,
            loading: false,
            disabled: false,
        }
    }

    /// Offers a create entry for text that matches no option.
    pub fn free_text(mut self, free_text: bool) -> Self {
        self.free_text = free_text;
        self
    }

    /// Shows a loading status row in the list.
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    crate::text_inputs::text_field::field_chrome_setters!();
    crate::text_inputs::text_field::field_chrome_setters!(text);

    crate::widget::setters! {
        /// Greys out the field; the list cannot open.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-combobox` agent key.
        agent_key(key)
    }

    fn field(&mut self, rt: &crate::Rt, arrow: Icon) -> TextField<'_> {
        TextField::new(self.salt.clone(), self.label.clone(), &mut *self.text)
            .with_chrome(std::mem::take(&mut self.chrome))
            .event_filter(text_field::editor_keys(false))
            .trailing_icon(arrow, crate::tr(rt.i18n(), msg::COMBOBOX_TOGGLE))
            .disabled(self.disabled)
            .with_agent_key(self.key.clone())
    }

    fn items(&self) -> Vec<ComboItem> {
        items_for(self.text, &self.options, self.free_text)
    }
}

impl Composite for Combobox<'_> {
    type Kit = Material;
    type Output = ComboboxOutput;

    egui_mineral::kit::widget_meta!(salt);

    fn compose(
        &mut self,
        ui: &mut MineralUi<'_, Material>,
        constraints: Constraints,
    ) -> WidgetResponse<ComboboxOutput> {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let base = ui.make_persistent_id(self.salt.id_salt());
        let field_id = text_field::field_id(ui, &self.salt);
        let edit_id = field_id.with("edit");
        let state_id = base.with("combobox");
        let mut st = ComboboxState::load(ctx, state_id);
        // Rows matter to the keys only while the editor is focused or the list is open.
        let filtered_before = ctx.memory(|m| m.has_focus(edit_id)) || st.combo.is_open();
        let items_before = if filtered_before { self.items() } else { Vec::new() };
        let (pick, moved, enter_free) = self.editor_keys(ctx, &mut st.combo, edit_id, items_before.len());

        let arrow = if st.combo.is_open() {
            ui.icons().arrow_drop_up
        } else {
            ui.icons().arrow_drop_down
        };
        let mut field = self.field(rt, arrow);
        field.combo = Some(if st.combo.is_open() {
            text_field::Combo::Expanded
        } else {
            text_field::Combo::Collapsed
        });
        field.active_descendant = st.active_id.filter(|_| st.combo.is_open());
        let out = ui.mineral(field.constrain(constraints));
        let Self {
            salt,
            key,
            label,
            text,
            options,
            free_text,
            loading,
            disabled,
            ..
        } = self;
        let (free_text, loading, disabled) = (*free_text, *loading, *disabled);
        // The pick owns this Enter; a dialog default action must not see it.
        if pick.is_some() || enter_free {
            ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        }
        let trailing = out.output().trailing_clicked;
        // `ExposedDropdownMenu` anchors to the whole text field, supporting row included.
        let field_rect = out.zones().outer;
        let focused = ctx.memory(|m| m.has_focus(edit_id));
        // Re-filter when the text changed or a click opens the list unfiltered so far.
        let refilter = out.changed() || !filtered_before && (trailing || out.response().clicked());
        let edited = refilter.then(|| items_for(text, options, free_text));
        let items = edited.as_deref().unwrap_or(&items_before);

        if !disabled {
            if out.changed() && focused {
                st.combo.open(None, items.len());
            }
            if trailing {
                if st.combo.is_open() {
                    st.combo.close();
                } else {
                    st.combo.open((!items.is_empty()).then_some(0), items.len());
                }
                ctx.memory_mut(|m| m.request_focus(edit_id));
            } else if out.response().clicked() && !st.combo.is_open() {
                st.combo.open(None, items.len());
            }
            if !focused && !trailing && !select::focus_within(ctx, st.popup) {
                st.combo.close();
            }
        }
        st.combo.retain_active(items.len());

        let mut chosen = pick.and_then(|i| items_before.get(i).cloned());
        if st.combo.is_open() && !disabled {
            let create_label = rt.i18n().tr(msg::ComboboxCreate {
                query: text.trim().into(),
            });
            let status = status_row(rt, loading, items.is_empty());
            let add = rt.icons().add;
            let row = |i: usize| option_row(items, options, &create_label, status.as_ref(), add, i);
            let key = key.id(salt);
            let list = Listbox {
                popup: salt.child("popup"),
                trigger: out.response(),
                anchor: field_rect,
                key: &key,
                label,
                len: items.len() + usize::from(status.is_some()),
                row: &row,
                active: st.combo.active(),
                scroll_to_active: moved,
            }
            .show(ui);
            st.popup = list.layer;
            st.active_id = list.active_id;
            if let Some(i) = list.clicked.filter(|i| *i < items.len()) {
                chosen = items.get(i).cloned();
                ctx.memory_mut(|m| m.request_focus(edit_id));
            } else if list.closed {
                st.combo.close();
            }
        } else {
            st.popup = None;
        }

        let (selected, created) = commit(chosen.as_ref(), enter_free, text, options, &mut st.combo);
        st.store(ctx, state_id);
        let changed = out.changed() || selected.is_some();
        let response = out.into_response();
        WidgetResponse::new(response, ComboboxOutput { selected, created }).with_changed(changed)
    }
}

impl Combobox<'_> {
    fn editor_keys(
        &self,
        ctx: &egui::Context,
        combo: &mut ComboState,
        edit_id: egui::Id,
        len: usize,
    ) -> (Option<usize>, bool, bool) {
        if self.disabled || !ctx.memory(|m| m.has_focus(edit_id)) {
            return (None, false, false);
        }
        let enter = ctx.input(|i| i.key_pressed(egui::Key::Enter));
        let step = row_parts::drive_list(ctx, combo, ComboKind::Editable, len);
        // Enter without an active row commits free text.
        (
            step.commit,
            step.moved,
            self.free_text && step.commit.is_none() && enter,
        )
    }
}

fn status_row(rt: &Rt, loading: bool, empty: bool) -> Option<(String, Icon)> {
    if loading {
        Some((
            crate::tr(rt.i18n(), msg::COMBOBOX_LOADING),
            rt.icons().progress_activity,
        ))
    } else if empty {
        Some((crate::tr(rt.i18n(), msg::COMBOBOX_EMPTY), rt.icons().search_off))
    } else {
        None
    }
}

// Row `i` of the popup: the items, then the status row.
fn option_row<'a>(
    items: &'a [ComboItem],
    options: &'a [String],
    create_label: &'a str,
    status: Option<&'a (String, Icon)>,
    add: Icon,
    i: usize,
) -> OptionRow<'a> {
    match items.get(i) {
        Some(ComboItem::Option { index, hits, .. }) => OptionRow {
            label: options.get(*index).map_or("", String::as_str),
            hits,
            ..OptionRow::default()
        },
        Some(ComboItem::Create) => OptionRow {
            label: create_label,
            leading: Some(Leading::Icon(add)),
            ..OptionRow::default()
        },
        Some(_) => OptionRow::default(),
        None => status.map_or_default(|(s, icon)| OptionRow {
            label: s,
            disabled: true,
            leading: Some(Leading::Icon(*icon)),
            ..OptionRow::default()
        }),
    }
}

fn commit(
    chosen: Option<&ComboItem>,
    enter_free: bool,
    text: &mut String,
    options: &[String],
    combo: &mut ComboState,
) -> (Option<usize>, Option<String>) {
    match chosen {
        Some(&ComboItem::Option { index, .. }) => {
            if let Some(label) = options.get(index) {
                text.clone_from(label);
            }
            combo.close();
            (Some(index), None)
        }
        Some(ComboItem::Create) => {
            combo.close();
            (None, Some(text.trim().to_owned()))
        }
        None if enter_free && !text.trim().is_empty() => (None, Some(text.trim().to_owned())),
        _ => (None, None),
    }
}

fn items_for(text: &str, options: &[String], free_text: bool) -> Vec<ComboItem> {
    let labels = options.iter().map(String::as_str);
    if free_text {
        creatable_combo_items(text, labels)
    } else {
        combo_items(text, labels)
    }
}
