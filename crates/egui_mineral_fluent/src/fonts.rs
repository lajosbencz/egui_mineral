use egui::{Context, FontData, FontFamily, FontInsert, FontPriority, InsertFontFamily};
use egui_mineral::icon::{IconFamily, IconFont};

pub(crate) const SEMIBOLD: &str = "mineral::semibold";

#[cfg(feature = "bundled-icons")]
pub(crate) const BUNDLED_ICONS: &[u8] = include_bytes!("../assets/FluentSystemIcons-Resizable.ttf");

pub(crate) fn has_semibold(ctx: &Context) -> bool {
    ctx.fonts(|f| {
        f.definitions()
            .families
            .contains_key(&FontFamily::Name(SEMIBOLD.into()))
    })
}

/// Registers `ttf` as the semibold family; strong ramp styles and button labels use it instead of the regular face.
pub fn install_semibold(ctx: &Context, name: &str, ttf: &'static [u8]) {
    ctx.add_font(FontInsert::new(
        name,
        FontData::from_static(ttf),
        vec![InsertFontFamily {
            family: FontFamily::Name(SEMIBOLD.into()),
            priority: FontPriority::Highest,
        }],
    ));
}

/// Family of [`FluentIcon`](crate::FluentIcon) glyphs.
pub const ICON_FAMILY: IconFamily = IconFamily::new("mineral::fluent-icons");

/// Fluent UI System Icons resizable font `ttf` (or a subset of it) as an installable [`IconFont`].
pub const fn icon_font(ttf: &'static [u8]) -> IconFont {
    ICON_FAMILY.font(ttf).name("FluentSystemIcons-Resizable")
}
