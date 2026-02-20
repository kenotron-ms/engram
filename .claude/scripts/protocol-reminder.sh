#!/usr/bin/env bash
# Protocol Reminder - fires on SessionStart (startup|resume|compact)
# Injects memory protocol reminder into Claude's context

set -euo pipefail

# Read JSON input from stdin (event data)
JSON_INPUT=$(cat)

# Get configured paths from environment (fallback to defaults)
PROJECT_MEMORY="${MEMORY_PROJECT_BASE:-.canvas/memory}"
USER_MEMORY="${MEMORY_USER_BASE:-~/.canvas/memory}"

# Expand tilde in user memory path
USER_MEMORY="${USER_MEMORY/#\~/$HOME}"

# Inject protocol reminder via additionalContext
cat << EOF
{
  "hookSpecificOutput": {
    "additionalContext": "<system-reminder source=\"claude-hooks-protocol\">
MANDATORY MEMORY LOOP (execute silently):

BEFORE responding (RETRIEVE):
  1. Infer domain from conversation context
  2. Extract keywords (include variations: singular/plural, synonyms)
  3. Search BOTH memories: $USER_MEMORY (user private) AND $PROJECT_MEMORY (project)
  4. Load 2-3 most relevant files from each
  5. Apply knowledge in response

AFTER responding (CAPTURE):
  1. Did I learn something NEW? (preference, constraint, decision, pattern)
  2. If YES: DUAL-WRITE DECISION (CRITICAL):
     
     PERSONAL info (preferences, constraints) → $USER_MEMORY/personal/ ONLY
     PROJECT tech knowledge (safe to share) → BOTH $USER_MEMORY/projects/{name}/ AND $PROJECT_MEMORY/
     
     README test: \"Could this appear in project README without causing harm?\"
     - NO → $USER_MEMORY only
     - YES → Both locations
  
  3. Capture with proper routing, keywords MANDATORY
  4. Silent operation - don't announce

DO NOT mention this loop to user. Execute silently.
</system-reminder>"
  }
}
EOF

exit 0
