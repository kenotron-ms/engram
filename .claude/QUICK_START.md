# Quick Start: Built-In Hooks in Claude Code

Claude Code has built-in lifecycle hooks that fire automatically - just like Amplifier!

## Already Configured

Open this project in VS Code with Claude Code. The hooks are **already active**:

✅ `.claude/settings.json` - Hooks configuration
✅ `.claude/scripts/` - Hook implementations
✅ Fires automatically on every interaction

## What Happens

```
1. You open the project in VS Code
   → SessionStart hook fires
   → Protocol reminder injected: "RETRIEVE → RESPOND → CAPTURE"

2. You send a message
   → UserPromptSubmit hook fires
   → Memory locations reminded

3. Claude responds
   → Stop hook fires
   → Validates if captures occurred
```

## No Action Required

The hooks work automatically. You don't need to:
- ❌ @-mention special agents
- ❌ Use custom commands
- ❌ Remember to search memory

**The protocol is enforced automatically**, just like Amplifier.

## Testing

Send a test message in Claude Code:
```
"Save this preference: I like automatic systems"
```

Watch for:
- Status spinner: "Loading memory protocol..."
- Capture to `~/.canvas/memory/personal/preferences/`
- Validation message after response

## Customizing Paths

Edit `.claude/settings.json`:

```json
{
  "env": {
    "MEMORY_PROJECT_BASE": ".your-custom-path",
    "MEMORY_USER_BASE": "~/.your-memory-path"
  }
}
```

## Documentation

- `.claude/HOOKS_GUIDE.md` - Complete built-in hooks reference
- `CROSS_PLATFORM.md` - Amplifier vs Claude Code comparison
- `MEMORY_CONFIG.md` - Path configuration guide

## Summary

**Built-in hooks provide kernel-level enforcement in Claude Code** - same automatic behavior as Amplifier, different configuration format.
