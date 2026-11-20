use std::collections::HashSet;

use egui::{Pos2, Rangef, Rect, Sense, vec2};
use egui_mineral::{
    combobox::{ComboItem, ComboKind, ComboList, combo_items},
    i18n::I18n,
    kit,
    layout::{Constraints, Measure, MinTarget},
    select::Selection,
    semantic::{AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{Availability, MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use super::field::Validation;
use super::listbox::{self, Flat, ListEntry, ListOption, View};
use super::textbox::{self, Chrome, Content, FieldOutput, Fill, InputAppearance};
use super::value::{self, ListState, Value, selected_text};
use crate::spec::ColorToken::{ColorNeutralForegroundDisabled, ColorNeutralStrokeAccessible};
use crate::{
    Fluent, FluentExt, Size,
    spec::{defaults::combobox as cb, tokens::geometry as g},
    widget::paint,
};

/// Fluent `Combobox`: an editable field opening a listbox that filters by the typed text.
/// [`Selection::multiple`] makes it a multiselect.
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Combobox<'a, K = String> {
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    entries: Vec<ListEntry<K>>,
    value: Value<'a, K>,
    text: &'a mut String,
    appearance: InputAppearance,
    size: Size,
    placeholder: Option<String>,
    before: Option<egui_mineral::icon::Icon>,
    freeform: bool,
    filter: bool,
    list_max: Option<f32>,
    invalid: bool,
    disabled: bool,
}

impl<'a, K: Ord + Clone + 'a> Combobox<'a, K> {
    /// `label` is the accessible name; `text` is the typed value. Agent key `<id_salt>-combobox`.
    pub fn new(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        entries: impl IntoIterator<Item = impl Into<ListEntry<K>>>,
        selection: &'a mut Selection<K>,
        text: &'a mut String,
    ) -> Self {
        Self {
            salt: id_salt.into(),
            key: egui_mineral::agent_key!("combobox"),
            label: label.into(),
            entries: listbox::entries(entries),
            value: Value::new(selection),
            text,
            appearance: InputAppearance::Outline,
            size: Size::Medium,
            placeholder: None,
            before: None,
            freeform: false,
            filter: true,
            list_max: None,
            invalid: false,
            disabled: false,
        }
    }
}

impl<K> Combobox<'_, K> {
    /// Default [`InputAppearance::Outline`].
    pub fn appearance(mut self, appearance: InputAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// Default [`Size::Medium`].
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Hint shown while empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// `contentBefore`: icon leading the text.
    pub fn icon_before(mut self, icon: impl Into<egui_mineral::icon::Icon>) -> Self {
        self.before = Some(icon.into());
        self
    }

    /// `freeform`: typed text that matches no option stays after focus leaves.
    pub fn freeform(mut self, freeform: bool) -> Self {
        self.freeform = freeform;
        self
    }

    /// Default on: options not matching the typed text are hidden.
    pub fn filter(mut self, filter: bool) -> Self {
        self.filter = filter;
        self
    }

    pub(crate) fn list_max(mut self, max: f32) -> Self {
        self.list_max = Some(kit::sanitize::at_least(max, 0.0.., "Combobox list_max"));
        self
    }

    /// Validation state, as [`FieldContext::validation`](super::FieldContext::validation) hands it over;
    /// [`Validation::Error`] is `aria-invalid`: red border until focused.
    pub fn validation(mut self, validation: Option<Validation>) -> Self {
        self.invalid = validation == Some(Validation::Error);
        self
    }

    crate::widget::setters! {
        /// Greys out the field; the list cannot open.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-combobox` agent key.
        agent_key(key)
    }

    fn chrome(&self) -> Chrome {
        Chrome {
            appearance: self.appearance,
            size: self.size,
            invalid: self.invalid,
            disabled: self.disabled,
        }
    }

    // Options matching the typed text, as the `combobox` model ranks them.
    fn visible(&self, filtering: bool) -> HashSet<usize> {
        let all = Flat::new(&self.entries, &|_, _| true);
        let labels: Vec<&str> = all.options.iter().map(|o| o.text.as_str()).collect();
        let query = if filtering { self.text.as_str() } else { "" };
        combo_items(query, labels)
            .into_iter()
            .filter_map(|item| match item {
                ComboItem::Option { index, .. } => all.positions.get(index).copied(),
                _ => None,
            })
            .collect()
    }
}

impl<K> Combobox<'_, K> {
    fn chevron(&self, ui: &mut MineralUi<'_, Fluent>, base: egui::Id, visual: Rect) -> egui::Response {
        let t = ui.tokens();
        let dir = ui.i18n().direction();
        let m = self.chrome().metrics();
        let inner = visual.shrink(g::STROKE_WIDTH_THIN);
        let icon_x = dir.start_x(
            inner,
            Rangef::new(inner.width() - m.root - m.icon, inner.width() - m.root),
        );
        let icon_rect =
            Rect::from_center_size(Pos2::new(icon_x + m.icon / 2.0, inner.center().y), vec2(m.icon, m.icon));
        let chevron = ui.interact(
            icon_rect.expand2(vec2(m.root / 2.0, 0.0)),
            base.with("chevron"),
            if self.disabled {
                Sense::hover()
            } else {
                Sense::click() - Sense::FOCUSABLE
            },
        );
        paint::icon(
            ui.painter(),
            icon_rect.center(),
            ui.icons().chevron_down,
            false,
            m.icon,
            t.color(if self.disabled {
                ColorNeutralForegroundDisabled
            } else {
                ColorNeutralStrokeAccessible
            }),
        );
        chevron
    }

    fn popup(
        &self,
        ui: &mut MineralUi<'_, Fluent>,
        trigger: &egui::Response,
        visual: Rect,
        flat: &Flat<'_, K>,
        st: &mut ListState,
        moved: bool,
    ) -> (Option<usize>, bool) {
        let key = self.key.id(&self.salt);
        let selected = |o: &ListOption<K>| self.value.is_selected(&o.value);
        let shown = listbox::show(
            ui,
            self.salt.child("popup"),
            trigger,
            visual,
            &key,
            &self.label,
            &View {
                flat,
                multiselect: self.value.multiselect(),
                selected: &selected,
                active: st.combo.active(),
                scroll_to_active: moved,
                max_height: self.list_max.unwrap_or(f32::INFINITY),
            },
        );
        st.popup = shown.layer;
        st.active_id = shown.active_id;
        (shown.clicked, shown.closed)
    }
}

pub(super) struct Click {
    pub edited: bool,
    pub chevron: bool,
    pub field: bool,
    pub focused: bool,
    pub pointer_down: bool,
}

// Opens, closes and refilters the list from pointer and edit events; `true` when the active row must scroll.
pub(super) fn toggle(st: &mut ListState, len: usize, selected: Option<usize>, c: &Click) -> bool {
    let mut moved = false;
    if c.edited {
        st.filtering = true;
        if !st.combo.is_open() {
            st.combo.open(None, len);
        }
        st.combo.set_active(None, len);
    }
    if c.chevron {
        if st.combo.is_open() {
            st.combo.close();
        } else {
            st.combo.open(selected, len);
            moved = true;
        }
    } else if c.field && !st.combo.is_open() {
        st.combo.open(selected, len);
        moved = true;
    }
    if st.combo.is_open() && !c.focused && !c.pointer_down {
        st.combo.close();
    }
    moved
}

struct Commit {
    chosen: Option<usize>,
    restore: bool,
    edited: bool,
}

fn commit<K>(
    value: &mut Value<'_, K>,
    text: &mut String,
    entries: &[ListEntry<K>],
    st: &mut ListState,
    flat: &Flat<'_, K>,
    c: &Commit,
) -> bool {
    let multiselect = value.multiselect();
    let mut changed = c.edited;
    if let Some(i) = c.chosen
        && let Some(o) = flat.value(i)
    {
        changed |= value.pick(&o.value);
        st.filtering = false;
        if multiselect {
            text.clear();
            st.combo.open(None, flat.len());
        } else {
            text.clone_from(&o.text);
            st.combo.close();
        }
    } else if c.restore {
        let restored = if multiselect {
            String::new()
        } else {
            selected_text(entries, &**value)
        };
        if *text != restored {
            *text = restored;
            changed = true;
        }
        st.filtering = false;
    }
    changed
}

impl<K> MineralWidget for Combobox<'_, K> {
    type Kit = Fluent;
    type State = ListState;
    type Output = FieldOutput;

    kit::widget_meta!(salt, enabled, frame_sense);

    fn min_target(&self, _: &MineralUi<'_, Self::Kit>) -> MinTarget {
        MinTarget::None
    }

    fn layout(&self, _: &MineralUi<'_, Fluent>, c: Constraints, _: &ListState) -> Measure {
        let width = if c.max.x.is_finite() { c.max.x } else { cb::MIN_WIDTH };
        c.constrain(vec2(width, self.chrome().metrics().height)).into()
    }

    fn update(
        &mut self,
        ui: &mut MineralUi<'_, Fluent>,
        frame: &WidgetFrame<'_>,
        st: &mut ListState,
    ) -> WidgetResponse<FieldOutput> {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let base = ui.make_persistent_id(self.salt.id_salt());
        let edit_id = base.with("edit");
        let focused = ctx.memory(|m| m.has_focus(edit_id));
        let visual = frame.zones.visual;
        let chrome = self.chrome();
        let m = chrome.metrics();

        let keep = if self.filter {
            Some(self.visible(st.filtering))
        } else {
            None
        };
        let keep_ref = keep.as_ref();
        let visible = |pos: usize, _: &ListOption<K>| keep_ref.is_none_or(|k| k.contains(&pos));
        let flat = Flat::new(&self.entries, &visible);
        let selected_index = flat.options.iter().position(|o| self.value.is_selected(&o.value));
        let enabled = |i: usize| flat.enabled(i);
        let list = ComboList {
            kind: ComboKind::Editable,
            selected: selected_index,
            len: flat.len(),
            enabled: &enabled,
        };
        let (mut chosen, mut moved) = (None, false);
        if focused && !self.disabled {
            (chosen, moved) = value::drive(ctx, &mut st.combo, list);
        }

        let reserve = m.root + m.icon + m.gap - m.combined;
        let shown = textbox::show(
            ui,
            self.text,
            &frame.response,
            &Fill {
                base,
                frame: visual,
                chrome,
                content: Content {
                    before: self.before,
                    after: None,
                },
                placeholder: self.placeholder.as_deref(),
                mask: false,
                rows: None,
                reserve_end: reserve,
                text_rect: None,
                metrics: None,
            },
        );
        let chevron = self.chevron(ui, base, visual);
        let edited = shown.response.changed();
        if !self.disabled {
            let click = Click {
                edited,
                chevron: chevron.clicked(),
                field: frame.response.clicked() || shown.response.clicked(),
                focused,
                pointer_down: value::pointer_busy(ctx),
            };
            if toggle(st, flat.len(), selected_index, &click) {
                moved = true;
            }
            if click.chevron {
                ctx.memory_mut(|m| m.request_focus(edit_id));
            }
        }
        st.combo.retain_active(flat.len());

        if st.combo.is_open() && !self.disabled {
            let (clicked, closed) = self.popup(ui, &frame.response, visual, &flat, st, moved);
            if let Some(i) = clicked {
                chosen = Some(i);
                ctx.memory_mut(|m| m.request_focus(edit_id));
            } else if closed {
                st.combo.close();
            }
        } else {
            st.popup = None;
            st.active_id = None;
        }

        let changed = commit(
            &mut self.value,
            self.text,
            &self.entries,
            st,
            &flat,
            &Commit {
                chosen,
                restore: shown.response.lost_focus() && !self.freeform && !value::pointer_busy(ctx),
                edited,
            },
        );

        let out = FieldOutput {
            submitted: textbox::submitted(&shown.response),
            blurred: shown.response.lost_focus(),
        };
        WidgetResponse::new(shown.response, out).with_changed(changed)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse<FieldOutput>, st: &ListState) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(self.key.id(&self.salt), Role::ComboBox, &self.label)
            .value(self.text.clone())
            .editable(Availability::disabled_if(self.disabled));
        n.state.expanded = Some(st.combo.is_open());
        n.state.invalid = self.invalid;
        n.active_descendant = st.active_id.filter(|_| st.combo.is_open());
        Some(n)
    }
}
