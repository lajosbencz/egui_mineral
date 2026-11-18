#!/usr/bin/env bash
# Vendors the Compose Material3 token sources at an androidx commit into vendor/<sha>/,
# replacing any previous pin. Usage: fetch.sh [sha]  (default: current androidx-main head).
# Needs gh (authenticated) and curl.
set -euo pipefail

repo=androidx/androidx
src=compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/tokens
here=$(cd "$(dirname "$0")" && pwd)

sha=${1:-$(gh api "repos/$repo/commits/androidx-main" --jq .sha)}
if [[ ! $sha =~ ^[0-9a-f]{40}$ ]]; then
    echo "not a full commit sha: $sha" >&2
    exit 1
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

gh api "repos/$repo/contents/$src?ref=$sha" --jq '.[] | select(.type == "file") | .name' >"$tmp/files"
[[ -s $tmp/files ]] || { echo "no files listed at $sha" >&2; exit 1; }

mkdir "$tmp/$sha"
while read -r name; do
    curl -sSfL "https://raw.githubusercontent.com/$repo/$sha/$src/$name" -o "$tmp/$sha/$name"
done <"$tmp/files"

rm -rf "$here/vendor"
mkdir "$here/vendor"
mv "$tmp/$sha" "$here/vendor/$sha"
echo "vendored $(wc -l <"$tmp/files") files at $sha"
