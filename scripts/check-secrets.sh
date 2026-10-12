#!/bin/sh
# Release gate (feature 009, FR-013): every named variable must be set and non-empty. Reports
# each name as present or missing, and never prints a value.
#
# Usage: sh scripts/check-secrets.sh NAME…
set -u

if [ "$#" -eq 0 ]; then
    echo "usage: check-secrets.sh NAME…" >&2
    exit 1
fi

missing=0
for name in "$@"; do
    case "$name" in
        "" | [0-9]* | *[!A-Za-z0-9_]*)
            echo "check-secrets: invalid variable name '$name'" >&2
            exit 1
            ;;
    esac
    # The name is validated above, so eval only ever expands a plain variable.
    eval "value=\${$name-}"
    if [ -n "$value" ]; then
        echo "check-secrets: present: $name"
    else
        echo "check-secrets: missing: $name" >&2
        missing=$((missing + 1))
    fi
done
value=""

if [ "$missing" -ne 0 ]; then
    echo "check-secrets: $missing publishing secret(s) missing" >&2
    exit 1
fi
