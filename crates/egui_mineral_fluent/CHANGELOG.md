# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.9.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_fluent-v0.0.0...egui_mineral_fluent-v0.9.0) - 2026-11-21

### Added

- *(fluent)* publish egui_mineral_fluent under GPL-3.0-or-later AND MIT with the upstream notices.
- *(stylekits)* forward core platform, spawn and web-save-stream features so apps depend on one stylekit only.
- *(core)* [**breaking**] semantic tree nests by egui's widget hierarchy, widget parts and popup owners, replacing emit_parent and hand-set depth; containers emit with their Ui id so AccessKit gets their roles, and tree items carry level.
- *(fluent)* [**breaking**] action items take .key(..) and report action::Clicked (Dialog, Drawer, Menu, MenuBar, MenuGrid, Toolbar, Breadcrumb, Toast), TabList, ToolbarRadioGroup, Select and SwatchPicker gain value-bound ::bound forms. FluentUi alias, AvatarGroupResponse and MenuResponse; the prelude exports every response.
- *(core)* [**breaking**] opt-in spawn-tokio and spawn-wasm Spawner adapters that own their tasks, count setters take plain integers clamped to at least 1, and the prelude exports setup errors. Showcases use the adapters and borrow the runtime per pass; docs record action keys, layout wrappers, the Stylekit openness and the MineralWidget::Kit divergence.
- *(core)* [**breaking**] MineralUi layout wrappers hand closures a MineralUi, and action items carry an app key reported as action::Clicked (PaletteCommand::key, CommandPaletteResponse::clicked). The prelude exports InitError and Clicked.
- *(material)* [**breaking**] density-following Toolbar heights and CheckButton/Radio/Switch hit targets, TreeView::node_text_style, text_align and tabular_figures on fields, InlineEdit::editing, SideSheet::initial_width. Core: MineralWidget::min_target takes the ui, kit::text::tabular_figures, StatusItem::tabular_figures.
- *(showcase-fluent)* Extras page; docs: Fluent extras divergences.
- *(fluent)* MenuBar of subtle buttons opening Fluent Menus, sharing kit::menubar.
- *(fluent)* CommandPalette on the Dialog surface with SearchBox and Option row styling, sharing kit::palette.
- *(fluent)* Pagination from Fluent subtle buttons and the shared Spread model.
- *(fluent)* StatusBar with ProgressBar and Spinner leads; Spinner::label_visible.
- *(fluent)* default extras feature, catalog Origin and Category::Extras, SplitPane.
- *(material)* [**breaking**] MaterialIcon is a local type with From<MaterialIcon> for Icon; drop the core material-icons feature
- *(fluent)* [**breaking**] icons take impl Into<Icon>, kit glyphs come from IconSet, Fluent::with_icons takes any IconSet and icon fonts.
- *(fluent)* [**breaking**] SpinButton placeholder, content_before/after and unit suffix, content_before on Combobox, Dropdown, Select, TimePicker and DatePicker, Field::label_width
- *(core)* [**breaking**] shared icon types, IconFont installer and glyph shape helper.
- *(core)* kit::semantic::emit_culled and VirtualList::show_culled; Material TreeView keeps off-screen rows reachable to agents.
- *(core)* [**breaking**] persist-fs no longer implies the codecs, defaults ship persist-toml only, so serde_json is opt-in via persist-json.
- *(core)* [**breaking**] MineralWidget::pan_axis routes single-axis touch drags through PanClaim so off-axis swipes scroll the page; slider, switch and color bars migrate.
- *(core)* [**breaking**] embedded windows get an owned close control with a modality-sized hit target, Stylekit::paint_window_close draws its glyph, WindowSpec::show requires Stylekit.
- *(fluent)* scroll::page helper and a nested floating scroll style in the Fluent style, replacing the showcase margin workaround.
- *(stylekit)* add fluent 2 stylekit
- implement core and material stylekit

### Fixed

- *(form)* [**breaking**] NumberRange is non_exhaustive with at_least, at_most and between constructors.
- *(fluent)* RangeSlider thumbs are Tab stops, so arrow, Home and End keys reach them; a single Slider keeps the rail as its stop.
- *(material,fluent)* Escape after switching menus returns focus to the open menu's title, which becomes the bar's Tab stop.
- *(core)* kit doc comments state facts; test: status bar fixture robust to font widths.
- *(fluent)* SpinButton draws the unit right after the typed value, RTL-aware.
- box toolbar slot base, rustfmt, readme and doc lint fixes for icon sets.
- *(material,fluent)* satisfy private-doc and builder-unchecked-float rules
- drop the unused fluent shadow tint flag, doc links and private-doc lints after the merge.
- *(fluent)* image reports a load error once and paints a failed state instead of staying pending.
- *(fluent)* nav keeps one sub item map per widget, pruned to the placed categories.
- *(material,fluent)* table paging, password reveal and menu sizing discard only in pass 0.
- *(fluent)* Persona text lines wrap within the available width as usePersonaStyles block lines do.
- *(fluent)* MenuGrid rows size from their cells and share the widest row up to the 290 maximum instead of a fixed width.
- *(fluent)* Table and DataGrid emit row nodes for culled rows with ScrollIntoView so agents can reach them.
- *(fluent)* menus opened by code in the same pass as a press survive it, and material Menu uses the core opening-frame guard.
- *(fluent)* table cells clip their content and TableCellLayout truncates its lines with an ellipsis
- *(fluent)* card selection checkbox sits at the end top corner
- *(fluent)* dialog actions size from their natural width instead of the leftover row width

### Other

- *(fluent)* drop the catalog check that read the gitignored vendor sources.
- guard published manifests against unstable features, mark unstable modules in rustdoc, fix the bump-breaking doc comment.
- *(tools)* fetch spec sources at committed pins instead of vendoring them in git.
- drop comments that restate code and the stale speedup figure; keep invariants, quirks and spec pointers.
- *(core)* [**breaking**] typed i18n::Locale replaces locale strings in I18n, Localizer, LocaleFormat and both formats; locale! checks tags at compile time and messages! emits msg::CATALOG tagged by its file name.
- *(fluent)* [**breaking**] RangeSlider binds &mut [N; 2] of any Numeric like Material ValueSlider::range; RangeValue is removed.
- [**breaking**] Stylekit::chrome_icon(ChromeIcon) replaces the window_close shape hook; core paints the kit's glyph or its vector cross, both kits supply their close/dismiss icon.
- [**breaking**] both kits build with new() or from_icon_fonts(fonts) plus with_icons(IconSet) and with_* start values; Material::set_density takes the Context and repaints.
- *(core)* [**breaking**] forms take numbers (Blank for numeric primitives, NumberRange over Numeric, kit FormUiExt::number), Form::check becomes Form::rule, FormState::field/field_mut, CheckError is opaque and finish_submit takes per-field errors.
- [**breaking**] NumberField, ValueSlider and Fluent Slider and SpinButton bind any emath::Numeric with typed format closures; core adds RangeModel::for_numeric and range::to_numeric.
- *(core)* [**breaking**] forms bind the app's value: Form<T> fields bind &mut V parts through the kits' FormUiExt with typed Validator<V>s and Fn(&T) rules, FormState keeps only meta state; FieldValue, FormValues, FieldKind and FieldOption are removed.
- *(core)* [**breaking**] CommandPalette and StatusBar pair entries with typed values (action/actions, trailing_action), ActionKey, Clicked and the item key setters are removed, and the action docs describe typed actions.
- *(fluent)* [**breaking**] Menu, MenuBar, Toolbar, MenuGrid, Breadcrumb, Toast, Dialog and Drawer pair each action with a typed app value reported as clicked: Option<A>; item key setters and MenuGridClicked are removed.
- *(fluent)* [**breaking**] Table and DataGrid sort rows from sort_by_key like Material DataTable; the unchecked row_order index list is removed.
- *(core)* [**breaking**] roving focus takes focus::Items (count plus enabled) instead of loose len/enabled, kit::menubar::adjacent steps toward a layout::Edge, and kit::scroll::page takes named PageBars.
- *(core)* [**breaking**] SortState is opaque behind column/by/clear/cycle so it can grow multi-column sort, and kit::command_palette::Shortcuts is a non_exhaustive builder.
- [**breaking**] value-keyed ids and agent keys for every value collection with one ItemKey bound, typed Combobox/Carousel/TransferList bindings, kit modules renamed off core names, and typed Form values (FieldValue, FormValues, checkbox/select/date kinds).
- [**breaking**] bindings are new/optional/multi with item(value, x) builders, accordions, lists and tables key state by app keys (table::Rows<K>), and Material Combobox/Search report values; adds #[derive(KeySalt)], Fluent FormExt, own dnd/child responses and constructors for non_exhaustive ComboList, PullInput, Surroundings and OverflowItem.
- [**breaking**] selection groups bind item values through kit::choice::Choice and return a generic SelectionResponse<K>; index binding, bound constructors and stale-index clamping are removed.
- [**breaking**] non_exhaustive on growth-prone enums and configs, Fluent tokens only under spec, KeyKind agent kinds checked at compile time, DockedToolbar/FloatingToolbar constructors, EframeApp::show removed.
- *(kits)* Fluent menu rows, enum-indexed tables, positional bool flags and the md3 shape-side match become typed; shadow_dp sanitizes its input.
- *(kits)* [**breaking**] Fluent list options, swatches and tags carry typed app values instead of strings, Material breadcrumbs report Clicked, shared id salts derive from one helper.
- master into typed message ids
- *(i18n)* [**breaking**] typed message ids generated from the source catalog by i18n::messages!; I18n::tr, kit::tr and ValidationError::localized take them instead of string ids. Stylekit catalogs inherit the core message schema at compile time and unused catalog messages fail dead_code.
- [**breaking**] typed keys and values: persist::PersistKey<T>, IconFamily as the source of Glyph and IconFont, layout::Fraction, UndoCommand::merge(&Self), MineralUi::with_scoped, TypeId-keyed pass state and Setting<T>. MinTarget::points is crate-private and ColorScheme::role replaces ColorRole::resolve, retiring their lint rules.
- [**breaking**] typed action keys (action::ActionKey, Clicked::key::<K>), ItemResponses for per-item response values, keyed StatusItem and TagGroup, named Fluent container responses and SemanticNode::shortcut as KeyboardShortcut. No response mixes pub and private fields.
- fluent API future-proofing
- material API future-proofing; Elide moves to the core and StatusTone::Custom replaces StatusItem::colors
- *(fluent)* [**breaking**] future-proof the public API: non_exhaustive option enums, slice ALL lists, sealed FluentExt, named structs for shadows, toast actions and overflow space. Align names with Material (show_modal, axis, secondary_text, icon_before/after, validation, Selection-bound Tree).
- *(core)* [**breaking**] Stylekit::window_close returns the close glyph shapes with a kit-styled default; the core submits them, so kits need no override.
- *(core)* [**breaking**] Stylekit::paint_window_close stays required, so the core never paints; kits supply the default chrome.
- *(core)* [**breaking**] future-proof the public API with non_exhaustive configs, payloads and responses, private fields behind constructors (Icon, PickedFile, FileFilter, Catalog, SaveRequest) and enums instead of positional bools; shared SelectionResponse, status bar and command palette data move into the core and both kits re-export them.
- default-features leg tests the libraries without showcase feature unification; smaller culling fixtures, thinner holds_up matrix, agent and route tests join behavior.
- Merge branch 'worktree-agent-a84b4023cee9248d6'
- Merge branch 'worktree-agent-a36e4fcc22cc574d6'
- *(material)* cover color picker and OTP density forwarding; mutation-checked density, label_visible, affix and forwarding tests.
- *(core)* window close glyph shapes live in kit::paint, kits only add them
- Merge branch 'worktree-agent-a27a0aca72025ecf8'
- *(material,fluent)* Density::dense_to, AffixCx and extracted helpers to satisfy clippy and fmt
- cross-kit icon sets, showcase-fluent MINERAL_ICONS=material, icon set tests.
- Merge branch 'worktree-agent-a9996d56bdb434124'
- fmt and line-count lint fixes for the dialog, menu and test changes.
- *(fluent,material)* [**breaking**] Tokens::shadow takes ShadowTint, StatusItem::progress takes StatusProgress replacing linear_progress.
- *(core)* shared NumberPass::consume_step_keys, kit::i18n::tr_or, kit::rating::value_at; Fluent menus share text_width.
- *(core)* kit::scroll::page replaces the duplicated Material and Fluent page containers.
- *(core)* PassCache freezes per-pass derived state; Fluent style() shares the pass snapshot, so set_brand mid-pass never mixes styles.
- *(material,fluent)* table select-all counts the selection, carousel discards only on pass 0 and re-measures on width change and skips off-screen autoplay wakes, button group measures once, data table reuses its cell buffer.
- *(material,fluent)* [**breaking**] ui.tokens() serves the tokens and scheme frozen for the egui pass; Material and Fluent setters apply from the next pass.
- *(fluent)* [**breaking**] Tree draws only rows crossing the clip rect plus the Tab stop, culled rows are tree-only nodes; TreeResponse::rows holds drawn rows.
- *(fluent)* culled table rows are tree-only nodes, skipped unless a tree is collected; widgets only for a pending ScrollIntoView.

