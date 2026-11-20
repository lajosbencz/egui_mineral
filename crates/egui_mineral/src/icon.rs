use egui::{Context, FontData, FontFamily, FontId, FontInsert, FontPriority, InsertFontFamily};

/// Named egui font family of an icon font: the one source of its [`Glyph`]s ([`Self::glyph`]) and of the
/// [`IconFont`] that installs it ([`Self::font`]), so the two cannot name different families.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct IconFamily(&'static str);

impl IconFamily {
    /// Family registered as `name`; keep one constant per icon font.
    pub const fn new(name: &'static str) -> Self {
        Self(name)
    }

    /// Registered family name.
    pub const fn name(self) -> &'static str {
        self.0
    }

    /// Glyph `ch` of this family.
    pub const fn glyph(self, ch: char) -> Glyph {
        Glyph { family: self, ch }
    }

    /// Font file `ttf` that provides this family, unshifted.
    pub const fn font(self, ttf: &'static [u8]) -> IconFont {
        IconFont {
            name: self.0,
            ttf,
            family: self,
            y_offset_factor: 0.0,
        }
    }

    /// egui family of the glyphs.
    pub fn font_family(self) -> FontFamily {
        FontFamily::Name(self.0.into())
    }
}

/// One icon character of an [`IconFamily`]; built by [`IconFamily::glyph`] only.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Glyph {
    family: IconFamily,
    ch: char,
}

impl Glyph {
    /// Family holding [`Self::ch`].
    pub const fn family(self) -> IconFamily {
        self.family
    }

    /// Codepoint in [`Self::family`].
    pub const fn ch(self) -> char {
        self.ch
    }

    /// Font family to lay [`Self::ch`] out with.
    pub fn font_family(self) -> FontFamily {
        self.family.font_family()
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

/// Font file of an [`IconFamily`], built by [`IconFamily::font`]; installed once per context.
#[derive(Clone, Copy, Debug)]
pub struct IconFont {
    name: &'static str,
    ttf: &'static [u8],
    family: IconFamily,
    y_offset_factor: f32,
}

impl IconFont {
    /// Family the font is registered under.
    pub const fn family(&self) -> IconFamily {
        self.family
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

    /// Registers the font under its [`IconFamily`], and as lowest-priority proportional fallback.
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
                    family: self.family.font_family(),
                    priority: FontPriority::Highest,
                },
            ],
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: IconFamily = IconFamily::new("a");
    const FILLED: Glyph = A.glyph('x');
    const OUTLINED: Glyph = IconFamily::new("b").glyph('x');

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
        assert_eq!(A.font(&[]).family(), FILLED.family());
    }
}
