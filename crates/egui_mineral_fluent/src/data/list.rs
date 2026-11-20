use crate::msg;
use egui::{Align, Id, Key, Response, Sense, Stroke, StrokeKind, Ui, UiBuilder};
use egui_mineral::{
    focus::{self as roving, RovingFocus, TabStop},
    kit,
    layout::Axis,
    select::{ItemKey, Selection, SelectionMode},
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
    /// Selection changed this pass by a click, Space or a checkmark.
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
}

impl<'a, K: ItemKey> List<'a, K> {
    /// Plain list, no navigation or selection; agent key `<id_salt>-list`.
    pub fn new(id_salt: impl Into<KeyedSalt>) -> Self {
        Self {
            salt: id_salt.into(),
            key: egui_mineral::agent_key!("list"),
            navigation: None,
            selection: None,
        }
    }

    /// Arrow key navigation; items become focusable.
    pub fn navigation(mut self, navigation: Option<ListNavigation>) -> Self {
        self.navigation = navigation;
        self
    }

    /// Selection of item keys owned by the app; items become focusable and selectable.
    pub fn selection(mut self, selection: &'a mut Selection<K>) -> Self {
        self.selection = Some(selection);
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
        let scope = ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
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
                index: 0,
                ids: Vec::new(),
                first: None,
                activated: None,
                changed: false,
            };
            let inner = add(&mut lui);
            if focusable {
                RovingFocus::new(Axis::Vertical)
                    .wrap(false)
                    .update(rt.core(), &lui.ids, |_| true);
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
        semantic::emit(ui.ctx(), None, response.rect, || {
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
    index: usize,
    ids: Vec<Id>,
    first: Option<usize>,
    activated: Option<K>,
    changed: bool,
}

kit::opaque_debug!(ListUi<'_, '_, K>);

impl<K: ItemKey> ListUi<'_, '_, K> {
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
                        self.changed |= flip(sel, key.clone());
                    }
                    rui.add_space(d::CHECKMARK_MARGIN);
                }
                add(rui)
            },
        );
        let response = scope.response;
        let by_enter = response.has_focus() && self.ui.input(|inp| inp.key_pressed(Key::Enter));
        let by_space = response.has_focus() && self.ui.input(|inp| inp.key_pressed(Key::Space));
        if response.clicked() && !disabled {
            if let Some(sel) = self.selection.as_deref_mut()
                && !by_enter
            {
                self.changed |= flip(sel, key.clone());
            }
            if !(selectable && by_space) {
                self.activated = Some(key);
            }
        }
        if self.focusable && !disabled {
            self.ids.push(response.id);
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

fn flip<K: ItemKey>(selection: &mut Selection<K>, key: K) -> bool {
    if selection.mode() == SelectionMode::Multiple {
        selection.toggle(key)
    } else {
        selection.select(key)
    }
}
