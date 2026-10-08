#!/usr/bin/env bash
# Common functions and variables for all spec-kit scripts.
# Compatible with bash 3.2 (macOS default).

# Repository root: git top-level if available, otherwise the nearest
# ancestor of this script that contains a .specify directory.
get_repo_root() {
    if git rev-parse --show-toplevel >/dev/null 2>&1; then
        git rev-parse --show-toplevel
        return
    fi
    local dir
    dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    while [ "$dir" != "/" ]; do
        if [ -d "$dir/.specify" ]; then
            echo "$dir"
            return
        fi
        dir="$(dirname "$dir")"
    done
    echo "ERROR: could not determine repository root" >&2
    return 1
}

has_git() {
    git rev-parse --show-toplevel >/dev/null 2>&1
}

# Current feature: SPECIFY_FEATURE env var, then git branch, then the
# highest-numbered directory under specs/.
get_current_branch() {
    if [ -n "${SPECIFY_FEATURE:-}" ]; then
        echo "$SPECIFY_FEATURE"
        return
    fi

    if has_git; then
        local branch
        branch="$(git rev-parse --abbrev-ref HEAD 2>/dev/null)"
        # On a repo with no commits, rev-parse prints HEAD; use symbolic-ref instead
        if [ "$branch" = "HEAD" ] || [ -z "$branch" ]; then
            branch="$(git symbolic-ref --short HEAD 2>/dev/null)"
        fi
        if [ -n "$branch" ]; then
            echo "$branch"
            return
        fi
    fi

    local repo_root specs_dir latest="" highest=0 dir name num
    repo_root="$(get_repo_root)"
    specs_dir="$repo_root/specs"
    if [ -d "$specs_dir" ]; then
        for dir in "$specs_dir"/*/; do
            [ -d "$dir" ] || continue
            name="$(basename "$dir")"
            case "$name" in
                [0-9][0-9][0-9]-*)
                    num=$((10#${name%%-*}))
                    if [ "$num" -gt "$highest" ]; then
                        highest=$num
                        latest=$name
                    fi
                    ;;
            esac
        done
    fi

    if [ -n "$latest" ]; then
        echo "$latest"
    else
        echo "main"
    fi
}

# Feature branches must look like NNN-short-name. Without git, only warn.
check_feature_branch() {
    local branch="$1"
    local has_git_repo="$2"

    if [ "$has_git_repo" != "true" ]; then
        echo "[specify] Warning: git repository not detected; skipped branch validation" >&2
        return 0
    fi

    if ! echo "$branch" | grep -Eq '^[0-9]{3}-'; then
        echo "ERROR: Not on a feature branch. Current branch: $branch" >&2
        echo "Feature branches should be named like: 001-feature-name" >&2
        return 1
    fi
    return 0
}

# Resolve the spec directory by numeric prefix, so several branches
# (e.g. 004-fix-a, 004-fix-b) can work on the same specs/004-* directory.
find_feature_dir_by_prefix() {
    local repo_root="$1"
    local branch="$2"
    local specs_dir="$repo_root/specs"

    if ! echo "$branch" | grep -Eq '^[0-9]{3}-'; then
        echo "$specs_dir/$branch"
        return
    fi

    local prefix="${branch%%-*}"
    local matches=() dir
    if [ -d "$specs_dir" ]; then
        for dir in "$specs_dir"/"$prefix"-*; do
            [ -d "$dir" ] && matches+=("$(basename "$dir")")
        done
    fi

    if [ ${#matches[@]} -eq 0 ]; then
        echo "$specs_dir/$branch"
    elif [ ${#matches[@]} -eq 1 ]; then
        echo "$specs_dir/${matches[0]}"
    else
        echo "ERROR: Multiple spec directories found with prefix '$prefix': ${matches[*]}" >&2
        echo "Please ensure only one spec directory exists per numeric prefix." >&2
        echo "$specs_dir/$branch"
    fi
}

# Prints shell assignments; use with: eval "$(get_feature_paths)"
get_feature_paths() {
    local repo_root current_branch has_git_repo="false" feature_dir
    repo_root="$(get_repo_root)" || return 1
    current_branch="$(get_current_branch)"
    has_git && has_git_repo="true"
    feature_dir="$(find_feature_dir_by_prefix "$repo_root" "$current_branch")"

    printf "REPO_ROOT=%q\n" "$repo_root"
    printf "CURRENT_BRANCH=%q\n" "$current_branch"
    printf "HAS_GIT=%q\n" "$has_git_repo"
    printf "FEATURE_DIR=%q\n" "$feature_dir"
    printf "FEATURE_SPEC=%q\n" "$feature_dir/spec.md"
    printf "IMPL_PLAN=%q\n" "$feature_dir/plan.md"
    printf "TASKS=%q\n" "$feature_dir/tasks.md"
    printf "RESEARCH=%q\n" "$feature_dir/research.md"
    printf "DATA_MODEL=%q\n" "$feature_dir/data-model.md"
    printf "QUICKSTART=%q\n" "$feature_dir/quickstart.md"
    printf "CONTRACTS_DIR=%q\n" "$feature_dir/contracts"
}

# Escape a string for inclusion in a JSON string literal.
json_escape() {
    local s="$1"
    s="${s//\\/\\\\}"
    s="${s//\"/\\\"}"
    s="${s//$'\n'/\\n}"
    s="${s//$'\r'/\\r}"
    s="${s//$'\t'/\\t}"
    printf '%s' "$s"
}

check_file() { [ -f "$1" ] && echo "  ✓ $2" || echo "  ✗ $2"; }
check_dir() { [ -d "$1" ] && [ -n "$(ls -A "$1" 2>/dev/null)" ] && echo "  ✓ $2" || echo "  ✗ $2"; }
