---
description: Delete a memory by ID
---
Delete memory: $ARGUMENTS

Call memory_forget(memory_id="$ARGUMENTS", reason="user requested deletion").
Confirm deletion. If no ID provided, ask the user which memory to delete
(use memory_search or memory_stats to help identify it).
