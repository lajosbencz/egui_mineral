use crate::MaterialUiExt as _;
use crate::adaptive::Density;
use crate::msg;
use std::sync::Arc;

use crate::Material;
use crate::MaterialExt;
use crate::Rt;
use egui::{Color32, CornerRadius, EventFilter, Frame, Id, Key, Pos2, Rangef, Rect, Sense, Ui, pos2, vec2};
use egui_mineral::i18n::I18n;
use egui_mineral::i18n::Sides;
use egui_mineral::kit::state::{Memo, TempState};
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::Composite;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    anchor::{Align, Anchor, Side},
    filter::Query,
    input::{Change, Debounce},
    kit,
    layout::{Constraints, Edge, Measure},
    overlay::{self, Dismiss, Popup},
    semantic::{self, Action, Role, SemanticNode, StableId},
    widget::{Availability, MineralWidget, SizeExt, WidgetFrame, WidgetResponse},
};

use crate::text_inputs::text_field;
use crate::widget::{
    common,
    row_parts::{self, OptionRow, RowMetrics},
};
use crate::{
    Icon,
    spec::{
        ColorRole, ShapeRole,
        defaults::search_bar as sb,
        tokens::{divider, search_bar, search_view},
    },
    tokens::Tokens,
};

const BAR_H: f32 = search_bar::CONTAINER_HEIGHT;

// Dense down to one input line with padding.
fn bar_h(ui: &MineralUi<'_, Material>) -> f32 {
    ui.density().dense_to(BAR_H, text_field::line_h() + 8.0)
}

// Compose SearchBarIconOffsetX
pub(crate) fn icon_center(rt: &Rt, density: Density, rect: Rect, edge: Edge) -> Pos2 {
    let d = sb::ICON_OFFSET_X + text_field::icon_slot(density) / 2.0;
    pos2(
        rt.i18n().direction().from_edge(edge).start_x(rect, Rangef::point(d)),
        rect.center().y,
    )
}

pub(crate) fn input_rect(rt: &Rt, density: Density, rect: Rect, trailing: bool) -> Rect {
    let Sides { left, right } = rt.i18n().direction().sides(
        text_field::content_inset(density, true),
        text_field::content_inset(density, trailing),
    );
    let (x0, x1) = (rect.left() + left, rect.right() - right);
    let h = text_field::line_h();
    Rect::from_min_max(
        pos2(x0, rect.center().y - h / 2.0),
        pos2(x1.max(x0), rect.center().y + h / 2.0),
    )
}

// Compose SearchBarDefaults.InputField
pub(crate) struct SearchInput<'a> {
    pub text: &'a mut String,
    pub id: Id,
    pub interactive: bool,
    /// Keeps focus on Escape so an open popup can consume it (egui surrenders focus otherwise).
    pub keep_escape: bool,
    pub placeholder: Option<&'a str>,
    pub input: Color32,
    pub hint: Color32,
}

impl SearchInput<'_> {
    pub(crate) fn show(self, ui: &mut MineralUi<'_, Material>, rect: Rect) -> egui::Response {
        let Self {
            text,
            id,
            interactive,
            keep_escape,
            placeholder,
            input,
            hint,
        } = self;
        let rt = ui.rt();
        let t = rt.tokens();
        if let (Some(ph), true) = (placeholder, text.is_empty()) {
            let g = text_field::line_galley(ui.painter(), ph, search_bar::SUPPORTING_TEXT_FONT, hint, rect.width());
            let x = rt.i18n().direction().start_x(rect, Rangef::new(0.0, 0.0 + g.size().x));
            ui.painter().galley(pos2(x, rect.top()), g, hint);
        }
        let colors = text_field::colors(
            text_field::TextFieldVariant::Filled,
            text_field::FieldState::default(),
            &t,
        );
        let format = search_bar::INPUT_TEXT_FONT.format(ui.ctx(), input);
        text_field::BareEdit::new(id, rect, format, &colors, &t).show(ui, text, |ui, edit| {
            let filter = EventFilter {
                escape: keep_escape,
                ..edit.get_event_filter()
            };
            ui.add(edit.interactive(interactive).event_filter(filter))
        })
    }
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct SearchBarState {
    rect: Option<Rect>,
}

impl TempState for SearchBarState {}

/// MD3 search bar (Compose `SearchBar`).
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct SearchBar<'a> {
    salt: KeyedSalt,
    value: &'a mut String,
    label: String,
    placeholder: Option<String>,
    back: bool,
    trailing: Option<(Icon, String)>,
    joined: Option<Side>,
    disabled: bool,
}

/// Events of [`SearchBar`], read from `WidgetResponse::output`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct SearchBarOutput {
    /// Back button clicked; only with [`SearchBar::back_button`].
    pub leading_clicked: bool,
    /// Trailing icon button clicked.
    pub trailing_clicked: bool,
    /// Enter pressed in the field.
    pub submitted: bool,
}

impl<'a> SearchBar<'a> {
    /// `label` is the accessible name of the field, `value` the bound query; agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, value: &'a mut String) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            value,
            label: label.into(),
            placeholder: None,
            back: false,
            trailing: None,
            joined: None,
            disabled: false,
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    /// Hint shown while the query is empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Back arrow button instead of the search icon.
    pub fn back_button(mut self, back_button: bool) -> Self {
        self.back = back_button;
        self
    }

    /// Trailing icon button; `label` is its accessible name.
    pub fn trailing_icon(mut self, icon: impl Into<Icon>, label: impl Into<String>) -> Self {
        self.trailing = Some((icon.into(), label.into()));
        self
    }

    /// Square bottom corners and an expanded state, for a bar attached to a list below.
    pub fn joined(mut self, joined: bool) -> Self {
        self.joined = joined.then_some(Side::Bottom);
        self
    }

    crate::widget::setters! {
        /// Greys out the bar; the query is not editable.
        disabled
    }

    fn button_node(&self, kind: &'static str, label: &str) -> SemanticNode {
        SemanticNode::button(
            StableId::of(&self.salt, kind),
            label,
            Availability::disabled_if(self.disabled),
        )
    }
}

impl MineralWidget for SearchBar<'_> {
    type Kit = Material;
    type State = SearchBarState;
    type Output = SearchBarOutput;

    egui_mineral::kit::widget_meta!(salt, enabled, no_target, frame_sense);

    fn layout(&self, ui: &MineralUi<'_, Material>, c: Constraints, _: &Self::State) -> Measure {
        c.constrain(vec2(sb::MIN_WIDTH, bar_h(ui))).into()
    }

    fn update(
        &mut self,
        ui: &mut MineralUi<'_, Material>,
        frame: &WidgetFrame<'_>,
        state: &mut SearchBarState,
    ) -> WidgetResponse<SearchBarOutput> {
        let rt = ui.rt();
        let t = ui.tokens();
        let ctx = ui.ctx().clone();
        let base = ui.make_persistent_id(self.salt.id_salt());
        let edit_id = edit_id(base);
        let rect = frame.zones.visual;
        state.rect = Some(rect);
        let c = |role: ColorRole| {
            if self.disabled {
                common::disabled_content(&t)
            } else {
                t.role(role)
            }
        };

        let round = |role: ShapeRole| CornerRadius::from(common::radius(rect, role.scale().radius(&t.shape)).nw);
        let docked = round(search_view::DOCKED_CONTAINER_SHAPE);
        let r = match self.joined {
            Some(Side::Bottom) => CornerRadius { sw: 0, se: 0, ..docked },
            Some(Side::Top) => CornerRadius { nw: 0, ne: 0, ..docked },
            _ => round(search_bar::CONTAINER_SHAPE),
        };
        ui.painter().rect_filled(rect, r, t.role(search_bar::CONTAINER_COLOR));

        let mut out = SearchBarOutput::default();
        let lead = icon_center(rt, ui.density(), rect, Edge::Start);
        if self.back {
            let node = self.button_node("back-button", &crate::tr(ui.i18n(), msg::BACK));
            out.leading_clicked = slot_icon(
                ui,
                lead,
                base.with("leading"),
                rt.i18n()
                    .direction()
                    .pick(ui.icons().arrow_back, ui.icons().arrow_forward),
                c(search_bar::LEADING_ICON_COLOR),
                node,
            );
        } else {
            let (_, size) = crate::widget::parts::small_icon_metrics();
            common::icon(
                ui.painter(),
                lead,
                ui.icons().search,
                size,
                c(search_bar::LEADING_ICON_COLOR),
            );
        }
        let text_rect = input_rect(rt, ui.density(), rect, self.trailing.is_some());
        let te = SearchInput {
            text: &mut *self.value,
            id: edit_id,
            interactive: !self.disabled,
            keep_escape: self.joined.is_some(),
            placeholder: self.placeholder.as_deref(),
            input: c(search_bar::INPUT_TEXT_COLOR),
            hint: c(search_bar::SUPPORTING_TEXT_COLOR),
        }
        .show(ui, text_rect);
        // After the editor: Tab order is registration order.
        if let Some((icon, label)) = self.trailing.clone() {
            let node = self.button_node("trailing-button", &label);
            out.trailing_clicked = slot_icon(
                ui,
                icon_center(rt, ui.density(), rect, Edge::End),
                base.with("trailing"),
                icon,
                c(search_bar::TRAILING_ICON_COLOR),
                node,
            );
        }
        if !te.has_focus() && frame.response.clicked() {
            ctx.memory_mut(|m| m.request_focus(edit_id));
        }
        out.submitted = text_field::submitted(&te);
        let changed = te.changed();
        WidgetResponse::new(te, out).with_changed(changed)
    }

    fn semantics(&self, _: &I18n, _: &WidgetResponse<SearchBarOutput>, _s: &SearchBarState) -> Option<SemanticNode> {
        let mut node = SemanticNode::new(StableId::of(&self.salt, "search"), Role::SearchInput, &self.label)
            .value(self.value.clone())
            .editable(Availability::disabled_if(self.disabled));
        node.state.expanded = Some(self.joined.is_some());
        if !self.disabled {
            node.actions.push(Action::ReplaceSelectedText);
        }
        Some(node)
    }
}

type DebounceCallback<'a> = Box<dyn FnMut(&str) + 'a>;

#[derive(Clone, Default)]
struct SuggestState {
    active: Option<usize>,
    suppress: bool,
    open: bool,
    popup: Option<Placed>,
    shown: usize,
    ranked: Memo<Ranked>,
}

impl TempState for SuggestState {}

#[derive(Clone, Copy)]
struct Placed {
    rect: Rect,
    side: Side,
}

const SUGGESTIONS_SIDE: Side = Side::Bottom;

/// Events of [`SearchSuggestions`], read from `WidgetResponse::output`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct SearchSuggestionsOutput<K> {
    /// Value of the suggestion picked this frame; the query holds its text.
    pub selected: Option<K>,
}

impl<K> Default for SearchSuggestionsOutput<K> {
    fn default() -> Self {
        Self { selected: None }
    }
}

/// [`SearchBar`] with a docked suggestion list.
#[must_use = "show with ui.mineral(..)"]
pub struct SearchSuggestions<'a, K> {
    salt: KeyedSalt,
    query: &'a mut String,
    suggestions: Vec<String>,
    values: Vec<K>,
    label: String,
    placeholder: Option<String>,
    back: bool,
    trailing: Option<(Icon, String)>,
    max_items: usize,
    debounce: Option<(Debounce, DebounceCallback<'a>)>,
    disabled: bool,
}

egui_mineral::kit::opaque_debug!(SearchSuggestions<'_, K>);

impl<'a, K: Clone> SearchSuggestions<'a, K> {
    /// `label` is the accessible name of the field; suggestions filter by `query`, the bound text. Agent key
    /// prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, query: &'a mut String) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            query,
            suggestions: Vec::new(),
            values: Vec::new(),
            label: label.into(),
            placeholder: None,
            back: false,
            trailing: None,
            max_items: 6,
            debounce: None,
            disabled: false,
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    /// Appends one suggestion carrying `value`; picking it writes `text` to the query.
    pub fn item(mut self, value: K, text: impl Into<String>) -> Self {
        self.values.push(value);
        self.suggestions.push(text.into());
        self
    }

    /// Appends `(value, text)` pairs.
    pub fn items(mut self, suggestions: impl IntoIterator<Item = (K, impl Into<String>)>) -> Self {
        for (value, text) in suggestions {
            self = self.item(value, text);
        }
        self
    }

    crate::widget::setters! {
        /// Greys out the field and keeps the suggestions closed.
        disabled
    }

    /// Hint shown while the query is empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Back arrow button instead of the search icon.
    pub fn back_button(mut self, back_button: bool) -> Self {
        self.back = back_button;
        self
    }

    /// Trailing icon button; `label` is its accessible name.
    pub fn trailing_icon(mut self, icon: impl Into<Icon>, label: impl Into<String>) -> Self {
        self.trailing = Some((icon.into(), label.into()));
        self
    }

    /// Visible suggestion rows; default 6, minimum 1.
    #[track_caller]
    pub fn max_items(mut self, max_items: usize) -> Self {
        self.max_items = egui_mineral::kit::sanitize::at_least(max_items, 1.., "SearchBar max_items");
        self
    }

    /// Calls `on_settled` with the query once it stops changing for `debounce`.
    pub fn on_debounce(mut self, debounce: Debounce, on_settled: impl FnMut(&str) + 'a) -> Self {
        self.debounce = Some((debounce, Box::new(on_settled)));
        self
    }
}

impl<K: Clone> Composite for SearchSuggestions<'_, K> {
    type Kit = Material;
    type Output = SearchSuggestionsOutput<K>;

    egui_mineral::kit::widget_meta!(salt);

    fn compose(
        &mut self,
        ui: &mut MineralUi<'_, Material>,
        c: Constraints,
    ) -> WidgetResponse<SearchSuggestionsOutput<K>> {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let mut bar = SearchBar::new(self.salt.clone(), self.label.clone(), &mut *self.query);
        bar.placeholder.clone_from(&self.placeholder);
        bar.back = self.back;
        bar.disabled = self.disabled;
        bar.trailing.clone_from(&self.trailing);
        let base = ui.make_persistent_id(bar.id_salt());
        let edit_id = edit_id(base);
        let state_id = base.with("suggest");
        let mut st = SuggestState::load(ctx, state_id);

        // Up/Down/Escape act before the text edit sees them.
        let focused_before = ctx.memory(|m| m.has_focus(edit_id));
        let shown_before = if st.open { st.shown } else { 0 };
        if st.open && shown_before > 0 && kit::overlay::consume_escape_from(ctx, edit_id) {
            st.suppress = true;
        }
        let mut picked = None;
        if focused_before {
            if st.open && shown_before > 0 {
                picked = row_parts::drive_embedded(ctx, &mut st.active, shown_before).commit;
            } else if st.suppress && ctx.input(|i| i.key_pressed(Key::ArrowDown)) {
                st.suppress = false;
            }
        }

        bar.joined = st.open.then(|| st.popup.map_or(SUGGESTIONS_SIDE, |p| p.side));
        let out = ui.mineral(bar.constrain(c));
        let focused = ctx.memory(|m| m.has_focus(edit_id));
        let changed = out.changed();
        if changed {
            st.active = None;
            st.suppress = false;
        }
        if focused && !focused_before {
            st.suppress = false;
        }
        if let Some((d, cb)) = &mut self.debounce
            && d.update(
                ctx,
                base.with("debounce"),
                if out.changed() {
                    Change::Changed
                } else {
                    Change::Unchanged
                },
            )
        {
            cb(self.query);
        }

        let pointer_in_popup = st.open
            && st
                .popup
                .map(|p| p.rect)
                .zip(ctx.pointer_hover_pos())
                .is_some_and(|(r, p)| r.contains(p));
        let can_open = !self.disabled && !st.suppress && (focused || pointer_in_popup);
        let ranked = if can_open || picked.is_some() {
            top_matches(&mut st.ranked, self.query, &self.suggestions)
        } else {
            Arc::default()
        };
        let matches = &ranked[..ranked.len().min(self.max_items)];
        let open = can_open && !matches.is_empty();
        if matches.is_empty() || st.active.is_some_and(|a| a >= matches.len()) {
            st.active = None;
        }

        let mut chosen = picked.and_then(|a| matches.get(a)).map(|m| m.0);
        let anchor = SearchBarState::try_load(ctx, base).and_then(|s| s.rect);
        st.open = open && anchor.is_some();
        if let Some(rect) = anchor.filter(|_| st.open) {
            if let Some(i) = self.show_popup(rt, out.response(), base, rect, matches, &mut st) {
                chosen = Some(i);
            }
        } else {
            st.popup = None;
        }
        st.shown = if st.open { matches.len() } else { 0 };

        let mut result = None;
        if let Some(i) = chosen {
            self.query.clone_from(&self.suggestions[i]);
            st.suppress = true;
            st.active = None;
            st.open = false;
            ctx.memory_mut(|m| m.request_focus(edit_id));
            result = self.values.get(i).cloned();
        }
        st.store(ctx, state_id);
        let changed = changed || result.is_some();
        let response = out.into_response();
        WidgetResponse::new(response, SearchSuggestionsOutput { selected: result }).with_changed(changed)
    }
}

impl<K> SearchSuggestions<'_, K> {
    fn show_popup(
        &self,
        rt: &Rt,
        anchor: &egui::Response,
        base: Id,
        rect: Rect,
        matches: &[(usize, Vec<usize>)],
        st: &mut SuggestState,
    ) -> Option<usize> {
        let edit_id = edit_id(base);
        let t = rt.tokens();
        let dir = rt.i18n().direction();
        let mut chosen = None;
        let shown = Popup::new(self.salt.child("view"), Anchor::new(SUGGESTIONS_SIDE, Align::Start))
            .anchor_rect(rect)
            .return_focus(edit_id)
            .show(rt, anchor, &mut st.open, |ui, placement| {
                // The bar and this list form one docked search view.
                let docked = search_view::DOCKED_CONTAINER_SHAPE.corner_radius(&t.shape, dir);
                let below = placement.side != Side::Top;
                let r = if below {
                    CornerRadius { nw: 0, ne: 0, ..docked }
                } else {
                    CornerRadius { sw: 0, se: 0, ..docked }
                };
                let framed = Frame::new()
                    .fill(t.role(search_view::CONTAINER_COLOR))
                    .corner_radius(r)
                    .show(ui, |ui| {
                        ui.set_width(rect.width());
                        ui.spacing_mut().item_spacing.y = 0.0;
                        if below {
                            view_divider(ui, rect.width(), &t);
                        }
                        let cx = Suggestions {
                            base,
                            salt: &self.salt,
                            width: rect.width(),
                            t: &t,
                        };
                        for (row, (idx, hits)) in matches.iter().enumerate() {
                            if let Some(i) = suggestion_row(
                                &mut rt.ui(ui),
                                &cx,
                                *idx,
                                &self.suggestions[*idx],
                                hits,
                                st.active == Some(row),
                            ) {
                                chosen = Some(i);
                            }
                        }
                        if !below {
                            view_divider(ui, rect.width(), &t);
                        }
                    });
                semantic::emit(ui.ctx(), None, framed.response.rect, || {
                    SemanticNode::new(
                        StableId::of(&self.salt, "suggestions"),
                        Role::ListBox,
                        crate::tr(ui.i18n(), msg::SUGGESTIONS),
                    )
                });
                Placed {
                    rect: framed.response.rect,
                    side: placement.side,
                }
            });
        st.popup = shown.inner;
        // Stays closed until the query changes, the field refocuses or Down reopens it.
        st.suppress |= shown.dismissed.is_some();
        chosen
    }
}

// Compose HorizontalDivider
pub(crate) fn view_divider(ui: &mut Ui, width: f32, t: &Tokens) {
    let (line, _) = ui.allocate_exact_size(vec2(width, divider::THICKNESS), Sense::hover());
    ui.painter().rect_filled(line, 0, t.role(search_view::DIVIDER_COLOR));
}

struct Suggestions<'a> {
    base: Id,
    salt: &'a KeyedSalt,
    width: f32,
    t: &'a Tokens,
}

fn suggestion_row(
    ui: &mut MineralUi<'_, Material>,
    cx: &Suggestions<'_>,
    idx: usize,
    text: &str,
    hits: &[usize],
    active: bool,
) -> Option<usize> {
    let Suggestions { base, salt, width, t } = *cx;
    let row = OptionRow {
        label: text,
        hits,
        leading: Some(ui.icons().search.into()),
        active,
        ..OptionRow::default()
    };
    let resp = row.show(ui, base.with(("item", idx)), width, &RowMetrics::list(ui.density()), t);
    semantic::emit(ui.ctx(), Some(resp.id), resp.rect, || {
        row.pick_node(StableId::of(salt, (idx, "suggestion")))
    });
    resp.clicked().then_some(idx)
}

/// Presentation of a [`SearchView`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SearchViewMode {
    #[default]
    /// Dropdown below the bar (`DockedSearchBar`).
    Docked,
    /// Covers the window (`ExpandedFullScreenSearchBar`).
    FullScreen,
}

/// Result of [`SearchView::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SearchViewResponse<K, R> {
    /// Collapsed bar response.
    pub response: egui::Response,
    /// Open view surface; `None` while closed.
    pub popup: Option<egui::Response>,
    /// Output of the content closure; `None` while closed.
    pub inner: Option<R>,
    /// Query edit or suggestion pick.
    pub changed: bool,
    /// Value of the suggestion picked this frame; the query holds its text.
    pub selected: Option<K>,
    /// Enter pressed in the field.
    pub submitted: bool,
    /// `None` when a suggestion pick closed the view.
    pub dismissed: Option<Dismiss>,
}

#[derive(Clone, Default)]
struct ViewState {
    active: Option<usize>,
    focus: bool,
    ranked: Memo<Ranked>,
}

impl TempState for ViewState {}

/// MD3 search view, docked or full-screen (Compose `ExpandedDockedSearchBar`, `ExpandedFullScreenSearchBar`).
#[derive(Debug)]
#[must_use = "call .show(..)"]
pub struct SearchView<'a, K> {
    salt: KeyedSalt,
    query: &'a mut String,
    suggestions: Vec<String>,
    values: Vec<K>,
    label: String,
    placeholder: Option<String>,
    mode: SearchViewMode,
    width: Rangef,
    max_items: usize,
    disabled: bool,
}

impl<'a> SearchView<'a, ()> {
    /// `label` is the accessible name of the field, `query` the bound text; agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, query: &'a mut String) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            query,
            suggestions: Vec::new(),
            values: Vec::new(),
            label: label.into(),
            placeholder: None,
            mode: SearchViewMode::default(),
            width: Rangef::point(sb::MIN_WIDTH),
            max_items: 8,
            disabled: false,
        }
    }
}

impl<'a, K: Clone> SearchView<'a, K> {
    crate::widget::setters! {
        /// Keeps the view closed.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    /// Rows shown below the field as `(value, text)` pairs, filtered by the query; replaces earlier ones. A pick
    /// writes `text` to the query and reports `value`.
    pub fn suggestions<V>(self, suggestions: impl IntoIterator<Item = (V, impl Into<String>)>) -> SearchView<'a, V> {
        let (values, suggestions) = suggestions.into_iter().map(|(v, t)| (v, t.into())).unzip();
        SearchView {
            salt: self.salt,
            query: self.query,
            suggestions,
            values,
            label: self.label,
            placeholder: self.placeholder,
            mode: self.mode,
            width: self.width,
            max_items: self.max_items,
            disabled: self.disabled,
        }
    }

    /// Hint shown while the query is empty.
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Default [`SearchViewMode::Docked`].
    pub fn mode(mut self, mode: SearchViewMode) -> Self {
        self.mode = mode;
        self
    }

    /// Docked width bounds in dp: the view takes the available width within them; equal bounds fix the width.
    /// Default the minimum; limited to `search_bar::MIN_WIDTH..=MAX_WIDTH` (`spec::defaults`). A reversed range is
    /// swapped.
    #[track_caller]
    pub fn width(mut self, width: impl Into<Rangef>) -> Self {
        let w = width.into();
        let limit = |v: f32| egui_mineral::kit::sanitize::within(v, sb::MIN_WIDTH..=sb::MAX_WIDTH, "width");
        let (min, max) = (limit(w.min.min(w.max)), limit(w.max.max(w.min)));
        self.width = Rangef::new(min, max.max(min));
        self
    }

    /// Visible suggestion rows; default 8, minimum 1.
    #[track_caller]
    pub fn max_items(mut self, max_items: usize) -> Self {
        self.max_items = egui_mineral::kit::sanitize::at_least(max_items, 1.., "SearchView max_items");
        self
    }

    /// `add` renders the view body and receives the live query.
    pub fn show<R>(
        mut self,
        ui: &mut MineralUi<'_, Material>,
        open: &mut bool,
        add: impl FnOnce(&mut MineralUi<'_, Material>, &str) -> R,
    ) -> SearchViewResponse<K, R> {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let t = rt.tokens();
        let base = ui.make_persistent_id(self.salt.id_salt());
        let state_id = base.with("view");
        let edit_id = base.with("view-edit");
        let mut st = ViewState::load(ctx, state_id);
        if self.disabled {
            *open = false;
        }
        let width = self.width.clamp(ui.available_width()).min(ui.available_width());
        let placeholder = crate::tr_or(rt.i18n(), self.placeholder.as_deref(), msg::SEARCH);
        let name = std::mem::take(&mut self.label);

        // Collapsed bar; activating it opens the view.
        let bar_pos = ui.next_widget_position();
        let bar = SearchBar::new(
            self.salt.child("bar").with_key(self.salt.key().to_owned()),
            name.clone(),
            &mut *self.query,
        )
        .placeholder(placeholder.clone())
        .disabled(self.disabled);
        let bar_out = ui.mineral(bar.preferred_width(width));
        let bar_rect = Rect::from_min_size(bar_pos, vec2(width, bar_h(ui)));
        if !*open && (bar_out.has_focus() || bar_out.clicked()) {
            *open = true;
            st.focus = true;
            st.active = None;
        }
        if !*open {
            st.store(ctx, state_id);
            return SearchViewResponse {
                response: bar_out.into_response(),
                popup: None,
                inner: None,
                changed: false,
                selected: None,
                submitted: false,
                dismissed: None,
            };
        }

        let geo = self.view_geometry(ctx, bar_rect, width);
        let mut ev = ViewEvents::default();
        let ranked = self.matches_and_keys(ctx, edit_id, &mut st, &mut ev);
        let matches = &ranked[..ranked.len().min(self.max_items)];

        let full = geo.full;
        let r = if full {
            search_view::FULL_SCREEN_CONTAINER_SHAPE
        } else {
            search_view::DOCKED_CONTAINER_SHAPE
        }
        .corner_radius(&t.shape, rt.i18n().direction());
        let panel_pos = geo.panel_pos;
        let vc = ViewCtx {
            base,
            edit_id,
            geo,
            placeholder: &placeholder,
            name: &name,
            matches,
        };
        let mut inner = None;
        let area = egui_mineral::kit::overlay::floating(rt.core(), base.with("view-area"), panel_pos).show(ctx, |ui| {
            let framed = Frame::new()
                .fill(t.role(search_view::CONTAINER_COLOR))
                .corner_radius(r)
                .show(ui, |ui| {
                    inner = Some(self.view_body(&mut rt.ui(ui), &vc, &mut st, &mut ev, add));
                });
            framed.response.rect
        });
        egui_mineral::kit::overlay::keep_above(ctx, area.response.layer_id, ui.layer_id());
        let view = area.response.clone();
        // After the opening press is released: egui drops focus on a click that ends off the pressed widget.
        if st.focus && !ctx.input(|i| i.pointer.any_down()) {
            ctx.memory_mut(|m| m.request_focus(edit_id));
            st.focus = false;
        }
        ev.dismissed = ev.dismissed.or_else(|| view_dismissal(rt, &view, area.inner, full));
        self.apply_events(ctx, edit_id, &mut ev, &mut st, open);
        st.store(ctx, state_id);
        let mut response = bar_out.into_response();
        let changed = ev.typed || ev.selected.is_some();
        if changed {
            response.mark_changed();
        }
        SearchViewResponse {
            response,
            popup: Some(view),
            inner,
            changed,
            selected: ev.selected.and_then(|i| self.values.get(i).cloned()),
            submitted: ev.submitted,
            dismissed: ev.dismissed,
        }
    }

    fn apply_events(
        &mut self,
        ctx: &egui::Context,
        edit_id: Id,
        ev: &mut ViewEvents,
        st: &mut ViewState,
        open: &mut bool,
    ) {
        if let Some(i) = ev.selected {
            self.query.clone_from(&self.suggestions[i]);
            ev.dismissed = None;
        }
        if ev.dismissed.is_some() || ev.selected.is_some() {
            *open = false;
            st.active = None;
            ctx.memory_mut(|m| m.surrender_focus(edit_id));
        }
    }

    fn matches_and_keys(
        &self,
        ctx: &egui::Context,
        edit_id: Id,
        st: &mut ViewState,
        ev: &mut ViewEvents,
    ) -> Arc<Ranked> {
        let ranked = top_matches(&mut st.ranked, self.query, &self.suggestions);
        let matches = &ranked[..ranked.len().min(self.max_items)];
        if st.active.is_some_and(|a| a >= matches.len()) {
            st.active = None;
        }
        view_keys(ctx, edit_id, matches, st, ev);
        ranked
    }

    fn view_geometry(&self, ctx: &egui::Context, bar_rect: Rect, width: f32) -> ViewGeometry {
        let full = self.mode == SearchViewMode::FullScreen;
        let screen = ctx.content_rect();
        let (panel_pos, panel_w, header_h) = if full {
            (
                screen.min,
                screen.width(),
                search_view::FULL_SCREEN_HEADER_CONTAINER_HEIGHT,
            )
        } else {
            (bar_rect.min, width, search_view::DOCKED_HEADER_CONTAINER_HEIGHT)
        };
        // Docked: the list below the header takes `DockedExpandedTableMinHeight` up to a share of the window.
        let (min_h, max_h) = if full {
            (screen.height(), screen.height())
        } else {
            let max = screen.height() * sb::DOCKED_MAX_HEIGHT_RATIO;
            (header_h + sb::DOCKED_MIN_HEIGHT.min(max), header_h + max)
        };
        ViewGeometry {
            full,
            panel_pos,
            panel_w,
            header_h,
            min_h,
            max_h,
        }
    }

    fn view_body<R>(
        &mut self,
        ui: &mut MineralUi<'_, Material>,
        vc: &ViewCtx<'_>,
        st: &mut ViewState,
        ev: &mut ViewEvents,
        add: impl FnOnce(&mut MineralUi<'_, Material>, &str) -> R,
    ) -> R {
        let rt = ui.rt();
        let geo = &vc.geo;
        ui.set_width(geo.panel_w);
        ui.set_min_height(geo.min_h);
        ui.set_max_height(geo.max_h);
        ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
        let (head, _) = ui.allocate_exact_size(vec2(geo.panel_w, geo.header_h), Sense::hover());
        self.view_header(ui, head, vc, st, ev);
        view_divider(ui, geo.panel_w, &rt.tokens());
        egui::ScrollArea::vertical()
            .id_salt(vc.base.with("view-scroll"))
            .auto_shrink([false, !geo.full])
            .show(ui, |ui| {
                self.view_list(&mut rt.ui(ui), vc, st.active, &mut ev.selected, add)
            })
            .inner
    }

    fn view_header(
        &mut self,
        ui: &mut MineralUi<'_, Material>,
        head: Rect,
        vc: &ViewCtx<'_>,
        st: &mut ViewState,
        ev: &mut ViewEvents,
    ) {
        let rt = ui.rt();
        let t = rt.tokens();
        let back_label = crate::tr(rt.i18n(), msg::BACK);
        let back = slot_icon(
            ui,
            icon_center(ui.rt(), ui.density(), head, Edge::Start),
            vc.base.with("view-back"),
            rt.i18n()
                .direction()
                .pick(ui.icons().arrow_back, ui.icons().arrow_forward),
            t.role(search_view::HEADER_LEADING_ICON_COLOR),
            enabled_button(&self.salt, "back-button", &back_label),
        );
        if back {
            ev.dismissed.get_or_insert(Dismiss::Close);
        }
        let clear = !self.query.is_empty();
        if clear {
            let clear_label = crate::tr(rt.i18n(), msg::SEARCH_CLEAR);
            if slot_icon(
                ui,
                icon_center(ui.rt(), ui.density(), head, Edge::End),
                vc.base.with("view-clear"),
                ui.icons().close,
                t.role(search_view::HEADER_TRAILING_ICON_COLOR),
                enabled_button(&self.salt, "clear-button", &clear_label),
            ) {
                self.query.clear();
                st.active = None;
                st.focus = true;
            }
        }
        let te = SearchInput {
            text: &mut *self.query,
            id: vc.edit_id,
            interactive: true,
            keep_escape: false,
            placeholder: Some(vc.placeholder),
            input: t.role(search_view::HEADER_INPUT_TEXT_COLOR),
            hint: t.role(search_view::HEADER_SUPPORTING_TEXT_COLOR),
        }
        .show(ui, input_rect(ui.rt(), ui.density(), head, clear));
        if te.changed() {
            st.active = None;
            ev.typed = true;
        }
        semantic::emit(ui.ctx(), Some(vc.edit_id), head, || {
            SemanticNode::new(StableId::of(&self.salt, "search"), Role::SearchInput, vc.name)
                .value(self.query.clone())
                .editable(Availability::Enabled)
        });
    }

    fn view_list<R>(
        &self,
        ui: &mut MineralUi<'_, Material>,
        vc: &ViewCtx<'_>,
        active: Option<usize>,
        selected: &mut Option<usize>,
        add: impl FnOnce(&mut MineralUi<'_, Material>, &str) -> R,
    ) -> R {
        let rt = ui.rt();
        let t = rt.tokens();
        let panel_w = vc.geo.panel_w;
        ui.set_width(panel_w);
        let list_top = ui.cursor().min;
        let cx = Suggestions {
            base: vc.base,
            salt: &self.salt,
            width: panel_w,
            t: &t,
        };
        for (row, (idx, hits)) in vc.matches.iter().enumerate() {
            if let Some(i) = suggestion_row(ui, &cx, *idx, &self.suggestions[*idx], hits, active == Some(row)) {
                *selected = Some(i);
            }
        }
        if !vc.matches.is_empty() {
            semantic::emit(
                ui.ctx(),
                None,
                Rect::from_min_max(list_top, pos2(list_top.x + panel_w, ui.cursor().min.y)),
                || {
                    SemanticNode::new(
                        StableId::of(&self.salt, "suggestions"),
                        Role::ListBox,
                        crate::tr(rt.i18n(), msg::SUGGESTIONS),
                    )
                },
            );
        }
        add(ui, self.query.as_str())
    }
}

struct ViewGeometry {
    full: bool,
    panel_pos: Pos2,
    panel_w: f32,
    header_h: f32,
    min_h: f32,
    max_h: f32,
}

struct ViewCtx<'a> {
    base: Id,
    edit_id: Id,
    geo: ViewGeometry,
    placeholder: &'a str,
    name: &'a str,
    matches: &'a [(usize, Vec<usize>)],
}

#[derive(Default)]
struct ViewEvents {
    selected: Option<usize>,
    submitted: bool,
    typed: bool,
    dismissed: Option<Dismiss>,
}

// Focus is dropped since a focused bar reopens the view
fn view_dismissal(rt: &Rt, view: &egui::Response, panel: Rect, full: bool) -> Option<Dismiss> {
    let inside = if full { rt.ctx().content_rect() } else { panel };
    overlay::dismissal(rt.core(), view.layer_id, &[inside], None)
}

fn view_keys(
    ctx: &egui::Context,
    edit_id: Id,
    matches: &[(usize, Vec<usize>)],
    st: &mut ViewState,
    ev: &mut ViewEvents,
) {
    if ctx.memory(|m| m.has_focus(edit_id)) {
        if let Some(a) = row_parts::drive_embedded(ctx, &mut st.active, matches.len()).commit {
            ev.selected = matches.get(a).map(|m| m.0);
        } else if ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Enter)) {
            ev.submitted = true;
        }
    }
}

// Suggestion index and matched char indices, best first.
type Ranked = Vec<(usize, Vec<usize>)>;

fn top_matches(memo: &mut Memo<Ranked>, query: &str, suggestions: &[String]) -> Arc<Ranked> {
    memo.get_or_compute(&(query, suggestions), || {
        Query::new(query)
            .rank(suggestions.iter().enumerate().map(|(i, s)| (i, s.as_str())))
            .into_iter()
            .map(|(i, m)| (i, m.indices))
            .collect()
    })
}

fn slot_icon(
    ui: &mut MineralUi<'_, Material>,
    center: Pos2,
    id: Id,
    icon: Icon,
    color: Color32,
    node: SemanticNode,
) -> bool {
    let r = text_field::slot_button(ui, center, id, icon, color, node.state.disabled);
    semantic::emit(ui.ctx(), Some(r.id), r.rect, || node);
    r.clicked()
}

fn enabled_button(salt: &KeyedSalt, kind: &'static str, label: &str) -> SemanticNode {
    SemanticNode::button(StableId::of(salt, kind), label, Availability::Enabled)
}

fn edit_id(base: egui::Id) -> egui::Id {
    base.with("edit")
}
