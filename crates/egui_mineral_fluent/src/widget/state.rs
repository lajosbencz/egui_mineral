use egui::Color32;
use egui_mineral::kit::paint::{StateRamp, VisualState};

use crate::{spec::ColorToken, theme::Tokens};

// One property's token per state; `None` is `transparent`.
pub(crate) type Ramp = StateRamp<Option<ColorToken>>;

pub(crate) trait Pick {
    fn pick(&self, s: VisualState, t: &Tokens) -> Color32;
}

impl Pick for Ramp {
    fn pick(&self, s: VisualState, t: &Tokens) -> Color32 {
        self.get(s).map_or(Color32::TRANSPARENT, |c| t.color(c))
    }
}
