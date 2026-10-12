#!/bin/sh
# Tests scripts/changelog-section.sh (feature 009, FR-032). Runs in the test-tools gate.
#
# Usage: sh scripts/test-changelog-section.sh
set -u

here=$(cd "$(dirname "$0")" && pwd)
script="$here/changelog-section.sh"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

cat > "$work/CHANGELOG.md" <<'MD'
# Changelog

## [Unreleased]

### Fixed
- unreleased entry

## [2.0.10] - 2026-12-01

### Fixed
- ten entry

## [2.0.1] - 2026-10-12

### Fixed
- one entry
- one second entry

### Notes
- one note

## [2.0.0] - 2026-10-11

### Notes

## [1.0.0] - 2024-11-27

### Added
- last entry
MD

# Same file with CRLF line endings.
sed 's/$/\r/' "$work/CHANGELOG.md" > "$work/crlf.md"

failures=0

fail() {
    echo "FAIL: $*"
    failures=$((failures + 1))
}

# expect <wanted exit> <version> <file> <text the output must contain, or empty> [text it must not contain]
expect() {
    want=$1
    version=$2
    file=$3
    needle=$4
    absent=${5:-}
    out=$(sh "$script" "$version" "$file" 2>&1)
    got=$?
    if [ "$got" -ne "$want" ]; then
        fail "version '$version' ($file): exit $got, wanted $want ($out)"
        return
    fi
    if [ -n "$needle" ]; then
        case "$out" in
            *"$needle"*) ;;
            *) fail "version '$version' ($file): output lacks '$needle' ($out)" ;;
        esac
    fi
    if [ -n "$absent" ]; then
        case "$out" in
            *"$absent"*) fail "version '$version' ($file): output contains '$absent' ($out)" ;;
        esac
    fi
}

md="$work/CHANGELOG.md"
expect 0 2.0.1 "$md" "- one second entry" "ten entry"
expect 0 2.0.1 "$md" "### Notes" "## [2.0.1]"
expect 0 2.0.10 "$md" "- ten entry" "one entry"
expect 0 1.0.0 "$md" "- last entry"
expect 1 2.0.0 "$md" "no entry"
expect 1 3.0.0 "$md" "no section"
expect 1 Unreleased "$md" ""
expect 1 "" "$md" "usage"
expect 1 2.0.1 "$work/missing.md" ""

# CRLF input: same body, no carriage returns in the output.
out=$(sh "$script" 2.0.1 "$work/crlf.md" 2>&1) || fail "CRLF: exit $?"
case "$out" in
    *"- one second entry"*) ;;
    *) fail "CRLF: output lacks the entry ($out)" ;;
esac
if printf '%s' "$out" | grep -q "$(printf '\r')"; then
    fail "CRLF: carriage returns in the output"
fi

if [ "$failures" -ne 0 ]; then
    echo "$failures check(s) failed"
    exit 1
fi
echo "changelog-section: all checks passed"
