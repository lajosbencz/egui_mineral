use egui::{Context, FontData, FontFamily, FontId, FontInsert, FontPriority, InsertFontFamily};

/// One icon character of a named font family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Glyph {
    family: &'static str,
    ch: char,
}

impl Glyph {
    /// Glyph `ch` of `family`.
    pub const fn new(family: &'static str, ch: char) -> Self {
        Self { family, ch }
    }

    /// Name of the egui font family holding [`Self::ch`] ([`IconFont::family`]).
    pub const fn family(self) -> &'static str {
        self.family
    }

    /// Codepoint in [`Self::family`].
    pub const fn ch(self) -> char {
        self.ch
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
    regular: Glyph,
    filled: Glyph,
}

/// Form of an [`Icon`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IconStyle {
    /// Outlined form.
    #[default]
    Regular,
    /// Filled form, shown when selected, hovered or checked.
    Filled,
}

impl IconStyle {
    /// [`Self::Filled`] for `true`.
    pub const fn filled_if(filled: bool) -> Self {
        if filled { Self::Filled } else { Self::Regular }
    }
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

    /// Outlined form.
    pub const fn regular(self) -> Glyph {
        self.regular
    }

    /// Filled form.
    pub const fn filled(self) -> Glyph {
        self.filled
    }

    /// Glyph of `style`.
    pub const fn glyph(self, style: IconStyle) -> Glyph {
        match style {
            IconStyle::Regular => self.regular,
            IconStyle::Filled => self.filled,
        }
    }
}

/// Font bytes plus the family name [`Glyph::family`] refers to; installed once per context.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
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
    /// Font `ttf` registered as `family`, unshifted.
    pub const fn new(family: &'static str, ttf: &'static [u8]) -> Self {
        Self {
            name: family,
            ttf,
            family,
            y_offset_factor: 0.0,
        }
    }

    /// Registered font name; the family name by default.
    #[must_use]
    pub const fn name(self, name: &'static str) -> Self {
        Self { name, ..self }
    }

    /// Vertical glyph shift as a fraction of the font size.
    #[must_use]
    pub const fn y_offset_factor(self, y_offset_factor: f32) -> Self {
        Self {
            y_offset_factor,
            ..self
        }
    }

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
        assert_eq!(icon.glyph(IconStyle::filled_if(false)), OUTLINED);
        assert_eq!(icon.glyph(IconStyle::Filled), FILLED);
        assert_eq!(Icon::single(FILLED).regular(), FILLED);
    }

    #[test]
    fn font_id_names_the_family() {
        assert_eq!(FILLED.font_id(12.0).family, FontFamily::Name("a".into()));
    }
}
