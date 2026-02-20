# Canvas Memory: How a Self-Learning Personal Knowledge Graph Creates Procedural Memory

> **This document explains the system architecture.** For operational instructions, see `AGENTS.md` which loads at every session start.

---

## The Problem

AI tools today offer two approaches to personalization:

**Explicit configuration.** Settings, preferences, toggles. Limited to what the developer anticipated. Can't capture nuanced preferences like "use hot memory for summaries, cold storage for detailed transcripts" or "search domain-scoped first, then expand if needed."

**Conversation memory.** Store facts from conversations. Retrieve them later. This captures what was said -- not how you want things done. Knowing that you prefer bottom-line-first presentations is declarative memory. Knowing how to route information through a three-tier architecture with domain-scoped search is procedural memory. They're fundamentally different.

**What's missing:** A system that learns *behavior* from corrections, encodes that behavior into persistent rules, and applies those rules in future sessions without repeating mistakes.

---

## The Architecture

Canvas Memory is a personal knowledge graph built on a simple architectural insight: **instead of storing preferences in a database, teach the system how to modify its own behavioral governance.**

The system uses flat files and grep-based search, organized into a three-tier architecture that keeps retrieval fast while supporting unlimited detail storage.

### The Bootstrap File

One file (`AGENTS.md`) loads at the start of every AI session. It teaches the agent four things:

1. **The mandatory loop.** RETRIEVE → RESPOND → CAPTURE. Every user message follows this pattern: search memory before responding, apply what you find in your response, capture new learnings after responding. Silent operation -- the user doesn't think about it.

2. **How to route by domain.** The system determines where content belongs based on signals in the conversation itself. This is a multi-axis model:
   - **projects/{name}/** -- project-specific knowledge
   - **professional/{area}/** -- domain expertise (healthcare, architecture, etc.)
   - **personal/{area}/** -- how the user works, personal preferences

   Domains are self-organizing. When content arrives that doesn't fit any existing domain, the system creates a new one. The folder structure IS the search optimization -- domain inference before grep keeps searches fast.

3. **Core principles.** Non-negotiable rules that govern all behavior. "Structure for retrieval, not entry." "Inductive writing -- state the conclusion first." "Self-verify before presenting work." These are immutable by the system -- only the user changes them.

4. **How to learn.** A meta-instruction that says: when you make a mistake, update the relevant protocol. When you learn something new, capture it. When you encounter a new pattern, encode it. This single instruction is the engine of self-improvement.

### The Three-Tier Architecture

**Tier 1: Hot Memory (Active Working Set)**

Location: `~/.canvas/memory/information/{domain}/`

Purpose: Grep-able summaries that enable fast search
- Size limit: 200-500 words per item
- Content: Core fact + reference to cold storage if needed
- Search speed: <50ms for domain-scoped grep

What belongs here:
- Key facts and decisions
- Preference statements  
- Pointers to detailed content in archive/

**Tier 2: Cold Storage (Long-term Archive)**

Location: `~/.canvas/memory/archive/{domain}/`

Purpose: Detailed content loaded on-demand
- Size limit: Unlimited (full transcripts, research, detailed discussions)
- Content: Complete context, supporting evidence, full details
- Search: Not searched directly (hot items point here)

What belongs here:
- Full conversation transcripts
- Detailed research notes
- Supporting documentation
- Multi-page analyses

**The Hot → Cold Reference Pattern:**
```markdown
# Hot item (information/projects/memory-system/architecture.md)
Memory system uses hot/cold tiers to keep search fast.
Hot: summaries (200-500 words), Cold: details (unlimited).

**See also:** archive/2026-02-18-architecture-discussion.md
```

**Tier 3: Project Shareable Memory**

Location: `./.canvas/memory/`

Purpose: Project-specific knowledge safe to share publicly
- Same hot/cold split: `knowledge/` (hot) and `archive/` (cold)
- Can be committed to project git repo
- Critical rule: ONLY information safe to share publicly

**Why this architecture:**
- **Grep speed:** Searching 500-word items is 10x faster than 5000-word items
- **Context efficiency:** Load summaries, expand to details only when needed
- **Cognitive load:** Process small items first, dive deep only when relevant
- **Storage efficiency:** Cold storage can grow unlimited without slowing searches

### Protocol Files

Each content type or operational pattern has a dedicated protocol file. These are self-contained decision procedures stored as markdown files in a `_protocols/` directory. Each protocol includes:

- **Steps**: What to do, in order, for this pattern
- **A gate function**: A quality checklist the AI must pass before marking work complete. "Did you infer the domain? Did you extract keywords? Did you check for cross-references?" If any check fails, the work isn't done.
- **Anti-patterns**: Documented ways the system has failed in the past, with corrections. These are guards against regression -- the system has a record of its own failure modes.
- **Rationalization tables**: Common excuses the AI might generate for taking shortcuts, paired with rebuttals. "I can skip domain inference, the search will find it anyway" -> "Domain inference is mandatory. Searching all domains is 10x slower."

The anti-patterns and rationalizations are where months of corrections crystallize. They aren't abstract principles -- they're specific failure modes discovered through real use, encoded as permanent guardrails.

### Keywords Simulate Embeddings

Instead of vector embeddings, the system uses **keyword arrays with natural language variations** in YAML frontmatter:

```yaml
keywords: [
  presentation, presentations,
  concise, brief, terse,
  "bottom line", conclusion, summary,
  executive-audience,
  "Claude Sonnet", claude-sonnet-4
]
```

This enables grep-based retrieval while supporting:
- Singular AND plural forms
- Synonyms and common phrasings
- Multi-word phrases (quoted)
- Acronyms and product names
- Technical terms with variations

The YAML-aware search tool (`scripts/canvas-memory-search.py`) parses multi-line frontmatter properly, making keyword-based retrieval as effective as embeddings for personal knowledge graphs.

### Domain-Scoped Search

The critical optimization: **always infer domain before searching.**

| Without domain scoping | With domain scoping |
|------------------------|---------------------|
| Search all 1000 items | Search 20-50 items in domain |
| Get 50+ matches | Get 3-5 matches |
| Need ranking algorithm | Directly relevant results |

The folder structure IS the optimization. Domain inference first, then grep within that domain.

```
User message: "This project needs HIPAA compliance"
  ↓
Agent thinks: What domain?
  - HIPAA = healthcare → professional/healthcare/
  - Current project → projects/memory-system/
  ↓
Search ONLY those domains
  ↓
3-5 relevant results (not 50+)
```

### The Cross-Reference Cascade

This is the mechanism that makes the system feel seamless in daily use.

**The principle: every piece of knowledge touches more than one file.** When you capture a preference, it might relate to existing knowledge about presentation style, executive communication, and writing patterns. A naive system creates the preference file and stops. Canvas Memory automatically identifies every secondary effect and updates every affected file.

The user never has to say "did you cross-reference this?" If the knowledge relates to something the system already knows, the system finds the relationship and records it. If the knowledge is the 2nd+ occurrence of a pattern, the system recognizes it and marks it as such.

This is implemented through a cascade check table: after capturing any knowledge, the system scans for effects on related items, projects, patterns, and temporal knowledge. Each finding is recorded, and each recording is checked for FURTHER secondary effects. The cascade continues until there's nothing left to update.

**The user provides information. Everything else is automatic.**

---

## The Self-Learning Loop

The key innovation is not any single architectural component -- it's how they work together to create a self-improving system.

```
User provides information
  -> System infers domain and extracts keywords
  -> System searches memory in that domain
  -> System applies found knowledge in response
  -> System captures new learnings (hot or cold)
  -> Cascade identifies and records secondary effects
  -> If the user corrects anything:
     -> System identifies the gap in its protocols
     -> Updates the protocol (or creates a new one)
     -> Adds anti-pattern to prevent regression
     -> Correction persists across all future sessions
```

**A concrete example:**

Early in the system's life, the user mentions they prefer presentations that start with the bottom line. The system captures this in `personal/preferences/presentation-style.md`. Good.

Later, the user provides feedback on a document: "This is too verbose. Lead with the conclusion." The system recognizes this is the 2nd occurrence of the same preference. It updates the presentation-style file, increases confidence to 0.95, and adds `relates-to` links to writing-style preferences. Pattern recognized.

The correction happened twice. The system wrote it into its own rules with cross-references. It never repeated the error. And it added anti-patterns to guard against regression: "I can write deductively since that's how I think" -> "User wants inductive writing. Conclusion first, always."

---

## The Convergence Property

A well-functioning system exhibits convergence: corrections decrease over time as the knowledge graph becomes more complete and accurate.

```
Week 1:  ████████████  (many captures and corrections)
Week 4:  ██████        (fewer corrections)
Week 8:  ██            (rare corrections)
Week 12: █             (occasional edge case)
```

This isn't guaranteed -- if your needs change frequently, you may never fully converge. But for stable workflows, convergence is the expected behavior. The knowledge graph asymptotically approaches a complete model of your preferences and domain knowledge.

Each capture makes the system permanently smarter. Not through accumulated conversation history or embedding lookups, but through organized knowledge that the system reads, follows, and continues to refine.

---

## What "Memory" Really Means Here

Most AI "memory" systems remember facts: your name, your preferences, your past conversations. That's declarative memory -- knowing WHAT.

Canvas Memory creates **procedural memory** -- knowing HOW. The system doesn't just remember that you prefer hot/cold tiered architecture. It has a decision procedure for evaluating any new information: does this exceed 500 words? Route to cold with hot reference. Under 500? Hot memory direct. And it knows this because you taught it through corrections that were encoded into persistent protocols.

The distinction matters because procedural memory produces **behavioral change**. A system with declarative memory might remember "user prefers organized memory." A system with procedural memory evaluates each piece of information individually and makes the right routing decision based on the specific characteristics -- even for information types it has never seen before, because the decision procedure generalizes.

The `AGENTS.md` file is the durable artifact. It accumulates principles, protocols, and the routing intelligence that maps content to handlers. Because it loads at the start of every session, the learning carries forward even though each AI session starts fresh. No conversation history required. No embedding database. No RAG system. Just markdown files that the system reads, follows, and improves.

---

## What Makes This Reproducible

The architecture is simple enough that anyone can build their own version:

1. **Start with principles.** Write down your non-negotiable rules for how you want things done. These don't need to be comprehensive -- start with 3-5 and grow over time.

2. **Add the meta-instruction.** Tell the system: when you make a mistake, update your own rules. When you capture knowledge, check for cross-references. When you see patterns, encode them. This single instruction is the engine that makes everything else possible.

3. **Build protocols through use.** Don't try to anticipate every pattern upfront. Have real conversations. When the system makes a mistake, correct it, and let it create the protocol. The protocols will be better than anything you could design in advance because they emerge from actual failure modes.

4. **Watch it converge.** The first week requires many corrections. By the second month, the system handles most patterns correctly. The protocols encode your preferences, your workflow, your domain knowledge -- and they continue to improve.

The bootstrap file and meta-instruction together are fewer than 200 lines. Everything else emerges from use. The system you end up with is genuinely yours -- shaped by your corrections, encoding your judgment, optimized for your retrieval patterns.

---

## Appendix: The Six Mechanisms

| Mechanism | What it does | Implementation |
|-----------|-------------|----------------|
| **Bootstrap** | Loads routing intelligence every session | `AGENTS.md` loaded at session start |
| **Mandatory Loop** | RETRIEVE → RESPOND → CAPTURE every message | Quick Dispatch table + domain inference |
| **Domain routing** | Routes content to the correct location by context | Domain signals + folder structure |
| **Three-tier architecture** | Keeps search fast while supporting unlimited detail | Hot memory (200-500 words) + Cold storage (unlimited) + Project memory (shareable) |
| **Keywords** | Enable grep-based retrieval without embeddings | Natural language variations in YAML frontmatter |
| **Cascade** | Finds and records all secondary effects automatically | Cross-reference check table + reconciliation |
| **Meta-instruction** | Tells the system how to learn from corrections | Core principle: "when corrected, update the protocol" |

---

## The `.canvas/` Convention

Canvas Memory uses a specific directory convention:

**User Private:** `~/.canvas/memory/`
- Your private knowledge graph
- Organized by domain (projects/, professional/, personal/)
- Never shared

**Project Shareable:** `./.canvas/memory/`
- Project-specific knowledge
- Safe to share publicly (test: "Could this go in README?")
- Helps collaborators understand project context

This two-space architecture enables dual-write: personal observations go to user memory only, while project-helpful and public-safe knowledge goes to both.

---

## Technical Details

### YAML-Aware Search Tool

The system includes a specialized search tool that properly parses multi-line YAML frontmatter:

```bash
python scripts/canvas-memory-search.py --keyword "assigned" --domain "projects/"
python scripts/canvas-memory-search.py --keyword "tasks,work" --tag "epic"
```

This handles the common case where keywords span multiple lines in YAML arrays, which standard grep cannot parse correctly.

### File Format

Every hot memory item follows this format:

```markdown
---
id: info-{date}-{sequence}
created: 2026-02-18T23:00:00Z
modified: 2026-02-18T23:00:00Z
project: {project-name}
tags: [tag1, tag2, tag3]
keywords: [term1, term2, "multi word phrase"]
relates-to: [info-001, info-002]
dimensions:
  confidence: 0.85
  importance: high
  relevance: [domain1, domain2]
  expires: null
visibility: private
---

# Title

## Core Understanding
What's the main insight? (1-2 sentences)

## Supporting Context
Where did this come from? (2-3 bullets)

## Connections
How does this relate to other knowledge?
```

Cold storage items have similar frontmatter but no size limit on content.

### Temporal Awareness

Knowledge can have expiration:

```yaml
dimensions:
  expires: 2025-Q1  # Pricing data
  expires: null     # Writing style (timeless)
```

When applying old knowledge, the system checks expiration and verifies if needed.

---

**For operational instructions, see:** `AGENTS.md` (loads every session)

**For detailed protocols, see:** `_protocols/` (loaded on demand)
