# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.13.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral-v0.12.3...egui_mineral-v0.13.0) - 2026-11-27

### Added

- *(widget)* ShowIn::show_in for egui containers with a MineralUi body; kits and showcases drop the rt.ui re-wrap.
- *(kit)* kit::paint state ramps, state-layer overlay and focus strokes shared by both kits.
- *(kit)* kit::text::BareEdit and submitted, the frameless editor shared by both kits.
- *(semantic)* [**breaking**] impl KeySalt for str, so borrowed &str salts of any lifetime key; key parts are Copy inline text, with key benches.
- *(eframe)* [**breaking**] EframeApp::new sets GlobalStyle::Kit; document what GlobalStyle::Egui leaves unstyled.
- *(form)* [**breaking**] declaration-first forms via #[derive(Form)] with typed field keys, model-owned rules, widget kinds dispatched to kit FieldWidget impls and Fluent attribute labels; Material, Fluent and showcase ported.

### Fixed

- *(widget)* show_in gives Area and Window content the kit style, as Runtime::ui does for root Uis.

## [0.12.3](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral-v0.12.2...egui_mineral-v0.12.3) - 2026-11-27

### Fixed

- *(date)* port to jiff 1.0 ZonedDateTime rename.

## [0.12.2](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral-v0.12.1...egui_mineral-v0.12.2) - 2026-11-22

### Added

- *(overlay)* core overlay::ModalFocus replaces both kits' copies; kit::layer::default_action_pressed_with lets a kit pick which focused widgets keep Enter.

## [0.12.1](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral-v0.12.0...egui_mineral-v0.12.1) - 2026-11-22

### Fixed

- *(menu)* Walk::Bar on End from a submenu leaf moves to the next bar menu; Fluent MenuBar and Material MenuStrip consume the arrow that opens a menu so it opens on the first row.
- *(runtime)* GlobalStyle::Kit hands egui the resolved theme under a set_system_theme override.

## [0.12.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral-v0.11.0...egui_mineral-v0.12.0) - 2026-11-22

### Added

- *(select)* [**breaking**] Enter never selects and AccessKit clicks get RowSource::Assistive; Fluent List action_selects, Fluent toggles ignore Enter, TreeView collapse keeps focus shown, semantic multiselectable state.
- *(select)* [**breaking**] SelectionBehavior (Toggle, Replace) drives Selection::apply and follow_focus with mouse, touch tap/long press, Space and Enter sources and focus-scoped Ctrl/Cmd+A; Material TreeView/DataTable and Fluent Table/DataGrid/List adopt it, Fluent List arrows keep focus.
- *(select)* RowInput and Selection::apply map a row's press and click onto Selection in core; Fluent Table rows select on press like the Material TreeView.
- *(select)* Selection::follow_focus, select::is_select_all and RovingFocus::on_select_key flatten keyboard selection into core; Fluent DataGrid Shift+arrows extend and Ctrl+A selects all, Material TreeView shares them.
- *(select)* [**breaking**] Selection tracks a current key apart from the anchor (current, set_current, anchor), press/release keep a dragged multiple selection, extend_to and select_all; tree::on_key takes modifiers and reports Focus { row, extend } and SelectAll. Material TreeView selects on press, Shift+arrows extend, Ctrl+A selects all.
- *(tree)* TreeData::drop_zones picks Halves, Quarters or Inside drop slots per node; drop_slot honors it.
- *(tree)* Expansion::reveal expands a node's ancestors so the tree shows it; Expansion::is_single.
- *(semantic)* DisplaySalt<T> keys a foreign type by its Display text and hashes, compares and debug-prints as T.
- *(menu)* [**breaking**] core kit::menu::MenuNav owns the open submenu path, hover intent, arrow walk and opening focus; Material and Fluent menus run on it. Fluent MenuResponse::nav reports root-level arrows.
- *(shortcut)* [**breaking**] Mnemonic::parse reads &-marked labels and underlines the marked letter; non-ASCII mnemonics match typed text; items sharing a letter cycle focus. Kit builders take impl Into<Mnemonic>.
- *(runtime)* Core::of(ctx) returns the core of the running pass, else the last outermost one; runtimes register as Weak<Core>.
- *(runtime)* [**breaking**] Policies::wheel is optional, None leaves egui's line_scroll_speed to the app; Core::set_system_theme supplies the OS theme that System resolves to.

### Fixed

- *(range)* a step that would move against its direction leaves the value, as HTML stepUp/stepDown; raising at an off-grid max no longer lowers it.
- *(shortcut)* a lone Alt fires again; AltTrack ignores the Alt key's own Key::AltLeft/AltRight press.
- *(hotpatch)* pin subsecond and dioxus-devtools to =0.7.10, the dx_version they must match; dependabot leaves them to dx bumps.

## [0.11.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral-v0.10.0...egui_mineral-v0.11.0) - 2026-11-22

### Fixed

- *(input)* [**breaking**] touch swipes scroll pages over 2D pannables (PanMode::Free claims only after GestureConfig::pan_hold) and nested scroll areas chain unused drag and fling to the page (kit::scroll::chain).
- *(overlays)* [**breaking**] popups, menus, tooltips, search and Material dialogs/pickers fit small viewports; Popup::place caps to the anchor room, dialogs scroll when taller.

## [0.10.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral-v0.9.0...egui_mineral-v0.10.0) - 2026-11-21

### Added

- *(tree)* [**breaking**] accept_drop adds a payload type per call (DropAccept::push), first acceptor wins; Fluent TreeResponse::rows is ItemResponses like Material.
- *(splitter)* [**breaking**] collapsible(pane) Enter toggle per APG, CollapseRequest from Enter, drag and double click on a collapsed handle, hit zone at the collapsed edge; both kits' SplitPane.
- *(window)* [**breaking**] resize_handle, resize_border, end_corner and can_resize replace status::resize_grip; status bar grips hide while maximized or fullscreen.
- *(form)* [**breaking**] FieldPlacement and LabelSlot compose forms with layouts; core FormRows and StackBelow; FormGrid::show_form, stacking and labelled_by rows, FormUiExt::section, in both kits.
- *(status)* StatusBar tone() for the whole bar and grip(bool) window resize corner (core resize_grip, grip_dots), in both kits.
- *(table)* row_style tints and per-row responses in both kits' tables; Material DataTable stick_to_end via VirtualList::stick_to_end.
- *(splitter)* Enter and double click reset to the default fraction (SplitterResponse::reset), collapse(Option<Pane>), in both kits' SplitPane.
- *(form)* FormGrid in both kits, a two-column form with a shared label column (core LabelColumn), section headings and full-width rows.
- *(number)* optional (blank) number fields, pixel scrub rate with log doubling, clamped output and select-on-step, in Material NumberField and Fluent SpinButton.
- *(tree)* disabled nodes, foreign drops (DropAccept, TreeDrop), reveal, and Material unfocused selection color, in both kits' trees.
- *(input)* WheelScroll policy sets wheel line speed from the OS setting (Windows, KDE) and accelerates fast spins; Runtime::before_pass applies it, egui_mineral_eframe calls it.

### Fixed

- *(dnd)* drags sense click_and_drag so a press is no drag (tree preview flicker, carousel cursor, table grip clicks sorting); grips sit above headers and follow the pointer via input::drag_from_start; drag-only-sense lint.
- *(input)* hand-rolled hit tests and tap/pan respect overlays and modals via reaches_layer and InputContext::events_on.

### Other

- [**breaking**] core Tone (from StatusTone) and RowStyle for both kits' row_style, RadioMark and keep-open menu toggles, ScrubRate::Pixels per scale unit with one scrub(rate) builder; Fluent Text inherits override_text_color.
- *(number)* NumberEdit::begin takes the NumberRef (blank mode from Optional), NumberEdit::finish writes back and reports the change for both kits.
- *(runtime)* the runtime's egui plugin applies the wheel policy, so every integration gets it; drop Runtime::before_pass.

## [0.9.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral-v0.8.0...egui_mineral-v0.9.0) - 2026-11-21

### Fixed

- *(form)* [**breaking**] NumberRange is non_exhaustive with at_least, at_most and between constructors.

## [0.8.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral-v0.7.0...egui_mineral-v0.8.0) - 2026-11-21

### Added

- *(mcp)* add egui_mineral_mcp, an MCP server over the agent API (Streamable HTTP, bearer token, loopback only), served by the showcase when MINERAL_MCP_TOKEN is set; SemanticTree::subtree returns an owned tree; docs/agents.md.
- *(core)* [**breaking**] AgentHandle::snapshot returns the SemanticTree, act_then_snapshot returns the tree after the UI reacted, NotFound lists similar keys, and Runtime::viewports lists viewports; SemanticTree serializes as nested JSON.
- *(core)* [**breaking**] semantic tree nests by egui's widget hierarchy, widget parts and popup owners, replacing emit_parent and hand-set depth; containers emit with their Ui id so AccessKit gets their roles, and tree items carry level.
- *(core)* spawn::ThreadSpawner runs futures and blocking jobs on owned OS threads for apps without an async runtime.

### Other

- drop comments that restate code and the stale speedup figure; keep invariants, quirks and spec pointers.
- [**breaking**] Steps<K>, Stepper<K> and Tour<K> key flows by app values like Selection<K>; step labels, responses and the bound tour step carry keys, not indices.
- *(core)* [**breaking**] typed i18n::Locale replaces locale strings in I18n, Localizer, LocaleFormat and both formats; locale! checks tags at compile time and messages! emits msg::CATALOG tagged by its file name.
- *(core)* Form::rule links CheckError::on_field instead of the private field.
- [**breaking**] Stylekit::chrome_icon(ChromeIcon) replaces the window_close shape hook; core paints the kit's glyph or its vector cross, both kits supply their close/dismiss icon.
- *(core)* [**breaking**] UndoStack::group(label, |stack| ..) replaces begin_group/end_group, so a group cannot stay open; nested groups join the outer one.
- *(core)* [**breaking**] forms take numbers (Blank for numeric primitives, NumberRange over Numeric, kit FormUiExt::number), Form::check becomes Form::rule, FormState::field/field_mut, CheckError is opaque and finish_submit takes per-field errors.
- [**breaking**] NumberField, ValueSlider and Fluent Slider and SpinButton bind any emath::Numeric with typed format closures; core adds RangeModel::for_numeric and range::to_numeric.
- *(core)* [**breaking**] PersistKey<T, C> carries its Codec, so load and save take no codec turbofish and a key cannot be read in another format.
- *(core)* [**breaking**] ModalStack<K> keys modals by app value through ModalStack::new(id_salt); content closes or pushes through ModalEntry instead of returning bool.
- *(core)* [**breaking**] forms bind the app's value: Form<T> fields bind &mut V parts through the kits' FormUiExt with typed Validator<V>s and Fn(&T) rules, FormState keeps only meta state; FieldValue, FormValues, FieldKind and FieldOption are removed.
- *(core)* [**breaking**] CommandPalette and StatusBar pair entries with typed values (action/actions, trailing_action), ActionKey, Clicked and the item key setters are removed, and the action docs describe typed actions.
- *(material)* [**breaking**] icon and text action collections pair each action with a typed app value (app bar, toolbars, tree view, alert, rich tooltip, snackbar), reported as clicked: Option<A> and ItemResponses<A, T> keyed by value.
- *(core)* [**breaking**] roving focus takes focus::Items (count plus enabled) instead of loose len/enabled, kit::menubar::adjacent steps toward a layout::Edge, and kit::scroll::page takes named PageBars.
- *(core)* [**breaking**] SortState is opaque behind column/by/clear/cycle so it can grow multi-column sort, and kit::command_palette::Shortcuts is a non_exhaustive builder.
- [**breaking**] value-keyed ids and agent keys for every value collection with one ItemKey bound, typed Combobox/Carousel/TransferList bindings, kit modules renamed off core names, and typed Form values (FieldValue, FormValues, checkbox/select/date kinds).
- [**breaking**] bindings are new/optional/multi with item(value, x) builders, accordions, lists and tables key state by app keys (table::Rows<K>), and Material Combobox/Search report values; adds #[derive(KeySalt)], Fluent FormExt, own dnd/child responses and constructors for non_exhaustive ComboList, PullInput, Surroundings and OverflowItem.
- [**breaking**] selection groups bind item values through kit::choice::Choice and return a generic SelectionResponse<K>; index binding, bound constructors and stale-index clamping are removed.
- [**breaking**] non_exhaustive on growth-prone enums and configs, Fluent tokens only under spec, KeyKind agent kinds checked at compile time, DockedToolbar/FloatingToolbar constructors, EframeApp::show removed.

## [0.7.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral-v0.6.0...egui_mineral-v0.7.0) - 2026-11-20

### Added

- *(eframe)* egui_mineral_eframe adapter owns the Runtime and runs a MineralApp inside its pass; showcases migrate to it.
- *(core)* [**breaking**] opt-in spawn-tokio and spawn-wasm Spawner adapters that own their tasks, count setters take plain integers clamped to at least 1, and the prelude exports setup errors. Showcases use the adapters and borrow the runtime per pass; docs record action keys, layout wrappers, the Stylekit openness and the MineralWidget::Kit divergence.
- *(core)* [**breaking**] MineralUi layout wrappers hand closures a MineralUi, and action items carry an app key reported as action::Clicked (PaletteCommand::key, CommandPaletteResponse::clicked). The prelude exports InitError and Clicked.
- *(material)* [**breaking**] density applies to every component: Stylekit::min_target shrinks all minimum hit targets 4dp per step, and sliders, top app bar, navigation items, split button, menu strip and status bar containers follow density. Core adds Stylekit::min_target and MineralUi::target; the min-target-points lint guards it.
- *(material)* [**breaking**] density-following Toolbar heights and CheckButton/Radio/Switch hit targets, TreeView::node_text_style, text_align and tabular_figures on fields, InlineEdit::editing, SideSheet::initial_width. Core: MineralWidget::min_target takes the ui, kit::text::tabular_figures, StatusItem::tabular_figures.

### Other

- Merge remote-tracking branch 'origin/master'
- *(kits)* [**breaking**] Fluent list options, swatches and tags carry typed app values instead of strings, Material breadcrumbs report Clicked, shared id salts derive from one helper.
- *(core)* [**breaking**] nav::__private::segment drops its unused field parameter.
- *(i18n)* reword IntoMessage docs for the restating-doc rule
- master into typed message ids
- *(i18n)* [**breaking**] typed message ids generated from the source catalog by i18n::messages!; I18n::tr, kit::tr and ValidationError::localized take them instead of string ids. Stylekit catalogs inherit the core message schema at compile time and unused catalog messages fail dead_code.
- [**breaking**] typed keys and values: persist::PersistKey<T>, IconFamily as the source of Glyph and IconFont, layout::Fraction, UndoCommand::merge(&Self), MineralUi::with_scoped, TypeId-keyed pass state and Setting<T>. MinTarget::points is crate-private and ColorScheme::role replaces ColorRole::resolve, retiring their lint rules.
- [**breaking**] typed action keys (action::ActionKey, Clicked::key::<K>), ItemResponses for per-item response values, keyed StatusItem and TagGroup, named Fluent container responses and SemanticNode::shortcut as KeyboardShortcut. No response mixes pub and private fields.

## [0.6.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral-v0.5.0...egui_mineral-v0.6.0) - 2026-11-19

### Added

- *(material)* [**breaking**] MaterialIcon is a local type with From<MaterialIcon> for Icon; drop the core material-icons feature
- *(core)* [**breaking**] shared icon types, IconFont installer and glyph shape helper.
- *(core)* kit::nodes::emit_culled and VirtualList::show_culled; Material TreeView keeps off-screen rows reachable to agents.
- *(core)* [**breaking**] persist-fs no longer implies the codecs, defaults ship persist-toml only, so serde_json is opt-in via persist-json.

### Fixed

- *(material,fluent)* Escape after switching menus returns focus to the open menu's title, which becomes the bar's Tab stop.
- *(core)* kit doc comments state facts; test: status bar fixture robust to font widths.
- box toolbar slot base, rustfmt, readme and doc lint fixes for icon sets.
- drop the unused fluent shadow tint flag, doc links and private-doc lints after the merge.
- *(core,material)* clippy similar_names in state eviction, TreeView culled emit extracted.
- *(perf)* size_or_discard requests a discard on the first pass only.
- *(core)* per-frame log sites report once and log under the egui_mineral target.
- *(core)* SetValue warnings log the widget id and text length, not the text.
- *(persist)* FsStorage::set removes its temp file on every error path.
- *(platform)* native file read, writer creation and clipboard run on a worker thread instead of the polling thread.
- *(core)* prune per-viewport pass state of closed viewports.

### Other

- material API future-proofing; Elide moves to the core and StatusTone::Custom replaces StatusItem::colors
- *(core)* [**breaking**] Stylekit::window_close returns the close glyph shapes with a kit-styled default; the core submits them, so kits need no override.
- *(core)* [**breaking**] Stylekit::paint_window_close stays required, so the core never paints; kits supply the default chrome.
- *(core)* [**breaking**] future-proof the public API with non_exhaustive configs, payloads and responses, private fields behind constructors (Icon, PickedFile, FileFilter, Catalog, SaveRequest) and enums instead of positional bools; shared SelectionResponse, status bar and command palette data move into the core and both kits re-export them.
- *(core)* kit::palette, kit::menubar and kit::strip hold the palette, menu bar and status strip models Material's extras used.
- Merge branch 'worktree-agent-a84b4023cee9248d6'
- *(core,material)* pull-to-refresh and sheet handle read the per-pass input; document remaining Core::input callers
- *(core)* window close glyph shapes live in kit::paint, kits only add them
- Merge branch 'worktree-agent-a9996d56bdb434124'
- *(core)* [**breaking**] native platform I/O runs on the app's BlockingSpawner, NativePlatform::new takes one, so the core stays threadless.
- *(core)* scope_of no longer links the private is_modal.
- lint fixes for windows harness and JsError.
- *(core)* widget states untouched for 3600 passes are evicted from egui temp data, which never frees them.
- *(core)* [**breaking**] PanClaim::update takes the MineralUi; kit interact, card and number field read the per-pass cached input, MinTarget::points_for skips the lookup.
- *(core)* [**breaking**] PanClaim::update drops the enabled flag for PanClaim::reset, PanGate holds one state enum, both join kit::prelude.
- *(core)* [**breaking**] declare_modal and is_modal are crate-private; kit::layer::modal_on and outside_press_after_open replace them for kits.
- *(core)* shared NumberPass::consume_step_keys, kit::tr_or, kit::rating::value_at; Fluent menus share text_width.
- *(core)* kit::scroll::page replaces the duplicated Material and Fluent page containers.
- *(wasm)* JsError keeps exception name, message and stack.
- *(i18n)* tr caches plain messages as Arc<str> and skips the second lock for arg messages; set_locale and add_resource negotiate outside the state lock.
- *(core)* PassCache freezes per-pass derived state; Fluent style() shares the pass snapshot, so set_brand mid-pass never mixes styles.

## [0.5.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral-v0.4.1...egui_mineral-v0.5.0) - 2026-11-19

### Added

- *(core)* [**breaking**] MineralWidget::pan_axis routes single-axis touch drags through PanClaim so off-axis swipes scroll the page; slider, switch and color bars migrate.
- *(core)* [**breaking**] kit::layer::modal builds egui::Modal on modal_area, raw-egui-area flags Modal::new.
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
