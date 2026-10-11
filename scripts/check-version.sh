#!/bin/sh
# Release gate (feature 008, FR-002): the tag must be "v" + the [package] version in Cargo.toml.
#
# Usage: sh scripts/check-version.sh <tag> [Cargo.toml]
set -u

tag=${1:-}
manifest=${2:-Cargo.toml}

if [ -z "$tag" ]; then
    echo "check-version: no tag given" >&2
    exit 1
fi
case "$tag" in
    v*) wanted=${tag#v} ;;
    *)
        echo "check-version: tag '$tag' does not start with 'v'" >&2
        exit 1
        ;;
esac

# The first `version = "…"` line inside the [package] table.
version=$(sed -n '/^\[package\]/,/^\[/{
/^version[[:space:]]*=/{
s/^version[[:space:]]*=[[:space:]]*"\([^"]*\)".*/\1/p
q
}
}' "$manifest")

if [ -z "$version" ]; then
    echo "check-version: no [package] version in $manifest" >&2
    exit 1
fi
if [ "$wanted" != "$version" ]; then
    echo "check-version: tag $tag is version $wanted, but $manifest has $version" >&2
    exit 1
fi
echo "check-version: tag $tag matches $manifest version $version"
