#!/usr/bin/env bash
# engram-lite session-start hook
# Reads MEMORY.md files and injects as <system-reminder>
set -euo pipefail

ENGRAM="${ENGRAM_BIN:-engram-lite}"
USER_MEMORY="${ENGRAM_USER_MEM:-$HOME/.engram/MEMORY.md}"
PROJECT_MEMORY="${ENGRAM_PROJECT_MEM:-.engram/MEMORY.md}"

printf '<system-reminder source="engram-lite">\n'

injected=0

# User-scope MEMORY.md
if [ -f "$USER_MEMORY" ]; then
    # Refresh ## Now section silently
    "$ENGRAM" refresh-now "$USER_MEMORY" 2>/dev/null || true
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

# First run — no MEMORY.md files yet
if [ "$injected" -eq 0 ]; then
    "$ENGRAM" init 2>/dev/null || true
    printf 'Memory initialized. Use memory_capture() to start building your knowledge store.\n'
fi

printf 'Full memory: memory_recall(query) | memory_search(query) | memory_graph_explore()\n'
printf '</system-reminder>\n'
