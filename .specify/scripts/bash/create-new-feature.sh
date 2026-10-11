#!/usr/bin/env bash
# Create a new feature: branch feature/NNN-short-name + specs/NNN-short-name/spec.md.
#
# Usage: create-new-feature.sh [--json] [--short-name <name>] [--number N] <feature description>
#
#   --json               Output in JSON format
#   --short-name <name>  Short name (2-4 words) for the branch
#   --number N           Branch number (overrides auto-detection)
#   --help, -h           Show this help message

set -e

JSON_MODE=false
SHORT_NAME=""
BRANCH_NUMBER=""
ARGS=()

while [ $# -gt 0 ]; do
    case "$1" in
        --json) JSON_MODE=true ;;
        --short-name)
            if [ -z "${2:-}" ] || [ "${2#--}" != "$2" ]; then
                echo "ERROR: --short-name requires a value" >&2
                exit 1
            fi
            SHORT_NAME="$2"
            shift
            ;;
        --number)
            if [ -z "${2:-}" ] || ! echo "$2" | grep -Eq '^[0-9]+$'; then
                echo "ERROR: --number requires a numeric value" >&2
                exit 1
            fi
            BRANCH_NUMBER="$2"
            shift
            ;;
        --help|-h)
            sed -n '2,9p' "$0" | sed 's/^# \{0,1\}//'
            exit 0
            ;;
        *) ARGS+=("$1") ;;
    esac
    shift
done

FEATURE_DESCRIPTION="${ARGS[*]:-}"
if [ -z "$FEATURE_DESCRIPTION" ]; then
    echo "Usage: $0 [--json] [--short-name <name>] [--number N] <feature description>" >&2
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=common.sh
source "$SCRIPT_DIR/common.sh"

REPO_ROOT="$(get_repo_root)"
cd "$REPO_ROOT"

HAS_GIT=false
has_git && HAS_GIT=true

SPECS_DIR="$REPO_ROOT/specs"
mkdir -p "$SPECS_DIR"

# Lowercase, non-alphanumerics to '-', collapse and trim dashes.
clean_name() {
    echo "$1" | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9]/-/g; s/--*/-/g; s/^-//; s/-$//'
}

# Derive a short name from the description, dropping common stop words.
generate_short_name() {
    local stop=" i a an the to for of in on at by with from is are was were be been being have has had do does did will would should could can may might must shall this that these those my your our their want need add get set "
    local words=() w
    for w in $(echo "$1" | tr '[:upper:]' '[:lower:]' | sed 's/[^a-z0-9]/ /g'); do
        case "$stop" in
            *" $w "*) continue ;;
        esac
        if [ ${#w} -ge 3 ] || echo "$1" | grep -q "\b$(echo "$w" | tr '[:lower:]' '[:upper:]')\b"; then
            words+=("$w")
        fi
    done

    local max=3 out="" i=0
    [ ${#words[@]} -eq 4 ] && max=4
    for w in ${words[@]+"${words[@]}"}; do
        [ $i -ge $max ] && break
        out="${out:+$out-}$w"
        i=$((i + 1))
    done

    if [ -z "$out" ]; then
        out="$(clean_name "$1" | cut -d- -f1-3)"
    fi
    echo "$out"
}

# Highest NNN used by any specs/ dir or any local/remote branch.
highest_feature_number() {
    local highest=0 n name dir
    for dir in "$SPECS_DIR"/*/; do
        [ -d "$dir" ] || continue
        name="$(basename "$dir")"
        case "$name" in
            [0-9][0-9][0-9]-*)
                n=$((10#${name%%-*}))
                [ "$n" -gt "$highest" ] && highest=$n
                ;;
        esac
    done
    if $HAS_GIT; then
        git fetch --all --prune >/dev/null 2>&1 || true
        for name in $(git branch -a --format='%(refname:short)' 2>/dev/null | sed 's#^.*/##' | grep -E '^[0-9]{3}-' || true); do
            n=$((10#${name%%-*}))
            [ "$n" -gt "$highest" ] && highest=$n
        done
    fi
    echo "$highest"
}

if [ -n "$SHORT_NAME" ]; then
    BRANCH_SUFFIX="$(clean_name "$SHORT_NAME")"
else
    BRANCH_SUFFIX="$(generate_short_name "$FEATURE_DESCRIPTION")"
fi

if [ -z "$BRANCH_NUMBER" ]; then
    BRANCH_NUMBER=$(( $(highest_feature_number) + 1 ))
fi
FEATURE_NUM="$(printf '%03d' "$((10#$BRANCH_NUMBER))")"
BRANCH_NAME="feature/$FEATURE_NUM-$BRANCH_SUFFIX"

# GitHub limits branch names to 244 bytes.
MAX_BRANCH_LENGTH=244
if [ ${#BRANCH_NAME} -gt $MAX_BRANCH_LENGTH ]; then
    ORIGINAL_BRANCH_NAME="$BRANCH_NAME"
    BRANCH_SUFFIX="$(echo "$BRANCH_SUFFIX" | cut -c1-$((MAX_BRANCH_LENGTH - 12)) | sed 's/-$//')"
    BRANCH_NAME="feature/$FEATURE_NUM-$BRANCH_SUFFIX"
    echo "[specify] Warning: branch name truncated to $MAX_BRANCH_LENGTH bytes" >&2
    echo "[specify] Original: $ORIGINAL_BRANCH_NAME" >&2
fi

if $HAS_GIT; then
    git checkout -q -b "$BRANCH_NAME"
else
    echo "[specify] Warning: git repository not detected; skipped branch creation for $BRANCH_NAME" >&2
fi

FEATURE_DIR="$SPECS_DIR/$FEATURE_NUM-$BRANCH_SUFFIX"
mkdir -p "$FEATURE_DIR"

TEMPLATE="$REPO_ROOT/.specify/templates/spec-template.md"
SPEC_FILE="$FEATURE_DIR/spec.md"
if [ ! -f "$SPEC_FILE" ]; then
    if [ -f "$TEMPLATE" ]; then
        cp "$TEMPLATE" "$SPEC_FILE"
    else
        touch "$SPEC_FILE"
    fi
fi

if $JSON_MODE; then
    printf '{"BRANCH_NAME":"%s","SPEC_FILE":"%s","FEATURE_NUM":"%s"}\n' \
        "$(json_escape "$BRANCH_NAME")" "$(json_escape "$SPEC_FILE")" "$FEATURE_NUM"
else
    echo "BRANCH_NAME: $BRANCH_NAME"
    echo "SPEC_FILE: $SPEC_FILE"
    echo "FEATURE_NUM: $FEATURE_NUM"
    echo "Set SPECIFY_FEATURE=$BRANCH_NAME to target this feature without git."
fi
