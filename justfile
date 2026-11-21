# Single source of the checks: .github/workflows/{ci,hygiene}.yml call these recipes.
# Tools: `just tools`; cargo-nextest, icu4x-datagen, a nightly toolchain, cargo-deny, cargo-machete,
# cargo-hack, cargo-rdme, typos, ast-grep, actionlint, shellcheck, zizmor; `api-diff` also needs cargo-public-api, `bump-breaking` cargo-edit.

set shell := ["bash", "-euo", "pipefail", "-c"]

# Nightly for the docs.rs build, minimal-versions and rustdoc JSON; at least a week old.
nightly_version := "nightly-2026-11-12"

# Tool pins, read by the workflows (`just --evaluate <name>`); Dependabot does not bump them.
nextest_version := "0.9.152"
deny_version := "0.20.2"
typos_version := "1.52.2"
machete_version := "0.9.2"
rdme_version := "2.2.3"
hack_version := "0.6.46"
public_api_version := "0.52.0"
cargo_edit_version := "0.13.14"
zizmor_version := "1.32.0"
ast_grep_version := "0.51.2"
dx_version := "0.7.10"
trunk_version := "0.22.1"

# Stylekit features that only forward a core feature
forwarded_features := "platform-pickers,platform-clipboard,platform-notify,platform-native,web-save-stream,spawn-tokio,spawn-wasm"

# Release asset digests of the downloaded binaries, CI only.
ast_grep_sha256 := "40604c21c38d4c7581c56bdddeef277c56bf3c0113620aa96e78ae87e82366df"
actionlint_version := "1.7.12"
actionlint_sha256 := "8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8"

# List the recipes
default:
    @just --list

# Installs the check tools with cargo at the pinned versions, and a nightly toolchain
tools: icu-datagen
    cargo install --locked cargo-nextest@{{ nextest_version }} cargo-deny@{{ deny_version }} typos-cli@{{ typos_version }}
    cargo install --locked cargo-machete@{{ machete_version }} cargo-hack@{{ hack_version }} cargo-rdme@{{ rdme_version }} ast-grep@{{ ast_grep_version }} zizmor@{{ zizmor_version }} cargo-edit@{{ cargo_edit_version }} cargo-public-api@{{ public_api_version }} trunk@{{ trunk_version }} dioxus-cli@{{ dx_version }}
    rustup toolchain install {{ nightly_version }} --profile minimal --target wasm32-unknown-unknown
    @command -v actionlint >/dev/null || echo "actionlint {{ actionlint_version }} is not on crates.io: https://github.com/rhysd/actionlint"
    @command -v shellcheck >/dev/null || echo "shellcheck is not on crates.io: install it with the system package manager"

# Every blocking check of CI and Hygiene, including the ones CI runs only on dependency or release pull requests
ci: check check-deps check-release hygiene

# Fast local loop: format check, default-feature clippy of the libraries, first test leg
quick: fmt
    cargo clippy -p egui_mineral -p egui_mineral_material -p egui_mineral_fluent --all-targets -- -D warnings
    just test-default

# Formats and applies machine-applicable clippy fixes
fix:
    cargo fmt --all
    cargo clippy --fix --allow-dirty --workspace --all-targets

# Every blocking check of the CI jobs clippy, test and docs
check: check-clippy check-test check-docs

# CI "Clippy" job
check-clippy: clippy

# CI "Test" job
check-test: test icu-trimmed

# CI "Docs and packaging" job
check-docs: doc readme-check versions-check fmt unstable-guard

# CI "Docs and packaging" job, pull requests that change a manifest, the lockfile or the toolchain
check-deps: minimal-versions

# CI "Docs and packaging" job, release pull requests
check-release: package

# `code=` and `deps=` lines for $GITHUB_OUTPUT: whether <base>..<head> touches anything a build checks, and the manifests, lockfile or toolchain
changes base head:
    #!/usr/bin/env bash
    set -euo pipefail
    files=$(git diff --name-only "{{ base }}" "{{ head }}")
    # Prose only: Markdown outside the generated crate READMEs.
    grep -qvE '^([^/]+|docs/.*)\.md$' <<<"$files" && echo code=true || echo code=false
    grep -qE '(^|/)Cargo\.toml$|^Cargo\.lock$|^rust-toolchain\.toml$' <<<"$files" && echo deps=true || echo deps=false

fmt:
    cargo fmt --all --check

# Every feature alone, the default set and none, per published crate, native and wasm32; the all-features line covers `unstable-hotpatch` (native only, never in the matrix)
clippy:
    cargo hack clippy -p egui_mineral --each-feature --exclude-features unstable-hotpatch --all-targets -- -D warnings
    cargo hack clippy -p egui_mineral --each-feature --exclude-features unstable-hotpatch --target wasm32-unknown-unknown -- -D warnings
    # Pure core forwarders, covered by the core legs
    for c in egui_mineral_material egui_mineral_fluent; do \
        cargo hack clippy -p "$c" --each-feature --exclude-features {{ forwarded_features }} --all-targets -- -D warnings; \
        cargo hack clippy -p "$c" --each-feature --exclude-features {{ forwarded_features }} --target wasm32-unknown-unknown -- -D warnings; \
    done
    # eframe needs a renderer, and winit a Linux windowing backend
    cargo hack clippy -p egui_mineral_eframe --feature-powerset --at-least-one-of glow,wgpu --at-least-one-of wayland,x11 --all-targets -- -D warnings
    cargo hack clippy -p egui_mineral_eframe --feature-powerset --at-least-one-of glow,wgpu --target wasm32-unknown-unknown -- -D warnings
    # Native only: Tokio networking
    cargo clippy -p egui_mineral_mcp --all-targets -- -D warnings
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    cargo clippy -p showcase-material --target wasm32-unknown-unknown -- -D warnings
    cargo clippy -p showcase-fluent --target wasm32-unknown-unknown -- -D warnings

# Features of the published crates but `unstable-hotpatch`
test_features := "egui_mineral/platform-native,egui_mineral/persist-json,egui_mineral/web-save-stream,egui_mineral/spawn-tokio,egui_mineral/spawn-wasm,egui_mineral_material/theme-file,egui_mineral_material/unstable-catalog,egui_mineral_material/unstable-spec,egui_mineral_fluent/unstable-catalog,egui_mineral_fluent/unstable-spec"

# Default features (+spec) without the showcases' feature unification, every feature, none; extra args go to nextest, e.g. `--no-run`
test *args: (test-default args)
    cargo nextest run --workspace --features {{ test_features }} {{ args }}
    cargo nextest run -p egui_mineral -p egui_mineral_material -p egui_mineral_fluent --no-default-features --features egui_mineral_material/bundled-icons,egui_mineral_fluent/bundled-icons {{ args }}

# First leg of `test`: default features (+spec)
test-default *args:
    cargo nextest run -p egui_mineral -p egui_mineral_material -p egui_mineral_fluent --features egui_mineral_material/unstable-spec,egui_mineral_fluent/unstable-spec {{ args }}

# icu4x-datagen matching the locked ICU4X version
icu-datagen:
    cargo install icu4x-datagen --locked --version "$(cargo pkgid icu_datetime | sed 's/.*@//')"

# Release wasm builds of both showcases (trunk) with ICU4X data for the showcases' locales only (pages::LOCALES of both); sites land in examples/*/dist
showcases public_url="/":
    #!/usr/bin/env bash
    set -euo pipefail
    data=$(mktemp -d)
    trap 'rm -rf "$data"' EXIT
    icu4x-datagen --format baked --markers all --locales en de hu ja ar --out "$data/icu"
    export ICU4X_DATA_DIR="$data/icu"
    for s in fluent material; do
        (cd "examples/showcase-$s" && trunk build --release --public-url "{{ public_url }}$s/")
    done

# Core tests on ICU4X data trimmed to en and ar
icu-trimmed:
    #!/usr/bin/env bash
    set -euo pipefail
    data=$(mktemp -d)
    trap 'rm -rf "$data"' EXIT
    icu4x-datagen --format baked --markers all --locales en ar --out "$data/icu"
    ICU4X_DATA_DIR="$data/icu" cargo nextest run -p egui_mineral --test icu_trimmed --run-ignored only

# `unstable-*` features stay opt-in: no published crate's dependency or stable feature enables one, and `unstable-hotpatch` stays out of the default graph
unstable-guard:
    #!/usr/bin/env bash
    set -euo pipefail
    leaks=$(cargo metadata --no-deps --format-version 1 | jq -r '
        .packages[] | select(.publish == null) | .name as $c
        | ([.dependencies[] | select(.kind != "dev") | .name as $d | .features[] | select(startswith("unstable")) | "\($c): dependency \($d) enables \(.)"]
            + [.features | to_entries[] | select(.key | startswith("unstable") | not) | .key as $f | .value[] | select(test("(^|/)unstable")) | "\($c): feature \($f) enables \(.)"])[]')
    if [ -n "$leaks" ]; then
        echo "$leaks"
        exit 1
    fi
    tree=$(cargo tree --workspace -e features --prefix none)
    if grep -q '^subsecond ' <<<"$tree"; then
        echo "subsecond is in the default dependency graph: unstable-hotpatch leaked"
        exit 1
    fi

# Doctests, rustdoc and the docs.rs build (each crate's `[package.metadata.docs.rs]` features and targets)
doc:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo test --doc --workspace --features {{ test_features }}
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --features {{ test_features }}
    RUSTDOCFLAGS="-D warnings" cargo doc -p egui_mineral -p egui_mineral_material -p egui_mineral_fluent --no-deps --no-default-features
    meta=$(cargo metadata --no-deps --format-version 1)
    for c in egui_mineral egui_mineral_material egui_mineral_fluent; do
        features=$(jq -r --arg c "$c" '.packages[] | select(.name == $c) | .metadata.docs.rs.features | join(",")' <<<"$meta")
        for t in $(jq -r --arg c "$c" '.packages[] | select(.name == $c) | .metadata.docs.rs.targets[]' <<<"$meta"); do
            RUSTDOCFLAGS="--cfg docsrs -D warnings" cargo +{{ nightly_version }} doc -p "$c" --no-deps --target "$t" --features "$features"
        done
    done

# Published crates (no `publish = false`), one per line
published:
    @cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.publish == null) | .name'

# `<crate> <target>` per published crate and `[package.metadata.docs.rs]` target, one per line
api-targets:
    @cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.publish == null) | .name as $c | (.metadata.docs.rs.targets // [])[] | "\($c) \(.)"'

# Regenerates the published crates' READMEs from their `lib.rs` docs (intra-doc links: docs/blockers.md)
readme:
    for c in $(just published); do cargo rdme -w "$c" -r "crates/$c/README.md" --intralinks-strip-links --force; done

# Fails when a crate README is stale
readme-check:
    for c in $(just published); do cargo rdme -w "$c" -r "crates/$c/README.md" --intralinks-strip-links --check; done

# Unverified: verification resolves sibling crates from crates.io, which lacks unreleased APIs. Dirty: runs before committing.
package:
    cargo package --no-verify --allow-dirty $(just published | sed 's/^/-p /')

# Published crates share 0.MINOR (MAJOR from 1.0); the pinned toolchain is the workspace MSRV
versions-check:
    #!/usr/bin/env bash
    set -euo pipefail
    cargo metadata --no-deps --format-version 1 | jq -e '
        [.packages[] | select(.publish == null) | .version | split(".") | if .[0] == "0" then .[0:2] else .[0:1] end | join(".")]
        | unique | length == 1' >/dev/null \
        || { echo "published crates differ in breaking version: run just align-breaking"; exit 1; }
    channel=$(sed -nE 's/^channel *= *"([0-9]+\.[0-9]+)(\.[0-9]+)?"/\1/p' rust-toolchain.toml)
    msrv=$(sed -nE '/^\[workspace\.package\]/,/^\[/s/^rust-version *= *"([0-9]+\.[0-9]+)(\.[0-9]+)?"/\1/p' Cargo.toml)
    [ -n "$channel" ] && [ "$channel" = "$msrv" ] \
        || { echo "rust-toolchain.toml channel (${channel:-unset}) must match workspace rust-version (${msrv:-unset}): the toolchain is the MSRV"; exit 1; }

# Release PR <branch> (<url>): commits `align-breaking` there and rewrites the versions release-plz planned in the PR body; GH_TOKEN pushes and edits (CI `align` job)
align-pr branch url:
    #!/usr/bin/env bash
    set -euo pipefail
    versions() { cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.publish == null) | "\(.name) \(.version)"' | sort; }
    before=$(versions)
    just align-breaking
    git diff --quiet && exit 0
    git config user.name "github-actions[bot]"
    git config user.email "41898282+github-actions[bot]@users.noreply.github.com"
    git commit -am "chore: align published crates on one breaking version"
    git push "https://x-access-token:${GH_TOKEN}@github.com/${GITHUB_REPOSITORY:?}.git" "HEAD:{{ branch }}"
    # The PR body lists the versions release-plz planned: `* `crate`: [old -> ]new` and the changelog headings.
    body=$(gh pr view "{{ url }}" --json body --jq .body)
    while read -r c old new; do
      [ "$old" = "$new" ] && continue
      o=${old//./\\.}
      body=$(sed -E -e "s/^(\* \`$c\`: (.* -> )?)$o([^0-9.]|$)/\1$new\3/" -e "s/^## \[$o\]\((.*)$c-v$o\)/## [$new](\1$c-v$new)/" <<<"$body")
    done < <(join <(echo "$before") <(versions))
    gh pr edit "{{ url }}" --body "$body"

# Raises every published crate to the highest version among them when their breaking versions differ; patches stay per crate
align-breaking:
    #!/usr/bin/env bash
    set -euo pipefail
    just versions-check >/dev/null 2>&1 && exit 0
    v=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.publish == null) | .version' | sort -V | tail -n1)
    just bump-breaking "$v"

# Sets every published crate and the workspace dependency requirements to <version> (a breaking release, e.g. 0.3.0), renaming each unreleased changelog heading
bump-breaking version:
    #!/usr/bin/env bash
    set -euo pipefail
    meta=$(cargo metadata --no-deps --format-version 1)
    for c in $(just published); do
        old=$(jq -r --arg c "$c" '.packages[] | select(.name == $c) | .version' <<<"$meta")
        cargo set-version -p "$c" "{{ version }}"
        log="crates/$c/CHANGELOG.md"
        if [ "$old" != "{{ version }}" ] && [ -f "$log" ] && [ -z "$(git ls-remote --tags origin "refs/tags/$c-v$old")" ]; then
            o=${old//./\\.}
            sed -i -E "0,/^## \\[$o\\]/s/^## \\[$o\\]\\((.*)-v$o\\)/## [{{ version }}](\\1-v{{ version }})/" "$log"
        fi
        sed -i -E "/^$c = /s/version = \"[^\"]*\"/version = \"{{ version }}\"/" Cargo.toml
    done
    cargo update --workspace

# Lower bounds of the direct dependencies hold (nightly -Z direct-minimal-versions, lockfile restored)
minimal-versions:
    #!/usr/bin/env bash
    set -euo pipefail
    lock=$(mktemp)
    cp Cargo.lock "$lock"
    trap 'cp "$lock" Cargo.lock; rm -f "$lock"' EXIT
    cargo +{{ nightly_version }} check --workspace --all-features -Z direct-minimal-versions

# Fetches the pinned sources, regenerates the MD3 and Fluent specs and the Material Symbols icons; fails when it changed (non-blocking in CI)
spec:
    #!/usr/bin/env bash
    set -euo pipefail
    tools/md3_tokens/fetch.sh
    tools/fluent_tokens/fetch.sh
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

# `api-diff-one` for every `api-targets` line, or only those of <target>; CI runs one job per target
api-diff base head="HEAD" release="false" target="":
    #!/usr/bin/env bash
    set -euo pipefail
    status=0
    while read -r crate target; do
        [ -z "{{ target }}" ] || [ "$target" = "{{ target }}" ] || continue
        just api-diff-one "$crate" "$target" "{{ base }}" "{{ head }}" "{{ release }}" || status=1
    done < <(just api-targets)
    exit "$status"

# Public API diff of <crate> on <target> between two commits; removals and changes fail unless the crate's breaking version (0.MINOR, MAJOR from 1.0) rises. With release=true the base is the crate's last `<crate>-v*` tag reachable from <base>.
api-diff-one crate target base head="HEAD" release="false":
    #!/usr/bin/env bash
    set -euo pipefail
    summary=${GITHUB_STEP_SUMMARY:-/dev/null}
    crate="{{ crate }}"
    breaking() {
        git show "$1:crates/$crate/Cargo.toml" | sed -nE '0,/^version/s/^version = "([0-9]+)\.([0-9]+)\..*/\1 \2/p' \
            | awk '{ print ($1 == "0") ? "0." $2 : $1 }'
    }
    # Stable features at `ref`; both commits build with the ones they share.
    features() {
        git show "$1:crates/$crate/Cargo.toml" | awk '/^\[features\]/ { f = 1; next } /^\[/ { f = 0 } f && /^[a-z0-9_-]+ *=/ { print $1 }' \
            | grep -v '^unstable' | grep -vx default | sort
    }
    base="{{ base }}"
    if [ "{{ release }}" = true ]; then
        base=$(git describe --tags --abbrev=0 --match "$crate-v*" "{{ base }}" 2>/dev/null || echo "{{ base }}")
    fi
    echo "## \`$crate\` ({{ target }})" | tee -a "$summary"
    if ! git cat-file -e "$base:crates/$crate/Cargo.toml" 2>/dev/null; then
        echo "new crate, no base" | tee -a "$summary"
        exit 0
    fi
    features=$(comm -12 <(features "$base") <(features "{{ head }}") | paste -sd, -)
    deny=(--deny removed --deny changed)
    if [ "$(breaking "$base")" != "$(breaking "{{ head }}")" ]; then
        deny=()
        echo "breaking version bump, removals and changes allowed" | tee -a "$summary"
    fi
    echo '```diff' >> "$summary"
    status=0
    cargo +{{ nightly_version }} public-api -p "$crate" --features "$features" --target "{{ target }}" -sss diff "${deny[@]}" "$base..{{ head }}" \
        | tee -a "$summary" || status=1
    echo '```' >> "$summary"
    exit "$status"

# eframe's Linux windowing headers (CI runners: apt retries, the mirrors time out now and then)
linux-deps:
    echo 'Acquire::Retries "5"; Acquire::http::Timeout "30"; Acquire::https::Timeout "30";' | sudo tee /etc/apt/apt.conf.d/80-retries >/dev/null
    sudo apt-get update
    sudo apt-get install -y --no-install-recommends libxkbcommon-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev

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
