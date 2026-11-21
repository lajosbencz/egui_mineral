#!/usr/bin/env bash
# Fetches the Compose Material3 token sources at the androidx commit in PIN into vendor/<sha>/ (gitignored),
# replacing any other commit. Usage: fetch.sh [sha|latest]  (an argument re-pins; without one a present
# vendor/<pin>/ is kept). Needs gh (authenticated) and curl.
set -euo pipefail

repo=androidx/androidx
src=compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/tokens
here=$(cd "$(dirname "$0")" && pwd)
pin() { sed -n "s/^$1=//p" "$here/PIN"; }
androidx=$(pin androidx)

case ${1:-} in
    "") sha=$androidx; [[ -d $here/vendor/$sha ]] && exit 0 ;;
    latest) sha=$(gh api "repos/$repo/commits/androidx-main" --jq .sha) ;;
    *) sha=$1 ;;
esac
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
sed -i "s/^androidx=.*/androidx=$sha/" "$here/PIN"
echo "fetched $(wc -l <"$tmp/files") files at $sha"
