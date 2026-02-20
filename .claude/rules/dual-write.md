# Dual-Write Decision Tree

Auto-loaded rule for routing knowledge to correct memory locations.

## The Two Memory Spaces

**User Private Memory**: `~/.canvas/memory/`
- Your private knowledge (preferences, patterns, constraints)
- Organized by domain (projects/, professional/, personal/)
- Never shared, never committed to git
- **Endures across ALL projects**

**Project Shareable Memory**: `.canvas/memory/`
- Knowledge specific to THIS project
- Safe to share publicly (treat as public)
- Helps collaborators understand project context
- **Endures across sessions within THIS project**

## Decision Tree

### Step 1: Personal Information?

**Is this about a specific individual personally?**

Ask: "Does this describe a person rather than a project?"

| Signal | Example | Action |
|--------|---------|--------|
| "I prefer..." | "I prefer morning coding" | User memory ONLY |
| "I don't have access..." | "I don't have access to X" | User memory ONLY |
| Work schedule/patterns | "Ken codes in mornings" | User memory ONLY |
| Communication preferences | "Bottom-line-first style" | User memory ONLY |
| Individual constraints | "Can't use service Y" | User memory ONLY |
| Biographical details | Birthday, location, personal background | User memory ONLY |
| Personal project preferences | "Ken prefers portable systems" | User memory ONLY |

**Exception — Team/People in Project Context:**
If the information is about a person's **role, membership, or goals within this project**, it belongs in project memory:

| Signal | Example | Action |
|--------|---------|--------|
| Role on the project | "Alice is the lead engineer" | Project memory OK |
| Project-specific goals | "Bob owns the MCP server milestone" | Project memory OK |
| Team membership | "Ken, Alice, Bob are contributors" | Project memory OK |

These go in `.canvas/memory/people/{name}.md` with project-scoped context only — no personal details.

**→ If personal (not project-role)**: Write to `~/.canvas/memory/personal/` ONLY. **STOP HERE.**

### Step 2: Helpful to Project?

**Is this knowledge about THIS project specifically?**

| Not Helpful | Helpful |
|-------------|---------|
| General domain knowledge | Project-specific architecture |
| Temporary conversation state | Technical decisions made |
| Pure opinion without context | Design patterns used |

**→ If NO**: Skip capture or write to user memory. **STOP HERE.**

### Step 3: README Test

**"Could this appear in the project README without causing harm?"**

**Safe to Share** (write to BOTH):
- ✅ Architecture decisions with rationale
- ✅ Technical patterns and why chosen
- ✅ Design tradeoffs explained
- ✅ Tool choices and reasons
- ✅ Public-safe performance characteristics

**NOT Safe to Share** (user memory ONLY):
- ❌ Personal observations ("Ken prefers X")
- ❌ Individual constraints ("Ken can't access Y")
- ❌ Internal politics or team dynamics
- ❌ Sensitive performance data
- ❌ Access limitations or credentials

**→ If SAFE**: Dual-write to BOTH locations

**→ If NOT SAFE**: User memory ONLY

## Dual-Write Pattern

When writing to both locations, create TWO files:

**User Memory** (full context):
```
~/.canvas/memory/projects/memory-system/hot-cold-architecture.md
```

Contains:
- Your relationship to the knowledge
- Personal notes and observations
- Links to related personal knowledge
- Full context including sensitive details

**Project Memory** (sanitized, factual):
```
.canvas/memory/architecture-decisions.md
```

Contains:
- Pure technical facts
- Architecture rationale
- Design patterns
- Tool choices and tradeoffs
- NO personal observations
- NO sensitive information

## Anti-Patterns

| DON'T | DO |
|-------|-----|
| Write "Ken prefers X" to project memory | Write "System uses X because [technical reason]" |
| Include individual constraints in shareable docs | Write technical constraints only |
| Default to dual-write | Default to user-only, dual-write requires README test |
| Share personal observations | Keep personal info in user memory |
| Write biographical details (birthday, location) anywhere in project memory | Keep biographical info in user memory only |
| Write "Ken prefers portable systems" to project memory | Write "Alice owns the MCP server milestone" to project memory (role, not preference) |

## Red Flags

If you think any of these, STOP and write to user memory ONLY:
- "This is probably safe to share"
- "Just this one personal detail won't hurt"
- "I'll sanitize it later"
- "Team needs to know about individual constraints"

**All of these mean: user memory ONLY.**

## When in Doubt

**Default: User memory ONLY.**

You can always add to project memory later. You can't un-share.
