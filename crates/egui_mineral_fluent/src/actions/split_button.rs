use crate::widget::vocab::{ButtonAppearance, ButtonShape};
use egui::{KeyboardShortcut, Response, Vec2};
use egui_mineral::{
    semantic::{AgentKey, KeyedSalt, StableId},
    widget::MineralUi,
};

use crate::{
    Fluent, Icon, Size,
    actions::{PushButton, button::Join},
    spec::defaults::split_button as d,
};

/// Result of [`SplitButton::show`].
#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct SplitButtonResponse {
    /// Primary action part.
    pub response: Response,
    /// Primary action clicked.
    pub clicked: bool,
    /// Menu part; anchor the app's menu to it.
    pub menu_response: Response,
    /// Menu part clicked: toggle the menu.
    pub menu_clicked: bool,
}

/// Fluent `SplitButton`: a primary action joined to a chevron menu button; the app shows the menu and passes its
/// open state to [`Self::expanded`].
#[derive(Debug)]
#[must_use = "show with show(ui)"]
pub struct SplitButton {
    salt: KeyedSalt,
    key: Option<StableId>,
    label: String,
    menu_label: Option<String>,
    icon: Option<Icon>,
    appearance: ButtonAppearance,
    size: Size,
    shape: ButtonShape,
    expanded: bool,
    disabled: bool,
    shortcut: Option<KeyboardShortcut>,
}

impl SplitButton {
    /// `label` is the primary action's text and accessible name; agent keys `<id_salt>-button` (primary action)
    /// and `<id_salt>-split-menu-button`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        Self {
            salt: id_salt.into(),
            key: None,
            label: label.into(),
            menu_label: None,
            icon: None,
            appearance: ButtonAppearance::Secondary,
            size: Size::Medium,
            shape: ButtonShape::Rounded,
            expanded: false,
            disabled: false,
            shortcut: None,
        }
    }

    /// Accessible name of the menu part; default "More options".
    pub fn menu_label(mut self, menu_label: impl Into<String>) -> Self {
        self.menu_label = Some(menu_label.into());
        self
    }

    /// Leading icon of the primary action.
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Default [`ButtonAppearance::Secondary`].
    pub fn appearance(mut self, appearance: ButtonAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// Default [`Size::Medium`].
    pub fn size(mut self, size: Size) -> Self {
        self.size = size;
        self
    }

    /// Default [`ButtonShape::Rounded`].
    pub fn shape(mut self, shape: ButtonShape) -> Self {
        self.shape = shape;
        self
    }

    /// Open state of the menu; default `false`.
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    crate::widget::setters! {
        /// Greys out both parts and ignores input.
        disabled
    }

    /// Activates the primary action like a click while enabled and visible.
    pub fn shortcut(mut self, shortcut: KeyboardShortcut) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    /// Replaces the `<id_salt>-button` agent key of the primary action.
    pub fn agent_key(mut self, key: impl Into<StableId>) -> Self {
        self.key = Some(key.into());
        self
    }

    /// Shows both parts in a row.
    pub fn show(self, ui: &mut MineralUi<'_, Fluent>) -> SplitButtonResponse {
        let rt = ui.rt();
        let menu_label = crate::tr_or(rt.i18n(), self.menu_label.as_deref(), "mineral-split-button-menu");
        let mut lead = PushButton::new(self.salt.clone(), self.label)
            .appearance(self.appearance)
            .size(self.size)
            .shape(self.shape)
            .disabled(self.disabled);
        if let Some(icon) = self.icon {
            lead = lead.icon(icon);
        }
        if let Some(shortcut) = self.shortcut {
            lead = lead.shortcut(shortcut);
        }
        if let Some(key) = self.key {
            lead = lead.agent_key(key);
        }
        lead.join = Some(Join::Start);
        let mut menu = PushButton::new(self.salt.child("menu"), menu_label)
            .appearance(self.appearance)
            .size(self.size)
            .shape(self.shape)
            .label_visible(false)
            .disabled(self.disabled);
        menu.key = AgentKey::Custom(StableId::of(&self.salt, "split-menu-button"));
        menu.menu_icon = true;
        menu.expanded = Some(self.expanded);
        menu.join = Some(Join::End);
        menu.min_width = Some(d::MIN_TARGET_SIZE);
        let dir = rt.i18n().direction();
        let (lead_r, menu_r) = ui
            .scope(|ui| {
                // The menu part has no start border: the primary action's end border divides them.
                ui.spacing_mut().item_spacing.x = 0.0;
                // Sized by the parts: `with_layout` would take, and center them in, all the space left.
                ui.allocate_ui_with_layout(Vec2::ZERO, dir.row(), |ui| {
                    (rt.ui(ui).mineral(lead), rt.ui(ui).mineral(menu))
                })
                .inner
            })
            .inner;
        SplitButtonResponse {
            clicked: lead_r.clicked(),
            response: lead_r.into_response(),
            menu_clicked: menu_r.clicked(),
            menu_response: menu_r.into_response(),
        }
    }
}
