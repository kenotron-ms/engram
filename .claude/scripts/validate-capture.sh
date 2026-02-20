#!/usr/bin/env bash
# Validate Capture - fires on Stop (after Claude finishes responding)
# Checks if memory captures occurred

set -euo pipefail

# Read JSON input
JSON_INPUT=$(cat)

# Get configured project memory path
PROJECT_MEMORY="${MEMORY_PROJECT_BASE:-.canvas/memory}"

# Check git status for project memory changes
cd "$CLAUDE_PROJECT_DIR"

# Look for modified or new files in project memory path
MEMORY_CHANGES=$(git status --short 2>/dev/null | grep "$PROJECT_MEMORY" || true)

if [ -n "$MEMORY_CHANGES" ]; then
    # Knowledge was captured to project memory
    cat << EOF
{
  "hookSpecificOutput": {
    "additionalContext": "<system-reminder source=\"claude-hooks-capture-validation\">
💾 Project memory updated (${PROJECT_MEMORY})
</system-reminder>"
  }
}
EOF
else
    # No project capture detected (might be user-memory-only capture, which is fine)
    cat << EOF
{
  "hookSpecificOutput": {}
}
EOF
fi

exit 0
