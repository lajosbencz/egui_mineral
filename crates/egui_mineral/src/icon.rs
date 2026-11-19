use egui::{Context, FontData, FontFamily, FontId, FontInsert, FontPriority, InsertFontFamily};

/// One icon character of a named font family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Glyph {
    /// Name of the egui font family holding `ch` ([`IconFont::family`]).
    pub family: &'static str,
    /// Codepoint in that family.
    pub ch: char,
}

impl Glyph {
    /// Glyph `ch` of `family`.
    pub const fn new(family: &'static str, ch: char) -> Self {
        Self { family, ch }
    }

    /// Font family to lay [`Self::ch`] out with.
    pub fn font_family(self) -> FontFamily {
        FontFamily::Name(self.family.into())
    }

    /// Font of this glyph at `size`.
    pub fn font_id(self, size: f32) -> FontId {
        FontId::new(size, self.font_family())
    }
}

/// An icon in its two forms; a font without a second form repeats the glyph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Icon {
    /// Outlined form.
    pub regular: Glyph,
    /// Filled form, shown when selected, hovered or checked.
    pub filled: Glyph,
}

impl Icon {
    /// Icon with distinct forms.
    pub const fn new(regular: Glyph, filled: Glyph) -> Self {
        Self { regular, filled }
    }

    /// Icon with one form.
    pub const fn single(glyph: Glyph) -> Self {
        Self::new(glyph, glyph)
    }

    /// [`Self::filled`] when `filled`, else [`Self::regular`].
    pub const fn glyph(self, filled: bool) -> Glyph {
        if filled { self.filled } else { self.regular }
    }
}

// Orphan rules put this impl here: neither `Icon` nor `MaterialIcon` is local to a stylekit.
#[cfg(feature = "material-icons")]
impl From<egui_material_icons::MaterialIcon> for Icon {
    /// Outlined and filled forms are one codepoint in two families.
    fn from(icon: egui_material_icons::MaterialIcon) -> Self {
        use egui_material_icons::{FONT_FAMILY, FONT_FAMILY_OUTLINED, IconStyle};
        let ch = icon.codepoint.chars().next().unwrap_or(char::REPLACEMENT_CHARACTER);
        let outlined = Glyph::new(FONT_FAMILY_OUTLINED, ch);
        match icon.style {
            IconStyle::Filled => Self::new(outlined, Glyph::new(FONT_FAMILY, ch)),
            _ => Self::single(outlined),
        }
    }
}

/// An icon font file and how it is registered.
#[derive(Clone, Copy, Debug)]
pub struct IconFont {
    /// Registered font name.
    pub name: &'static str,
    /// Font file.
    pub ttf: &'static [u8],
    /// Family [`Glyph::family`] refers to.
    pub family: &'static str,
    /// Vertical glyph shift as a fraction of the font size, to align with text baselines.
    pub y_offset_factor: f32,
}

impl IconFont {
    /// Registers the font under [`Self::family`], and as lowest-priority proportional fallback.
    pub fn install(&self, ctx: &Context) {
        let mut data = FontData::from_static(self.ttf);
        data.tweak.y_offset_factor = self.y_offset_factor;
        ctx.add_font(FontInsert::new(
            self.name,
            data,
            vec![
                InsertFontFamily {
                    family: FontFamily::Proportional,
                    priority: FontPriority::Lowest,
                },
                InsertFontFamily {
                    family: FontFamily::Name(self.family.into()),
                    priority: FontPriority::Highest,
                },
            ],
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FILLED: Glyph = Glyph::new("a", 'x');
    const OUTLINED: Glyph = Glyph::new("b", 'x');

    #[test]
    fn glyph_picks_the_form() {
        let icon = Icon::new(OUTLINED, FILLED);
        assert_eq!(icon.glyph(false), OUTLINED);
        assert_eq!(icon.glyph(true), FILLED);
        assert_eq!(Icon::single(FILLED).glyph(false), FILLED);
    }

    #[test]
    fn font_id_names_the_family() {
        assert_eq!(FILLED.font_id(12.0).family, FontFamily::Name("a".into()));
    }

    #[cfg(feature = "material-icons")]
    #[test]
    fn material_icon_maps_to_both_families() {
        let icon = Icon::from(egui_material_icons::icons::ICON_CLOSE);
        assert_eq!(icon.regular.family, egui_material_icons::FONT_FAMILY_OUTLINED);
        assert_eq!(icon.filled.family, egui_material_icons::FONT_FAMILY);
        assert_eq!(icon.regular.ch, icon.filled.ch);
        let outlined = Icon::from(egui_material_icons::icons::ICON_CLOSE.outlined());
        assert_eq!(outlined.glyph(true), outlined.regular);
    }
}
