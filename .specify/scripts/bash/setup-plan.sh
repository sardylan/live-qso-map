#!/usr/bin/env bash
# Prepare the implementation plan for the current feature.
#
# Usage: setup-plan.sh [--json]
#
#   --json      Output in JSON format
#   --help, -h  Show this help message

set -e

JSON_MODE=false
for arg in "$@"; do
    case "$arg" in
        --json) JSON_MODE=true ;;
        --help|-h)
            sed -n '2,7p' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *)
            echo "ERROR: Unknown option '$arg'. Use --help for usage information." >&2
            exit 1
            ;;
    esac
done

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=common.sh
source "$SCRIPT_DIR/common.sh"

eval "$(get_feature_paths)"
check_feature_branch "$CURRENT_BRANCH" "$HAS_GIT" || exit 1

mkdir -p "$FEATURE_DIR"

# Never overwrite an existing plan.
TEMPLATE="$REPO_ROOT/.specify/templates/plan-template.md"
if [ -f "$IMPL_PLAN" ]; then
    echo "[specify] plan.md already exists; left unchanged: $IMPL_PLAN" >&2
elif [ -f "$TEMPLATE" ]; then
    cp "$TEMPLATE" "$IMPL_PLAN"
else
    echo "[specify] Warning: plan template not found at $TEMPLATE" >&2
    touch "$IMPL_PLAN"
fi

if $JSON_MODE; then
    printf '{"FEATURE_SPEC":"%s","IMPL_PLAN":"%s","SPECS_DIR":"%s","BRANCH":"%s","HAS_GIT":"%s"}\n' \
        "$(json_escape "$FEATURE_SPEC")" "$(json_escape "$IMPL_PLAN")" \
        "$(json_escape "$FEATURE_DIR")" "$(json_escape "$CURRENT_BRANCH")" "$HAS_GIT"
else
    echo "FEATURE_SPEC: $FEATURE_SPEC"
    echo "IMPL_PLAN: $IMPL_PLAN"
    echo "SPECS_DIR: $FEATURE_DIR"
    echo "BRANCH: $CURRENT_BRANCH"
    echo "HAS_GIT: $HAS_GIT"
fi
