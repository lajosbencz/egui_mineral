use egui::Color32;
use egui_mineral::widget::Interaction;

use crate::{ColorToken, theme::Tokens};

// CSS rest, `:hover` and `:hover:active` / `:active:focus-visible`, and disabled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum State {
    Rest,
    Hover,
    Pressed,
    Disabled,
}

impl State {
    pub(crate) fn of(i: Interaction) -> Self {
        if i.disabled {
            Self::Disabled
        } else if i.pressed {
            Self::Pressed
        } else if i.hovered {
            Self::Hover
        } else {
            Self::Rest
        }
    }

    pub(crate) const fn engaged(self) -> bool {
        matches!(self, Self::Hover | Self::Pressed)
    }
}

// One property's token per state; `None` is `transparent`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Ramp {
    pub rest: Option<ColorToken>,
    pub hover: Option<ColorToken>,
    pub pressed: Option<ColorToken>,
    pub disabled: Option<ColorToken>,
}

impl Ramp {
    pub(crate) const fn new(rest: ColorToken, hover: ColorToken, pressed: ColorToken, disabled: ColorToken) -> Self {
        Self {
            rest: Some(rest),
            hover: Some(hover),
            pressed: Some(pressed),
            disabled: Some(disabled),
        }
    }

    pub(crate) const TRANSPARENT: Self = Self {
        rest: None,
        hover: None,
        pressed: None,
        disabled: None,
    };

    pub(crate) const fn disabled(mut self, disabled: Option<ColorToken>) -> Self {
        self.disabled = disabled;
        self
    }

    pub(crate) const fn token(self, s: State) -> Option<ColorToken> {
        match s {
            State::Rest => self.rest,
            State::Hover => self.hover,
            State::Pressed => self.pressed,
            State::Disabled => self.disabled,
        }
    }

    pub(crate) fn pick(self, s: State, t: &Tokens) -> Color32 {
        self.token(s).map_or(Color32::TRANSPARENT, |c| t.color(c))
    }
}
