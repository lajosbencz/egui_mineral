# Workspace

```
app  ->  egui_mineral_material (opinionated MD3 stylekit)
     ->  egui_mineral (unopinionated core)
     ->  egui + eframe backend
```

| Path | Published | Role |
|---|---|---|
| [`crates/egui_mineral`](crates/egui_mineral) | yes | Headless core |
| [`crates/egui_mineral_macros`](crates/egui_mineral_macros) | yes | `#[derive(Route)]` |
| [`crates/egui_mineral_material`](crates/egui_mineral_material) | yes | Material Design 3 stylekit |
| [`crates/egui_mineral_fluent`](crates/egui_mineral_fluent) | no | Fluent stylekit |
| [`examples/showcase`](examples/showcase) | no | Demo of features |
| [`tools/mineral_inspector`](tools/mineral_inspector) | no | Debug inspector |
| [`tools/md3_tokens`](tools/md3_tokens) | no | MD3 token generator |
| [`docs`](docs) | - | Prose: rules, conventions, testing |

Published crates share one version.

Toolchain pinned in [`rust-toolchain.toml`](rust-toolchain.toml).

Details:
[module map](docs/workspace.md#module-map),
[config](docs/workspace.md#config),
[commands](docs/workspace.md#commands),
[release](docs/workspace.md#release)
