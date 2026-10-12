#!/bin/sh
# Release notes (feature 009, FR-032): prints the body of the `## [X.Y.Z] - YYYY-MM-DD` section of
# CHANGELOG.md, without its heading. Fails when the section is missing or has no `- ` entry, so a
# release never gets empty notes.
#
# Usage: sh scripts/changelog-section.sh <X.Y.Z> [CHANGELOG.md]
set -u

version=${1:-}
file=${2:-CHANGELOG.md}

if [ -z "$version" ]; then
    echo "usage: changelog-section.sh <X.Y.Z> [CHANGELOG.md]" >&2
    exit 1
fi
if [ ! -r "$file" ]; then
    echo "changelog-section: cannot read $file" >&2
    exit 1
fi

# awk exit codes: 0 found with entries, 2 no section, 3 section without entries.
body=$(tr -d '\r' < "$file" | awk -v heading="## [$version] - " '
    found && /^## / { exit }
    found { lines[n++] = $0; if ($0 ~ /^- /) entries++ }
    !found && index($0, heading) == 1 && length($0) == length(heading) + 10 { found = 1 }
    END {
        if (!found) exit 2
        if (!entries) exit 3
        start = 0; while (start < n && lines[start] == "") start++
        end = n - 1; while (end >= start && lines[end] == "") end--
        for (i = start; i <= end; i++) print lines[i]
    }
')
status=$?

case $status in
    0) printf '%s\n' "$body" ;;
    2)
        echo "changelog-section: $file has no section for $version" >&2
        exit 1
        ;;
    3)
        echo "changelog-section: the $version section of $file has no entry" >&2
        exit 1
        ;;
    *)
        echo "changelog-section: cannot parse $file" >&2
        exit 1
        ;;
esac
