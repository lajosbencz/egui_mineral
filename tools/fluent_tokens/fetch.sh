#!/usr/bin/env bash
# Vendors the Fluent UI React v9 token sources and component style files at a microsoft/fluentui commit
# into vendor/<sha>/, and the Fluent UI System Icons resizable font's codepoint map at a
# microsoft/fluentui-system-icons commit into vendor-icons/<sha>/ with the font in the crate's assets/,
# replacing any previous pins. Usage: fetch.sh [sha] [icons-sha]  (default: current head of each).
# Needs gh (authenticated) and curl.
set -euo pipefail

repo=microsoft/fluentui
icons_repo=microsoft/fluentui-system-icons
icons_font=FluentSystemIcons-Resizable
here=$(cd "$(dirname "$0")" && pwd)
assets=$here/../../crates/egui_mineral_fluent/assets

sha=${1:-$(gh api "repos/$repo/commits/master" --jq .sha)}
icons_sha=${2:-$(gh api "repos/$icons_repo/commits/main" --jq .sha)}
for s in "$sha" "$icons_sha"; do
    if [[ ! $s =~ ^[0-9a-f]{40}$ ]]; then
        echo "not a full commit sha: $s" >&2
        exit 1
    fi
done

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

gh api "repos/$repo/git/trees/$sha?recursive=1" --jq '.tree[] | select(.type == "blob") | .path' >"$tmp/tree"

tokens='^packages/tokens/src/(alias/[A-Za-z]+|global/[A-Za-z]+|sharedColorNames|statusColorMapping|types|utils/shadows|utils/create(Light|Dark|HighContrast)Theme)\.ts$'
styles='^packages/react-components/react-[^/]+/library/src/components/(.*/)?use[A-Za-z]+Styles\.styles\.ts$'
extra='^packages/react-components/react-nav/library/src/components/(sharedNavStyles\.styles|NavCategoryItem/useNavCategoryItem\.styles)\.ts$'
grep -E "$tokens" "$tmp/tree" >"$tmp/tokens"
grep -E "$styles|$extra" "$tmp/tree" | sort >"$tmp/styles"
[[ -s $tmp/tokens && -s $tmp/styles ]] || { echo "no sources listed at $sha" >&2; exit 1; }

mkdir "$tmp/$sha"
cat "$tmp/tokens" "$tmp/styles" | xargs -P 8 -I{} sh -c '
    mkdir -p "$1/$(dirname "$2")" &&
    curl -sSfL "https://raw.githubusercontent.com/$3/$4/$2" -o "$1/$2"
' _ "$tmp/$sha" {} "$repo" "$sha"
curl -sSfL "https://raw.githubusercontent.com/$repo/$sha/LICENSE" -o "$tmp/$sha/LICENSE"
cp "$tmp/styles" "$tmp/$sha/STYLES.txt"

mkdir -p "$tmp/icons/$icons_sha"
for f in "fonts/$icons_font.json" "fonts/$icons_font.ttf" LICENSE; do
    curl -sSfL "https://raw.githubusercontent.com/$icons_repo/$icons_sha/$f" -o "$tmp/icons/$icons_sha/$(basename "$f")"
done

rm -rf "$here/vendor" "$here/vendor-icons"
mkdir -p "$here/vendor" "$here/vendor-icons" "$assets"
mv "$tmp/$sha" "$here/vendor/$sha"
mv "$tmp/icons/$icons_sha/$icons_font.ttf" "$assets/$icons_font.ttf"
mv "$tmp/icons/$icons_sha" "$here/vendor-icons/$icons_sha"
echo "vendored $(wc -l <"$tmp/tokens") token files and $(wc -l <"$tmp/styles") style files at $sha"
echo "vendored $icons_font at $icons_repo@$icons_sha"
