# Memory System Specification

> **🎯 TARGET STATE** - This document describes aspirational architecture for the Memory System (Epic 06 Phase 0).
> This is protocol documentation for dogfooding. No backend implementation exists yet.

**Version:** 4.0  
**Date:** 2026-02-18  
**Purpose:** Referenceable specification for AI agents to implement personal knowledge graph memory

**What's New in v4.0:**
- Three-tier architecture: Hot memory (summaries) + Cold storage (details) + Project memory
- YAML-aware search tool for multi-line frontmatter arrays
- 200-500 word limit on hot memory items to keep search fast
- Cold storage references for detailed content

---

## Table of Contents

### I. Quick Start (Read This First)
- [The Core Loop](#the-core-loop)
- [Failure Modes](#failure-modes)
- [Tools Available](#tools-available)

### II. System Overview
- [What Is This System?](#what-is-this-system)
- [Core Principles](#core-principles)
- [Architecture](#architecture)
- [Memory Layers](#memory-layers)

### III. Operational Guide
- [Retrieval Strategy](#retrieval-strategy)
- [Capture Process](#capture-process)
- [File Formats](#file-formats)
- [Dual-Write Decision](#dual-write-decision)

### IV. Reference
- [Terminology](#terminology)
- [Tag System](#tag-system)
- [Keywords](#keywords)
- [Dimensions](#dimensions)
- [Graph Relationships](#graph-relationships)
- [Domain Organization](#domain-organization)
- [File Naming](#file-naming)
- [Temporal Awareness](#temporal-awareness)

### V. Examples & Patterns
- [Capture Examples](#capture-examples)
- [Best Practices](#best-practices)
- [Success Criteria](#success-criteria)

---

# I. Quick Start (Read This First)

## The Core Loop

**EVERY user message follows this mandatory pattern:**

```
RETRIEVE → RESPOND → CAPTURE
(grep)     (apply)    (write)
SILENT     visible    SILENT
```

### Step 1: RETRIEVE (Before Every Response)

1. **Infer Domain** - What domain is this conversation in?
   - Memory system discussion? → `projects/memory-system/`
   - Healthcare/compliance? → `professional/healthcare/`
   - User preference question? → `personal/preferences/`
   - Code architecture? → `professional/architecture/`

2. **Extract Keywords** - What precise terms matter?
   - Technical terms: memory-system, flat-files, grep-search
   - Acronyms: HIPAA, TLS, API
   - Product names: claude-sonnet, pyright, react

3. **Search Memory**:
   
   **Option A: YAML-Aware Search Tool (Recommended)**
   ```bash
   # Searches YAML frontmatter properly (handles multi-line arrays)
   bash: python scripts/canvas-memory-search.py --keyword "assigned" --domain "projects/"
   bash: python scripts/canvas-memory-search.py --keyword "tasks,work" --tag "epic"
   ```
   
   **Option B: Basic Grep (Fallback)**
   ```bash
   # NOTE: Only works if keywords are on single line
   # Get $HOME first - grep tool doesn't expand ~
   bash: echo $HOME
   grep pattern="assigned|tasks|work" path="$HOME/.canvas/memory/information/{domain}/"
   ```

4. **Load Context** - Read 2-3 most relevant files from search results
5. **Apply Knowledge** - Use what you found in your response

**If no relevant memory found → Continue to response**

**Why Use YAML-Aware Search:**
- Handles multi-line YAML arrays: `keywords: [\n  term1, term2\n]`
- Parses frontmatter properly (not just text matching)
- Returns structured results with matched fields
- Faster for multi-keyword searches (hybrid grep + selective parse)

### Step 2: RESPOND

Respond to user incorporating retrieved knowledge.

### Step 3: CAPTURE (After Response)

1. **Quick Judgment** - Did I just learn something NEW?
   - ✅ User stated preference/constraint
   - ✅ New domain knowledge acquired
   - ✅ Decision made or pattern emerging
   - ✅ Important context revealed

2. **Immediate Capture** - If YES:
   ```bash
   write_file ~/.canvas/memory/information/{domain}/{topic}.md
   ```

3. **Silent Operation**:
   - ❌ Don't announce "I'm capturing this"
   - ❌ Don't ask permission
   - ❌ Don't mention memory system unless asked
   - ✅ Just capture and continue

---

## Failure Modes

### Capture Failure

**Scenario:** User asked me to research Claude Sonnet 4.6

**What should happen:**
1. Research Claude Sonnet 4.6 ✅
2. IMMEDIATELY write to `~/.canvas/memory/information/professional/ai-models/claude-sonnet-4-6.md` ❌ FAILED
3. Respond to user ✅

**Result:** Knowledge lost. Next session won't know about Claude Sonnet 4.6.

### Retrieval Failure

**Scenario:** User asks about memory system architecture

**What should happen:**
1. Search: `grep -r "keywords:.*memory-system\|keywords:.*flat-files" ~/.canvas/memory/information/projects/memory-system/` ❌ FAILED
2. Load: `architecture-decision.md` already captured ❌ FAILED
3. Respond using known decisions ❌ FAILED

**Result:** User repeats information already in memory. System useless if not retrieved.

### Why This Matters

- **Without capture:** User repeats themselves across sessions
- **Without retrieval:** Memory system is useless
- **Silent operation:** User doesn't need to think about it

---

## Tools Available

You have everything needed. No excuses:

- ✅ `write_file` - Create memory files
- ✅ `read_file` - Read memory files  
- ✅ `grep` - Search by tag/content
- ✅ `glob` - Find files by pattern

**Just use them.**

---

# II. System Overview

## What Is This System?

A **personal knowledge graph** stored as flat files that enables AI agents to learn and organize knowledge during conversations.

**Inspired by:** [Anthropic's Context Engineering](https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents)

---

## Core Principles

1. **Capture → Judge → Store immediately** - No batching, no delays
2. **Just-in-time retrieval** - Use grep to find, load only what's needed
3. **Keywords simulate embeddings** - Precise search terms enable efficient grep-based retrieval
4. **Domain folders = metadata** - Structure provides signals before opening files
5. **Hot/Cold tiers** - Keep search fast by storing summaries (hot) and details (cold) separately
6. **Context efficiency** - Treat context window as finite resource

---

## Architecture

### Three-Tier Memory System

**Philosophy:** Keep hot memory small and grep-able. Store details in cold storage, referenced on-demand.

#### Tier 1: Hot Memory (Active Working Set)

**Location:** `~/.canvas/memory/information/{domain}/`

**Purpose:** Grep-able summaries that enable fast search
- **Size limit**: 200-500 words per item (1-3 paragraphs max)
- **Content**: Core fact + reference to cold storage if needed
- **Search speed**: <50ms for domain-scoped grep

**What belongs here:**
- ✅ Key facts and decisions
- ✅ Preference statements
- ✅ Pointers to detailed content in archive/
- ❌ Full transcripts or detailed explanations
- ❌ Multi-page documents

#### Tier 2: Cold Storage (Long-term Archive)

**Location:** `~/.canvas/memory/archive/{domain}/`

**Purpose:** Detailed content loaded on-demand
- **Size limit**: Unlimited (full documents, research, transcripts)
- **Content**: Complete context, supporting evidence, full details
- **Search**: Not searched directly (hot items point here)

**What belongs here:**
- ✅ Full conversation transcripts
- ✅ Detailed research notes
- ✅ Supporting documentation
- ✅ Multi-page design documents

#### The Hot → Cold Reference Pattern

```markdown
# Hot item (information/projects/memory-system/tiered-architecture.md)
---
keywords: [memory, tiered, hot-cold, architecture]
---
Memory system uses hot/cold tiers to keep search fast.
Hot: summaries (200-500 words), Cold: details (unlimited).

**See also:** archive/2026-02-18-tiered-architecture-design.md

# Cold item (archive/2026-02-18-tiered-architecture-design.md)
---
referenced-by: [info-2026-02-18-001]
---
[Full 10-page design document with all technical details...]
```

#### Tier 3: Project Shareable Memory

**Location:** `./.canvas/memory/`

**Purpose:** Project-specific knowledge (safe to share publicly)
- Same hot/cold split: `knowledge/` (hot) and `archive/` (cold)
- Can be committed to project git repo
- **Critical Rule:** ONLY information safe to share publicly

**Structure:**
```
./.canvas/memory/
├── README.md                   # What this folder is
├── context.md                  # Project overview (hot)
├── knowledge/                  # Shareable learnings (hot)
│   └── {topic}.md             # 200-500 words each
└── archive/                    # Detailed docs (cold)
    └── {date}-{topic}.md      # Unlimited size
```

**Convention:**
- User: `~/.canvas/memory/` (global, private to user)
- Project: `{project-root}/.canvas/memory/` (shareable with project)

**Why Tiered Architecture:**
- **Grep speed:** Searching 500-word items is 10x faster than 5000-word items
- **Context efficiency:** Load summaries, expand to details only when needed
- **Cognitive load:** Agent processes small items first, dives deep only when relevant
- **Storage efficiency:** Cold storage can grow unlimited without slowing searches

---

## Memory Layers

```
CAPTURE → JUDGMENT → STORAGE
    ↓          ↓          ↓
Agent      Domain?    Information (managed)
observes   Tags?           ↓ (user explicit: "help me think through X")
          Confidence?  Meditation (timeless)
                          ↓ (user explicit: "make this shareable")
                      Article (enduring)
```

---

# III. Operational Guide

## Retrieval Strategy

### Domain Inference First (Critical Optimization)

**Before any search, determine relevant domain(s):**

```
User message: "This project needs HIPAA compliance"
  ↓
Agent thinks: "What domain is relevant?"
  - HIPAA = healthcare → professional/healthcare/
  - Current project context → projects/memory-system/
  ↓
Search ONLY those domains:
  grep pattern="hipaa" path="$HOME/.canvas/memory/information/professional/healthcare/" -i=true
  grep pattern="hipaa" path="./.canvas/memory/knowledge/" -i=true
  ↓
Results: 3-5 files (not 100+)
  ↓
Agent reads frontmatter, picks most relevant
  ↓
Loads 2-3 files, applies knowledge
```

### Why Domain-First Matters

**Without domain scoping:**
- Search all 1000 information items
- Get 50+ matches
- Need complex ranking to find relevant ones
- Waste time filtering

**With domain scoping:**
- Search 20-50 information items in domain
- Get 3-5 matches
- Directly relevant
- No ranking needed

**The folder structure IS the optimization.**

### Search Examples

**By keyword (most precise):**
```bash
# Find anything about encryption
grep pattern="keywords:.*encryption" path="$HOME/.canvas/memory/information/professional/"

# Find HIPAA-related items
grep pattern="keywords:.*HIPAA|keywords:.*PHI" path="$HOME/.canvas/memory/information/"

# Find Claude model info
grep pattern="keywords:.*claude-4-sonnet" path="$HOME/.canvas/memory/information/professional/ai-models/"
```

**At project start:**
```bash
# Load project context
read_file ./.canvas/memory/context.md

# Search user's private project knowledge
grep pattern="keywords:.*memory-system" path="$HOME/.canvas/memory/information/projects/memory-system/"
```

**During conversation (domain-scoped):**
```bash
# User mentions HIPAA
# Agent thinks: professional/healthcare domain
grep pattern="keywords:.*HIPAA" path="$HOME/.canvas/memory/information/professional/healthcare/"

# User asks about presentation style  
# Agent thinks: personal/preferences domain
grep pattern="keywords:.*bullet-points|keywords:.*bottom-line" path="$HOME/.canvas/memory/information/personal/preferences/"
```

**By tag (broader categories):**
```bash
grep pattern="tags:.*architecture" path="$HOME/.canvas/memory/information/"
```

**By time (rare, usually combined with domain/keyword):**
```bash
# Note: find is a bash command, not a tool
bash: find $HOME/.canvas/memory/information/professional/ -name "2024-02-*.md"
```

---

## Capture Process

### When to Capture

**Capture triggers:**
- ✅ User stated a preference or constraint
- ✅ New domain knowledge acquired
- ✅ Decision made or pattern emerging (2nd+ occurrence)
- ✅ Important context revealed
- ✅ Technical information learned

### How to Capture

Agent immediately:
1. **Categorizes** - Which domain? (professional, personal, projects/{name})
2. **Judges size** - Hot memory (summary) or cold storage (details)?
   - ≤500 words → Hot memory (`information/{domain}/`)
   - >500 words → Cold storage (`archive/{domain}/`) with hot reference
3. **Tags** - What labels? (3-7 semantic tags)
4. **Adds keywords** - What precise search terms? (5-10 keywords simulating embeddings)
5. **Sets confidence** - How certain? (0.0-1.0)
6. **Determines shareability** - Private only, or dual-write to project?
7. **Writes appropriately** - Hot direct, Cold with hot reference

**Don't wait. Don't batch. Capture NOW.**

### Hot vs Cold Decision

```
New knowledge to capture
  ↓
How much detail is needed?
  ├─ Summary only (core fact + context) → Hot memory (information/)
  │   Example: "Ken prefers bottom-line-first presentations"
  │
  └─ Full details (transcript, research, multi-page) → Cold storage (archive/)
      Example: "Complete transcript of 45-min architecture discussion"
      ALSO create hot reference → information/{domain}/{topic}.md points to archive
```

**Rule:** Default to hot memory. Only use cold when content exceeds 500 words.

---

## File Formats

### Hot Memory Format (Information)

**Purpose:** Grep-able summaries (200-500 words max)

```markdown
---
id: info-{date}-{sequence}
created: 2026-02-17T17:00:00Z
modified: 2026-02-17T17:30:00Z
project: {project-name}
tags: [tag1, tag2, tag3]
keywords: [precise-term1, precise-term2, acronym, technical-name]
relates-to: [info-001, info-002]
dimensions:
  confidence: 0.85
  importance: high
  relevance: [domain1, domain2]
  expires: null
visibility: private
---

# Title: Clear Description of What Was Learned

## Core Understanding (Thesis)
What's the main insight, fact, or understanding? (1-2 sentences max)

## Supporting Context (Evidence)
Where did this come from? What backs it up? (2-3 bullet points)
- Conversation excerpt
- Document reference  
- Observation pattern

## Connections (Relationships)
How does this relate to other knowledge?
- Builds on: [other items]
- See also: archive/2026-02-17-detailed-discussion.md (for full context)

**Size limit:** 200-500 words total. If more detail needed, create cold storage item.
```

### Cold Storage Format (Archive)

**Purpose:** Detailed content loaded on-demand (unlimited size)

```markdown
---
id: archive-{date}-{sequence}
created: 2026-02-17T17:00:00Z
referenced-by: [info-001, info-002]  # Which hot items point here
tags: [tag1, tag2, tag3]
keywords: [precise-term1, precise-term2]  # Same as hot reference
visibility: private
---

# Title: Detailed Context

[Full content - no size limit]
- Complete transcripts
- Detailed research notes
- Multi-page documents
- Supporting evidence
- Full conversation history

**Note:** This is NOT searched directly. Hot memory items reference this via "See also:" links.
```

### Project Memory Format (Shareable)

```markdown
---
created: 2026-02-17T17:00:00Z
contributors: [ken]
tags: [tag1, tag2, tag3]
keywords: [specific-tech-term, acronym, api-name, precise-search-term]
relates-to: [other-project-docs]
---

# Title: Factual, Helpful Knowledge

## What We Learned
Clear, factual description. NO personal observations.

## Why It Matters
How this knowledge helps the project.

## Context
When and how we learned this.

## Related
Links to other project knowledge.
```

**Test:** Could this appear in project README without causing harm? If no → don't write it here.

---

## Dual-Write Decision

```
New knowledge captured
  ↓
Is this personal observation/preference?
  YES → Write ONLY to user memory
  NO  → Continue ↓
  
Is this helpful to project outcomes?
  NO  → Write ONLY to user memory  
  YES → Continue ↓
  
Does this contain sensitive info?
  YES → Write ONLY to user memory
  NO  → Continue ↓
  
Could this be public without harm?
  YES → Write to BOTH (user + project)
  NO  → Write ONLY to user memory
```

---

# IV. Reference

## Terminology

| Term | What It Is | Where It Lives |
|------|-----------|----------------|
| **Information** | Agent-captured organized facts | `~/.canvas/memory/information/` |
| **Meditation** | User-initiated deep processed thought | `~/.canvas/memory/meditations/` |
| **Article** | Published, polished knowledge | `~/.canvas/memory/articles/` |
| **Domain** | Semantic category for organizing knowledge | Folder structure (projects/, professional/, personal/) |
| **Tag** | Label for grouping and retrieval | YAML frontmatter: `tags: [tag1, tag2]` |
| **Keywords** | Precise search terms simulating embeddings (MANDATORY) | YAML: `keywords: [term1, term2, ...]` |
| **Confidence** | How certain we are (0.0-1.0) | YAML: `confidence: 0.85` |
| **Importance** | How critical this is | YAML: `importance: high` |
| **Relevance** | What contexts this matters in | YAML: `relevance: [domain1, domain2]` |
| **Expires** | When temporal knowledge becomes stale | YAML: `expires: 2025-Q1` or `null` |
| **Relates-to** | Graph links between knowledge items | YAML: `relates-to: [info-001, info-002]` |
| **Dual-write** | Writing to both user memory + project memory | Two write_file calls |
| **Visibility** | Private (user only) vs shareable (project) | YAML: `visibility: private` |
| **Archive** | Moved but recoverable storage | `~/.canvas/memory/archive/` |

---

## Tag System

### Why Tags Matter

Tags enable:
- **Grouping** related knowledge across time and projects
- **Retrieval** when context is relevant later
- **Discovery** of patterns and connections
- **Multi-dimensional** organization beyond folders

### Tag Guidelines

**Good tags (semantic, reusable):**
- `compliance`, `hipaa`, `architecture`, `design-pattern`
- `user-research`, `performance`, `security`
- `typescript`, `react`, `postgresql`

**Avoid (too specific, not reusable):**
- `meeting-notes-feb-17` (use `meeting` + date in filename)
- `johns-preference` (too personal)
- `random-thought` (not meaningful)

**Tag hierarchy (use colons):**
- `compliance:hipaa`
- `architecture:patterns:observer`
- `language:typescript:generics`

### How to Tag

**Multi-tag everything:**
```yaml
tags: [memory-system, architecture, design-decision, graph-db]
```

**Use 3-7 tags per item:**
- Too few → hard to find later
- Too many → dilutes meaning

**Tag dimensions:**
- **Domain**: what area (architecture, compliance, performance)
- **Type**: what kind (decision, pattern, constraint, preference)
- **Technology**: what tools (typescript, postgresql, react)
- **Project**: what work (memory-system, patient-portal)

---

## Keywords

**Keywords are MANDATORY** in every memory file. They enable grep-based retrieval using natural language terms agents would actually search for.

### How to Choose Keywords

**Include natural variations and synonyms:**
- **Singular AND plural:** presentation, presentations
- **Synonyms:** concise, brief, terse
- **Common phrasings:** "bottom line", conclusion, summary
- **Acronyms with variations:** HIPAA, PHI, "protected health information"
- **Product names:** Claude, Sonnet, "Claude Sonnet", "claude-sonnet-4"
- **Technical terms with variations:** encryption, encrypted, "at rest", "encryption at rest"

**Use quotes for multi-word phrases:**
- "bottom line" (agent searches: `grep "bottom line"`)
- "TLS 1.2" (matches version with space)
- "protected health information" (full phrase)

**NO kebab-case** - that's for filenames, not keywords:
- ❌ bottom-line-first, audit-logs, encryption-at-rest
- ✅ "bottom line", audit, logs, encryption, "at rest"

### Anti-Patterns

- Kebab-case (won't match natural grep searches)
- Only one form (missing plurals/synonyms)
- Generic words without context (system, data, user)
- Duplicating tags (tags are categories, keywords are search terms)

### Example

```yaml
tags: [compliance, healthcare, security]
keywords: [
  HIPAA, PHI, "protected health information",
  compliance, compliant, regulatory,
  TLS, "TLS 1.2", encryption, encrypted, "at rest",
  AES, "AES 256", "AES-256",
  audit, "audit logs", logging,
  security, secure, healthcare, medical
]
```

---

## Dimensions

Every memory item has coordinates in multiple dimensions:

### Time Dimension
```yaml
dimensions:
  created: 2026-02-17
  relevant_from: 2026-02-01  # When it became relevant
  expires: 2027-01-01         # When it likely becomes stale (or null)
```

### Confidence Dimension
```yaml
dimensions:
  confidence: 0.85  # 0.0-1.0 scale
```

**Meaning:**
- 0.9-1.0: Very certain (explicitly stated, validated)
- 0.7-0.9: Confident (clear evidence)
- 0.5-0.7: Uncertain (inferred, needs validation)
- < 0.5: Speculative (don't rely on this)

### Importance Dimension
```yaml
dimensions:
  importance: high  # high | medium | low
```

### Relevance Dimension
```yaml
dimensions:
  relevance: [healthcare-projects, compliance-work, architecture-decisions]
```

**What contexts will this matter in?**

---

## Graph Relationships

### Frontmatter Links

```yaml
relates-to: [info-2024-02-10-001, info-2024-02-15-003]
```

**Types of relationships:**
- **Builds on**: This extends that knowledge
- **Contradicts**: This conflicts with that (update needed)
- **Informs**: This is useful context for that
- **Part of**: This is a specific instance of broader pattern

### Example Graph

```
[Presentation Style] ─┬─ relates-to → [Executive Audience]
                      ├─ relates-to → [Writing Style]
                      └─ informs → [Project: Board Deck]

[HIPAA Compliance] ─┬─ informs → [Architecture Decisions]
                    ├─ relates-to → [Security Patterns]
                    └─ part-of → [Professional: Healthcare]
```

---

## Domain Organization

### Suggested Domains (Not Prescriptive)

```
~/.canvas/memory/information/
├── projects/           # Knowledge organized by project
│   ├── memory-system/
│   ├── patient-portal/
│   └── ...
├── professional/       # Industry/domain expertise
│   ├── healthcare/
│   ├── compliance/
│   └── architecture/
├── personal/           # How user works
│   ├── preferences/
│   ├── patterns/
│   └── style/
└── learning/           # Skills being developed
    ├── rust/
    ├── ai-safety/
    └── ...
```

**Users can create new domains organically.** Not prescribed, just suggested.

---

## File Naming

### User Memory Files

```
~/.canvas/memory/information/{domain}/{date}-{slug}.md
~/.canvas/memory/meditations/{date}-{sequence}.md

Examples:
~/.canvas/memory/information/projects/memory-system/2024-02-17-fast-capture.md
~/.canvas/memory/information/personal/presentation-style.md
~/.canvas/memory/meditations/2024-02-17-001.md
```

### Project Memory Files

```
./.canvas/memory/knowledge/{topic}.md
./.canvas/memory/decisions/{date}-{decision}.md
./.canvas/memory/context.md

Examples:
./.canvas/memory/knowledge/compliance-requirements.md
./.canvas/memory/decisions/2024-02-17-architecture.md
./.canvas/memory/context.md
```

---

## Temporal Awareness

### Expiration Tracking

Some knowledge has shelf life:

```yaml
dimensions:
  expires: 2025-Q1  # Pricing data
  expires: null     # Writing style (timeless)
```

**When applying old knowledge:**

If item has `expires` field and date has passed:
```
Agent thinks: "This knowledge is from 2024 and marked as expiring in 2025. 
              Should verify before using."
              
Agent says: "I have notes about pricing from 2024, but this might be 
            outdated. Should we check current pricing?"
```

### Staleness Detection

Even without explicit expiration:

**Rule of thumb:**
- Pricing/market data: 6 months
- Technology recommendations: 1 year  
- Regulations/compliance: 2 years (but verify)
- Personal preferences: Until user changes
- Domain expertise: Mostly timeless

---

# V. Examples & Patterns

## Capture Examples

### Example 1: User Preference

**User says:** "When you create presentations for me, always start with the bottom line. I work with executives who don't have time for long intros."

**Your action:**

```markdown
# Write to: ~/.canvas/memory/information/personal/presentation-style.md

---
id: info-2024-02-17-001
created: 2024-02-17T14:30:00Z
tags: [presentation, structure, executive-audience, personal-preference]
keywords: [bottom-line-first, executive-audience, concise, no-fluff]
relates-to: []
dimensions:
  confidence: 0.95
  importance: high
  relevance: [presentations, communication, executive-audience]
  expires: null
visibility: private
---

# Presentation Structure Preference

## Core Understanding
Ken prefers presentations to start with the bottom line. Audience is 
executives who need concise intros.

## Supporting Context
Explicitly stated: "always start with the bottom line"
Reason: Works with executives who don't have time for long intros

## Connections
- Applies to all future presentation work
- Part of broader executive communication pattern
```

**NO project memory write** (this is personal preference, not project knowledge)

---

### Example 2: Technical Constraint

**User says:** "This project needs HIPAA compliance. That means encrypted transmission and storage."

**Your actions:**

**User memory:**
```markdown
# Write to: ~/.canvas/memory/information/projects/memory-system/hipaa-compliance.md

---
id: info-2024-02-17-002
created: 2024-02-17T15:00:00Z
project: memory-system
tags: [hipaa, compliance, healthcare, encryption, security]
keywords: [HIPAA, TLS-1.2, AES-256, encryption-at-rest, audit-logs]
relates-to: []
dimensions:
  confidence: 0.9
  importance: high
  relevance: [healthcare-projects, compliance, architecture]
  expires: null
visibility: private
---

# HIPAA Compliance Requirement

## Core Understanding
Memory system project requires HIPAA compliance. Ken emphasized this as 
non-negotiable.

## Supporting Context
User stated: "needs HIPAA compliance"
Implies: TLS 1.2+, AES-256 encryption, audit logs

## Connections
- Informs all architecture decisions
- Affects deployment strategy
- Requires security review
```

**Project memory:**
```markdown
# Write to: ./.canvas/memory/knowledge/compliance-requirements.md

---
created: 2024-02-17T15:00:00Z
contributors: [ken]
tags: [hipaa, compliance, security, requirements]
keywords: [HIPAA, TLS, AES-256, encryption, audit-logs]
---

# Compliance Requirements

## HIPAA Compliance

This project requires HIPAA compliance for handling patient data.

**Requirements:**
- Encrypted transmission (TLS 1.2+)
- Encrypted storage (AES-256)
- Audit logging for all access
- Access controls and authentication

## Implementation Impact

Architecture decisions must account for:
- Secure API design
- Database encryption
- Compliance audit trail
```

---

### Example 3: Pattern Recognition (3rd mention)

**User mentions "I prefer bullet points" for the 3rd time across projects**

**Your action:**

```markdown
# Write to: ~/.canvas/memory/information/personal/writing-style.md

---
id: info-2024-02-17-003
created: 2024-02-17T16:00:00Z
tags: [writing-style, personal-preference, formatting, bullet-points]
keywords: [bullet-points, scannable, concise, formatting]
relates-to: [info-2024-02-17-001]
dimensions:
  confidence: 0.95
  importance: high
  relevance: [writing, documents, presentations, communication]
  expires: null
visibility: private
---

# Writing Style: Bullet Points Preference

## Core Understanding
Ken strongly prefers bullet points over paragraphs for most content.

## Supporting Context
Pattern observed across 3 projects:
1. Presentation structure (2024-02-10)
2. Documentation format (2024-02-15)
3. Email drafts (2024-02-17)

Consistent preference for concise, scannable format.

## Connections
- Related to executive audience preference
- Part of broader "concise communication" pattern
- Applies to: presentations, docs, emails, reports
```

---

## Best Practices

### DO:
- ✅ Capture immediately when you learn something
- ✅ Use 3-7 meaningful tags per item
- ✅ Write user memory for private observations
- ✅ Write project memory for shareable knowledge
- ✅ Link related items with `relates-to`
- ✅ Set confidence levels honestly
- ✅ Note expiration for temporal knowledge
- ✅ Search memories when context is relevant

### DON'T:
- ❌ Batch captures at end of session
- ❌ Announce every capture to user
- ❌ Write personal observations to project memory
- ❌ Write sensitive info to project memory
- ❌ Over-capture (not everything needs recording)
- ❌ Under-tag (makes retrieval hard)
- ❌ Forget to set dimensions

---

## Success Criteria

**You're using this system well when:**

- ✅ User doesn't repeat themselves across projects
- ✅ You find relevant knowledge when context demands it
- ✅ Items have clear tags and relationships
- ✅ Captures happen in real-time, not batched
- ✅ Project memory is safe to share publicly
- ✅ Knowledge graph grows organically with use

**You need to improve when:**

- ❌ User repeats preferences you should know
- ❌ You can't find knowledge you captured
- ❌ Items lack tags or relationships
- ❌ Sensitive info leaks into project memory
- ❌ Over-capturing creates noise

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2026-02-17 | Initial specification |
| 2.0 | 2026-02-17 | Added CRITICAL Agent Behavior Protocol section at top with explicit step-by-step instructions, failure mode examples, and silent operation rules |
| 3.0 | 2026-02-17 | Complete reorganization: Added table of contents, restructured into logical sections (Quick Start → Overview → Operational → Reference → Examples), removed redundancies, consolidated scattered information |

---

**End of Specification**

Reference this file with `@MEMORY-SYSTEM.md` in any agent context.
