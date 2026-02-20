# Claude Code Built-In Hooks Guide

Claude Code has a **built-in lifecycle hooks system** similar to Amplifier's kernel hooks.

## Built-In Hooks vs Subagent Hooks

| Feature | Built-In Hooks | Subagent Hooks |
|---------|----------------|----------------|
| **Configuration** | `.claude/settings.json` | Agent frontmatter |
| **Trigger** | Automatic (kernel-level) | Manual (@-mention) |
| **Scope** | All interactions | Per-subagent |
| **Context injection** | ✅ Yes (`additionalContext`) | ⚠️ Via agent memory |
| **Cannot be bypassed** | ✅ True | ❌ False |

**For memory system: Use built-in hooks.** They're automatic and unavoidable, just like Amplifier.

## Available Hook Events

### Memory System Hooks (Primary)

| Event | When It Fires | Our Usage |
|-------|---------------|-----------|
| **SessionStart** | Session begins/resumes/after compact | Inject protocol reminder |
| **UserPromptSubmit** | Before processing each prompt | Remind about memory locations |
| **Stop** | After Claude finishes response | Validate captures occurred |

### Other Available Events

| Event | When It Fires | Use Case |
|-------|---------------|----------|
| **PreToolUse** | Before any tool executes | Pre-validation, context prep |
| **PostToolUse** | After tool succeeds | Trigger actions, capture |
| **PostToolUseFailure** | Tool fails | Error handling |
| **SessionEnd** | Session terminates | Cleanup, final validation |
| **PreCompact** | Before context compaction | Re-inject important context |
| **PermissionRequest** | Permission dialog shown | Custom approval logic |

See `.claude/settings.json` for complete configuration.

## Configuration Format

**Location**: `.claude/settings.json` (project) or `~/.claude/settings.json` (user)

```json
{
  "$schema": "https://json.schemastore.org/claude-code-settings.json",
  "hooks": {
    "EventName": [
      {
        "matcher": "optional-regex-filter",
        "hooks": [
          {
            "type": "command",
            "command": "path/to/script.sh",
            "statusMessage": "Custom message...",
            "timeout": 30
          }
        ]
      }
    ]
  }
}
```

## Hook Input/Output

### Input (via stdin)

Hooks receive JSON on stdin with event-specific data:

```json
{
  "event": "UserPromptSubmit",
  "prompt": "User's message here",
  "timestamp": "2026-02-20T10:00:00Z"
}
```

### Output (via stdout)

Hooks return JSON to stdout:

```json
{
  "hookSpecificOutput": {
    "additionalContext": "Text injected into Claude's context",
    "decision": "approve|deny",
    "reason": "Why denied (if decision=deny)"
  }
}
```

**`additionalContext`** is injected into Claude's context automatically!

## Memory System Implementation

### Our Configuration

**File**: `.claude/settings.json`

```json
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "startup|resume|compact",
        "hooks": [{
          "type": "command",
          "command": "$CLAUDE_PROJECT_DIR/.claude/scripts/protocol-reminder.sh",
          "statusMessage": "Loading memory protocol..."
        }]
      }
    ],
    "UserPromptSubmit": [
      {
        "hooks": [{
          "type": "command",
          "command": "$CLAUDE_PROJECT_DIR/.claude/scripts/memory-search.sh",
          "statusMessage": "Searching memory..."
        }]
      }
    ],
    "Stop": [
      {
        "hooks": [{
          "type": "command",
          "command": "$CLAUDE_PROJECT_DIR/.claude/scripts/validate-capture.sh"
        }]
      }
    ]
  },
  "env": {
    "MEMORY_PROJECT_BASE": ".canvas/memory",
    "MEMORY_USER_BASE": "~/.canvas/memory"
  }
}
```

### Our Scripts

**`.claude/scripts/protocol-reminder.sh`**:
- Fires once per session start
- Injects RETRIEVE → RESPOND → CAPTURE protocol
- Includes configured memory paths

**`.claude/scripts/memory-search.sh`**:
- Fires on EVERY prompt
- Reminds about memory locations
- Can be enhanced to run actual searches

**`.claude/scripts/validate-capture.sh`**:
- Fires after EVERY response
- Checks git status for captures
- Reports when memory was updated

## Environment Variables

Available in hook scripts:

| Variable | Value | Usage |
|----------|-------|-------|
| `$CLAUDE_PROJECT_DIR` | Project root path | Reference scripts/files |
| `$MEMORY_PROJECT_BASE` | Project memory path | Custom config |
| `$MEMORY_USER_BASE` | User memory path | Custom config |

## Comparison to Amplifier

| Aspect | Amplifier Hooks | Claude Code Built-In Hooks |
|--------|-----------------|----------------------------|
| **Automatic firing** | ✅ Yes | ✅ Yes |
| **Configuration** | Bundle YAML | settings.json |
| **Scripts** | Python modules | Shell scripts |
| **Context injection** | `HookResult.context_injection` | `hookSpecificOutput.additionalContext` |
| **Cannot be bypassed** | ✅ True | ✅ True |
| **Event granularity** | Fine (prompt:submit, execution:end) | Fine (UserPromptSubmit, Stop) |

**Both provide kernel-level automatic enforcement!**

## Advanced: Hook Types

### Type: `command` (Our Choice)

Shell script execution:
```json
{
  "type": "command",
  "command": "path/to/script.sh",
  "timeout": 30,
  "async": false
}
```

**Pro**: Fast, deterministic, no LLM cost
**Con**: Limited to what shell scripts can do

### Type: `prompt` (LLM-Based)

LLM evaluates prompt:
```json
{
  "type": "prompt",
  "prompt": "Analyze this prompt and decide: $ARGUMENTS",
  "model": "claude-sonnet-4-6"
}
```

**Pro**: Can make complex decisions
**Con**: Slower, uses API credits

### Type: `agent` (Subagent)

Delegates to subagent:
```json
{
  "type": "agent",
  "agent": "memory-keeper"
}
```

**Pro**: Full tool access, persistent memory
**Con**: Slowest option

## Best Practices

1. **Use `command` type for memory system** - Fast, deterministic
2. **SessionStart for protocol** - Load once per session
3. **UserPromptSubmit for awareness** - Lightweight reminder every prompt
4. **Stop for validation** - Check captures occurred
5. **Keep scripts fast** - Hooks block execution
6. **Use environment variables** - Makes paths configurable

## Testing Hooks

**Verify hooks fire**:

1. Open project in VS Code with Claude Code
2. Check terminal output for status messages:
   ```
   Loading memory protocol...
   Searching memory...
   ```
3. Look for `<system-reminder>` tags in Claude's context
4. Verify captures create expected git status

## Custom Path Configuration

**To use different memory paths**, edit `.claude/settings.json`:

```json
{
  "env": {
    "MEMORY_PROJECT_BASE": ".memory",
    "MEMORY_USER_BASE": "~/.my-memory"
  }
}
```

Scripts automatically use these paths.

## Summary

✅ **Claude Code has built-in hooks that are equivalent to Amplifier's kernel hooks**
✅ **Configured via `.claude/settings.json`**
✅ **Fire automatically on lifecycle events**
✅ **Support context injection**
✅ **Cannot be bypassed**

**The memory system now uses built-in hooks in both platforms:**
- Amplifier: Python hook modules
- Claude Code: Shell script hooks

Both provide automatic protocol enforcement! 🎯
