use egui::{Context, FontData, FontFamily, FontInsert, FontPriority, InsertFontFamily};
use egui_mineral::icon::{IconFamily, IconFont};

// `egui_material_icons` family names, which `Fonts::Bundled` registers
pub(crate) const FONT_FAMILY_FILLED: IconFamily = IconFamily::new("material-icons");
pub(crate) const FONT_FAMILY_OUTLINED: IconFamily = IconFamily::new("material-icons-outlined");
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

/// Installs the bundled Material Symbols fonts, the ones [`Material::new`](crate::Material::new) uses.
#[cfg(feature = "bundled-icons")]
pub fn install_bundled_icons(ctx: &Context) {
    egui_material_icons::initialize(ctx);
}

/// Material Symbols fonts the kit draws icons with: `Material::from_icon_fonts(IconFonts::new(..).fonts())`
/// ([`Material::from_icon_fonts`](crate::Material::from_icon_fonts)).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct IconFonts {
    /// Font of filled icons, the default style.
    pub filled: &'static [u8],
    /// Font of outlined icons.
    pub outlined: &'static [u8],
}

impl IconFonts {
    /// Font files of the filled and the outlined style.
    pub const fn new(filled: &'static [u8], outlined: &'static [u8]) -> Self {
        Self { filled, outlined }
    }

    /// Fonts under the families `MaterialIcon` draws with, with the baseline offset of `egui_material_icons`.
    pub const fn fonts(self) -> [IconFont; 2] {
        // egui_material_icons' baseline tweak
        const Y_OFFSET: f32 = 0.05;
        [
            FONT_FAMILY_FILLED.font(self.filled).y_offset_factor(Y_OFFSET),
            FONT_FAMILY_OUTLINED.font(self.outlined).y_offset_factor(Y_OFFSET),
        ]
    }
}
