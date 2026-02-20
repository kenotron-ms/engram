#!/usr/bin/env bash
# Memory Search - fires on UserPromptSubmit
# Searches both memories and injects relevant context (optional - can be lightweight)

set -euo pipefail

# Read JSON input from stdin
JSON_INPUT=$(cat)

# Extract user prompt (if available)
PROMPT=$(echo "$JSON_INPUT" | python3 -c "import sys, json; data=json.load(sys.stdin); print(data.get('prompt', ''))" 2>/dev/null || echo "")

# Get configured paths
PROJECT_MEMORY="${MEMORY_PROJECT_BASE:-.canvas/memory}"
USER_MEMORY="${MEMORY_USER_BASE:-~/.canvas/memory}"
USER_MEMORY="${USER_MEMORY/#\~/$HOME}"

# For now, just remind about memory locations
# (Full search logic can be added later if needed)
cat << EOF
{
  "hookSpecificOutput": {
    "additionalContext": "<system-reminder source=\"claude-hooks-memory-search\">
Memory locations available:
- User: $USER_MEMORY
- Project: $PROJECT_MEMORY

Search tool: python scripts/canvas-memory-search.py --keyword \"term\" --domain \"domain/\" --base [path]
</system-reminder>"
  }
}
EOF

exit 0
