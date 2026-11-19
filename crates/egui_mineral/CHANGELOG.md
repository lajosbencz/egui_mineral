# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.5.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral-v0.4.1...egui_mineral-v0.5.0) - 2026-11-19

### Added

- *(core)* [**breaking**] MineralWidget::pan_axis routes single-axis touch drags through PanClaim so off-axis swipes scroll the page; slider, switch and color bars migrate.
- *(core)* [**breaking**] kit::overlay::modal builds egui::Modal on modal_area, raw-egui-area flags Modal::new.
- *(core)* [**breaking**] embedded windows get an owned close control with a modality-sized hit target, Stylekit::paint_window_close draws its glyph, WindowSpec::show requires Stylekit.
- *(core)* PanClaim claims a pan on one axis and relays the rest, the overshoot and the release fling to the enclosing scroll area.
- *(core)* opt-in GlobalStyle::Kit writes the kit style and theme preference into egui's globals from the outermost pass; default stays Egui

### Fixed

- *(core)* request a repaint while queued agent acts wait for a tree after a non-collecting pass
- *(core)* PanClaim relays pans and coasts through the owner widget's Ui so sibling scroll areas never take them, unserved deltas fall back to the root Ui.
- *(core)* [**breaking**] PanClaim relays pans and coasts from the widget's own Ui, so only its nearest scrolling ancestor scrolls.
- *(core)* library modals bind shortcuts and mnemonics in their own scope from the pass they open in.
- *(core)* embedded WindowSpec windows get a close button with a pointer-sized hit target
- *(core)* a popup ignores presses of its opening frame and Popover::show_at focuses the first focusable of its content.
- *(core)* fraction digits beyond f64 precision leave the value, so snapping stays idempotent.

### Other

- *(material)* style() shares the per-pass token snapshot, so a mid-pass setter never mixes schemes; fix perf doc lints.
- *(core)* input adapter translates events under ctx.input and shares the empty event slice
- *(core)* semantic wanted check takes one ctx.data read and skips AccessKit when collecting
- *(core)* shortcut and mnemonic duplicate detection through a hash index
- *(core)* MineralUi fetches the pass input once and widget state is moved out of temp data instead of cloned.
- *(core)* composites discard only for resizes over half a point and only while visible.
- *(core)* reword PanClaim::relaying doc.
- *(core)* rustfmt runtime imports.
- window close control comment as spec-free quirk note.
- regenerate egui_mineral README for the Stylekit hook.
- *(core)* drop doc comments on crate-private pan relay items.
- bump published crates to 0.5.0 for the dismissal behavior change.
- *(core)* [**breaking**] overlay::dismissal ignores presses in the frame the overlay opened, outside_press_after_open covers kit-owned checks.

## [0.4.1](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral-v0.4.0...egui_mineral-v0.4.1) - 2026-11-19

### Added

- *(core)* FileFilter::directory resolves when the dialog opens (an existing file proposes itself in its folder, a missing path opens its parent), FileFilter::start names that intent.
- *(core)* PlatformPathPickers returns picked file paths without reading them.

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
