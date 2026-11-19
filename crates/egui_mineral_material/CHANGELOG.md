# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_material-v0.5.0...egui_mineral_material-v0.6.0) - 2026-11-19

### Added

- *(material)* [**breaking**] default `extras` feature gates the non-MD3 components, ValueSlider::value_field, catalog entries and their tests; drag handle painter moves to containment.
- *(material)* [**breaking**] Accordion::variant with a flat section-header look; showcase groups the Inspector rows.
- *(material)* OtpField label_visible shows the label above the cells; no affixes, recorded in divergences.
- *(material)* [**breaking**] buttons, icon buttons, tabs, segmented buttons, chips and tree rows dense down to their measured content; remove COMPACT_FLOOR.
- *(material)* [**breaking**] MaterialIcon is a local type with From<MaterialIcon> for Icon; drop the core material-icons feature
- *(tools)* material_symbols generates the Material icon constants, checked by just spec
- *(material)* [**breaking**] Toolbar::custom content slots (measured, hidden on overflow, roving) on docked and floating toolbars.
- *(material)* [**breaking**] ValueSlider::value_field adds an inline NumberField bound to the slider value
- *(material)* [**breaking**] modal SideSheet is resizable like the standard one, width persists per sheet id.
- *(material)* [**breaking**] DateField and TagsField paint affixes through the shared field chrome and take label_visible; OtpField cells follow density
- *(material)* [**breaking**] icons take impl Into<Icon>, kit glyphs come from IconSet, Material::with_icons takes any IconSet and icon fonts.
- *(material)* [**breaking**] NumberField, Combobox, DatePicker, ColorPicker and InlineEdit forward the TextField chrome
- *(material)* [**breaking**] density down to -6 clamping at content minimum, TextField::label_visible and text chrome in FieldChrome
- *(core)* [**breaking**] shared icon types, IconFont installer and glyph shape helper.
- *(core)* kit::semantic::emit_culled and VirtualList::show_culled; Material TreeView keeps off-screen rows reachable to agents.
- *(core)* [**breaking**] persist-fs no longer implies the codecs, defaults ship persist-toml only, so serde_json is opt-in via persist-json.

### Fixed

- *(material,fluent)* Escape after switching menus returns focus to the open menu's title, which becomes the bar's Tab stop.
- *(material)* egui_material_icons is optional behind bundled-icons, so builds without it skip the dependency.
- box toolbar slot base, rustfmt, readme and doc lint fixes for icon sets.
- *(material,fluent)* satisfy private-doc and builder-unchecked-float rules
- *(core,material)* clippy similar_names in state eviction, TreeView culled emit extracted.
- *(material)* accordion keeps one height list per widget, pruned to its panels.
- *(core)* SetValue warnings log the widget id and text length, not the text.
- *(material,fluent)* table paging, password reveal and menu sizing discard only in pass 0.
- *(material)* dialog discards once per growth burst instead of every frame while content grows.

### Other

- material API future-proofing; Elide moves to the core and StatusTone::Custom replaces StatusItem::colors
- *(material)* [**breaking**] future-proof the public API: non_exhaustive option and slot enums, slice ALL lists, sealed ext traits, Material::new() with builder setters, TextAction, label-taking picker and search constructors, variant enums instead of bool flags and one name per setter concept. Prelude exports responses and outputs.
- *(core)* [**breaking**] Stylekit::window_close returns the close glyph shapes with a kit-styled default; the core submits them, so kits need no override.
- *(core)* [**breaking**] Stylekit::paint_window_close stays required, so the core never paints; kits supply the default chrome.
- *(core)* [**breaking**] future-proof the public API with non_exhaustive configs, payloads and responses, private fields behind constructors (Icon, PickedFile, FileFilter, Catalog, SaveRequest) and enums instead of positional bools; shared SelectionResponse, status bar and command palette data move into the core and both kits re-export them.
- *(core)* kit::palette, kit::menubar and kit::strip hold the palette, menu bar and status strip models Material's extras used.
- *(material)* extras tests stay in behavior and spec, which require the feature, instead of two duplicated binaries.
- default-features leg tests the libraries without showcase feature unification; smaller culling fixtures, thinner holds_up matrix, agent and route tests join behavior.
- Merge branch 'worktree-agent-a84b4023cee9248d6'
- private-doc lint.
- Merge branch 'worktree-agent-a36e4fcc22cc574d6'
- *(material)* MaterialIcon docs state facts
- *(material)* cover color picker and OTP density forwarding; mutation-checked density, label_visible, affix and forwarding tests.
- *(core,material)* pull-to-refresh and sheet handle read the per-pass input; document remaining Core::input callers
- *(core)* window close glyph shapes live in kit::paint, kits only add them
- Merge branch 'worktree-agent-a27a0aca72025ecf8'
- Merge branch 'worktree-agent-a5d70f788e9a9e74e'
- *(material,fluent)* Density::dense_to, AffixCx and extracted helpers to satisfy clippy and fmt
- cross-kit icon sets, showcase-fluent MINERAL_ICONS=material, icon set tests.
- Merge branch 'worktree-agent-a9996d56bdb434124'
- Merge branch 'worktree-agent-ae375825d85bd2df9'
- lint fixes for windows harness and JsError.
- *(material)* TreeView drop-expand extracted to satisfy too_many_lines.
- fmt and line-count lint fixes for the dialog, menu and test changes.
- *(fluent,material)* [**breaking**] Tokens::shadow takes ShadowTint, StatusItem::progress takes StatusProgress replacing linear_progress.
- *(core)* [**breaking**] PanClaim::update takes the MineralUi; kit interact, card and number field read the per-pass cached input, MinTarget::points_for skips the lookup.
- *(core)* [**breaking**] PanClaim::update drops the enabled flag for PanClaim::reset, PanGate holds one state enum, both join kit::prelude.
- *(core)* [**breaking**] declare_modal and is_modal are crate-private; kit::overlay::modal_on and outside_press_after_open replace them for kits.
- *(core)* shared NumberPass::consume_step_keys, kit::i18n::tr_or, kit::rating::value_at; Fluent menus share text_width.
- *(material)* TreeView culled rows stay agent-reachable without widgets.
- *(core)* kit::scroll::page replaces the duplicated Material and Fluent page containers.
- *(material)* window harness reports every open viewport in RawInput like integrations do.
- *(core)* PassCache freezes per-pass derived state; Fluent style() shares the pass snapshot, so set_brand mid-pass never mixes styles.

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
