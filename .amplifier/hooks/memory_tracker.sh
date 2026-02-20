#!/usr/bin/env bash
# Memory Tracker - fires on Stop (after response complete)
# Validates that new knowledge was captured to memory system

set -euo pipefail

# Read JSON from stdin
JSON_INPUT=$(cat)

# Check git status for new/modified files in .canvas/memory/
cd "${AMPLIFIER_PROJECT_DIR:-.}"

# Look for modified or new files in memory paths
MEMORY_CHANGES=$(git status --short 2>/dev/null | grep "/.canvas/memory/" || true)

if [ -n "$MEMORY_CHANGES" ]; then
    # Knowledge was captured
    cat << EOF
{
  "decision": "approve",
  "systemMessage": "💾 Knowledge captured to memory system"
}
EOF
else
    # No capture detected - just approve (gentle enforcement)
    cat << EOF
{
  "decision": "approve"
}
EOF
fi

exit 0
