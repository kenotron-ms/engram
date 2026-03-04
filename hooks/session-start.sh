#!/usr/bin/env bash
# engram-lite session-start hook
# Reads MEMORY.md files and injects as <system-reminder>
# Zero-install: uses uvx if engram-lite is not already in PATH.
set -euo pipefail

REPO="git+https://github.com/kenotron-ms/engram-lite"
USER_MEMORY="${ENGRAM_USER_MEM:-$HOME/.engram/MEMORY.md}"
PROJECT_MEMORY="${ENGRAM_PROJECT_MEM:-.engram/MEMORY.md}"

# Run engram-lite — prefers a locally installed binary, falls back to uvx.
_engram() {
    if command -v engram-lite &>/dev/null; then
        engram-lite "$@"
    else
        uvx --from "$REPO" engram-lite "$@"
    fi
}

printf '<system-reminder source="engram-lite">\n'

# Memory instructions and tool awareness
cat "${CLAUDE_PLUGIN_ROOT}/context/memory-instructions.md" 2>/dev/null || true
printf '\n'
cat "${CLAUDE_PLUGIN_ROOT}/context/memory-awareness.md" 2>/dev/null || true
printf '\n'

injected=0

# User-scope MEMORY.md
if [ -f "$USER_MEMORY" ]; then
    _engram refresh-now "$USER_MEMORY" 2>/dev/null || true
    printf '[MEMORY — user]\n'
    cat "$USER_MEMORY"
    printf '\n'
    injected=1
fi

# Project-scope MEMORY.md
if [ -f "$PROJECT_MEMORY" ]; then
    printf '[MEMORY — project]\n'
    cat "$PROJECT_MEMORY"
    printf '\n'
    injected=1
fi

# First run — initialize memory store
if [ "$injected" -eq 0 ]; then
    _engram init 2>/dev/null || true
    printf 'Memory initialized. Use memory_capture() to start building your knowledge store.\n'
fi

printf 'Full memory: memory_recall(query) | memory_search(query) | memory_graph_explore()\n'
printf '</system-reminder>\n'
