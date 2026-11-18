use egui::{Context, FontData, FontFamily, FontInsert, FontPriority, InsertFontFamily};
use egui_material_icons::{FONT_FAMILY, FONT_FAMILY_OUTLINED};

pub(crate) const MEDIUM: &str = "mineral::medium";

pub(crate) fn has_medium(ctx: &Context) -> bool {
    ctx.fonts(|f| f.definitions().families.contains_key(&FontFamily::Name(MEDIUM.into())))
}

/// Registers `ttf` as the medium-weight family; titles and labels use it instead of synthesized bold.
pub fn install_medium(ctx: &Context, name: &str, ttf: &'static [u8]) {
    ctx.add_font(FontInsert::new(
        name,
        FontData::from_static(ttf),
        vec![InsertFontFamily {
            family: FontFamily::Name(MEDIUM.into()),
            priority: FontPriority::Highest,
        }],
    ));
}

/// Material Symbols fonts the kit draws icons with ([`Material::icon_fonts`](crate::Material::icon_fonts)).
#[derive(Clone, Copy, Debug)]
pub struct IconFonts {
    /// Font of filled icons, the default style.
    pub filled: &'static [u8],
    /// Font of outlined icons.
    pub outlined: &'static [u8],
}

pub(crate) fn install_icons(ctx: &Context, fonts: IconFonts) {
    for (family, ttf) in [(FONT_FAMILY, fonts.filled), (FONT_FAMILY_OUTLINED, fonts.outlined)] {
        let mut data = FontData::from_static(ttf);
        // egui_material_icons' baseline tweak
        data.tweak.y_offset_factor = 0.05;
        ctx.add_font(FontInsert::new(
            family,
            data,
            vec![
                InsertFontFamily {
                    family: FontFamily::Proportional,
                    priority: FontPriority::Lowest,
                },
                InsertFontFamily {
                    family: FontFamily::Name(family.into()),
                    priority: FontPriority::Highest,
                },
            ],
        ));
    }
}
