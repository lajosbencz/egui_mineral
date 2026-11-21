# Release size

Opt-outs for a smaller build, in the order of the [README table](../README.md#release-size).

## Icon fonts

Disable `bundled-icons` and build with `Material::from_icon_fonts` (plus `Material::with_icons` for another icon font). The showcase's [`build.rs`](../examples/showcase-material/build.rs) subsets the fonts to the icons it uses.

## Text fonts

Disable eframe's `default_fonts` feature and install your own fonts with `egui::Context::set_fonts`.

## Locale data

Set `ICU4X_DATA_DIR` to data for your locales ([`i18n`](https://docs.rs/egui_mineral/latest/egui_mineral/i18n/)), or disable `icu` (`AsciiFormat`).

## Renderer

Build eframe without its default features and enable `glow`, as the [showcase](../examples/showcase-material/Cargo.toml) does.
