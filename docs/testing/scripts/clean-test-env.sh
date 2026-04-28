#!/usr/bin/env bash
# clean-test-env.sh — remove the test environment and unset env vars
#
# Usage: source ./docs/testing/scripts/clean-test-env.sh [--dir <path>] [--yes]

# Detect if sourced or executed (works in bash and zsh)
_SOURCED=0
if [ -n "${ZSH_VERSION:-}" ]; then
    case "$ZSH_EVAL_CONTEXT" in *:file*) _SOURCED=1 ;; esac
elif [ -n "${BASH_VERSION:-}" ]; then
    [[ "${BASH_SOURCE[0]}" != "$0" ]] && _SOURCED=1
fi

TEST_DIR="${LLM_WIKI_TEST_DIR:-$HOME/llm-wiki-testing}"
CONFIRMED=0

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dir) TEST_DIR="$2"; shift 2 ;;
        --yes) CONFIRMED=1; shift ;;
        *) echo "Unknown argument: $1"; return 1 2>/dev/null || exit 1 ;;
    esac
done

green() { printf '\033[32m%s\033[0m\n' "$*"; }

if [ ! -d "$TEST_DIR" ]; then
    echo "Nothing to clean — $TEST_DIR does not exist."
else
    if [ "$CONFIRMED" = "0" ]; then
        if [ ! -t 0 ]; then
            echo "Non-interactive shell — pass --yes to confirm deletion."
            return 0 2>/dev/null || exit 0
        fi
        echo "This will permanently delete: $TEST_DIR"
        printf "Continue? [y/N] "
        read -r answer
        case "$answer" in
            [yY]*) ;;
            *) echo "Aborted."; return 0 2>/dev/null || exit 0 ;;
        esac
    fi
    rm -rf "$TEST_DIR"
    green "  ✓ Removed $TEST_DIR"
fi

if [ "$_SOURCED" = "1" ]; then
    unset LLM_WIKI_TEST_DIR
    unset LLM_WIKI_CONFIG
    green "  ✓ Unset LLM_WIKI_TEST_DIR and LLM_WIKI_CONFIG"
fi
