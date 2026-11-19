# Single source of the checks: .github/workflows/{ci,hygiene}.yml call these recipes.
# Tools: `just tools`; cargo-nextest, icu4x-datagen, a nightly toolchain, cargo-deny, cargo-machete,
# cargo-rdme, typos, ast-grep, actionlint, shellcheck, zizmor; `api-diff` also needs cargo-public-api, `bump-breaking` cargo-edit.

set shell := ["bash", "-euo", "pipefail", "-c"]

# Tool pins, read by the workflows (`just --evaluate <name>`); Dependabot does not bump them.
machete_version := "0.9.2"
rdme_version := "2.2.3"
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
    cargo install --locked cargo-machete@{{ machete_version }} cargo-rdme@{{ rdme_version }} ast-grep@{{ ast_grep_version }} zizmor@{{ zizmor_version }} cargo-edit@{{ cargo_edit_version }}
    rustup toolchain install nightly --profile minimal --target wasm32-unknown-unknown
    @command -v actionlint >/dev/null || echo "actionlint {{ actionlint_version }} is not on crates.io: https://github.com/rhysd/actionlint"
    @command -v shellcheck >/dev/null || echo "shellcheck is not on crates.io: install it with the system package manager"

# Every blocking check of CI and Hygiene
ci: check hygiene

# CI "Check" job
check: fmt hotpatch-guard clippy test icu-trimmed doc readme-check package versions-check minimal-versions

fmt:
    cargo fmt --all --check

# Feature and target matrix
clippy:
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    cargo clippy -p egui_mineral_material --all-targets -- -D warnings
    cargo clippy -p egui_mineral -p egui_mineral_material --all-targets --no-default-features -- -D warnings
    cargo clippy -p egui_mineral --target wasm32-unknown-unknown --no-default-features -- -D warnings
    cargo clippy -p egui_mineral_material --target wasm32-unknown-unknown -- -D warnings
    cargo clippy -p egui_mineral_material --target wasm32-unknown-unknown --no-default-features -- -D warnings
    cargo clippy -p egui_mineral_material --target wasm32-unknown-unknown --all-features -- -D warnings
    cargo clippy -p showcase --target wasm32-unknown-unknown -- -D warnings

# Extra args go to nextest, e.g. `--no-run`
test *args:
    cargo nextest run --workspace --all-features {{ args }}
    cargo nextest run -p egui_mineral -p egui_mineral_material --no-default-features --features egui_mineral_material/bundled-icons {{ args }}

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
    ! cargo tree --workspace -e features -i subsecond 2>/dev/null | grep -q .

# Doctests, rustdoc and the docs.rs build
doc:
    cargo test --doc --workspace --all-features
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features
    RUSTDOCFLAGS="-D warnings" cargo doc -p egui_mineral -p egui_mineral_material --no-deps --no-default-features
    RUSTDOCFLAGS="--cfg docsrs -D warnings" cargo +nightly doc -p egui_mineral -p egui_mineral_material --all-features --no-deps

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

# Published crates share 0.MINOR (MAJOR from 1.0)
versions-check:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo metadata --no-deps --format-version 1 | jq -e --arg crates "{{ readme_crates }}" '
        [.packages[] | select(.name | IN($crates | split(" ")[])) | .version | split(".") | if .[0] == "0" then .[0:2] else .[0:1] end | join(".")]
        | unique | length == 1' >/dev/null \
        || { echo "published crates differ in breaking version: run just bump-breaking <version>"; exit 1; }

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
    cargo +nightly check --workspace --all-features -Z direct-minimal-versions

# Regenerates the MD3 spec; fails when it changed (non-blocking in CI)
spec:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo run -p md3_tokens -- generate
    changes=$(git status --porcelain -- crates/egui_mineral_material/src/spec)
    [ -z "$changes" ] && exit 0
    if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
        printf '## Stale generated spec\n```\n%s\n```\n' "$changes" >> "$GITHUB_STEP_SUMMARY"
    fi
    echo "$changes"
    exit 1

# Public API diff of the libraries between two commits (non-blocking in CI)
api-diff base head="HEAD":
    #!/usr/bin/env bash
    set -euo pipefail
    summary=${GITHUB_STEP_SUMMARY:-/dev/null}
    status=0
    for crate in egui_mineral egui_mineral_material; do
    for target in x86_64-unknown-linux-gnu wasm32-unknown-unknown; do
        echo "## \`$crate\` ($target)" | tee -a "$summary"
        echo '```diff' >> "$summary"
        features=$(cargo metadata --no-deps --format-version 1 | jq -r --arg c "$crate" \
            '.packages[] | select(.name == $c) | .features | keys | map(select(startswith("unstable") | not)) | join(",")')
        cargo public-api -p "$crate" --features "$features" --target "$target" -sss diff --deny removed --deny changed "{{ base }}..{{ head }}" \
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
    touch examples/showcase/src/main.rs
    dx serve --hotpatch --interactive false --platform linux --package showcase --features egui_mineral/unstable-hotpatch

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
