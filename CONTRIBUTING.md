# Contributing

Participation follows the [Code of Conduct](CODE_OF_CONDUCT.md).

LLM agents start at [AGENTS.md](AGENTS.md).

## Styleguide

The tooling is the styleguide, CI runs all of it:

- [commands](docs/workspace.md#commands)
- `rustfmt` ([rustfmt.toml](rustfmt.toml)) and `clippy` ([clippy.toml](clippy.toml), workspace lints in [Cargo.toml](Cargo.toml))
- `ast-grep` project rules ([lints/rules](lints/rules))
- `typos` ([_typos.toml](_typos.toml)) and [.editorconfig](.editorconfig)
- [Conventional Commits](https://www.conventionalcommits.org)

## Docs

| Doc | Covers |
|---|---|
| [docs/rules.md](docs/rules.md) | Hard rules and their enforcement |
| [docs/conventions.md](docs/conventions.md) | API shape, naming, components, i18n, keyboard, accessibility |
| [docs/testing.md](docs/testing.md) | What to test, harnesses, test rules |
| [docs/agents.md](docs/agents.md) | Agent API, snapshot formats, MCP server |
| [docs/workspace.md](docs/workspace.md) | Layers, crates |
| [docs/blockers.md](docs/blockers.md) | Gaps blocked upstream |
| [docs/divergences.md](docs/divergences.md) | Deliberate divergences from stylekit specs |
| [docs/todo.md](docs/todo.md) | Core APIs not yet implemented |

API behavior lives in rustdoc:
`cargo doc --workspace --no-deps --all-features --open`

## Setup

`cargo install just`, then `just tools` (cargo tools at CI's pins, nightly toolchain). `actionlint` and `shellcheck` come from the system package manager.

## Workflow

1. Read the rules and conventions for the area you touch
2. Run `just ci` before pushing ([commands](docs/workspace.md#commands))
3. Use [conventional commits](https://www.conventionalcommits.org/en/v1.0.0/) (`feat(core): ...`, `fix(material): ...`)

## Compatibility

Pre-1.0: no backward compatibility, no deprecation shims.
