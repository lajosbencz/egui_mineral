# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral-v0.3.1...egui_mineral-v0.4.0) - 2026-11-19

### Added

- *(core)* [**breaking**] split persist-fs into persist-json, persist-toml and native-only FsStorage.
- *(core)* split platform-native into platform-pickers, -clipboard and -notify, add FileFilter::file_name, route save_file through the filter.
- *(material)* Popover::show_at anchors at a rect without a toggle.
- *(core)* [**breaking**] runtime owns the theme preference (default System) and passes the resolved theme to Stylekit::style; Material::new takes a Contrast, showcase cycles System/Light/Dark.

### Fixed

- *(core)* keep the touch modality through eframe's touch-less pointerdown pass, switch to mouse only on hover or wheel evidence.
- doc comments satisfy restating-doc.
- *(core)* gate platform unsupported tests per feature.
- *(core)* gate platform unsupported tests.
- *(core)* log range steps from the current value, round decimal ties away from zero, honor jump and add up stalled steps.

### Other

- cargo-hack feature matrix for clippy, default-feature test run, wasm32 docs.rs build and explicit docs.rs features.
- *(core)* [**breaking**] split PlatformService into per-capability traits implemented only with their feature, drop PlatformError::Unsupported.
- bump the published crates to 0.4.0 for the breaking theme change.

## [0.3.1](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral-v0.3.0...egui_mineral-v0.3.1) - 2026-11-19

### Added

- *(core)* unstable-hotpatch feature wires Dioxus subsecond into Runtime for dx serve --hotpatch.

### Fixed

- *(core)* hotpatch recipe forces the relink dx 0.7.10 misses under cargo's build-dir layout; blocker recorded.

## [0.3.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral-v0.2.0...egui_mineral-v0.3.0) - 2026-11-19

### Added

- *(material)* [**breaking**] collection items take an optional salt, defaulting to their command ActionId, so inserting items keeps later agent keys and egui ids stable.
- *(core)* RangeModel increment and fraction_digits split the keyboard unit and decimal precision from the min-anchored step grid.

### Fixed

- *(core)* [**breaking**] focused fields adopt bound values the app changed during an edit via EditBuffer::wrote.

### Other

- bump published crates to 0.3.0 for the breaking release.
- per-crate versions and changelogs, caret workspace requirements, per-crate tags.
