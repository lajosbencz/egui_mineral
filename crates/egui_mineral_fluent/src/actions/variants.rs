use crate::widget::vocab::{ButtonAppearance, ButtonShape};
use egui::KeyboardShortcut;
use egui_mineral::{
    i18n::I18n,
    kit,
    layout::{Constraints, Measure},
    semantic::{KeyedSalt, SemanticNode, StableId},
    widget::{MineralUi, MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::{Fluent, FluentIcon, IconPosition, Size, actions::PushButton};

// Setters every wrapper forwards to its `PushButton`.
macro_rules! forward {
    ($($(#[$m:meta])* $name:ident($arg:ident: $ty:ty);)*) => {$(
        $(#[$m])*
        pub fn $name(mut self, $arg: $ty) -> Self {
            self.0 = self.0.$name($arg);
            self
        }
    )*};
}

macro_rules! common {
    () => {
        forward! {
            /// Default [`ButtonAppearance::Secondary`].
            appearance(appearance: ButtonAppearance);
            /// Default [`Size::Medium`].
            size(size: Size);
            /// Default [`ButtonShape::Rounded`].
            shape(shape: ButtonShape);
            /// Leading icon; filled while hovered (subtle, transparent) or checked.
            icon(icon: FluentIcon);
            /// `false` with an icon is the icon-only button; the label stays the accessible name.
            label_visible(visible: bool);
            /// Greys out the button and ignores input.
            disabled(disabled: bool);
            /// Alt+letter activates the button; the letter is underlined while mnemonics are visible.
            mnemonic(mnemonic: char);
            /// Activates like a click while enabled and visible.
            shortcut(shortcut: KeyboardShortcut);
        }

        /// Replaces the `<id_salt>-button` agent key.
        pub fn agent_key(mut self, key: impl Into<StableId>) -> Self {
            self.0 = self.0.agent_key(key);
            self
        }
    };
}

macro_rules! delegate {
    ($ty:ty) => {
        impl MineralWidget for $ty {
            type Kit = Fluent;
            type State = ();
            type Output = ();

            kit::delegate_widget_meta!(0);

            fn layout(&self, ui: &MineralUi<'_, Fluent>, constraints: Constraints, state: &()) -> Measure {
                self.0.layout(ui, constraints, state)
            }

            fn update(
                &mut self,
                ui: &mut MineralUi<'_, Fluent>,
                frame: &WidgetFrame<'_>,
                state: &mut (),
            ) -> WidgetResponse {
                self.0.update(ui, frame, state)
            }

            fn semantics(&self, i18n: &I18n, response: &WidgetResponse, state: &()) -> Option<SemanticNode> {
                self.0.semantics(i18n, response, state)
            }
        }
    };
}

/// Fluent `ToggleButton`: a click flips the bound value and reports `changed`; checked shows the selected
/// colors and the filled icon.
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct ToggleButton<'a>(PushButton<'a>);

impl<'a> ToggleButton<'a> {
    /// `label` is the visible text and accessible name; agent key `<id_salt>-button`; `checked` flips on click.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, checked: &'a mut bool) -> Self {
        Self(PushButton::new(id_salt, label).bind(checked))
    }

    common!();

    forward! {
        /// Default [`IconPosition::Before`].
        icon_position(position: IconPosition);
    }
}

delegate!(ToggleButton<'_>);

/// Fluent `CompoundButton`: a label over secondary text, beside a 40pt icon.
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct CompoundButton(PushButton<'static>);

impl CompoundButton {
    /// `label` is the visible text and accessible name, `secondary` the text under it (the node's value);
    /// agent key `<id_salt>-button`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>, secondary: impl Into<String>) -> Self {
        let mut b = PushButton::new(id_salt, label);
        b.compound = true;
        b.secondary = Some(secondary.into());
        Self(b)
    }

    common!();

    forward! {
        /// Default [`IconPosition::Before`].
        icon_position(position: IconPosition);
    }
}

delegate!(CompoundButton);

/// Fluent `MenuButton`: a button with a trailing chevron that opens a menu the app shows; `expanded`
/// mirrors that menu's open state (`aria-expanded`).
#[derive(Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct MenuButton(PushButton<'static>);

impl MenuButton {
    /// `label` is the visible text and accessible name; agent key `<id_salt>-button`.
    pub fn new(id_salt: impl Into<KeyedSalt>, label: impl Into<String>) -> Self {
        let mut b = PushButton::new(id_salt, label);
        b.menu_icon = true;
        b.expanded = Some(false);
        Self(b)
    }

    common!();

    /// Open state of the menu it controls: selected colors and the `expanded` state; default `false`.
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.0.expanded = Some(expanded);
        self
    }
}

delegate!(MenuButton);
