use egui::{Context, FontData, FontFamily, FontInsert, FontPriority, InsertFontFamily};

pub(crate) const SEMIBOLD: &str = "mineral::semibold";
pub(crate) const ICONS: &str = "mineral::fluent-icons";

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

/// Family the kit draws [`FluentIcon`](crate::FluentIcon)s with.
pub fn icon_family() -> FontFamily {
    FontFamily::Name(ICONS.into())
}

pub(crate) fn install_icons(ctx: &Context, ttf: &'static [u8]) {
    ctx.add_font(FontInsert::new(
        "FluentSystemIcons-Resizable",
        FontData::from_static(ttf),
        vec![
            InsertFontFamily {
                family: FontFamily::Proportional,
                priority: FontPriority::Lowest,
            },
            InsertFontFamily {
                family: icon_family(),
                priority: FontPriority::Highest,
            },
        ],
    ));
}
