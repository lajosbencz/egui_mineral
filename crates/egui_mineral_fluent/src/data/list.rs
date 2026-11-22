use crate::msg;
use egui::{Align, Id, Response, Sense, Stroke, StrokeKind, Ui, UiBuilder};
use egui_mineral::{
    focus::{self as roving, RovingFocus, TabStop},
    kit,
    layout::Axis,
    select::{self, ItemKey, RowInput, RowSource, Selection, SelectionBehavior, SelectionMode},
    semantic::{self, AgentKey, KeyedSalt, Role, SemanticNode},
    widget::{Availability, MineralUi},
};

use crate::spec::ColorToken::ColorStrokeFocus2;
use crate::{
    Fluent, FluentExt, Rt,
    inputs::{Checkbox, CheckboxShape},
    spec::{defaults::list as d, tokens::geometry},
    theme::Radius,
};

/// Arrow key navigation of a [`List`] (`navigationMode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ListNavigation {
    /// `items`: arrow keys move between the items, one tab stop.
    Items,
    /// `composite`: the same, with the `grid` role.
    Composite,
}

/// Result of [`ListUi::item`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ListItemResponse<R> {
    /// Spans the item.
    pub response: Response,
    /// Value of the content closure.
    pub inner: R,
}

/// Result of showing a [`List`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ListResponse<K, R = ()> {
    /// Spans all items.
    pub response: Response,
    /// Value of the content closure.
    pub inner: R,
    /// Key of the item that was clicked or entered.
    pub activated: Option<K>,
    /// Selection changed this pass by a click, a key or a checkmark.
    pub changed: bool,
}

/// Fluent `List`: items with optional arrow key navigation and selection. Selection mode follows the
/// [`Selection`]: [`Selection::single`] or [`Selection::multiple`].
#[derive(Debug)]
#[must_use = "call .show(..)"]
pub struct List<'a, K> {
    salt: KeyedSalt,
    key: AgentKey,
    navigation: Option<ListNavigation>,
    selection: Option<&'a mut Selection<K>>,
    behavior: SelectionBehavior,
}

impl<'a, K: ItemKey> List<'a, K> {
    /// Plain list, no navigation or selection; agent key `<id_salt>-list`.
    pub fn new(id_salt: impl Into<KeyedSalt>) -> Self {
        Self {
            salt: id_salt.into(),
            key: egui_mineral::agent_key!("list"),
            navigation: None,
            selection: None,
            behavior: SelectionBehavior::Toggle,
        }
    }

    /// Arrow key navigation; items become focusable.
    pub fn navigation(mut self, navigation: Option<ListNavigation>) -> Self {
        self.navigation = navigation;
        self
    }

    /// Selection of item keys owned by the app; items become focusable and select per
    /// [`Self::selection_behavior`] ([`Selection::apply`], [`Selection::follow_focus`]). Enter is the item action;
    /// Ctrl/Cmd+A selects every enabled item.
    pub fn selection(mut self, selection: &'a mut Selection<K>) -> Self {
        self.selection = Some(selection);
        self
    }

    /// How clicks and arrows select; [`SelectionBehavior::Toggle`] by default (`useListSelection`).
    pub fn selection_behavior(mut self, behavior: SelectionBehavior) -> Self {
        self.behavior = behavior;
        self
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>-list` agent key.
        agent_key(key)
    }

    /// Shows the list; `add` places its items.
    pub fn show<R>(
        self,
        ui: &mut MineralUi<'_, Fluent>,
        add: impl FnOnce(&mut ListUi<'_, '_, K>) -> R,
    ) -> ListResponse<K, R> {
        let rt = ui.rt();
        let id = ui.make_persistent_id(self.salt.id_salt());
        let count_id = id.with("count");
        let first_id = id.with("first-selected");
        let known: usize = ui.ctx().data(|d| d.get_temp(count_id)).unwrap_or(0);
        let navigation = self.navigation;
        let focusable = navigation.is_some() || self.selection.is_some();
        let role = match (navigation, self.selection.is_some()) {
            (Some(ListNavigation::Composite), _) => Role::Grid,
            (_, true) => Role::ListBox,
            _ => Role::List,
        };
        let key = self.key.id(&self.salt);
        let selection = self.selection;
        // Position of the first selected item last pass.
        let first: Option<usize> = ui.ctx().data(|d| d.get_temp(first_id));
        let behavior = self.behavior;
        let scope = ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            let items_scope = ui.unique_id();
            let stop = roving::tab_stop(rt.ctx(), id, first, &roving::Items::all(known));
            let mut lui = ListUi {
                ui,
                rt,
                id,
                salt: self.salt.clone(),
                role,
                focusable,
                stop,
                selection,
                behavior,
                index: 0,
                ids: Vec::new(),
                keys: Vec::new(),
                input: None,
                first: None,
                activated: None,
                changed: false,
            };
            let inner = add(&mut lui);
            let roving = RovingFocus::new(Axis::Vertical).wrap(false);
            if let Some(sel) = lui.selection.as_deref_mut() {
                if let Some((k, input)) = lui.input.take() {
                    lui.changed |= sel.apply(k, input, behavior, &lui.keys);
                }
                if let Some((i, m)) = roving.update_select_group(rt.core(), id, &lui.ids, |_| true) {
                    lui.changed |= sel.follow_focus(lui.keys[i].clone(), m, behavior, &lui.keys);
                }
                if select::select_all_shortcut(rt.ctx(), items_scope) {
                    lui.changed |= sel.select_all(&lui.keys);
                }
            } else if focusable {
                roving.update_group(rt.core(), id, &lui.ids, |_| true);
            }
            if lui.index != known {
                rt.ctx().data_mut(|d| d.insert_temp(count_id, lui.index));
            }
            if lui.first != first {
                rt.ctx().data_mut(|d| match lui.first {
                    Some(i) => {
                        d.insert_temp(first_id, i);
                    }
                    None => d.remove::<usize>(first_id),
                });
            }
            (inner, lui.activated, lui.changed)
        });
        let response = scope.response;
        let (inner, activated, changed) = scope.inner;
        semantic::emit(ui.ctx(), Some(response.id), response.rect, || {
            SemanticNode::new(key, role, crate::tr(rt.i18n(), msg::LIST))
        });
        ListResponse {
            response,
            inner,
            activated,
            changed,
        }
    }
}

/// Fluent `ListItem`: the content of a [`ListUi`] row.
#[derive(Clone, Debug)]
#[must_use]
pub struct ListItem {
    label: String,
    disabled: bool,
    salt: Option<KeyedSalt>,
}

impl ListItem {
    /// Accessible name.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            disabled: false,
            salt: None,
        }
    }

    crate::widget::setters! {
        item_salt
    }

    crate::widget::setters! {
        /// Keeps the item from selecting and activating.
        disabled
    }
}

impl From<&str> for ListItem {
    fn from(label: &str) -> Self {
        Self::new(label)
    }
}

impl From<String> for ListItem {
    fn from(label: String) -> Self {
        Self::new(label)
    }
}

/// Places the items of a [`List`].
pub struct ListUi<'u, 'e, K> {
    ui: &'u mut Ui,
    rt: &'u Rt,
    id: Id,
    salt: KeyedSalt,
    role: Role,
    focusable: bool,
    stop: usize,
    selection: Option<&'e mut Selection<K>>,
    behavior: SelectionBehavior,
    index: usize,
    ids: Vec<Id>,
    // Keys of `ids`.
    keys: Vec<K>,
    // Applied once every key is known, for Shift.
    input: Option<(K, RowInput)>,
    first: Option<usize>,
    activated: Option<K>,
    changed: bool,
}

kit::opaque_debug!(ListUi<'_, '_, K>);

impl<K: ItemKey> ListUi<'_, '_, K> {
    // Pointer and key input of an enabled item: activation now, selection once every key is known.
    fn take_input(&mut self, key: &K, response: &Response) {
        let mut input = RowInput::of(&self.rt.core().input(), response);
        let source = input.click.map(|c| c.1);
        if let Some(sel) = self.selection.as_deref() {
            let activates = match source {
                Some(RowSource::Space) | None => false,
                Some(RowSource::Touch) => {
                    self.behavior == SelectionBehavior::Toggle || sel.activates(input, self.behavior)
                }
                Some(RowSource::Mouse | RowSource::Enter) => true,
            };
            // Fluent List: Enter is the item action only.
            if source == Some(RowSource::Enter) {
                input.click = None;
            }
            if !input.is_empty() {
                self.input = Some((key.clone(), input));
            }
            if activates {
                self.activated = Some(key.clone());
            }
        } else if source.is_some() {
            self.activated = Some(key.clone());
        }
    }

    /// One [`ListItem`] keyed by `key` in the selection, activation and its default agent key; `add` places its
    /// content.
    pub fn item<R>(
        &mut self,
        key: K,
        item: impl Into<ListItem>,
        add: impl FnOnce(&mut MineralUi<'_, Fluent>) -> R,
    ) -> ListItemResponse<R> {
        let i = self.index;
        self.index += 1;
        let ListItem { label, disabled, salt } = item.into();
        let rt = self.rt;
        let t = rt.tokens();
        let dir = rt.i18n().direction();
        let selectable = self.selection.is_some();
        let availability = Availability::disabled_if(disabled);
        let sense = if self.focusable && !disabled {
            roving::roving_sense(Sense::click(), TabStop::at(i, self.stop))
        } else {
            Sense::hover()
        };
        let item_salt = salt.unwrap_or_else(|| self.salt.child(("item", key.clone())));
        let id = self.id.with(("item", item_salt.id_salt()));
        let selected = self.selection.as_deref().is_some_and(|s| s.is_selected(&key));
        if selected && self.first.is_none() {
            self.first = Some(i);
        }
        let scope = self.ui.scope_builder(
            UiBuilder::new()
                .id_salt(id)
                .sense(sense)
                .layout(dir.row().with_cross_align(Align::Min)),
            |ui| {
                let rui = &mut rt.ui(ui);
                if let Some(sel) = self.selection.as_deref_mut() {
                    let mut checked = sel.is_selected(&key);
                    let before = checked;
                    rui.add_space(d::CHECKMARK_MARGIN);
                    let shape = if sel.mode() == SelectionMode::Multiple {
                        CheckboxShape::Square
                    } else {
                        CheckboxShape::Circular
                    };
                    rui.mineral(
                        Checkbox::new(item_salt.child("checkmark"), label.clone(), &mut checked)
                            .label_visible(false)
                            .shape(shape)
                            .disabled(disabled),
                    );
                    if checked != before {
                        self.changed |= if sel.mode() == SelectionMode::Multiple {
                            sel.toggle(key.clone())
                        } else {
                            sel.select(key.clone())
                        };
                    }
                    rui.add_space(d::CHECKMARK_MARGIN);
                }
                add(rui)
            },
        );
        let response = scope.response;
        if !disabled {
            self.take_input(&key, &response);
        }
        if self.focusable && !disabled {
            self.ids.push(response.id);
            self.keys.push(key);
            if response.has_focus() && roving::visible(self.ui.ctx(), response.id) {
                self.ui.painter().rect_stroke(
                    response.rect,
                    Radius::Medium.on(response.rect),
                    Stroke::new(geometry::STROKE_WIDTH_THICK, t.color(ColorStrokeFocus2)),
                    StrokeKind::Outside,
                );
            }
        }
        let role = match self.role {
            Role::Grid => Role::Row,
            Role::ListBox => Role::ListBoxOption,
            _ => Role::ListItem,
        };
        let key = egui_mineral::agent_key!("item").id(&item_salt);
        semantic::emit(self.ui.ctx(), Some(response.id), response.rect, || {
            let mut n = SemanticNode::new(key, role, label);
            if self.focusable {
                n = n.clickable(availability);
            }
            if selectable {
                n.state.selected = Some(selected);
            }
            n
        });
        ListItemResponse {
            response,
            inner: scope.inner,
        }
    }
}
