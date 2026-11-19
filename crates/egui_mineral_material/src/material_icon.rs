use egui_mineral::icon::{Glyph, Icon};

use crate::fonts::{FONT_FAMILY_FILLED, FONT_FAMILY_OUTLINED};

/// Material Symbols codepoint, filled when selected or hovered unless [`Self::outlined`]; constants in [`icons`](crate::icons).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MaterialIcon {
    codepoint: char,
    filled: bool,
}

impl MaterialIcon {
    /// Icon of `codepoint` with an outlined and a filled form.
    pub const fn new(codepoint: char) -> Self {
        Self {
            codepoint,
            filled: true,
        }
    }

    /// Outlined font in every state, the filled form is dropped.
    #[must_use]
    pub const fn outlined(self) -> Self {
        Self { filled: false, ..self }
    }

    /// Codepoint in the Material Symbols fonts.
    pub const fn codepoint(self) -> char {
        self.codepoint
    }
}

impl From<MaterialIcon> for Icon {
    fn from(icon: MaterialIcon) -> Self {
        let outlined = Glyph::new(FONT_FAMILY_OUTLINED, icon.codepoint);
        if icon.filled {
            Self::new(outlined, Glyph::new(FONT_FAMILY_FILLED, icon.codepoint))
        } else {
            Self::single(outlined)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::icons::ICON_CLOSE;

    #[test]
    fn converts_to_outlined_and_filled_families() {
        let icon = Icon::from(ICON_CLOSE);
        assert_eq!(
            (icon.regular.family, icon.filled.family),
            (FONT_FAMILY_OUTLINED, FONT_FAMILY_FILLED)
        );
        assert_eq!(
            (icon.regular.ch, icon.filled.ch),
            (ICON_CLOSE.codepoint(), ICON_CLOSE.codepoint())
        );
    }

    #[test]
    fn outlined_uses_one_family_in_every_state() {
        let icon = Icon::from(ICON_CLOSE.outlined());
        assert_eq!(icon.glyph(true).family, FONT_FAMILY_OUTLINED);
        assert_eq!(icon.glyph(true), icon.glyph(false));
    }

    #[test]
    fn families_match_the_bundled_fonts() {
        assert_eq!(FONT_FAMILY_FILLED, egui_material_icons::FONT_FAMILY);
        assert_eq!(FONT_FAMILY_OUTLINED, egui_material_icons::FONT_FAMILY_OUTLINED);
    }
}
