#!/usr/bin/env bash
# engram-lite UserPromptSubmit hook — recall nudge
printf '<system-reminder source="engram-lite">\n'
printf 'Relevant prior context? → memory_recall("your query") before responding.\n'
printf '</system-reminder>\n'
