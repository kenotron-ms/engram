# Agent Instructions: Canvas Memory System

> **Version:** 3.0 | **Updated:** 2026-02-18 | **Primary Reader:** AI agents

This file tells any AI agent how to operate with this personal knowledge graph memory system.

**Violating the letter of the rules is violating the spirit of the rules.**

---

# How This System Works

This is a **bootstrap + on-demand protocol** architecture. This file (AGENTS.md) is the bootstrap -- it loads every session and teaches you how to route content and find detailed protocols.

**Detailed processing protocols live in `_protocols/`.** Each protocol file is self-contained with steps, completion checklists, and quality safeguards. You load them on demand when you need them.

```
EVERY user message follows this mandatory pattern:
RETRIEVE → RESPOND → CAPTURE
(search)   (apply)    (write)
SILENT     visible    SILENT
```

**Before responding to ANY user message:**
1. Infer the domain (what area is this conversation in?)
2. Extract keywords (what precise terms matter?)
3. Search memory in that domain using those keywords
4. Load 2-3 most relevant files
5. Apply that knowledge in your response

**After responding:**
1. Did I learn something NEW? (preference, constraint, decision, pattern, context)
2. If YES: Capture immediately (hot or cold storage, appropriate domain)
3. Silent operation - don't announce captures

See `_protocols/inline-capture.md` for complete RETRIEVE → RESPOND → CAPTURE details.

---

# AI Quick Dispatch

**Use this table first. Match what you learned -> read the protocol file.**

| You learned... | Protocol | Hot Storage | Cold Storage |
|----------------|----------|-------------|--------------|
| User preference or constraint | `_protocols/inline-capture.md` | `~/.canvas/memory/information/{domain}/` | `~/.canvas/memory/archive/{domain}/` |
| New domain knowledge | `_protocols/knowledge-extraction.md` | `~/.canvas/memory/information/{domain}/` | `~/.canvas/memory/archive/{domain}/` |
| Decision or pattern (2nd+ occurrence) | `_protocols/inline-capture.md` | `~/.canvas/memory/information/{domain}/` | N/A |
| Important context about project | `_protocols/dual-write-decision.md` | Both user + project if shareable | N/A |
| Cross-reference opportunity | `_protocols/cross-reference-cascade.md` | Update existing items | N/A |

**Storage routing:**

| Size | Destination | Example |
|------|-------------|---------|
| ≤500 words (summary, core fact) | Hot memory: `information/{domain}/{topic}.md` | "Ken prefers bottom-line-first presentations" |
| >500 words (full details) | Cold storage: `archive/{domain}/{date}-{topic}.md` + hot reference | "Complete 45-min architecture discussion transcript" |

**Domain routing:**

First: **What domain is this conversation in?**

| Domain | Signals |
|--------|---------|
| `projects/memory-system/` | Memory system discussion |
| `professional/healthcare/` | HIPAA, compliance, medical |
| `professional/architecture/` | System design, patterns |
| `personal/preferences/` | How user likes to work |

See `_protocols/scope-routing.md` for domain inference strategy.

**Dual-write decision:**

| Question | If YES |
|----------|--------|
| Is this personal observation/preference? | Write ONLY to `~/.canvas/memory/` |
| Is this helpful to project AND safe to share publicly? | Write to BOTH user + project |

See `_protocols/dual-write-decision.md` for complete decision tree.

---

# Memory Protocols

Each protocol is a self-contained file in `_protocols/`. **Read the protocol file before processing any content.**

| Protocol | File | Use when |
|----------|------|----------|
| Inline Capture | `_protocols/inline-capture.md` | The RETRIEVE → RESPOND → CAPTURE loop (always) |
| Knowledge Extraction | `_protocols/knowledge-extraction.md` | Capturing new domain knowledge |
| Dual-Write Decision | `_protocols/dual-write-decision.md` | Deciding user-only vs user+project |
| Scope Routing | `_protocols/scope-routing.md` | Domain inference and search strategy |
| Cross-Reference Cascade | `_protocols/cross-reference-cascade.md` | Finding secondary effects (always after capture) |

---

# Core Principles

## 1. RETRIEVE → RESPOND → CAPTURE (Mandatory)

**Every user message follows this pattern. No exceptions.**

```
BEFORE responding:
  1. Infer domain from conversation context
  2. Extract precise keywords
  3. Search memory: python scripts/canvas-memory-search.py --keyword "term" --domain "domain/"
  4. Load 2-3 most relevant files
  5. Apply knowledge in response

AFTER responding:
  1. Quick judgment: Did I learn something NEW?
  2. If YES: Capture immediately (categorize, tag, keywords, write)
  3. Silent operation: Don't announce, don't ask permission
```

**Failure mode:** Knowledge learned but not captured = lost forever. Next session won't know it.

## 2. Hot/Cold Tiered Architecture

Keep search fast by separating summaries from details.

| Tier | Purpose | Size Limit | Search |
|------|---------|-----------|--------|
| **Hot** | Grep-able summaries | 200-500 words | Searched directly |
| **Cold** | Detailed content | Unlimited | Referenced from hot |

**Decision:** Default to hot memory. Only use cold when content exceeds 500 words.

**Hot → Cold reference pattern:**
```markdown
# Hot item (information/projects/memory-system/architecture.md)
Memory system uses hot/cold tiers to keep search fast.
**See also:** archive/2026-02-18-detailed-design.md
```

## 3. Domain-First Search (Critical Optimization)

**Before any search, determine relevant domain(s).**

| Without domain scoping | With domain scoping |
|------------------------|---------------------|
| Search all 1000 items | Search 20-50 items in domain |
| Get 50+ matches | Get 3-5 matches |
| Need ranking | Directly relevant |

**The folder structure IS the optimization.**

See `_protocols/scope-routing.md` for inference strategy.

## 4. Keywords Simulate Embeddings

**Keywords are MANDATORY** in every memory file. They enable grep-based retrieval.

**Include natural variations:**
- Singular AND plural: presentation, presentations
- Synonyms: concise, brief, terse
- Common phrasings: "bottom line", conclusion, summary
- Acronyms: HIPAA, PHI, "protected health information"
- Product names: Claude, "Claude Sonnet", claude-sonnet-4

**Use quotes for multi-word phrases:**
- "bottom line" (not bottom-line)
- "TLS 1.2"
- "protected health information"

## 5. Inductive Writing (Conclusion First)

Write **inductively**: state the crux up front, then build supporting detail below it.

| Deductive (don't) | Inductive (do) |
|-------------------|----------------|
| Reader must read everything to find the point | Reader sees the point immediately |
| Optimized for writing | Optimized for retrieval |

**Apply at every level:**
- Document: Opening section states what matters
- Section: Opens with takeaway, then supporting evidence
- Paragraph: Lead sentence is conclusion

## 6. Structure for Retrieval, Not Entry

When creating ANY structured content, optimize for how you'll RETRIEVE data, not how you'll ENTER it.

**Ask:** "What questions will I ask when looking at this?"

## 7. Self-Verify Before Completing

Before marking any processing complete:
1. Re-scan source for missed information
2. Verify files in correct locations
3. Check dimensions (confidence, keywords, tags)
4. Verify cross-references identified
5. **Do NOT ask user to verify** - that's your job

## 8. Always Update System

When you make a mistake or learn something new:
1. Fix the immediate issue
2. Update AGENTS.md or relevant `_protocols/` file so it doesn't happen again
3. Learnings that stay in conversation only = lost learnings

## 9. Read Before Asking

Before asking the user:
1. Check if the file exists
2. Read it
3. Only ask if genuinely unclear after reading

## 10. Cross-Reference Cascade

**Rule: Every piece of knowledge touches more than one file. Find ALL of them.**

After capturing any knowledge, scan for secondary effects:

| Check | What to look for |
|-------|------------------|
| **Related items** | Does this connect to existing knowledge? |
| **Projects** | Does this mention progress on active work? |
| **Patterns** | Is this the 2nd+ occurrence of something? |
| **Temporal** | Does this make older knowledge stale? |

See `_protocols/cross-reference-cascade.md` for complete cascade process.

## 11. Temporal Awareness

Some knowledge has shelf life:

```yaml
dimensions:
  expires: 2025-Q1  # Pricing data
  expires: null     # Writing style (timeless)
```

**When applying old knowledge:** If expired, verify before using.

## 12. Dual-Write Thoughtfully

Project memory (`./.canvas/memory/`) is **safe to share publicly**. Test: "Could this appear in project README without causing harm?"

- Personal observations → user memory ONLY
- Project-helpful + public-safe → BOTH

See `_protocols/dual-write-decision.md` for decision tree.

---

# Memory Architecture

## Two Memory Spaces

**User Private Memory:** `~/.canvas/memory/`
- Your private knowledge (preferences, patterns, constraints)
- Organized by domain (projects/, professional/, personal/)
- Never shared

**Project Shareable Memory:** `./.canvas/memory/`
- Knowledge specific to THIS project
- Safe to share (treat as public)
- Helps collaborators understand project context

## Three-Tier System

### Tier 1: Hot Memory (Active Working Set)

**Location:** `~/.canvas/memory/information/{domain}/`

**Purpose:** Grep-able summaries that enable fast search
- Size limit: 200-500 words per item
- Content: Core fact + reference to cold storage if needed
- Search speed: <50ms for domain-scoped grep

### Tier 2: Cold Storage (Long-term Archive)

**Location:** `~/.canvas/memory/archive/{domain}/`

**Purpose:** Detailed content loaded on-demand
- Size limit: Unlimited
- Content: Complete context, supporting evidence, full details
- Search: Not searched directly (hot items point here)

### Tier 3: Project Shareable Memory

**Location:** `./.canvas/memory/`

**Purpose:** Project-specific knowledge (safe to share publicly)
- Same hot/cold split: `knowledge/` (hot) and `archive/` (cold)
- Can be committed to project git repo
- **Critical Rule:** ONLY information safe to share publicly

---

# File Formats

## Hot Memory Format (Information)

```markdown
---
id: info-{date}-{sequence}
created: 2026-02-18T23:00:00Z
modified: 2026-02-18T23:00:00Z
project: {project-name}
tags: [tag1, tag2, tag3]
keywords: [precise-term1, precise-term2, "multi word phrase", acronym]
relates-to: [info-001, info-002]
dimensions:
  confidence: 0.85
  importance: high
  relevance: [domain1, domain2]
  expires: null
visibility: private
---

# Title: Clear Description

## Core Understanding (Thesis)
What's the main insight? (1-2 sentences max)

## Supporting Context (Evidence)
Where did this come from? (2-3 bullet points)

## Connections (Relationships)
How does this relate to other knowledge?
- See also: archive/2026-02-18-detailed-discussion.md

**Size limit:** 200-500 words total.
```

## Cold Storage Format (Archive)

```markdown
---
id: archive-{date}-{sequence}
created: 2026-02-18T23:00:00Z
referenced-by: [info-001, info-002]
tags: [tag1, tag2, tag3]
keywords: [same as hot reference]
visibility: private
---

# Title: Detailed Context

[Full content - no size limit]
```

## Project Memory Format

```markdown
---
created: 2026-02-18T23:00:00Z
contributors: [ken]
tags: [tag1, tag2, tag3]
keywords: [specific-tech-term, acronym, precise-search-term]
relates-to: [other-project-docs]
---

# Title: Factual, Helpful Knowledge

## What We Learned
Clear, factual description. NO personal observations.

## Why It Matters
How this knowledge helps the project.

**Test:** Could this appear in project README without causing harm? If no → don't write it here.
```

---

# Retrieval Strategy

## The YAML-Aware Search Tool

**Primary method** (handles multi-line arrays properly):
```bash
python scripts/canvas-memory-search.py --keyword "assigned" --domain "projects/"
python scripts/canvas-memory-search.py --keyword "tasks,work" --tag "epic"
```

**Fallback method** (basic grep, only works if keywords on single line):
```bash
# Get $HOME first - grep tool doesn't expand ~
bash: echo $HOME
grep pattern="assigned|tasks|work" path="$HOME/.canvas/memory/information/{domain}/"
```

## Domain Inference Examples

```
User message: "This project needs HIPAA compliance"
  ↓
Agent thinks: "What domain is relevant?"
  - HIPAA = healthcare → professional/healthcare/
  - Current project → projects/memory-system/
  ↓
Search ONLY those domains
```

**At project start:**
```bash
# Load project context
read_file ./.canvas/memory/context.md

# Search user's private project knowledge
python scripts/canvas-memory-search.py --keyword "memory-system" --domain "projects/memory-system/"
```

---

# Tools Available

You have everything needed:

- ✅ `write_file` - Create memory files
- ✅ `read_file` - Read memory files
- ✅ `grep` - Search by tag/content (with path expansion workaround)
- ✅ `glob` - Find files by pattern
- ✅ `bash` - Run YAML-aware search: `python scripts/canvas-memory-search.py`

**Path expansion note:** The `grep` tool doesn't expand `~`. When using grep with `~/.canvas/memory/` paths, first get `$HOME` via bash, then use `$HOME/.canvas/memory/` in the path parameter.

---

# Success Criteria

**You're using this system well when:**

- ✅ User doesn't repeat themselves across sessions
- ✅ You find relevant knowledge when context demands it
- ✅ Captures happen in real-time, not batched
- ✅ Items have clear tags, keywords, and relationships
- ✅ Project memory is safe to share publicly
- ✅ Search is fast (domain-scoped)

**You need to improve when:**

- ❌ User repeats preferences you should know
- ❌ You can't find knowledge you captured
- ❌ Items lack keywords or relationships
- ❌ Sensitive info leaks into project memory
- ❌ Over-capturing creates noise
- ❌ Searching all domains instead of inferring first

---

# Meta: Updating This File

## When to Update

| Trigger | Action |
|---------|--------|
| Made a mistake | Add anti-pattern to relevant protocol + fix |
| Learned something new | Capture in relevant section |
| New content type | Add to Quick Dispatch + create protocol |
| User taught you something | Abstract the learning, add to system |

## How to Update

1. Identify the learning (what went wrong? what's new?)
2. Abstract it (don't just fix the specific case)
3. Find right location: principles/routing → AGENTS.md, processing details → `_protocols/`
4. Verify consistency with other sections
5. **Update version number and date**

---

**Memory system specification:** @MEMORY-SYSTEM.md (reference for terminology and deep dives)

**Protocols:** @_protocols/

---

**Version History:**

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-02-17 | Initial version |
| 2.0 | 2026-02-17 | Added CRITICAL Agent Behavior Protocol |
| 3.0 | 2026-02-18 | Complete rewrite: Life OS architecture + our unique features (hot/cold, .canvas/, keywords, YAML-aware search) |
