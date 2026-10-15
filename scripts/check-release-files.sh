#!/bin/sh
# Release files check (feature 015, contracts/release-deliverables.md): the example
# configuration is a release file of its own and sits, byte for byte, in every archive and
# package. Runs in CI before anything is published (publish-github-release, rehearsal-summary).
#
# Usage: sh scripts/check-release-files.sh <release-dir> <example>
set -u

dir=${1:?usage: check-release-files.sh <release-dir> <example>}
example=${2:?usage: check-release-files.sh <release-dir> <example>}
doc=./usr/share/doc/live-qso-map/config.example.toml
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

fail() {
    echo "FAIL [$1]: $2"
    exit 1
}

# same <file> <extracted copy>
same() {
    cmp -s "$2" "$example" || fail "$1" "config.example.toml is missing or differs from $example"
    echo "ok $1"
}

[ -f "$example" ] || fail "$example" "the source example does not exist"

[ -f "$dir/config.example.toml" ] || fail config.example.toml "no standalone file in $dir"
same config.example.toml "$dir/config.example.toml"

for kind in tar.gz zip deb; do
    set -- "$dir"/*."$kind"
    [ -e "$1" ] || fail "*.$kind" "no $kind file in $dir"
done

for f in "$dir"/*.tar.gz; do
    tar -xzOf "$f" config.example.toml > "$work/x" 2>/dev/null || : > "$work/x"
    same "$(basename "$f")" "$work/x"
done

for f in "$dir"/*.zip; do
    unzip -p "$f" config.example.toml > "$work/x" 2>/dev/null || : > "$work/x"
    same "$(basename "$f")" "$work/x"
done

for f in "$dir"/*.deb; do
    dpkg-deb --fsys-tarfile "$f" | tar -xOf - "$doc" > "$work/x" 2>/dev/null || : > "$work/x"
    same "$(basename "$f")" "$work/x"
done

echo "check-release-files: the example is in every release file"
