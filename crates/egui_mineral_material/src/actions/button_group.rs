use crate::Material;
use crate::MaterialUiExt as _;
use egui::{Id, Response, Sense, Vec2, vec2};
use egui_mineral::kit::state::TempState;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;
use egui_mineral::{
    action::Clicked,
    focus::{self, RovingFocus, TabStop},
    layout::{Axis, Constraints},
    select::{Selection, SelectionMode},
    semantic::{self, Role, SemanticNode, StableId},
    widget::MineralWidget,
};

use crate::SelectionResponse;
use crate::{
    Icon, motion,
    spec::{
        defaults::button_group as group,
        tokens::{baseline_button, button_group_small, connected_button_group_small},
    },
};
use crate::{
    Size,
    actions::button::{ButtonShape, ContainerVariant, Joined, PushButton},
};

/// Entry of a [`ButtonGroup`].
#[derive(Clone, Debug)]
#[must_use]
pub struct ButtonGroupItem {
    label: String,
    label_visible: bool,
    icon: Option<Icon>,
    disabled: bool,
    salt: Option<egui_mineral::semantic::KeyedSalt>,
    action_key: Option<egui_mineral::action::ActionKey>,
}

impl ButtonGroupItem {
    crate::widget::setters! {
        /// Default the index.
        item_salt
    }

    /// `label` is the accessible name; shown unless [`Self::label_visible`] is `false`.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            label_visible: true,
            icon: None,
            disabled: false,
            salt: None,
            action_key: None,
        }
    }

    crate::widget::setters! {
        action_key
    }

    /// `false` with an [`icon`](Self::icon) makes an icon-only button; the label stays for
    /// assistive technology.
    pub fn label_visible(mut self, label_visible: bool) -> Self {
        self.label_visible = label_visible;
        self
    }
    /// Leading icon.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }
    crate::widget::setters! {
        /// Greys out this item only.
        disabled
    }
}

/// Result of [`ButtonGroup::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ButtonGroupResponse {
    /// Toggle outcome; empty without [`ButtonGroup::selection`].
    pub selection: SelectionResponse,
    /// Button clicked this frame, with its [`ButtonGroupItem::key`].
    pub clicked: Option<Clicked>,
}

/// MD3 button group types.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ButtonGroupVariant {
    #[default]
    /// Separate buttons; the pressed one widens, its neighbours shrink. Wraps when narrow.
    Standard,
    /// Joined buttons with inner corners (`ConnectedButtonGroup`).
    Connected,
}

/// MD3 Expressive button group; `connected` is Compose `ConnectedButtonGroup`.
#[derive(Debug)]
#[must_use = "call .show(..)"]
pub struct ButtonGroup<'a> {
    salt: KeyedSalt,
    label: String,
    items: Vec<ButtonGroupItem>,
    connected: bool,
    size: Option<Size>,
    button_variant: ContainerVariant,
    shape: ButtonShape,
    selection: Option<&'a mut Selection<usize>>,
    disabled: bool,
}

impl<'a> ButtonGroup<'a> {
    /// `label` is the group's accessible name; item agent keys are `<id_salt>-<item>-button`, `<item>` the item's `salt(..)`, else its index.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            label: label.into(),
            items: Vec::new(),
            connected: false,
            size: None,
            button_variant: ContainerVariant::default(),
            shape: ButtonShape::default(),
            selection: None,
            disabled: false,
        }
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix of the group and its items.
        agent_key(salt)
    }

    /// Default [`ButtonGroupVariant::Standard`].
    pub fn variant(mut self, variant: ButtonGroupVariant) -> Self {
        self.connected = variant == ButtonGroupVariant::Connected;
        self
    }

    /// Appends one button.
    pub fn item(mut self, item: impl Into<ButtonGroupItem>) -> Self {
        self.items.push(item.into());
        self
    }

    /// Appends buttons; labels convert into [`ButtonGroupItem`].
    pub fn items(mut self, items: impl IntoIterator<Item = impl Into<ButtonGroupItem>>) -> Self {
        self.items.extend(items.into_iter().map(Into::into));
        self
    }

    /// Toggle state of the buttons; without it they are click-only.
    pub fn selection(mut self, selection: &'a mut Selection<usize>) -> Self {
        self.selection = Some(selection);
        self
    }

    /// M3 Expressive size family; unset uses the baseline button metrics.
    pub fn size(mut self, size: Size) -> Self {
        self.size = Some(size);
        self
    }

    /// Style of every button in the group; default [`ContainerVariant::Filled`]. Selection toggles the
    /// buttons, which have no text variant.
    pub fn button_variant(mut self, button_variant: ContainerVariant) -> Self {
        self.button_variant = button_variant;
        self
    }

    /// Default [`ButtonShape::Round`].
    pub fn shape(mut self, shape: ButtonShape) -> Self {
        self.shape = shape;
        self
    }

    crate::widget::setters! {
        /// Greys out every button.
        disabled
    }

    fn button(&self, i: usize, item: &ButtonGroupItem) -> PushButton<'static> {
        let own = crate::widget::item_salt(&self.salt, item.salt.as_ref(), None, i);
        let key = StableId::of(&own, "button");
        let mut b = PushButton::new(own, item.label.clone())
            .agent_key(key)
            .variant(self.button_variant.into())
            .shape(self.shape)
            .disabled(self.disabled || item.disabled);
        if let Some(size) = self.size {
            b = b.size(size);
        }
        if let Some(icon) = item.icon {
            b = b.icon(icon);
        }
        b.icon_only = !item.label_visible && item.icon.is_some();
        if let Some(sel) = self.selection.as_deref() {
            b = b.selected(sel.is_selected(&i));
            if sel.mode() == SelectionMode::Single {
                b.role = Role::RadioButton;
            }
        }
        if self.connected {
            b.joined = Some(Joined {
                start: i == 0,
                end: i + 1 == self.items.len(),
                inner: connected_button_group_small::INNER_CORNER_CORNER_SIZE,
                inner_pressed: connected_button_group_small::PRESSED_INNER_CORNER_CORNER_SIZE,
                checked_full: true,
            });
        }
        b
    }

    // Compose ButtonGroupMeasurePolicy, ExpandedRatio
    fn widths(rt: &crate::Rt, id: Id, natural: &[f32], pressed: Option<usize>) -> Vec<f32> {
        let ctx = rt.ctx();
        // `ButtonDefaults.ContentPadding` end, the default compression limit.
        let limit = baseline_button::TRAILING_SPACE;
        let mut widths = natural.to_vec();
        let last = natural.len().saturating_sub(1);
        for (k, &w) in natural.iter().enumerate() {
            let key = id.with(("press", k));
            let hold_key = key.with("hold");
            let held = pressed == Some(k) || bool::load(ctx, hold_key);
            let p = egui_mineral::motion::spring(rt.core(), key, f32::from(held), motion::FAST_SPATIAL);
            // A release first finishes most of the expansion.
            if pressed == Some(k) || (held && p < group::RELEASE_AFTER) {
                true.store(ctx, hold_key);
            } else if held {
                bool::remove(ctx, hold_key);
            }
            if p <= 0.0 || last == 0 {
                continue;
            }
            let grow = |share: f32| (p * share.min(limit)).round();
            let growth = if k == 0 {
                let g = grow(group::EXPANDED_RATIO * w).min(widths[1]);
                widths[1] -= g;
                g
            } else if k == last {
                let g = grow(group::EXPANDED_RATIO * w).min(widths[k - 1]);
                widths[k - 1] -= g;
                g
            } else {
                let g = grow(group::EXPANDED_RATIO * w / 2.0);
                let (gl, gr) = (g.min(widths[k - 1]), g.min(widths[k + 1]));
                widths[k - 1] -= gl;
                widths[k + 1] -= gr;
                gl + gr
            };
            widths[k] += growth;
        }
        widths
    }

    /// Lays out the buttons in a row; a standard group wraps onto more rows when narrow.
    pub fn show(mut self, ui: &mut MineralUi<'_, Material>) -> ButtonGroupResponse {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let id = ui.make_persistent_id(self.salt.id_salt());
        let pressed_key = id.with("pressed");
        let last_pressed = Option::<usize>::load(ctx, pressed_key);
        let len = self.items.len();
        let enabled = |i: usize| !self.disabled && !self.items[i].disabled;

        let mut buttons: Vec<PushButton> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, it)| self.button(i, it))
            .collect();
        // Selection groups (radio or toggle) are one Tab stop with arrow roving; plain click groups keep a
        // stop per button, as in a standard APG button row.
        if let Some(sel) = self.selection.as_deref() {
            let selected = (0..len).find(|i| sel.is_selected(i));
            let stop = focus::tab_stop(ctx, id, selected, len, enabled);
            for (i, b) in buttons.iter_mut().enumerate() {
                b.tab_stop = TabStop::at(i, stop);
            }
        }
        let measure = |b: &PushButton| b.layout(ui, Constraints::loose(Vec2::INFINITY), &()).visual;
        let mut first_height = None;
        if !self.connected {
            let sizes: Vec<Vec2> = buttons.iter().map(measure).collect();
            first_height = sizes.first().map(|v| v.y);
            let natural: Vec<f32> = sizes.iter().map(|v| v.x).collect();
            let widths = Self::widths(rt, id, &natural, last_pressed.filter(|k| *k < len));
            for (b, w) in buttons.iter_mut().zip(widths) {
                b.width = Some(w);
            }
        }

        let dir = rt.i18n().direction();
        let gap = if self.connected {
            connected_button_group_small::BETWEEN_SPACE
        } else {
            button_group_small::BETWEEN_SPACE
        };
        let mut clicked = None;
        let mut pressed = None;
        let mut ids: Vec<Id> = Vec::with_capacity(len);
        let height = first_height.unwrap_or_else(|| buttons.first().map_or(0.0, |b| measure(b).y));
        let row = vec2(
            ui.available_width(),
            height.max(crate::adaptive::min_interactive(ui.density())),
        );
        let layout = dir.row().with_main_wrap(!self.connected);
        let outer = ui.scope(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(gap);
            ui.allocate_ui_with_layout(row, layout, |ui| {
                for (i, b) in buttons.into_iter().enumerate() {
                    let r = rt.ui(ui).mineral(b);
                    if r.clicked() {
                        clicked = Some(i);
                    }
                    if r.response().is_pointer_button_down_on() {
                        pressed = Some(i);
                    }
                    ids.push(r.response().id);
                }
            })
        });
        pressed.store(ctx, pressed_key);
        if last_pressed != pressed && !self.connected {
            #[expect(clippy::disallowed_methods, reason = "one pass after the pressed item changed")]
            ctx.request_repaint();
        }

        let mut changed = false;
        let mut selected = None;
        if let Some(sel) = self.selection.as_deref_mut() {
            if let Some(i) = clicked {
                changed = sel.toggle(i);
                selected = Some(i);
            }
            // APG radio group: arrows move focus and select; toggles only move focus.
            if let Some(i) = RovingFocus::new(Axis::Horizontal).update_group(rt.core(), id, &ids, enabled)
                && sel.mode() == SelectionMode::Single
                && !sel.is_selected(&i)
            {
                changed |= sel.select(i);
                selected = Some(i);
            }
        }

        let response = outer.response.interact(Sense::hover());
        self.announce(ctx, &response);
        ButtonGroupResponse {
            selection: SelectionResponse::new(response, selected, changed),
            clicked: crate::widget::clicked(&self.items, clicked, |i| i.action_key.as_ref()),
        }
    }

    fn announce(&self, ctx: &egui::Context, response: &Response) {
        semantic::emit(ctx, Some(response.id), response.rect, || {
            let role = match self.selection.as_deref() {
                Some(s) if s.mode() == SelectionMode::Single => Role::RadioGroup,
                _ => Role::Group,
            };
            let mut n = SemanticNode::new(StableId::of(&self.salt, "button-group"), role, &self.label);
            n.state.disabled = self.disabled;
            n
        });
    }
}
