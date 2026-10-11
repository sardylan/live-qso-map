#!/usr/bin/env bash
# Update the agent context file with technology from the current feature's plan.md.
#
# Usage: update-agent-context.sh [claude]
#
# Reads Language/Version, Primary Dependencies, Storage and Project Type from
# plan.md and updates a managed block in CLAUDE.md (created if missing).
# Content outside the SPECKIT markers is preserved untouched.

set -e

AGENT="${1:-claude}"
case "$AGENT" in
    claude) ;;
    --help|-h)
        sed -n '2,8p' "$0" | sed 's/^# \{0,1\}//'
        exit 0
        ;;
    *)
        echo "ERROR: Unsupported agent '$AGENT'. Supported: claude" >&2
        exit 1
        ;;
esac

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=common.sh
source "$SCRIPT_DIR/common.sh"

eval "$(get_feature_paths)"

if [ ! -f "$IMPL_PLAN" ]; then
    echo "ERROR: No plan.md found at $IMPL_PLAN" >&2
    echo "Run /speckit-plan first." >&2
    exit 1
fi

AGENT_FILE="$REPO_ROOT/CLAUDE.md"
START_MARKER="<!-- SPECKIT START -->"
END_MARKER="<!-- SPECKIT END -->"
MAX_RECENT=3

# Value of a "**Field**: value" line; empty for placeholders/NEEDS CLARIFICATION/N/A.
plan_field() {
    local value
    value="$(awk -v f="**$1**: " 'index($0, f) == 1 { v = substr($0, length(f) + 1); sub(/[[:space:]]+$/, "", v); print v; exit }' "$IMPL_PLAN")"
    case "$value" in
        ""|"N/A"|*"NEEDS CLARIFICATION"*|"["*) echo "" ;;
        *) echo "$value" ;;
    esac
}

LANG_VER="$(plan_field 'Language/Version')"
DEPS="$(plan_field 'Primary Dependencies')"
STORAGE="$(plan_field 'Storage')"
PROJECT_TYPE="$(plan_field 'Project Type')"

TECH=""
[ -n "$LANG_VER" ] && TECH="$LANG_VER"
[ -n "$DEPS" ] && TECH="${TECH:+$TECH + }$DEPS"
NEW_TECH_LINES=()
[ -n "$TECH" ] && NEW_TECH_LINES+=("- $TECH ($FEATURE_ID)")
[ -n "$STORAGE" ] && NEW_TECH_LINES+=("- $STORAGE ($FEATURE_ID)")

CHANGE_LINE="- $FEATURE_ID: Added ${TECH:-plan}"

# Extract existing lines of a "### Section" inside the managed block.
existing_section() {
    [ -f "$AGENT_FILE" ] || return 0
    awk -v s="$START_MARKER" -v e="$END_MARKER" -v sec="### $1" '
        $0 == s { inblock = 1; next }
        $0 == e { inblock = 0; next }
        inblock && /^### / { insec = ($0 == sec); next }
        inblock && insec && /^- / { print }
    ' "$AGENT_FILE"
}

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

# Active technologies: other branches' entries + this branch's current ones,
# de-duplicated, order preserved.
{
    existing_section "Active Technologies" | grep -vF " ($FEATURE_ID)" || true
    for l in ${NEW_TECH_LINES[@]+"${NEW_TECH_LINES[@]}"}; do echo "$l"; done
} | awk '!seen[$0]++' > "$TMP_DIR/tech"

# Recent changes: newest first, this branch replaced, capped.
{
    echo "$CHANGE_LINE"
    existing_section "Recent Changes" | grep -v "^- $FEATURE_ID:" || true
} | head -n "$MAX_RECENT" > "$TMP_DIR/changes"

{
    echo "$START_MARKER"
    echo "<!-- Managed by .specify/scripts/bash/update-agent-context.sh; edit outside these markers. -->"
    echo ""
    echo "### Active Technologies"
    echo ""
    cat "$TMP_DIR/tech"
    echo ""
    echo "### Recent Changes"
    echo ""
    cat "$TMP_DIR/changes"
    echo ""
    echo "$END_MARKER"
} > "$TMP_DIR/block"

if [ ! -f "$AGENT_FILE" ]; then
    {
        echo "# $(basename "$REPO_ROOT") Development Guidelines"
        echo ""
        echo "Project principles: \`.specify/memory/constitution.md\`."
        echo ""
        cat "$TMP_DIR/block"
    } > "$AGENT_FILE"
    echo "[specify] Created $AGENT_FILE"
elif grep -qF "$START_MARKER" "$AGENT_FILE"; then
    awk -v s="$START_MARKER" -v e="$END_MARKER" -v blockfile="$TMP_DIR/block" '
        $0 == s { while ((getline line < blockfile) > 0) print line; skip = 1; next }
        $0 == e { skip = 0; next }
        !skip { print }
    ' "$AGENT_FILE" > "$TMP_DIR/out"
    cat "$TMP_DIR/out" > "$AGENT_FILE"
    echo "[specify] Updated $AGENT_FILE"
else
    { echo ""; cat "$TMP_DIR/block"; } >> "$AGENT_FILE"
    echo "[specify] Appended spec-kit block to $AGENT_FILE"
fi
