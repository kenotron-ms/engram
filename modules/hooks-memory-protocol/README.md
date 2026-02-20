# Memory Protocol Enforcement Hook

Enforces the RETRIEVE → RESPOND → CAPTURE loop for Canvas Memory system.

## What It Does

**Before each LLM call:**
- Injects reminder about the mandatory memory loop
- Reminds to search memory before responding
- Reminds to capture new knowledge after responding

**On user input:**
- Detects signals that new knowledge is being provided
- Sets flag for potential capture validation

**After execution:**
- Validates capture occurred when needed (future enhancement)

## Installation

```bash
cd ~/workspace/memory-system/modules/hooks-memory-protocol
pip install -e .
```

## Configuration

Add to your bundle:

```yaml
hooks:
  - module: hooks-memory-protocol
    source: file:///Users/ken/workspace/memory-system/modules/hooks-memory-protocol
    config:
      inject_pre_request: true          # Reminder before each LLM call
      inject_validation: true           # Validation after execution
      priority: 5                       # After status-context, before todo-reminder
      
      # Customize capture triggers (optional)
      capture_triggers:
        - "i prefer"
        - "remember that"
        - "my style"
        - "don't have access"
        - "we decided"
```

## How It Works

### Phase 1: Pre-Request Reminder

Fires on `provider:request` (before each LLM call):

```
<system-reminder source="hooks-memory-protocol">
MANDATORY MEMORY LOOP:
  BEFORE: Search memory, load relevant files
  AFTER: Capture new knowledge silently
</system-reminder>
```

### Phase 2: Trigger Detection

Fires on `prompt:submit` when user message contains:
- "I prefer..." → preference
- "Don't have access to..." → constraint
- "We decided..." → decision
- Other configurable triggers

Sets flag: `needs_capture = true`

### Phase 3: Validation (Future)

Fires on `execution:end`:
- Checks if write_file was called to `~/.canvas/memory/`
- If needed but missing: injects non-ephemeral reminder
- If completed: clears flag

**Current status:** Phase 3 validation not yet implemented (requires coordinator API enhancement).

## Development

Run tests:
```bash
pytest tests/
```

## Design Philosophy

**Gentle enforcement:**
- Pre-request reminder is ephemeral (doesn't bloat history)
- Trigger detection is signal-based (not hard requirement)
- Validation reminder only when high confidence capture needed

**Silent operation:**
- User never sees protocol machinery
- All reminders include "DO NOT mention to user"
- Infrastructure, not UX

**Fail-safe:**
- If hook fails, session continues
- If state management unavailable, gracefully degrades
- Logging for debugging, no crashes
