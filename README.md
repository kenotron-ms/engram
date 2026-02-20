# Engram

Agentic memory augmentation - external memory storage with automatic capture, intelligent retrieval, and cognitive compute offload.

A personal knowledge graph system that works in both **Amplifier** and **Claude Code**.

## Quick Start

### In Amplifier

Add Engram to your bundle's `includes:` section:

```yaml
# In your bundle.md
includes:
  - bundle: git+https://github.com/kenotron-ms/engram@main
```

Then use your bundle as normal:

```bash
amplifier --bundle your-bundle
```

### In Claude Code

1. Open this project in VS Code
2. Claude Code automatically loads:
   - `.claude/CLAUDE.md` (instructions)
   - `.claude/settings.json` (built-in hooks)
   - `.claude/rules/` (protocol rules)
3. Start interacting - **hooks fire automatically on every prompt**

**Built-in hooks configured:**
- `SessionStart` → Protocol reminder injected
- `UserPromptSubmit` → Memory locations reminded
- `Stop` → Capture validation

## Core Concept

Every interaction follows **RETRIEVE → RESPOND → CAPTURE**:

1. **RETRIEVE**: Search memory before responding
2. **RESPOND**: Apply retrieved knowledge
3. **CAPTURE**: Save new learnings after responding

**Silent operation** - you see results, not mechanics.

## Dual-Memory Architecture

| Memory Type | Location | Purpose |
|-------------|----------|---------|
| **User Memory** | `~/.canvas/memory/` | Personal knowledge (endures across ALL projects) |
| **Project Memory** | `.canvas/memory/` | Project-specific knowledge (safe to share) |

### Routing Decision

**Personal info** (preferences, constraints) → User memory ONLY

**Technical knowledge** (README-safe) → BOTH locations

## Files

```
.
├── bundle.md                     # Root bundle (for distribution)
├── AGENTS.md                     # Bootstrap file (loaded every session)
├── MEMORY-SYSTEM.md              # Complete architecture
├── MEMORY_CONFIG.md              # Path configuration
├── CROSS_PLATFORM.md             # Amplifier vs Claude Code
│
├── modules/                      # Local hook modules
│   ├── hooks-protocol-reminder/  # Protocol injection hook (Python)
│   └── hooks-memory-tracker/     # Capture validation hook (Python)
│
├── .claude/                      # Claude Code configuration
│   ├── settings.json             # Built-in hooks config (AUTOMATIC)
│   ├── CLAUDE.md                 # Auto-loaded instructions
│   ├── HOOKS_GUIDE.md            # Built-in hooks documentation
│   ├── scripts/                  # Hook implementations
│   │   ├── protocol-reminder.sh  # SessionStart hook
│   │   ├── memory-search.sh      # UserPromptSubmit hook
│   │   └── validate-capture.sh   # Stop hook
│   ├── rules/                    # Auto-loaded protocol rules
│   │   ├── memory-protocol.md
│   │   ├── dual-write.md
│   │   └── search-strategy.md
│   ├── agents/                   # Optional subagents
│   │   ├── protocol-enforcer.md
│   │   └── memory-keeper.md
│   └── commands/                 # Custom shortcuts
│       ├── memory-search.md
│       └── memory-capture.md
│
├── _protocols/                   # Detailed processing protocols
│   ├── inline-capture.md
│   ├── dual-write-decision.md
│   ├── scope-routing.md
│   └── cross-reference-cascade.md
│
├── scripts/
│   └── canvas-memory-search.py   # YAML-aware search tool
│
└── .canvas/memory/               # Project memory (shareable)
```

## Key Features

### Cross-Platform

Same memory files work in both Amplifier and Claude Code:
- Amplifier: Kernel-level hooks (automatic enforcement)
- Claude Code: Subagent hooks (manual activation)

### Configurable Paths

Both platforms support custom base directories:

```yaml
# In your bundle that includes Engram
includes:
  - bundle: git+https://github.com/kenotron-ms/engram@main

# Then configure if needed (optional - defaults work)
# The hooks from Engram will use .canvas/memory by default
```

```markdown
# Claude Code (.claude/CLAUDE.md)
Memory Locations:
- User: ~/.my-memory/
- Project: .memory/
```

### Protocol Enforcement

**Amplifier** (automatic):
- Runtime hooks fire on every prompt
- Cannot be bypassed

**Claude Code** (opt-in):
- Use `@protocol-enforcer` for hook-based enforcement
- Or rely on auto-loaded rules

## Documentation

| File | Purpose |
|------|---------|
| `AGENTS.md` | Bootstrap instructions for AI agents |
| `MEMORY-SYSTEM.md` | Complete system architecture |
| `MEMORY_CONFIG.md` | Custom path configuration |
| `CROSS_PLATFORM.md` | Amplifier vs Claude Code comparison |
| `.claude/HOOKS_GUIDE.md` | Claude Code subagent hooks |
| `_protocols/*.md` | Detailed processing protocols |

## Examples

### Capture Personal Preference

```
User: "I prefer morning coding sessions"
→ Routes to: ~/.canvas/memory/personal/preferences/
```

### Capture Technical Knowledge

```
User: "Engram uses YAML frontmatter for keyword search"
→ Routes to BOTH:
  - ~/.canvas/memory/projects/engram/
  - .canvas/memory/
```

### Search Memory

```bash
python scripts/canvas-memory-search.py \
  --keyword "morning,coding,schedule" \
  --domain "personal/preferences/"
```

## Testing

### Test Amplifier

```bash
# Test the bundle directly
amplifier run --bundle git+https://github.com/kenotron-ms/engram@main \
  "Test: Save this: Ken likes portable systems"
```

### Test Claude Code

1. Open in VS Code
2. Use: `@protocol-enforcer "Test: Save this: Ken likes portable systems"`
3. Check: `git status` shows files in `.canvas/memory/`

## Why Two Memory Locations?

**User memory** (`~/.canvas/memory/`):
- Your private knowledge
- Endures across ALL projects
- Never shared or committed to git

**Project memory** (`.canvas/memory/`):
- Project-specific knowledge
- Safe to share publicly (treat as README)
- Can be committed to git
- Helps collaborators

**Routing**: Personal info goes to user memory ONLY. Technical knowledge that passes the "README test" goes to BOTH.

## Requirements

- Python 3.11+
- Git (for capture validation)
- Amplifier OR Claude Code

## License

See LICENSE file.

## Contributing

This is a personal knowledge graph system. Fork and adapt for your own use!
