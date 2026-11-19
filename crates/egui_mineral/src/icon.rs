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

/// Outlined and filled glyph of one icon; fonts without a filled form use the same glyph twice.
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

/// Font bytes plus the family name [`Glyph::family`] refers to; installed once per context.
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
}
