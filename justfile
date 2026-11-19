# Single source of the checks: .github/workflows/{ci,hygiene}.yml call these recipes.
# Tools: `just tools`; cargo-nextest, icu4x-datagen, a nightly toolchain, cargo-deny, cargo-machete,
# cargo-hack, cargo-rdme, typos, ast-grep, actionlint, shellcheck, zizmor; `api-diff` also needs cargo-public-api, `bump-breaking` cargo-edit.

set shell := ["bash", "-euo", "pipefail", "-c"]

# Nightly for the docs.rs build, minimal-versions and rustdoc JSON; at least a week old.
nightly_version := "nightly-2026-11-12"

# Tool pins, read by the workflows (`just --evaluate <name>`); Dependabot does not bump them.
machete_version := "0.9.2"
rdme_version := "2.2.3"
hack_version := "0.6.46"
public_api_version := "0.52.0"
cargo_edit_version := "0.13.14"
zizmor_version := "1.32.0"
ast_grep_version := "0.51.2"
dx_version := "0.7.10"
# Release asset digests of the downloaded binaries, CI only.
ast_grep_sha256 := "40604c21c38d4c7581c56bdddeef277c56bf3c0113620aa96e78ae87e82366df"
actionlint_version := "1.7.12"
actionlint_sha256 := "8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8"

# List the recipes
default:
    @just --list

# Installs the check tools with cargo at the pinned versions, and a nightly toolchain
tools: icu-datagen
    cargo install --locked cargo-nextest cargo-deny typos-cli
    cargo install --locked cargo-machete@{{ machete_version }} cargo-hack@{{ hack_version }} cargo-rdme@{{ rdme_version }} ast-grep@{{ ast_grep_version }} zizmor@{{ zizmor_version }} cargo-edit@{{ cargo_edit_version }}
    rustup toolchain install {{ nightly_version }} --profile minimal --target wasm32-unknown-unknown
    @command -v actionlint >/dev/null || echo "actionlint {{ actionlint_version }} is not on crates.io: https://github.com/rhysd/actionlint"
    @command -v shellcheck >/dev/null || echo "shellcheck is not on crates.io: install it with the system package manager"

# Every blocking check of CI and Hygiene
ci: check hygiene

# CI "Check" job
check: fmt hotpatch-guard clippy test icu-trimmed doc readme-check package versions-check minimal-versions

fmt:
    cargo fmt --all --check

# Every feature alone, the default set and none, per published crate, native and wasm32; the all-features line covers `unstable-hotpatch` (native only, never in the matrix)
clippy:
    for c in egui_mineral egui_mineral_material egui_mineral_fluent; do \
        cargo hack clippy -p "$c" --each-feature --exclude-features unstable-hotpatch --all-targets -- -D warnings; \
        cargo hack clippy -p "$c" --each-feature --exclude-features unstable-hotpatch --target wasm32-unknown-unknown -- -D warnings; \
    done
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    cargo clippy -p showcase-material --target wasm32-unknown-unknown -- -D warnings
    cargo clippy -p showcase-fluent --target wasm32-unknown-unknown -- -D warnings

# Features of the published crates but `unstable-hotpatch`
test_features := "egui_mineral/platform-native,egui_mineral/persist-json,egui_mineral/web-save-stream,egui_mineral_material/theme-file,egui_mineral_material/unstable-catalog,egui_mineral_material/unstable-spec,egui_mineral_fluent/unstable-catalog,egui_mineral_fluent/unstable-spec"

# Default features (+spec) without the showcases' feature unification, every feature, none; extra args go to nextest, e.g. `--no-run`
test *args:
    cargo nextest run -p egui_mineral -p egui_mineral_material -p egui_mineral_fluent --features egui_mineral_material/unstable-spec,egui_mineral_fluent/unstable-spec {{ args }}
    cargo nextest run --workspace --features {{ test_features }} {{ args }}
    cargo nextest run -p egui_mineral -p egui_mineral_material -p egui_mineral_fluent --no-default-features --features egui_mineral_material/bundled-icons,egui_mineral_fluent/bundled-icons {{ args }}

# icu4x-datagen matching the locked ICU4X version
icu-datagen:
    cargo install icu4x-datagen --locked --version "$(cargo pkgid icu_datetime | sed 's/.*@//')"

# Core tests on ICU4X data trimmed to en and ar
icu-trimmed:
    #!/usr/bin/env bash
    set -euo pipefail
    data=$(mktemp -d)
    trap 'rm -rf "$data"' EXIT
    icu4x-datagen --format baked --markers all --locales en ar --out "$data/icu"
    ICU4X_DATA_DIR="$data/icu" cargo nextest run -p egui_mineral --test icu_trimmed --run-ignored only

# `unstable-hotpatch` must stay a command-line feature: no published crate or default set may enable it
hotpatch-guard:
    #!/usr/bin/env bash
    set -euo pipefail
    tree=$(cargo tree --workspace -e features --prefix none)
    if grep -q '^subsecond ' <<<"$tree"; then
        echo "subsecond is in the default dependency graph: unstable-hotpatch leaked"
        exit 1
    fi

# Mirrors `[package.metadata.docs.rs] features` of each crate
docsrs_core := "icu,persist-fs,persist-json,persist-toml,platform-native,web-save-stream"
docsrs_material := "icu,persist-fs,persist-json,persist-toml,bundled-icons,theme-file,extras,unstable-catalog,unstable-spec"
docsrs_fluent := "icu,persist-fs,persist-json,persist-toml,bundled-icons,unstable-catalog,unstable-spec"

# Doctests, rustdoc and the docs.rs build
doc:
    cargo test --doc --workspace --features {{ test_features }}
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --features {{ test_features }}
    RUSTDOCFLAGS="-D warnings" cargo doc -p egui_mineral -p egui_mineral_material -p egui_mineral_fluent --no-deps --no-default-features
    for t in x86_64-unknown-linux-gnu wasm32-unknown-unknown; do \
        RUSTDOCFLAGS="--cfg docsrs -D warnings" cargo +{{ nightly_version }} doc -p egui_mineral --no-deps --target "$t" --features {{ docsrs_core }}; \
        RUSTDOCFLAGS="--cfg docsrs -D warnings" cargo +{{ nightly_version }} doc -p egui_mineral_material --no-deps --target "$t" --features {{ docsrs_material }}; \
        RUSTDOCFLAGS="--cfg docsrs -D warnings" cargo +{{ nightly_version }} doc -p egui_mineral_fluent --no-deps --target "$t" --features {{ docsrs_fluent }}; \
    done

# Published crates; their READMEs are generated from the crate docs
readme_crates := "egui_mineral egui_mineral_material egui_mineral_macros"

# Regenerates the crate READMEs from their `lib.rs` docs (intra-doc links: docs/blockers.md)
readme:
    for c in {{ readme_crates }}; do cargo rdme -w "$c" -r "crates/$c/README.md" --intralinks-strip-links --force; done

# Fails when a crate README is stale
readme-check:
    for c in {{ readme_crates }}; do cargo rdme -w "$c" -r "crates/$c/README.md" --intralinks-strip-links --check; done

# Unverified: verification resolves sibling crates from crates.io, which lacks unreleased APIs. Dirty: runs before committing.
package:
    cargo package --no-verify --allow-dirty -p egui_mineral -p egui_mineral_macros -p egui_mineral_material

# Published crates share 0.MINOR (MAJOR from 1.0); the pinned toolchain is the workspace MSRV
versions-check:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo metadata --no-deps --format-version 1 | jq -e --arg crates "{{ readme_crates }}" '
        [.packages[] | select(.name | IN($crates | split(" ")[])) | .version | split(".") | if .[0] == "0" then .[0:2] else .[0:1] end | join(".")]
        | unique | length == 1' >/dev/null \
        || { echo "published crates differ in breaking version: run just align-breaking"; exit 1; }
    channel=$(sed -nE 's/^channel *= *"([0-9]+\.[0-9]+)(\.[0-9]+)?"/\1/p' rust-toolchain.toml)
    msrv=$(sed -nE '/^\[workspace\.package\]/,/^\[/s/^rust-version *= *"([0-9]+\.[0-9]+)(\.[0-9]+)?"/\1/p' Cargo.toml)
    [ -n "$channel" ] && [ "$channel" = "$msrv" ] \
        || { echo "rust-toolchain.toml channel (${channel:-unset}) must match workspace rust-version (${msrv:-unset}): the toolchain is the MSRV"; exit 1; }

# Raises every published crate to the highest version among them when their breaking versions differ; patches stay per crate
align-breaking:
    #!/usr/bin/env bash
    set -euo pipefail
    just versions-check >/dev/null 2>&1 && exit 0
    v=$(cargo metadata --no-deps --format-version 1 | jq -r --arg crates "{{ readme_crates }}" \
        '.packages[] | select(.name | IN($crates | split(" ")[])) | .version' | sort -V | tail -n1)
    just bump-breaking "$v"

# Sets every published crate to <version> (a breaking release, e.g. 0.3.0) and the workspace dependency requirements
bump-breaking version:
    for c in {{ readme_crates }}; do cargo set-version -p "$c" "{{ version }}"; done
    sed -i -E '/^egui_mineral(_macros|_material)? = /s/version = "[^"]*"/version = "{{ version }}"/' Cargo.toml
    cargo update --workspace

# Lower bounds of the direct dependencies hold (nightly -Z direct-minimal-versions, lockfile restored)
minimal-versions:
    #!/usr/bin/env bash
    set -euo pipefail
    lock=$(mktemp)
    cp Cargo.lock "$lock"
    trap 'cp "$lock" Cargo.lock; rm -f "$lock"' EXIT
    cargo +{{ nightly_version }} check --workspace --all-features -Z direct-minimal-versions

# Regenerates the MD3 and Fluent specs and the Material Symbols icons; fails when it changed (non-blocking in CI)
spec:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo run -p md3_tokens -- generate
    cargo run -p fluent_tokens -- generate
    cargo run -p material_symbols -- generate
    changes=$(git status --porcelain -- crates/egui_mineral_material/src/spec crates/egui_mineral_material/src/icons.rs crates/egui_mineral_fluent/src/spec)
    [ -z "$changes" ] && exit 0
    if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
        printf '## Stale generated spec\n```\n%s\n```\n' "$changes" >> "$GITHUB_STEP_SUMMARY"
    fi
    echo "$changes"
    exit 1

# Builds the spec generators without running them (cache warm-up)
spec-build:
    cargo build -p md3_tokens -p fluent_tokens -p material_symbols

# Builds the rustdoc JSON of HEAD that `api-diff` needs (cache warm-up)
api-build: (api-diff "HEAD" "HEAD")

# Public API diff of the libraries between two commits; removals and changes fail unless the crate's breaking version (0.MINOR, MAJOR from 1.0) rises. With release=true each crate's base is its last `<crate>-v*` tag reachable from <base>.
api-diff base head="HEAD" release="false":
    #!/usr/bin/env bash
    set -euo pipefail
    summary=${GITHUB_STEP_SUMMARY:-/dev/null}
    status=0
    breaking() {
        git show "$1:crates/$2/Cargo.toml" | sed -nE '0,/^version/s/^version = "([0-9]+)\.([0-9]+)\..*/\1 \2/p' \
            | awk '{ print ($1 == "0") ? "0." $2 : $1 }'
    }
    # Stable features of `crate` at `ref`; both commits build with the ones they share.
    features() {
        git show "$1:crates/$2/Cargo.toml" | awk '/^\[features\]/ { f = 1; next } /^\[/ { f = 0 } f && /^[a-z0-9_-]+ *=/ { print $1 }' \
            | grep -v '^unstable' | grep -vx default | sort
    }
    for crate in egui_mineral egui_mineral_material; do
    base="{{ base }}"
    if [ "{{ release }}" = true ]; then
        base=$(git describe --tags --abbrev=0 --match "$crate-v*" "{{ base }}" 2>/dev/null || echo "{{ base }}")
    fi
    features=$(comm -12 <(features "$base" "$crate") <(features "{{ head }}" "$crate") | paste -sd, -)
    deny=(--deny removed --deny changed)
    if [ "$(breaking "$base" "$crate")" != "$(breaking "{{ head }}" "$crate")" ]; then
        deny=()
        echo "\`$crate\`: breaking version bump, removals and changes allowed" | tee -a "$summary"
    fi
    for target in x86_64-unknown-linux-gnu wasm32-unknown-unknown; do
        echo "## \`$crate\` ($target)" | tee -a "$summary"
        echo '```diff' >> "$summary"
        cargo +{{ nightly_version }} public-api -p "$crate" --features "$features" --target "$target" -sss diff "${deny[@]}" "$base..{{ head }}" \
            | tee -a "$summary" || status=1
        echo '```' >> "$summary"
    done
    done
    exit "$status"

# Every Hygiene job
hygiene: typos deny rules machete workflows

# Native showcase under `dx` subsecond hot-patching (dioxus-cli `dx_version`)
hotpatch:
    @dx --version | grep -q ' {{ dx_version }}' || { echo "needs dioxus-cli {{ dx_version }}: cargo install dioxus-cli --version {{ dx_version }} --locked"; exit 1; }
    # dx 0.7.10 misses cargo's build-dir fingerprints (docs/blockers.md): force the relink it captures.
    touch examples/showcase-material/src/main.rs
    dx serve --hotpatch --interactive false --platform linux --package showcase-material --features egui_mineral/unstable-hotpatch

# CI runs crate-ci/typos
typos:
    typos

# CI runs cargo-deny-action with its defaults
deny:
    cargo deny --all-features check

# ast-grep rule tests and scan
rules:
    ast-grep test
    ast-grep scan crates tools examples

machete:
    cargo machete

workflows:
    actionlint
    zizmor .github
