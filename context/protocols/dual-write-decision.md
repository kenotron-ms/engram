# Dual-Write Decision Protocol

> **Use when:** Capturing knowledge that might be relevant to both user memory AND project memory.

```
NO SENSITIVE INFORMATION IN PROJECT MEMORY
```

**Violating the letter of this protocol is violating the spirit of this protocol.**

---

## The Two Memory Spaces

**User Private Memory:** `~/.canvas/memory/`
- Your private knowledge graph
- Organized by domain (projects/, professional/, personal/)
- Never shared, never committed to git

**Project Shareable Memory:** `./.canvas/memory/`
- Project-specific knowledge
- Safe to share publicly (treat as public)
- Can be committed to project git repo
- Helps collaborators understand project context

---

## The Decision Tree

When capturing knowledge, ask these questions in order:

### Question 1: Is this a personal observation or preference?

| If YES | Action |
|--------|--------|
| Contains "I prefer", "I don't have access", "my style" | Write ONLY to `~/.canvas/memory/` |
| About how you work | Write ONLY to `~/.canvas/memory/` |
| Your constraints or limitations | Write ONLY to `~/.canvas/memory/` |
| Your communication style | Write ONLY to `~/.canvas/memory/` |

**Stop here. Personal observations NEVER go to project memory.**

### Question 2: Is this helpful to the project?

| If NO | Action |
|-------|--------|
| Not related to project work | Write ONLY to `~/.canvas/memory/` |
| Pure domain knowledge (portable across projects) | Write ONLY to `~/.canvas/memory/professional/` |
| Temporary conversation state | Don't capture at all |

**Stop here. If not helpful to project, it doesn't belong in project memory.**

### Question 3: Is this safe to share publicly?

Apply the **README test**: "Could this appear in the project README without causing harm?"

| Category | Safe? | Example |
|----------|-------|---------|
| **Architecture decisions** | ✅ YES | "We use hot/cold tiers to optimize grep performance" |
| **Technical rationale** | ✅ YES | "Keywords simulate embeddings for retrieval" |
| **Design patterns** | ✅ YES | "Domain-scoped search keeps queries fast" |
| **Tool choices** | ✅ YES | "YAML-aware search handles multi-line arrays" |
| **Personal observations** | ❌ NO | "Ken prefers bottom-line-first style" |
| **Individual constraints** | ❌ NO | "Ken doesn't have access to X" |
| **Internal politics** | ❌ NO | "Team disagreed on approach" |
| **Sensitive data** | ❌ NO | API keys, credentials, private info |
| **Performance metrics** | ⚠️ MAYBE | "Search completes in <50ms" (OK), "Server can only handle 10 req/s" (sensitive) |

**If safe to share:** Write to BOTH `~/.canvas/memory/projects/{name}/` AND `./.canvas/memory/knowledge/`

**If not safe:** Write ONLY to `~/.canvas/memory/projects/{name}/`

---

## Dual-Write Mechanics

When writing to both locations:

### User Memory (Always)

Full capture with personal context:

**Location:** `~/.canvas/memory/information/projects/{project-name}/{topic}.md`

**Content:**
- Your relationship to the knowledge
- Personal notes and observations
- Links to related personal knowledge
- Full context including sensitive details

### Project Memory (Public-Safe Only)

Sanitized, factual capture:

**Location:** `./.canvas/memory/knowledge/{topic}.md`

**Content:**
- Pure technical facts
- Architecture decisions and rationale
- Design patterns and conventions
- Tool choices and tradeoffs
- NO personal observations
- NO sensitive information
- NO individual names or constraints

**Format:**
```markdown
---
created: 2026-02-18T23:00:00Z
contributors: [ken]
tags: [architecture, design, performance]
keywords: [hot-cold-tiers, domain-scoped-search, grep, performance]
relates-to: [other-project-docs]
---

# Title: Factual, Helpful Knowledge

## What We Learned
Clear, factual description of the knowledge.

## Why It Matters
How this knowledge helps the project.

## Technical Details
Implementation specifics, tradeoffs, rationale.
```

---

## Examples

### Example 1: Architecture Decision (Dual-Write)

**User memory:** `~/.canvas/memory/information/projects/memory-system/architecture.md`
```markdown
# Hot/Cold Architecture Decision

Ken designed this after observing grep slowness on large files.
Preference for grep over embeddings drove the tiered approach.

**See also:** personal/preferences/tooling-choices.md
```

**Project memory:** `./.canvas/memory/knowledge/architecture.md`
```markdown
# Hot/Cold Tiered Architecture

## What We Learned
Memory system uses three-tier architecture:
- Hot memory: 200-500 word summaries (grep-able)
- Cold storage: Unlimited details (referenced from hot)
- Project memory: Public-safe shareable knowledge

## Why It Matters
Keeps grep searches fast (<50ms) while supporting unlimited detail storage.

## Technical Details
Domain-scoped search on hot items returns 3-5 results instead of 50+.
Cold storage referenced from hot items only loaded on-demand.
```

### Example 2: Personal Preference (User-Only)

**User memory ONLY:** `~/.canvas/memory/information/personal/preferences/presentation-style.md`
```markdown
# Presentation Style Preference

Ken prefers inductive writing: conclusion first, supporting detail follows.
Applies to all writing: documents, code comments, commit messages.

Confidence: 0.95 (stated multiple times)
```

**Project memory:** NOTHING. This is personal.

### Example 3: Technical Pattern (Dual-Write)

**User memory:** `~/.canvas/memory/information/projects/memory-system/keyword-strategy.md`
```markdown
# Keyword Strategy

Decided to use keyword arrays after Ken's experience with embedding costs.
Natural language variations simulate semantic search without vector DB.

**See also:** professional/architecture/search-patterns.md
```

**Project memory:** `./.canvas/memory/knowledge/keyword-strategy.md`
```markdown
# Keywords Simulate Embeddings

## What We Learned
YAML frontmatter with keyword arrays enables grep-based retrieval:
- Include singular AND plural forms
- Synonyms and common phrasings
- Multi-word phrases (quoted)
- Acronyms and product names

## Why It Matters
Eliminates need for vector embeddings while maintaining semantic retrieval quality.
Grep with keyword variations is as effective as embeddings for personal knowledge graphs.

## Technical Details
YAML-aware search tool parses multi-line arrays properly.
Standard grep fails on multi-line YAML - use scripts/canvas-memory-search.py.
```

### Example 4: Sensitive Context (User-Only)

**User memory ONLY:** `~/.canvas/memory/information/projects/memory-system/constraints.md`
```markdown
# Project Constraints

Ken doesn't have access to external vector DB services.
Led to design choice for grep-based search instead.

**See also:** personal/constraints/tool-access.md
```

**Project memory:** NOTHING. Access constraints are personal.

---

## The README Test

Before writing to project memory, ask: **"Could this appear in the project README without causing harm?"**

| Would cause harm | Would NOT cause harm |
|------------------|---------------------|
| Personal observations | Architecture rationale |
| Individual constraints | Technical patterns |
| Access limitations | Design decisions |
| Team dynamics | Tool choices |
| Sensitive metrics | General approach |
| Internal debates | Public-safe tradeoffs |

**If in doubt, user-only.** You can always add to project memory later. You can't un-share.

---

## Gate Function

```
BEFORE dual-writing to project memory:
  1. CHECK: Passed the README test?
  2. CHECK: Contains NO personal observations?
  3. CHECK: Contains NO individual names or constraints?
  4. CHECK: Contains NO sensitive information?
  5. CHECK: Factual and helpful to project collaborators?
  If ANY check fails: Write to user memory ONLY.
```

---

## Three-Failure Escalation

If the Gate Function fails 3 times in the same session (e.g., repeatedly writing personal observations to project memory, or including sensitive information in shareable files), STOP immediately.

1. State what you attempted and what failed each time
2. Ask the user for explicit guidance
3. Do not resume until the user provides direction

Leaking personal information to project memory is a critical failure. Escalate immediately.

---

## Red Flags

If you catch yourself thinking:
- "This is probably safe to share"
- "Just this one personal detail won't hurt"
- "The team needs to know about individual constraints"
- "I'll sanitize it later"

**All of these mean: STOP. Write to user memory ONLY.**

---

## Common Rationalizations

| Excuse | Reality |
|--------|---------|
| "This is probably safe" | If you're not certain, it's not safe. User-only. |
| "Just this one personal detail" | Personal details NEVER go to project memory. Zero tolerance. |
| "Team needs context about individual constraints" | Team needs technical context. Individual constraints are private. |
| "I'll sanitize sensitive parts" | Write factual version to project, full version to user. |
| "This is borderline" | Borderline = user-only. Err on side of privacy. |

---

## Anti-Patterns

| Don't | Do |
|-------|-----|
| Write personal preferences to project memory | User memory only for personal content |
| Include "I prefer", "I don't have access" in project docs | Factual technical content only in project memory |
| Share individual constraints with project | Technical constraints (not individual) in project memory |
| Write "Ken prefers X" in shareable docs | Write "System uses X because [technical reason]" |
| Dual-write by default | Default to user-only. Dual-write requires passing README test. |

---

## Success Metrics

**You're doing this well when:**
- ✅ Project memory contains ONLY technical, factual knowledge
- ✅ No personal observations leak to project memory
- ✅ Collaborators find project memory helpful
- ✅ User memory contains full context
- ✅ Clear separation between personal and shareable

**You need to improve when:**
- ❌ Personal details in project memory
- ❌ Individual names or constraints in shareable docs
- ❌ Sensitive information accessible to collaborators
- ❌ Project memory contains "I prefer" or similar
- ❌ User asks "is this in the shareable memory?"
