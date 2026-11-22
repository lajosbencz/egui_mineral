# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.12.2](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_eframe-v0.12.1...egui_mineral_eframe-v0.12.2) - 2026-11-22

### Other

- updated the following local packages: egui_mineral

## [0.12.1](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_eframe-v0.12.0...egui_mineral_eframe-v0.12.1) - 2026-11-22

### Other

- updated the following local packages: egui_mineral

## [0.12.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_eframe-v0.11.0...egui_mineral_eframe-v0.12.0) - 2026-11-22

### Other

- updated the following local packages: egui_mineral

## [0.11.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_eframe-v0.10.0...egui_mineral_eframe-v0.11.0) - 2026-11-22

### Other

- updated the following local packages: egui_mineral

## [0.10.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_eframe-v0.9.0...egui_mineral_eframe-v0.10.0) - 2026-11-21

### Other

- updated the following local packages: egui_mineral

## [0.9.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_eframe-v0.8.0...egui_mineral_eframe-v0.9.0) - 2026-11-21

### Other

- updated the following local packages: egui_mineral

## [0.8.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_eframe-v0.7.0...egui_mineral_eframe-v0.8.0) - 2026-11-21

### Added

- *(eframe)* [**breaking**] MineralApp::ui receives the eframe::Frame and raw_input_hook is forwarded; the separate frame hook is removed.

### Fixed

- *(release)* align-breaking renames an aligned crate's unreleased changelog heading and covers egui_mineral_mcp; the eframe changelog names its published 0.7.0.

### Other

- [**breaking**] non_exhaustive on growth-prone enums and configs, Fluent tokens only under spec, KeyKind agent kinds checked at compile time, DockedToolbar/FloatingToolbar constructors, EframeApp::show removed.

## [0.7.0](https://github.com/lajosbencz/egui_mineral/compare/egui_mineral_eframe-v0.0.0...egui_mineral_eframe-v0.7.0) - 2026-11-20

### Added

- *(eframe)* egui_mineral_eframe adapter owns the Runtime and runs a MineralApp inside its pass; showcases migrate to it.
