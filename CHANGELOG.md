# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0](https://github.com/lajosbencz/egui_mineral/compare/v0.1.0...v0.2.0) - 2026-11-19

### Added

- *(core)* [**breaking**] NumberSource tells typed text from steps, the wheel, scrubs and AccessKit; NumberFieldOutput reports it.
- *(core)* RangeModel::significant rounds to significant digits, a value grid for log ranges; labels show the value's own digits on it.
- *(showcase)* subset Material Symbols at build time to the icons in use (3 MB to 38 KB).

### Fixed

- *(core)* press-to-focus also follows a click whose press and release arrive in one pass.
- *(material)* compact horizontal Stepper elides the current label instead of overflowing narrow widths.
- *(material)* key widget child scopes by the widget's salt, so sibling instances don't share scroll state.

### Other

- *(showcase)* render with glow instead of wgpu; the web bundle drops 0.8 MB gzipped.
- *(core)* [**breaking**] SizeExt fixed_* renamed preferred_*; the parent's constraints win, as with Compose Modifier.width.
- *(lints)* allow-indexing-slicing-in-tests replaces the cfg_attr(not(test)) lint gates in core modules.
- [**breaking**] remove error_boundary; panics are bugs, reported by the app's hook rather than caught.
- README size table measured on the deployed showcase; ICU data is 0.8 MB, not 1.8 MB.
- *(material)* full-width composite guard runs at each window class's narrowest width, read from the breakpoint policy.
- *(material)* full-width composites settle within their page once the page bar appears.

## [0.1.0](https://github.com/lajosbencz/egui_mineral/compare/v0.0.0...v0.1.0) - 2026-11-19

### Added

- implement core and material stylekit

### Other

- init
