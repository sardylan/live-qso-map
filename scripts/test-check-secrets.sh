#!/bin/sh
# Tests scripts/check-secrets.sh (feature 009, FR-013). Runs in the test-tools gate.
#
# Usage: sh scripts/test-check-secrets.sh
set -u

here=$(cd "$(dirname "$0")" && pwd)
check="$here/check-secrets.sh"

failures=0

fail() {
    echo "FAIL: $*"
    failures=$((failures + 1))
}

# expect <wanted exit> <text the output must contain, or empty> -- <env assignments…> -- <names…>
expect() {
    want=$1
    needle=$2
    shift 3
    assignments=""
    while [ "$1" != "--" ]; do
        assignments="$assignments $1"
        shift
    done
    shift
    out=$(env -i PATH="$PATH" $assignments sh "$check" "$@" 2>&1)
    got=$?
    if [ "$got" -ne "$want" ]; then
        fail "names '$*': exit $got, wanted $want ($out)"
        return
    fi
    if [ -n "$needle" ]; then
        case "$out" in
            *"$needle"*) ;;
            *) fail "names '$*': output lacks '$needle' ($out)" ;;
        esac
    fi
    case "$out" in
        *s3cr3t*) fail "names '$*': a secret value was printed ($out)" ;;
    esac
}

expect 0 "present: A_TOKEN" -- A_TOKEN=s3cr3t-a B_USER=s3cr3t-b -- A_TOKEN B_USER
expect 1 "missing: C_TOKEN" -- A_TOKEN=s3cr3t-a -- A_TOKEN C_TOKEN
expect 1 "missing: B_USER" -- A_TOKEN=s3cr3t-a B_USER= -- A_TOKEN B_USER C_TOKEN
expect 1 "missing: C_TOKEN" -- A_TOKEN=s3cr3t-a B_USER= -- A_TOKEN B_USER C_TOKEN
expect 1 "usage" -- A_TOKEN=s3cr3t-a --
expect 1 "invalid variable name" -- A_TOKEN=s3cr3t-a -- 'A_TOKEN;echo'

if [ "$failures" -ne 0 ]; then
    echo "$failures check(s) failed"
    exit 1
fi
echo "check-secrets: all checks passed"
