# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_material-v0.4.1...egui_mineral_material-v0.5.0) - 2026-11-19

### Added

- *(core)* [**breaking**] MineralWidget::pan_axis routes single-axis touch drags through PanClaim so off-axis swipes scroll the page; slider, switch and color bars migrate.
- *(core)* [**breaking**] kit::overlay::modal builds egui::Modal on modal_area, raw-egui-area flags Modal::new.
- *(core)* [**breaking**] embedded windows get an owned close control with a modality-sized hit target, Stylekit::paint_window_close draws its glyph, WindowSpec::show requires Stylekit.
- *(material)* StatusItem::linear_progress shows a determinate or indeterminate linear indicator before the text.

### Fixed

- *(core)* PanClaim relays pans and coasts through the owner widget's Ui so sibling scroll areas never take them, unserved deltas fall back to the root Ui.
- *(core)* [**breaking**] PanClaim relays pans and coasts from the widget's own Ui, so only its nearest scrolling ancestor scrolls.
- *(material)* modal sheets and drawers declare their floating layer, so they bind in their own scope from the opening pass.
- *(fluent)* menus opened by code in the same pass as a press survive it, and material Menu uses the core opening-frame guard.
- *(core)* embedded WindowSpec windows get a close button with a pointer-sized hit target
- *(material)* carousel claims only its own axis of a pan and chains the overshoot to the enclosing scroll
- *(core)* a popup ignores presses of its opening frame and Popover::show_at focuses the first focusable of its content.

### Other

- *(material)* style() shares the per-pass token snapshot, so a mid-pass setter never mixes schemes; fix perf doc lints.
- *(material,fluent)* table select-all counts the selection, carousel discards only on pass 0 and re-measures on width change and skips off-screen autoplay wakes, button group measures once, data table reuses its cell buffer.
- *(material,fluent)* [**breaking**] ui.tokens() serves the tokens and scheme frozen for the egui pass; Material and Fluent setters apply from the next pass.
- *(material)* TreeView locates pending, stop and selected rows in one scan and clones the visual order only for Shift-click.
- *(material)* carousel and swipeable list rows use core PanClaim, so a vertical flick over them keeps the page coasting.
- bump published crates to 0.5.0 for the dismissal behavior change.

## [0.4.1](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_material-v0.4.0...egui_mineral_material-v0.4.1) - 2026-11-19

### Other

- updated the following local packages: egui_mineral

## [0.4.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_material-v0.3.1...egui_mineral_material-v0.4.0) - 2026-11-19

### Added

- *(material)* [**breaking**] Material::new and Default exist only with bundled-icons, Material::with_icon_fonts is always available.
- *(core)* [**breaking**] split persist-fs into persist-json, persist-toml and native-only FsStorage.
- *(core)* split platform-native into platform-pickers, -clipboard and -notify, add FileFilter::file_name, route save_file through the filter.
- *(material)* Popover::show_at anchors at a rect without a toggle.
- *(material)* SideSheet edge, scroll and content_padding.
- *(material)* StatusItem tone, colors and elide.
- *(core)* [**breaking**] runtime owns the theme preference (default System) and passes the resolved theme to Stylekit::style; Material::new takes a Contrast, showcase cycles System/Light/Dark.

### Fixed

- *(core)* keep the touch modality through eframe's touch-less pointerdown pass, switch to mouse only on hover or wheel evidence.
- doc comments satisfy restating-doc.

### Other

- cargo-hack feature matrix for clippy, default-feature test run, wasm32 docs.rs build and explicit docs.rs features.
- move the layout tests into the spec binary so features gate tests in Cargo.toml only.
- bump the published crates to 0.4.0 for the breaking theme change.

## [0.3.1](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_material-v0.3.0...egui_mineral_material-v0.3.1) - 2026-11-19

### Other

- updated the following local packages: egui_mineral

## [0.3.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_material-v0.2.0...egui_mineral_material-v0.3.0) - 2026-11-19

### Added

- *(material)* [**breaking**] collection items take an optional salt, defaulting to their command ActionId, so inserting items keeps later agent keys and egui ids stable.
- *(core)* RangeModel increment and fraction_digits split the keyboard unit and decimal precision from the min-anchored step grid.

### Fixed

- *(core)* [**breaking**] focused fields adopt bound values the app changed during an edit via EditBuffer::wrote.

### Other

- bump published crates to 0.3.0 for the breaking release.
- per-crate versions and changelogs, caret workspace requirements, per-crate tags.
