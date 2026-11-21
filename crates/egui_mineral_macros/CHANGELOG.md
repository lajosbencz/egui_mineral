# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.8.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_macros-v0.7.0...egui_mineral_macros-v0.8.0) - 2026-11-21

### Other

- drop comments that restate code and the stale speedup figure; keep invariants, quirks and spec pointers.
- *(core)* [**breaking**] typed i18n::Locale replaces locale strings in I18n, Localizer, LocaleFormat and both formats; locale! checks tags at compile time and messages! emits msg::CATALOG tagged by its file name.
- [**breaking**] value-keyed ids and agent keys for every value collection with one ItemKey bound, typed Combobox/Carousel/TransferList bindings, kit modules renamed off core names, and typed Form values (FieldValue, FormValues, checkbox/select/date kinds).
- [**breaking**] bindings are new/optional/multi with item(value, x) builders, accordions, lists and tables key state by app keys (table::Rows<K>), and Material Combobox/Search report values; adds #[derive(KeySalt)], Fluent FormExt, own dnd/child responses and constructors for non_exhaustive ComboList, PullInput, Surroundings and OverflowItem.

## [0.7.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_macros-v0.6.0...egui_mineral_macros-v0.7.0) - 2026-11-20

### Other

- Merge remote-tracking branch 'origin/master'
- *(core)* [**breaking**] nav::__private::segment drops its unused field parameter.
- *(i18n)* [**breaking**] typed message ids generated from the source catalog by i18n::messages!; I18n::tr, kit::tr and ValidationError::localized take them instead of string ids. Stylekit catalogs inherit the core message schema at compile time and unused catalog messages fail dead_code.

## [0.6.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_macros-v0.5.0...egui_mineral_macros-v0.6.0) - 2026-11-19

### Added

- *(core)* [**breaking**] persist-fs no longer implies the codecs, defaults ship persist-toml only, so serde_json is opt-in via persist-json.

## [0.5.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_macros-v0.4.0...egui_mineral_macros-v0.5.0) - 2026-11-19

### Other

- bump published crates to 0.5.0 for the dismissal behavior change.

## [0.4.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_macros-v0.3.0...egui_mineral_macros-v0.4.0) - 2026-11-19

### Other

- bump the published crates to 0.4.0 for the breaking theme change.

## [0.3.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_macros-v0.2.0...egui_mineral_macros-v0.3.0) - 2026-11-19

### Other

- bump published crates to 0.3.0 for the breaking release.
- per-crate versions and changelogs, caret workspace requirements, per-crate tags.
