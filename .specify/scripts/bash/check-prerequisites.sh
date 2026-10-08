#!/usr/bin/env bash
# Consolidated prerequisite checking for the spec-driven workflow.
#
# Usage: check-prerequisites.sh [--json] [--require-tasks] [--include-tasks] [--paths-only]
#
#   --json           Output in JSON format
#   --require-tasks  Require tasks.md to exist (implementation phase)
#   --include-tasks  Include tasks.md in the AVAILABLE_DOCS list
#   --paths-only     Only output path variables (no validation)
#   --help, -h       Show this help message

set -e

JSON_MODE=false
REQUIRE_TASKS=false
INCLUDE_TASKS=false
PATHS_ONLY=false

for arg in "$@"; do
    case "$arg" in
        --json) JSON_MODE=true ;;
        --require-tasks) REQUIRE_TASKS=true ;;
        --include-tasks) INCLUDE_TASKS=true ;;
        --paths-only) PATHS_ONLY=true ;;
        --help|-h)
            sed -n '2,11p' "$0" | sed 's/^# \{0,1\}//'
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

if $PATHS_ONLY; then
    if $JSON_MODE; then
        printf '{"REPO_ROOT":"%s","BRANCH":"%s","FEATURE_DIR":"%s","FEATURE_SPEC":"%s","IMPL_PLAN":"%s","TASKS":"%s"}\n' \
            "$(json_escape "$REPO_ROOT")" "$(json_escape "$CURRENT_BRANCH")" \
            "$(json_escape "$FEATURE_DIR")" "$(json_escape "$FEATURE_SPEC")" \
            "$(json_escape "$IMPL_PLAN")" "$(json_escape "$TASKS")"
    else
        echo "REPO_ROOT: $REPO_ROOT"
        echo "BRANCH: $CURRENT_BRANCH"
        echo "FEATURE_DIR: $FEATURE_DIR"
        echo "FEATURE_SPEC: $FEATURE_SPEC"
        echo "IMPL_PLAN: $IMPL_PLAN"
        echo "TASKS: $TASKS"
    fi
    exit 0
fi

if [ ! -d "$FEATURE_DIR" ]; then
    echo "ERROR: Feature directory not found: $FEATURE_DIR" >&2
    echo "Run /speckit-specify first to create the feature structure." >&2
    exit 1
fi

if [ ! -f "$IMPL_PLAN" ]; then
    echo "ERROR: plan.md not found in $FEATURE_DIR" >&2
    echo "Run /speckit-plan first to create the implementation plan." >&2
    exit 1
fi

if $REQUIRE_TASKS && [ ! -f "$TASKS" ]; then
    echo "ERROR: tasks.md not found in $FEATURE_DIR" >&2
    echo "Run /speckit-tasks first to create the task list." >&2
    exit 1
fi

docs=()
[ -f "$RESEARCH" ] && docs+=("research.md")
[ -f "$DATA_MODEL" ] && docs+=("data-model.md")
if [ -d "$CONTRACTS_DIR" ] && [ -n "$(ls -A "$CONTRACTS_DIR" 2>/dev/null)" ]; then
    docs+=("contracts/")
fi
[ -f "$QUICKSTART" ] && docs+=("quickstart.md")
if $INCLUDE_TASKS && [ -f "$TASKS" ]; then
    docs+=("tasks.md")
fi

if $JSON_MODE; then
    json_docs=""
    for d in ${docs[@]+"${docs[@]}"}; do
        [ -n "$json_docs" ] && json_docs="$json_docs,"
        json_docs="$json_docs\"$(json_escape "$d")\""
    done
    printf '{"FEATURE_DIR":"%s","AVAILABLE_DOCS":[%s]}\n' "$(json_escape "$FEATURE_DIR")" "$json_docs"
else
    echo "FEATURE_DIR:$FEATURE_DIR"
    echo "AVAILABLE_DOCS:"
    check_file "$RESEARCH" "research.md"
    check_file "$DATA_MODEL" "data-model.md"
    check_dir "$CONTRACTS_DIR" "contracts/"
    check_file "$QUICKSTART" "quickstart.md"
    if $INCLUDE_TASKS; then
        check_file "$TASKS" "tasks.md"
    fi
fi
