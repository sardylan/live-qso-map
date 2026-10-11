#!/bin/sh
# Tests scripts/check-version.sh (feature 008, FR-002). Runs in CI before the gate itself.
#
# Usage: sh scripts/test-check-version.sh
set -u

here=$(cd "$(dirname "$0")" && pwd)
check="$here/check-version.sh"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

cat > "$work/Cargo.toml" <<'TOML'
[package]
name = "demo"
version = "2.0.0"
edition = "2024"

[dependencies]
serde = { version = "1" }
version = "9.9.9"
TOML

failures=0

# expect <wanted exit> <tag> <text the output must contain, or empty>
expect() {
    want=$1
    tag=$2
    needle=$3
    out=$(sh "$check" "$tag" "$work/Cargo.toml" 2>&1)
    got=$?
    if [ "$got" -ne "$want" ]; then
        echo "FAIL: tag '$tag': exit $got, wanted $want ($out)"
        failures=$((failures + 1))
        return
    fi
    if [ -n "$needle" ]; then
        case "$out" in
            *"$needle"*) ;;
            *)
                echo "FAIL: tag '$tag': output lacks '$needle' ($out)"
                failures=$((failures + 1))
                ;;
        esac
    fi
}

expect 0 v2.0.0 ""
expect 1 v2.0.1 "2.0.1"
expect 1 v2.0.1 "2.0.0"
expect 1 2.0.0 ""
expect 1 "" ""

if [ "$failures" -ne 0 ]; then
    echo "$failures check(s) failed"
    exit 1
fi
echo "check-version: all checks passed"
