use egui_mineral::icon::Icon;

use crate::icons;

/// Glyphs the kit draws for its own parts (chevrons, dismiss, checkmark, ...), one field per role named after
/// the Fluent UI System Icon it defaults to. [`Self::fluent`] is the default, other fonts fill the same fields
/// ([`Fluent::with_icons`](crate::Fluent::with_icons)).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IconSet {
    /// `arrow_down`.
    pub arrow_down: Icon,
    /// `arrow_up`.
    pub arrow_up: Icon,
    /// `calendar_month`.
    pub calendar_month: Icon,
    /// `checkmark`.
    pub checkmark: Icon,
    /// `checkmark_circle`.
    pub checkmark_circle: Icon,
    /// `chevron_down`.
    pub chevron_down: Icon,
    /// `chevron_left`.
    pub chevron_left: Icon,
    /// `chevron_right`.
    pub chevron_right: Icon,
    /// `chevron_up`.
    pub chevron_up: Icon,
    /// `dismiss`.
    pub dismiss: Icon,
    /// `error_circle`.
    pub error_circle: Icon,
    /// `image_off`.
    pub image_off: Icon,
    /// `info`.
    pub info: Icon,
    /// `more_horizontal`.
    pub more_horizontal: Icon,
    /// `pause`.
    pub pause: Icon,
    /// `person`.
    pub person: Icon,
    /// `play`.
    pub play: Icon,
    /// `prohibited`.
    pub prohibited: Icon,
    /// `search`.
    pub search: Icon,
    /// `square`.
    pub square: Icon,
    /// `star`.
    pub star: Icon,
    /// `warning`.
    pub warning: Icon,
}

impl IconSet {
    /// Fluent UI System Icons, the bundled font.
    pub fn fluent() -> Self {
        Self {
            arrow_down: Icon::from(icons::ICON_ARROW_DOWN),
            arrow_up: Icon::from(icons::ICON_ARROW_UP),
            calendar_month: Icon::from(icons::ICON_CALENDAR_MONTH),
            checkmark: Icon::from(icons::ICON_CHECKMARK),
            checkmark_circle: Icon::from(icons::ICON_CHECKMARK_CIRCLE),
            chevron_down: Icon::from(icons::ICON_CHEVRON_DOWN),
            chevron_left: Icon::from(icons::ICON_CHEVRON_LEFT),
            chevron_right: Icon::from(icons::ICON_CHEVRON_RIGHT),
            chevron_up: Icon::from(icons::ICON_CHEVRON_UP),
            dismiss: Icon::from(icons::ICON_DISMISS),
            error_circle: Icon::from(icons::ICON_ERROR_CIRCLE),
            image_off: Icon::from(icons::ICON_IMAGE_OFF),
            info: Icon::from(icons::ICON_INFO),
            more_horizontal: Icon::from(icons::ICON_MORE_HORIZONTAL),
            pause: Icon::from(icons::ICON_PAUSE),
            person: Icon::from(icons::ICON_PERSON),
            play: Icon::from(icons::ICON_PLAY),
            prohibited: Icon::from(icons::ICON_PROHIBITED),
            search: Icon::from(icons::ICON_SEARCH),
            square: Icon::from(icons::ICON_SQUARE),
            star: Icon::from(icons::ICON_STAR),
            warning: Icon::from(icons::ICON_WARNING),
        }
    }
}

impl Default for IconSet {
    fn default() -> Self {
        Self::fluent()
    }
}
