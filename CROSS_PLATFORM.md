# Cross-Platform Engram: Amplifier + Claude Code

Engram works in **both Amplifier and Claude Code** using platform-specific configuration while sharing the same core protocols and memory files.

## Architecture Comparison

| Aspect | Amplifier | Claude Code |
|--------|-----------|-------------|
| **Protocol Enforcement** | Runtime hooks (`prompt:submit`, `execution:end`) | Built-in hooks (`UserPromptSubmit`, `Stop`) |
| **Configuration** | `.amplifier/bundles/engram.md` | `.claude/settings.json` |
| **Hook Implementation** | Python modules | Shell scripts |
| **Memory Paths** | Configurable via hook config | Configurable via environment variables |
| **Protocol Injection** | Ephemeral context injection | `additionalContext` injection |
| **Validation** | Post-execution git status check | Post-response git status check |
| **Automatic Firing** | ✅ Yes | ✅ Yes (built-in hooks) |

## Shared Components

**Both platforms use the same:**

1. **Memory directories**:
   - `~/.canvas/memory/` (user-global, private)
   - `.canvas/memory/` (project-local, shareable)

2. **Protocol files**:
   - `_protocols/inline-capture.md`
   - `_protocols/dual-write-decision.md`
   - `_protocols/scope-routing.md`
   - `_protocols/cross-reference-cascade.md`

3. **Core instructions**:
   - `AGENTS.md` (bootstrap file)
   - `MEMORY-SYSTEM.md` (architecture docs)

4. **Search tool**:
   - `scripts/canvas-memory-search.py`

## Platform-Specific Setup

### Amplifier

**Configuration**: `.amplifier/bundles/engram.md`

```yaml
hooks:
  - module: hooks-protocol-reminder
    config:
      priority: 5
      project_memory_base: ".canvas/memory"
      user_memory_base: "~/.canvas/memory"

  - module: hooks-memory-tracker
    config:
      priority: 90
      project_memory_base: ".canvas/memory"
      user_memory_base: "~/.canvas/memory"
```

**How it works:**
- `hooks-protocol-reminder` fires on `prompt:submit` → injects protocol into context
- `hooks-memory-tracker` fires on `execution:end` → validates git status for captures
- Both hooks use configurable base directories

**Usage:**
```bash
amplifier --bundle engram
amplifier run --bundle engram "your prompt"
```

### Claude Code

**Configuration**: `.claude/CLAUDE.md` + `.claude/rules/`

**How it works:**
- `CLAUDE.md` auto-loaded at session start
- Rules in `.claude/rules/` auto-loaded (modular)
- @-mention imports pull in protocol files on-demand
- Custom commands provide shortcuts (`/memory-search`, `/memory-capture`)
- Optional memory-keeper subagent with persistent storage

**Usage:**
1. Open project in VS Code
2. Claude Code reads `.claude/CLAUDE.md` automatically
3. Protocol enforced through persistent instructions
4. Use commands: `/memory-search`, `/memory-capture`

## Key Differences

### Protocol Enforcement

**Amplifier** (dynamic, kernel-level):
- Hooks fire on EVERY prompt automatically
- Fresh protocol reminder each turn
- Validated after each response
- Cannot be bypassed

**Claude Code** (dynamic, built-in hooks):
- Hooks configured in `.claude/settings.json`
- Fire automatically on SessionStart, UserPromptSubmit, Stop
- Inject context via shell scripts
- Validate captures after each response
- **Just like Amplifier** - automatic enforcement

**Claude Code Built-In Hooks**:
```
User: "your prompt"
→ UserPromptSubmit hook fires → reminds about memory locations
→ Claude processes with protocol awareness
→ Stop hook fires → validates captures occurred
```

**Same automatic behavior as Amplifier**, just different config format.

### When to Use Which

| Use Case | Best Platform |
|----------|---------------|
| Guaranteed protocol enforcement | Amplifier (hooks guarantee it) |
| Quick prototyping, exploration | Claude Code (faster startup) |
| Long-running sessions with validation | Amplifier (hooks persist) |
| VS Code integration needed | Claude Code (native integration) |
| Maximum portability | Both (same memory files) |

## Custom Base Directories

**Both platforms support custom paths.**

### Amplifier

Edit `.amplifier/bundles/engram.md`:
```yaml
hooks:
  - module: hooks-protocol-reminder
    config:
      project_memory_base: ".memory"  # Custom
      user_memory_base: "~/.my-memory"
```

### Claude Code

Edit `.claude/CLAUDE.md`:
```markdown
## Memory Locations (Configurable)

| Memory Type | Path |
|-------------|------|
| **User Memory** | `~/.my-memory/` |
| **Project Memory** | `.memory/` |

Before every response, search:
- User: `~/.my-memory/`
- Project: `.memory/`

After responding, route captures to these locations.
```

**Both must use matching paths** for cross-platform compatibility.

## Switching Between Platforms

**The memory files are platform-agnostic** - you can switch between Amplifier and Claude Code freely:

1. **Memory persists** in `~/.canvas/memory/` and `.canvas/memory/`
2. **Same search tool** works in both
3. **Same file formats** (YAML frontmatter + markdown)
4. **Same protocols** (`_protocols/` files)

**Example workflow:**
```
Morning: Use Amplifier for focused work with protocol enforcement
Afternoon: Switch to Claude Code in VS Code for integration tasks
→ All memory is available in both environments
```

## Migration Guide

### From Amplifier-Only to Cross-Platform

1. Already using Amplifier with memory-bundle? **Add Claude Code config:**
   ```bash
   # Claude Code files are already created
   # Just open project in VS Code with Claude Code extension
   ```

2. Memory files work immediately in both platforms

### From Claude Code-Only to Cross-Platform

1. Using `.claude/` config? **Add Amplifier support:**
   ```bash
   # Copy memory-bundle.md to .amplifier/bundles/
   # Install hook modules to .amplifier/modules/
   # Configure paths to match your CLAUDE.md settings
   ```

2. Memory files work immediately in both platforms

## Best Practices

### Path Configuration

**Use the same base directories** in both platforms:
- Amplifier: `.amplifier/bundles/engram.md`
- Claude Code: `.claude/CLAUDE.md`

**Default convention** (recommended):
```
User memory: ~/.canvas/memory/
Project memory: .canvas/memory/
```

### Protocol Files

**Keep protocol files in `_protocols/`** - both platforms reference them:
- Amplifier: Hooks inject protocol content
- Claude Code: @-mentions import protocol files

### Git

**Commit to version control:**
- ✅ `.amplifier/bundles/` (Amplifier config)
- ✅ `.amplifier/modules/` (hook modules)
- ✅ `.claude/` (Claude Code config)
- ✅ `.canvas/memory/` (project memory - shareable)
- ✅ `_protocols/` (protocol files)
- ✅ `scripts/` (search tools)

**Exclude from version control:**
- ❌ `~/.canvas/memory/` (user memory - private)

### Testing

**Verify both platforms work:**

```bash
# Test Amplifier
amplifier run --bundle engram "Test: Save this to memory: test data"

# Test Claude Code
# Open in VS Code, ask Claude to save something
# Verify files appear in expected locations

# Check consistency
ls .canvas/memory/
ls ~/.canvas/memory/
```

## Troubleshooting

### Claude Code Not Loading Configuration

**Issue**: CLAUDE.md not being read

**Solution**:
1. Check filename: must be exactly `CLAUDE.md` (case-sensitive)
2. Must be in project root or `.claude/` directory
3. Restart VS Code after creating/editing

### Different Behavior Between Platforms

**Issue**: Amplifier captures correctly, Claude Code doesn't

**Cause**: Claude Code uses static instructions (loaded once), not dynamic hooks

**Solution**:
- Use custom `/memory-capture` command in Claude Code
- Explicitly remind Claude of protocol if needed
- Consider using memory-keeper subagent for persistent protocol awareness

### Memory Files Not Found

**Issue**: Search returns no results

**Solution**:
1. Verify base paths match in both platform configs
2. Check that memory files exist: `ls ~/.canvas/memory/`
3. Verify search tool works: `python scripts/canvas-memory-search.py --keyword "test"`

## Summary

✅ **Same memory files** - work in both platforms
✅ **Same protocols** - shared `_protocols/` directory  
✅ **Same search tool** - `scripts/canvas-memory-search.py`
✅ **Configurable paths** - match across platforms
✅ **Platform-specific enforcement** - hooks vs static config

**Engram is cross-platform compatible.** Use whichever tool fits your workflow - the memory persists across both.
