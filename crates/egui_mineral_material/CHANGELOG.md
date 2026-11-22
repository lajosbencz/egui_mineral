# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.12.2](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_material-v0.12.1...egui_mineral_material-v0.12.2) - 2026-11-22

### Added

- *(overlay)* core overlay::ModalFocus replaces both kits' copies; kit::layer::default_action_pressed_with lets a kit pick which focused widgets keep Enter.

## [0.12.1](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_material-v0.12.0...egui_mineral_material-v0.12.1) - 2026-11-22

### Added

- *(select)* select_all(false) on TreeView, DataTable, Fluent Table/DataGrid and List leaves Ctrl/Cmd+A to the app (React Aria disallowSelectAll).

### Fixed

- *(menu)* Walk::Bar on End from a submenu leaf moves to the next bar menu; Fluent MenuBar and Material MenuStrip consume the arrow that opens a menu so it opens on the first row.

## [0.12.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_material-v0.11.0...egui_mineral_material-v0.12.0) - 2026-11-22

### Added

- *(select)* [**breaking**] Enter never selects and AccessKit clicks get RowSource::Assistive; Fluent List action_selects, Fluent toggles ignore Enter, TreeView collapse keeps focus shown, semantic multiselectable state.
- *(select)* [**breaking**] SelectionBehavior (Toggle, Replace) drives Selection::apply and follow_focus with mouse, touch tap/long press, Space and Enter sources and focus-scoped Ctrl/Cmd+A; Material TreeView/DataTable and Fluent Table/DataGrid/List adopt it, Fluent List arrows keep focus.
- *(select)* RowInput and Selection::apply map a row's press and click onto Selection in core; Fluent Table rows select on press like the Material TreeView.
- *(select)* Selection::follow_focus, select::is_select_all and RovingFocus::on_select_key flatten keyboard selection into core; Fluent DataGrid Shift+arrows extend and Ctrl+A selects all, Material TreeView shares them.
- *(select)* [**breaking**] Selection tracks a current key apart from the anchor (current, set_current, anchor), press/release keep a dragged multiple selection, extend_to and select_all; tree::on_key takes modifiers and reports Focus { row, extend } and SelectAll. Material TreeView selects on press, Shift+arrows extend, Ctrl+A selects all.
- *(menu)* [**breaking**] core kit::menu::MenuNav owns the open submenu path, hover intent, arrow walk and opening focus; Material and Fluent menus run on it. Fluent MenuResponse::nav reports root-level arrows.
- *(shortcut)* [**breaking**] Mnemonic::parse reads &-marked labels and underlines the marked letter; non-ASCII mnemonics match typed text; items sharing a letter cycle focus. Kit builders take impl Into<Mnemonic>.

## [0.11.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_material-v0.10.0...egui_mineral_material-v0.11.0) - 2026-11-22

### Fixed

- *(input)* [**breaking**] touch swipes scroll pages over 2D pannables (PanMode::Free claims only after GestureConfig::pan_hold) and nested scroll areas chain unused drag and fling to the page (kit::scroll::chain).
- *(overlays)* [**breaking**] popups, menus, tooltips, search and Material dialogs/pickers fit small viewports; Popup::place caps to the anchor room, dialogs scroll when taller.

## [0.10.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_material-v0.9.0...egui_mineral_material-v0.10.0) - 2026-11-21

### Added

- *(tokens)* Material Tokens deserialize with only color required; shape, state and duration keys default when missing, durations exactly.
- *(menu)* Material Menu::radios(current, items) builds a radio group checked at the current value.
- *(tree)* [**breaking**] accept_drop adds a payload type per call (DropAccept::push), first acceptor wins; Fluent TreeResponse::rows is ItemResponses like Material.
- *(splitter)* [**breaking**] collapsible(pane) Enter toggle per APG, CollapseRequest from Enter, drag and double click on a collapsed handle, hit zone at the collapsed edge; both kits' SplitPane.
- *(window)* [**breaking**] resize_handle, resize_border, end_corner and can_resize replace status::resize_grip; status bar grips hide while maximized or fullscreen.
- *(form)* [**breaking**] FieldPlacement and LabelSlot compose forms with layouts; core FormRows and StackBelow; FormGrid::show_form, stacking and labelled_by rows, FormUiExt::section, in both kits.
- *(material)* checkbox and primary tab indicator radii follow the theme's extra_small shape step.
- *(menu)* Material MenuItem check and radio toggles in their own column beside the leading icon and shortcut; Fluent Menu::radio_dot.
- *(status)* StatusBar tone() for the whole bar and grip(bool) window resize corner (core resize_grip, grip_dots), in both kits.
- *(table)* row_style tints and per-row responses in both kits' tables; Material DataTable stick_to_end via VirtualList::stick_to_end.
- *(splitter)* Enter and double click reset to the default fraction (SplitterResponse::reset), collapse(Option<Pane>), in both kits' SplitPane.
- *(form)* FormGrid in both kits, a two-column form with a shared label column (core LabelColumn), section headings and full-width rows.
- *(number)* optional (blank) number fields, pixel scrub rate with log doubling, clamped output and select-on-step, in Material NumberField and Fluent SpinButton.
- *(tree)* disabled nodes, foreign drops (DropAccept, TreeDrop), reveal, and Material unfocused selection color, in both kits' trees.

### Fixed

- *(dnd)* drags sense click_and_drag so a press is no drag (tree preview flicker, carousel cursor, table grip clicks sorting); grips sit above headers and follow the pointer via input::drag_from_start; drag-only-sense lint.
- *(input)* hand-rolled hit tests and tap/pan respect overlays and modals via reaches_layer and InputContext::events_on.

### Other

- *(material)* checkbox and tab indicator radii follow the extra_small shape token.
- *(table)* [**breaking**] both kits' table rows are ItemResponses, keyed like tree and toolbar responses.
- [**breaking**] core Tone (from StatusTone) and RowStyle for both kits' row_style, RadioMark and keep-open menu toggles, ScrubRate::Pixels per scale unit with one scrub(rate) builder; Fluent Text inherits override_text_color.
- *(number)* NumberEdit::begin takes the NumberRef (blank mode from Optional), NumberEdit::finish writes back and reports the change for both kits.

## [0.9.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_material-v0.8.0...egui_mineral_material-v0.9.0) - 2026-11-21

### Added

- *(stylekits)* forward core platform, spawn and web-save-stream features so apps depend on one stylekit only.

### Fixed

- *(form)* [**breaking**] NumberRange is non_exhaustive with at_least, at_most and between constructors.

### Other

- guard published manifests against unstable features, mark unstable modules in rustdoc, fix the bump-breaking doc comment.
- *(tools)* fetch spec sources at committed pins instead of vendoring them in git.

## [0.8.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_material-v0.7.0...egui_mineral_material-v0.8.0) - 2026-11-21

### Added

- *(core)* [**breaking**] AgentHandle::snapshot returns the SemanticTree, act_then_snapshot returns the tree after the UI reacted, NotFound lists similar keys, and Runtime::viewports lists viewports; SemanticTree serializes as nested JSON.
- *(core)* [**breaking**] semantic tree nests by egui's widget hierarchy, widget parts and popup owners, replacing emit_parent and hand-set depth; containers emit with their Ui id so AccessKit gets their roles, and tree items carry level.
- *(material)* field tooltip setter, NumberField leading icon, container_color on TreeView and ListItem.

### Other

- drop comments that restate code and the stale speedup figure; keep invariants, quirks and spec pointers.
- [**breaking**] Steps<K>, Stepper<K> and Tour<K> key flows by app values like Selection<K>; step labels, responses and the bound tour step carry keys, not indices.
- *(core)* [**breaking**] typed i18n::Locale replaces locale strings in I18n, Localizer, LocaleFormat and both formats; locale! checks tags at compile time and messages! emits msg::CATALOG tagged by its file name.
- *(material)* [**breaking**] Menu::show binds open: &mut bool like every click-anchored popup, so apps can open it from code; hover and context popups keep internal state.
- [**breaking**] Stylekit::chrome_icon(ChromeIcon) replaces the window_close shape hook; core paints the kit's glyph or its vector cross, both kits supply their close/dismiss icon.
- [**breaking**] both kits build with new() or from_icon_fonts(fonts) plus with_icons(IconSet) and with_* start values; Material::set_density takes the Context and repaints.
- *(core)* [**breaking**] forms take numbers (Blank for numeric primitives, NumberRange over Numeric, kit FormUiExt::number), Form::check becomes Form::rule, FormState::field/field_mut, CheckError is opaque and finish_submit takes per-field errors.
- [**breaking**] NumberField, ValueSlider and Fluent Slider and SpinButton bind any emath::Numeric with typed format closures; core adds RangeModel::for_numeric and range::to_numeric.
- *(core)* [**breaking**] forms bind the app's value: Form<T> fields bind &mut V parts through the kits' FormUiExt with typed Validator<V>s and Fn(&T) rules, FormState keeps only meta state; FieldValue, FormValues, FieldKind and FieldOption are removed.
- *(core)* [**breaking**] CommandPalette and StatusBar pair entries with typed values (action/actions, trailing_action), ActionKey, Clicked and the item key setters are removed, and the action docs describe typed actions.
- *(material)* [**breaking**] Menu, MenuStrip, SplitButton, Dialog, FullScreenDialog, ButtonGroup, FabMenu and Breadcrumbs report typed item values; submenus move to Menu::submenu, default_action takes a value, MenuPath is removed.
- *(material)* [**breaking**] icon and text action collections pair each action with a typed app value (app bar, toolbars, tree view, alert, rich tooltip, snackbar), reported as clicked: Option<A> and ItemResponses<A, T> keyed by value.
- *(core)* [**breaking**] roving focus takes focus::Items (count plus enabled) instead of loose len/enabled, kit::menubar::adjacent steps toward a layout::Edge, and kit::scroll::page takes named PageBars.
- *(core)* [**breaking**] SortState is opaque behind column/by/clear/cycle so it can grow multi-column sort, and kit::command_palette::Shortcuts is a non_exhaustive builder.
- *(material)* [**breaking**] drop the root and prelude SelectionResponse re-exports; the core prelude already provides it.
- [**breaking**] value-keyed ids and agent keys for every value collection with one ItemKey bound, typed Combobox/Carousel/TransferList bindings, kit modules renamed off core names, and typed Form values (FieldValue, FormValues, checkbox/select/date kinds).
- [**breaking**] bindings are new/optional/multi with item(value, x) builders, accordions, lists and tables key state by app keys (table::Rows<K>), and Material Combobox/Search report values; adds #[derive(KeySalt)], Fluent FormExt, own dnd/child responses and constructors for non_exhaustive ComboList, PullInput, Surroundings and OverflowItem.
- [**breaking**] selection groups bind item values through kit::choice::Choice and return a generic SelectionResponse<K>; index binding, bound constructors and stale-index clamping are removed.
- [**breaking**] non_exhaustive on growth-prone enums and configs, Fluent tokens only under spec, KeyKind agent kinds checked at compile time, DockedToolbar/FloatingToolbar constructors, EframeApp::show removed.

## [0.7.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_material-v0.6.0...egui_mineral_material-v0.7.0) - 2026-11-20

### Added

- *(material)* [**breaking**] action items take .key(..) and collections report action::Clicked, per-item responses move behind accessors, Menu::show_at takes &Rt, MaterialUi alias. Select::value and SegmentedButton::value bind option values.
- *(core)* [**breaking**] opt-in spawn-tokio and spawn-wasm Spawner adapters that own their tasks, count setters take plain integers clamped to at least 1, and the prelude exports setup errors. Showcases use the adapters and borrow the runtime per pass; docs record action keys, layout wrappers, the Stylekit openness and the MineralWidget::Kit divergence.
- *(core)* [**breaking**] MineralUi layout wrappers hand closures a MineralUi, and action items carry an app key reported as action::Clicked (PaletteCommand::key, CommandPaletteResponse::clicked). The prelude exports InitError and Clicked.
- *(material)* [**breaking**] density applies to every component: Stylekit::min_target shrinks all minimum hit targets 4dp per step, and sliders, top app bar, navigation items, split button, menu strip and status bar containers follow density. Core adds Stylekit::min_target and MineralUi::target; the min-target-points lint guards it.
- *(material)* [**breaking**] density-following Toolbar heights and CheckButton/Radio/Switch hit targets, TreeView::node_text_style, text_align and tabular_figures on fields, InlineEdit::editing, SideSheet::initial_width. Core: MineralWidget::min_target takes the ui, kit::text::tabular_figures, StatusItem::tabular_figures.

### Other

- Merge remote-tracking branch 'origin/master'
- *(kits)* Fluent menu rows, enum-indexed tables, positional bool flags and the md3 shape-side match become typed; shadow_dp sanitizes its input.
- *(kits)* [**breaking**] Fluent list options, swatches and tags carry typed app values instead of strings, Material breadcrumbs report Clicked, shared id salts derive from one helper.
- *(core)* [**breaking**] nav::__private::segment drops its unused field parameter.
- master into typed message ids
- *(i18n)* [**breaking**] typed message ids generated from the source catalog by i18n::messages!; I18n::tr, kit::tr and ValidationError::localized take them instead of string ids. Stylekit catalogs inherit the core message schema at compile time and unused catalog messages fail dead_code.
- [**breaking**] typed keys and values: persist::PersistKey<T>, IconFamily as the source of Glyph and IconFont, layout::Fraction, UndoCommand::merge(&Self), MineralUi::with_scoped, TypeId-keyed pass state and Setting<T>. MinTarget::points is crate-private and ColorScheme::role replaces ColorRole::resolve, retiring their lint rules.
- [**breaking**] typed action keys (action::ActionKey, Clicked::key::<K>), ItemResponses for per-item response values, keyed StatusItem and TagGroup, named Fluent container responses and SemanticNode::shortcut as KeyboardShortcut. No response mixes pub and private fields.
- fluent API ergonomics pass; value-bound pickers are `bound(..)` in both kits

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
- *(core)* kit::nodes::emit_culled and VirtualList::show_culled; Material TreeView keeps off-screen rows reachable to agents.
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
- *(core)* [**breaking**] declare_modal and is_modal are crate-private; kit::layer::modal_on and outside_press_after_open replace them for kits.
- *(core)* shared NumberPass::consume_step_keys, kit::tr_or, kit::rating::value_at; Fluent menus share text_width.
- *(material)* TreeView culled rows stay agent-reachable without widgets.
- *(core)* kit::scroll::page replaces the duplicated Material and Fluent page containers.
- *(material)* window harness reports every open viewport in RawInput like integrations do.
- *(core)* PassCache freezes per-pass derived state; Fluent style() shares the pass snapshot, so set_brand mid-pass never mixes styles.

## [0.5.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_material-v0.4.1...egui_mineral_material-v0.5.0) - 2026-11-19

### Added

- *(core)* [**breaking**] MineralWidget::pan_axis routes single-axis touch drags through PanClaim so off-axis swipes scroll the page; slider, switch and color bars migrate.
- *(core)* [**breaking**] kit::layer::modal builds egui::Modal on modal_area, raw-egui-area flags Modal::new.
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
