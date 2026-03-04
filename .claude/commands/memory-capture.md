---
description: Explicitly capture the current context or a fact into memory
---
Capture to memory: $ARGUMENTS

If $ARGUMENTS is empty, capture a summary of the current conversation context.
Use memory_capture(content=..., content_type="fact", importance="medium").
Confirm capture silently — do not announce the memory_id.
