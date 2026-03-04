#!/usr/bin/env bash
# engram-lite Stop hook — capture reminder
printf '<system-reminder source="engram-lite">\n'
printf 'CAPTURE CHECK (silent — do not announce):\n'
printf '  New knowledge? → memory_capture(content, content_type=...)\n'
printf '  Correction? → memory_capture(..., importance="high")\n'
printf '  Pattern seen again? → memory_capture(..., content_type="event")\n'
printf '</system-reminder>\n'
