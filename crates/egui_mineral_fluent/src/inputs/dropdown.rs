use egui::{Pos2, Rangef, vec2};
use egui_mineral::{
    combobox::{ComboKind, ComboList},
    i18n::I18n,
    kit,
    layout::{Constraints, Measure, MinTarget},
    select::Selection,
    semantic::{AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{Availability, MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use super::listbox::{self, Flat, ListEntry, View};
use super::textbox::{self, Chrome, InputAppearance, Live};
use super::value::{self, ListState, Value};
use crate::ColorToken::{ColorNeutralForegroundDisabled, ColorNeutralStrokeAccessible};
use crate::{
    Fluent, FluentExt, Size,
    icons::ICON_CHEVRON_DOWN,
    spec::{
        defaults::{combobox as cb, select as sel},
        tokens::geometry as g,
    },
    widget::paint,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    Dropdown,
    Select,
}

/// Fluent `Dropdown`: a button-like field opening a listbox; [`Selection::multiple`] makes it a multiselect.
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct Dropdown<'a> {
    kind: Kind,
    salt: KeyedSalt,
    key: AgentKey,
    label: String,
    entries: Vec<ListEntry>,
    value: Value<'a>,
    appearance: InputAppearance,
    size: Size,
    placeholder: Option<String>,
    before: Option<crate::FluentIcon>,
    invalid: bool,
    disabled: bool,
}

impl<'a> Dropdown<'a> {
    /// `label` is the accessible name; `entries` are options and groups. Agent key `<id_salt>-dropdown`.
    pub fn new(
        id_salt: impl Into<KeyedSalt>,
        label: impl Into<String>,
        entries: impl IntoIterator<Item = impl Into<ListEntry>>,
        selection: &'a mut Selection<String>,
    ) -> Self {
        Self::with(
            Kind::Dropdown,
            id_salt.into(),
            label.into(),
            listbox::entries(entries),
            Value::Many(selection),
        )
    }

    pub(crate) fn with(kind: Kind, salt: KeyedSalt, label: String, entries: Vec<ListEntry>, value: Value<'a>) -> Self {
        Self {
            kind,
            salt,
            key: AgentKey::Kind(match kind {
                Kind::Dropdown => "dropdown",
                Kind::Select => "select",
            }),
            label,
            entries,
            value,
            appearance: InputAppearance::Outline,
            size: Size::Medium,
            placeholder: None,
            before: None,
            invalid: false,
            disabled: false,
        }
    }

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

    /// Hint shown while nothing is selected.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// `contentBefore`: icon leading the text.
    pub fn content_before(mut self, icon: crate::FluentIcon) -> Self {
        self.before = Some(icon);
        self
    }

    /// `aria-invalid`: red border until focused.
    pub fn invalid(mut self, invalid: bool) -> Self {
        self.invalid = invalid;
        self
    }

    crate::widget::setters! {
        /// Greys out the field; the list cannot open.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-dropdown` agent key.
        agent_key(key)
    }

    pub(crate) fn chrome(&self) -> Chrome {
        Chrome {
            appearance: self.appearance,
            size: self.size,
            invalid: self.invalid,
            disabled: self.disabled,
        }
    }
}

impl Dropdown<'_> {
    fn paint(
        &self,
        ui: &mut MineralUi<'_, Fluent>,
        r: &egui::Response,
        visual: egui::Rect,
        focused: bool,
        base: egui::Id,
    ) {
        let ctx = ui.rt().ctx().clone();
        let t = ui.tokens();
        let dir = ui.i18n().direction();
        let chrome = self.chrome();
        let m = chrome.metrics();
        let live = Live {
            hovered: r.contains_pointer(),
            pressed: r.contains_pointer() && ctx.input(|i| i.pointer.primary_down()),
            focused,
        };
        textbox::paint_frame(ui, base, visual, chrome, live, &t);
        let inner = visual.shrink(g::STROKE_WIDTH_THIN);
        let icon_color = t.color(if self.disabled {
            ColorNeutralForegroundDisabled
        } else {
            ColorNeutralStrokeAccessible
        });
        let icon_x = dir.start_x(
            inner,
            Rangef::new(inner.width() - m.root - m.icon, inner.width() - m.root),
        );
        paint::icon(
            ui.painter(),
            Pos2::new(icon_x + m.icon / 2.0, inner.center().y),
            ICON_CHEVRON_DOWN,
            false,
            m.icon,
            icon_color,
        );
        let text = value::selected_text(&self.entries, &self.value);
        let format = chrome.text_format(&ctx, &t);
        let (shown_text, color) = match (text.is_empty(), self.placeholder.as_deref()) {
            (true, Some(p)) => (p.to_owned(), chrome.placeholder_color(&t)),
            _ => (text, format.color),
        };
        let pads = textbox::pads(&m, self.before.is_some(), false);
        if let Some(icon) = self.before {
            let x = dir.start_x(inner, Rangef::new(m.root, m.root + m.icon));
            paint::icon(
                ui.painter(),
                Pos2::new(x + m.icon / 2.0, inner.center().y),
                icon,
                false,
                m.icon,
                chrome.icon_color(&t),
            );
        }
        let text_w = (inner.width() - pads.start - m.root - m.icon - m.gap).max(1.0);
        let mut job = egui::text::LayoutJob::single_section(shown_text, format);
        job.wrap = egui::text::TextWrapping::truncate_at_width(text_w);
        let galley = ui.painter().layout_job(job);
        let x = dir.start_x(inner, Rangef::new(pads.start, pads.start + galley.size().x));
        ui.painter()
            .galley(Pos2::new(x, inner.center().y - galley.size().y / 2.0), galley, color);
    }
}

impl MineralWidget for Dropdown<'_> {
    type Kit = Fluent;
    type State = ListState;
    type Output = ();

    kit::widget_meta!(salt, enabled);

    fn min_target(&self) -> MinTarget {
        MinTarget::None
    }

    fn layout(&self, _: &MineralUi<'_, Fluent>, c: Constraints, _: &ListState) -> Measure {
        let default = match self.kind {
            Kind::Dropdown => cb::MIN_WIDTH,
            Kind::Select => sel::DEFAULT_WIDTH,
        };
        let width = if c.max.x.is_finite() { c.max.x } else { default };
        c.constrain(vec2(width, self.chrome().metrics().height)).into()
    }

    fn update(
        &mut self,
        ui: &mut MineralUi<'_, Fluent>,
        frame: &WidgetFrame<'_>,
        st: &mut ListState,
    ) -> WidgetResponse<()> {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let r = &frame.response;
        let base = ui.make_persistent_id(self.salt.id_salt());
        let visual = frame.zones.visual;
        let focused = r.has_focus();
        let flat = Flat::new(&self.entries, &|_| true);
        let selected_index = flat.options.iter().position(|o| self.value.is_selected(&o.value));
        let enabled = |i: usize| flat.enabled(i);
        let list = ComboList {
            kind: ComboKind::SelectOnly,
            selected: selected_index,
            len: flat.len(),
            enabled: &enabled,
        };
        let multiselect = self.value.multiselect();
        let (mut chosen, mut moved) = (None, false);
        if focused && !self.disabled {
            (chosen, moved) = value::drive(ctx, &mut st.combo, list);
            let typed = value::typed_text(ctx);
            if !typed.is_empty()
                && flat.len() > 0
                && let Some(i) = value::typeahead(&flat, st.combo.active().or(selected_index), &typed)
            {
                if !st.combo.is_open() {
                    st.combo.open(Some(i), flat.len());
                }
                st.combo.set_active(Some(i), flat.len());
                moved = true;
            }
        }
        if r.clicked() && !self.disabled {
            if st.combo.is_open() {
                st.combo.close();
            } else {
                st.combo.open_selected(list);
                moved = true;
            }
        }
        if st.combo.is_open() && !focused && !value::pointer_busy(ctx) {
            st.combo.close();
        }
        st.combo.retain_active(flat.len());

        if st.combo.is_open() && !self.disabled {
            let key = self.key.id(&self.salt);
            let selected = |o: &listbox::ListOption| self.value.is_selected(&o.value);
            let shown = listbox::show(
                ui,
                self.salt.child("popup"),
                r,
                visual,
                &key,
                &self.label,
                &View {
                    flat: &flat,
                    multiselect,
                    selected: &selected,
                    active: st.combo.active(),
                    scroll_to_active: moved,
                    max_height: f32::INFINITY,
                },
            );
            st.popup = shown.layer;
            st.active_id = shown.active_id;
            if let Some(i) = shown.clicked {
                chosen = Some(i);
                ctx.memory_mut(|m| m.request_focus(r.id));
            } else if shown.closed {
                st.combo.close();
            }
        } else {
            st.popup = None;
            st.active_id = None;
        }

        let mut changed = false;
        if let Some(i) = chosen
            && let Some(o) = flat.value(i)
        {
            changed = self.value.pick(&o.value);
            if multiselect {
                st.combo.open(Some(i), flat.len());
            } else {
                st.combo.close();
            }
        }

        self.paint(ui, r, visual, focused, base);
        WidgetResponse::new(r.clone(), ()).with_changed(changed)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse<()>, st: &ListState) -> Option<SemanticNode> {
        let mut n = SemanticNode::new(self.key.id(&self.salt), Role::ComboBox, &self.label)
            .value(value::selected_text(&self.entries, &self.value))
            .clickable(Availability::disabled_if(self.disabled));
        n.state.expanded = Some(st.combo.is_open());
        n.state.invalid = self.invalid;
        n.active_descendant = st.active_id.filter(|_| st.combo.is_open());
        Some(n)
    }
}
