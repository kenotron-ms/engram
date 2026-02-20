# Engram - Claude Code Configuration

This project uses **Engram** - agentic memory augmentation with automatic capture, intelligent retrieval, and cognitive compute offload.

## Memory Locations (Configurable)

| Memory Type | Default Path | Purpose |
|-------------|--------------|---------|
| **User Memory** | `~/.canvas/memory/` | Personal knowledge (endures across ALL projects) |
| **Project Memory** | `.canvas/memory/` | Project-specific knowledge (safe to share publicly) |

> **Note**: Base paths are configurable. See `MEMORY_CONFIG.md` for custom path configuration.

## Core Protocol

Every interaction follows the **RETRIEVE → RESPOND → CAPTURE** loop:

See @AGENTS.md for complete agent instructions
See @_protocols/inline-capture.md for the full RETRIEVE → RESPOND → CAPTURE protocol
See @_protocols/dual-write-decision.md for user vs project memory routing

## Before Every Response (RETRIEVE)

1. **Infer domain** from conversation context (personal/professional/projects)
2. **Extract keywords** with variations (singular/plural, synonyms)
3. **Search BOTH memories**:
   - User: `~/.canvas/memory/` (or configured user_memory_base)
   - Project: `.canvas/memory/` (or configured project_memory_base)
4. **Load 2-3 most relevant files** from each location
5. **Apply knowledge** in your response

Use the memory search tool: `python scripts/canvas-memory-search.py --keyword "term" --domain "domain/"`

## After Every Response (CAPTURE)

**Question**: Did I learn something NEW? (preference, constraint, decision, pattern, context)

**If YES, apply the Dual-Write Decision:**

### Personal Information → User Memory ONLY

Examples:
- "Ken prefers morning coding sessions" → `~/.canvas/memory/personal/preferences/`
- "Ken doesn't have access to X" → `~/.canvas/memory/personal/constraints/`
- Individual work patterns → `~/.canvas/memory/personal/`

**README Test**: Could this appear in project README? **NO** → User memory ONLY

### Technical Knowledge → BOTH Locations (Dual-Write)

Examples:
- "Memory system uses YAML frontmatter" → BOTH user + project
- "Hot/cold tiers optimize grep performance" → BOTH user + project  
- Architecture decisions with rationale → BOTH user + project

**README Test**: Could this appear in project README? **YES** → Write to BOTH:
1. `~/.canvas/memory/projects/{project-name}/` (user copy for portability)
2. `.canvas/memory/` (project copy for team sharing)

## Silent Operation

- **DO NOT announce** when you search memory
- **DO NOT announce** when you capture knowledge
- **Execute the protocol silently** and naturally

The memory system works in the background - users see results, not mechanics.

## Built-In Hooks (Automatic Protocol Enforcement)

**Claude Code has kernel-level hooks** configured in `.claude/settings.json` that fire automatically:

| Hook Event | When It Fires | What It Does |
|------------|---------------|--------------|
| `SessionStart` | Session starts/resumes | Injects protocol reminder |
| `UserPromptSubmit` | Before each prompt | Reminds about memory locations |
| `Stop` | After each response | Validates project memory captures |

**These hooks fire automatically** - no need to @-mention anything. Just like Amplifier!

See `.claude/settings.json` for configuration and `.claude/scripts/` for hook implementations.

## Optional: Subagent Hooks for Advanced Use

If you need additional control, use subagents with lifecycle hooks:

### Option 1: Protocol-Enforcer (Recommended)

`@protocol-enforcer` - Wrapper agent with hooks that enforce protocol:
- **Start hook**: Searches memory BEFORE responding
- **End hook**: Validates and captures AFTER responding
- Closest equivalent to Amplifier's kernel hooks

**Usage**: `@protocol-enforcer "your prompt"`

### Option 2: Memory-Keeper

`@memory-keeper` - Specialized memory management agent:
- Has lifecycle hooks for protocol compliance
- Maintains persistent memory across sessions
- Best for explicit memory operations

**Usage**: `@memory-keeper capture this knowledge: ...`

### Limitation vs Amplifier

| Feature | Amplifier | Claude Code |
|---------|-----------|-------------|
| Auto-fires on EVERY prompt | ✅ Yes | ❌ No (must @-mention) |
| Cannot be bypassed | ✅ True | ❌ False |
| Post-execution validation | ✅ Yes | ✅ Yes (in subagent) |

**Trade-off**: Subagent hooks provide enforcement when used, but aren't automatic like Amplifier's kernel hooks.

## For More Detail

Full documentation:
- @MEMORY-SYSTEM.md - Complete system architecture
- @_protocols/ - Detailed processing protocols
- @MEMORY_CONFIG.md - Path configuration guide
- @CROSS_PLATFORM.md - Amplifier vs Claude Code comparison

## Modular Rules

Additional rules are auto-loaded from `.claude/rules/`:
- `memory-protocol.md` - Full RETRIEVE → RESPOND → CAPTURE details
- `dual-write.md` - Complete routing decision tree
- `search-strategy.md` - Domain inference and keyword extraction

## Available Agents

- `@memory-keeper` - Memory management with lifecycle hooks
- `@protocol-enforcer` - Protocol enforcement wrapper with hooks

## Custom Commands

- `/memory-search` - Search both memory locations
- `/memory-capture` - Capture with dual-write decision

---

**This configuration provides both static (rules) and dynamic (subagent hooks) enforcement. Use subagents for Amplifier-like behavior.**
