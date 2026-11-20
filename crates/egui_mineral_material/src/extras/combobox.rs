use crate::Material;
use crate::MaterialExt;
use crate::Rt;
use crate::msg;
use crate::text_inputs::text_field::FieldChrome;
use egui_mineral::kit::{choice::Choice, state::TempState};
use egui_mineral::select::ItemKey;
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
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct ComboboxOutput<K> {
    /// Value of the option picked this frame, also when already bound; `text` holds its label.
    pub selected: Option<K>,
    /// Typed text submitted as a new entry; only with [`Combobox::free_text`].
    pub created: Option<String>,
}

impl<K> Default for ComboboxOutput<K> {
    fn default() -> Self {
        Self {
            selected: None,
            created: None,
        }
    }
}

/// Text field with a filtered option list binding an option value; the typed text is bound too. Options are
/// supplied every pass; async sources pass the latest results plus [`Self::loading`].
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Combobox<'a, K> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    text: &'a mut String,
    options: Vec<String>,
    choice: Choice<'a, K>,
    free_text: bool,
    loading: bool,
    disabled: bool,
    chrome: FieldChrome,
}

impl<'a, K: ItemKey> Combobox<'a, K> {
    /// Binds the picked option's value to `selected`; `label` is the field label, options filter by `text`, the
    /// bound field text. Agent key `<id_salt>-combobox`, options `<id_salt>-combobox-<value>-option`.
    pub fn new(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        selected: &'a mut K,
        text: &'a mut String,
    ) -> Self {
        Self::with(id_salt.into(), label.into(), Choice::new(selected), text)
    }

    /// [`Self::new`] with no option picked while `selected` is `None`.
    pub fn optional(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        selected: &'a mut Option<K>,
        text: &'a mut String,
    ) -> Self {
        Self::with(id_salt.into(), label.into(), Choice::optional(selected), text)
    }

    fn with(salt: KeyedSalt, label: String, choice: Choice<'a, K>, text: &'a mut String) -> Self {
        Self {
            chrome: FieldChrome::default(),
            key: egui_mineral::agent_key!("combobox"),
            salt,
            label,
            text,
            options: Vec::new(),
            choice,
            free_text: false,
            loading: false,
            disabled: false,
        }
    }

    /// Appends one option carrying `value`; picking it writes `label` to the text.
    pub fn item(mut self, value: K, label: impl Into<String>) -> Self {
        self.choice.push(value);
        self.options.push(label.into());
        self
    }

    /// Appends `(value, label)` pairs.
    pub fn items(mut self, options: impl IntoIterator<Item = (K, impl Into<String>)>) -> Self {
        for (value, label) in options {
            self = self.item(value, label);
        }
        self
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

    fn combo_field(&mut self, rt: &crate::Rt, st: &ComboboxState) -> TextField<'_> {
        let open = st.combo.is_open();
        let arrow = if open {
            rt.icons().arrow_drop_up
        } else {
            rt.icons().arrow_drop_down
        };
        let mut field = TextField::new(self.salt.clone(), self.label.clone(), &mut *self.text)
            .with_chrome(std::mem::take(&mut self.chrome))
            .event_filter(text_field::editor_keys(false))
            .trailing_icon(arrow, crate::tr(rt.i18n(), msg::COMBOBOX_TOGGLE))
            .disabled(self.disabled)
            .with_agent_key(self.key.clone());
        field.combo = Some(if open {
            text_field::Combo::Expanded
        } else {
            text_field::Combo::Collapsed
        });
        field.active_descendant = st.active_id.filter(|_| open);
        field
    }

    fn rows(&self) -> Vec<ComboItem> {
        items_for(self.text, &self.options, self.free_text)
    }
}

impl<K: ItemKey> Composite for Combobox<'_, K> {
    type Kit = Material;
    type Output = ComboboxOutput<K>;

    egui_mineral::kit::widget_meta!(salt);

    fn compose(
        &mut self,
        ui: &mut MineralUi<'_, Material>,
        constraints: Constraints,
    ) -> WidgetResponse<ComboboxOutput<K>> {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let base = ui.make_persistent_id(self.salt.id_salt());
        let field_id = text_field::field_id(ui, &self.salt);
        let edit_id = field_id.with("edit");
        let state_id = base.with("combobox");
        let mut st = ComboboxState::load(ctx, state_id);
        // Rows matter to the keys only while the editor is focused or the list is open.
        let filtered_before = ctx.memory(|m| m.has_focus(edit_id)) || st.combo.is_open();
        let items_before = if filtered_before { self.rows() } else { Vec::new() };
        let (pick, moved, enter_free) = self.editor_keys(ctx, &mut st.combo, edit_id, items_before.len());

        let out = ui.mineral(self.combo_field(rt, &st).constrain(constraints));
        let Self {
            salt,
            key,
            label,
            text,
            options,
            choice,
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
            let part = |i: usize| row_part(items, choice, i);
            let key = key.id(salt);
            let list = Listbox {
                popup: salt.child("popup"),
                trigger: out.response(),
                anchor: field_rect,
                key: &key,
                label,
                len: items.len() + usize::from(status.is_some()),
                row: &row,
                part: &part,
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

        let (selected, picked, created) = commit(chosen.as_ref(), enter_free, text, (options, choice), &mut st.combo);
        st.store(ctx, state_id);
        let (changed, response) = (out.changed() || picked, out.into_response());
        WidgetResponse::new(response, ComboboxOutput { selected, created }).with_changed(changed)
    }
}

impl<K> Combobox<'_, K> {
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

fn row_part<K: ItemKey>(items: &[ComboItem], choice: &Choice<'_, K>, i: usize) -> KeyedSalt {
    match items.get(i) {
        Some(ComboItem::Option { index, .. }) => KeyedSalt::from(choice.value(*index)),
        Some(ComboItem::Create) => KeyedSalt::from("create"),
        _ => KeyedSalt::from("status"),
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

// (picked value, binding changed, created text)
fn commit<K: ItemKey>(
    chosen: Option<&ComboItem>,
    enter_free: bool,
    text: &mut String,
    (options, choice): (&[String], &mut Choice<'_, K>),
    combo: &mut ComboState,
) -> (Option<K>, bool, Option<String>) {
    match chosen {
        Some(&ComboItem::Option { index, .. }) => {
            if let Some(label) = options.get(index) {
                text.clone_from(label);
            }
            combo.close();
            let changed = choice.select(index);
            (choice.value(index).cloned(), changed, None)
        }
        Some(ComboItem::Create) => {
            combo.close();
            (None, false, Some(text.trim().to_owned()))
        }
        None if enter_free && !text.trim().is_empty() => (None, false, Some(text.trim().to_owned())),
        _ => (None, false, None),
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
