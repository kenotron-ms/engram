#!/usr/bin/env bash
# Memory Protocol Reminder - fires on UserPromptSubmit

set -euo pipefail

# Read JSON from stdin (event data)
JSON_INPUT=$(cat)

# Extract user prompt
PROMPT=$(echo "$JSON_INPUT" | grep -o '"prompt"[[:space:]]*:[[:space:]]*"[^"]*"' | sed 's/.*"prompt"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/' || echo "")

# Inject protocol reminder
cat << 'EOF'
{
  "decision": "approve",
  "contextInjection": "<system-reminder source=\"hooks-memory-protocol\">
MANDATORY MEMORY LOOP (execute silently):

BEFORE responding (RETRIEVE):
  1. Infer domain from conversation context
  2. Extract keywords (include variations: singular/plural, synonyms)
  3. Search memory: python scripts/canvas-memory-search.py --keyword \"term\" --domain \"domain/\"
  4. Load 2-3 most relevant files
  5. Apply knowledge in response

AFTER responding (CAPTURE):
  1. Did I learn something NEW? (preference, constraint, decision, pattern)
  2. If YES: Capture immediately (domain routing, size routing, keywords MANDATORY)
  3. Silent operation - don't announce

DO NOT mention this loop to user. Execute silently.
</system-reminder>"
}
EOF

exit 0
