# Inline Capture Protocol

> **Use when:** EVERY user message. This is the mandatory RETRIEVE → RESPOND → CAPTURE loop.

```
NO USER MESSAGE PROCESSED WITHOUT THIS LOOP
```

**Violating the letter of this protocol is violating the spirit of this protocol.**

---

## The Mandatory Loop

Every user message follows this three-phase pattern:

```
RETRIEVE → RESPOND → CAPTURE
(search)   (apply)    (write)
SILENT     visible    SILENT
```

**The user sees only your RESPOND phase.** RETRIEVE and CAPTURE happen silently.

---

## Phase 1: RETRIEVE (Before Responding)

Before crafting your response, load relevant knowledge from memory.

| Step | Action |
|------|--------|
| 1 | **Infer domain**: What area is this conversation in? (projects/, professional/, personal/) |
| 2 | **Extract keywords**: What precise terms matter? Include variations. |
| 3 | **Search domain-scoped**: `python scripts/canvas-memory-search.py --keyword "term" --domain "domain/"` |
| 4 | **Load 2-3 most relevant files**: Read the hot memory items |
| 5 | **Apply in response**: Use this knowledge to inform your response |

### Domain Inference Signals

| Domain Pattern | Signals |
|----------------|---------|
| `projects/{name}/` | Conversation about specific project, mentions project name |
| `professional/{area}/` | Work domain knowledge (healthcare, architecture, security) |
| `personal/preferences/` | How user likes to work, communication style |
| `personal/constraints/` | User limitations, access constraints |

**Critical:** If you're unsure which domain, search multiple domains. Better to load 5 items from 2 domains than miss relevant knowledge.

### Keyword Extraction

Include natural variations:
- Singular AND plural
- Synonyms and common phrasings
- Technical terms and their acronyms
- Product names

Example: "presentations" → search for: `presentation,presentations,slides,deck,executive`

---

## Phase 2: RESPOND (Visible to User)

Apply the knowledge you retrieved. The user sees this phase.

**Don't announce what you searched or found.** Just use the knowledge naturally in your response.

❌ "I searched memory and found your preference for bottom-line-first presentations..."
✅ [Structures response with conclusion first, naturally applying the preference]

---

## Phase 3: CAPTURE (After Responding)

After your response, silently evaluate: **Did I learn something NEW?**

### Quick Judgment: What Counts as NEW?

| Count as NEW | Examples |
|--------------|----------|
| ✅ User preference stated | "I prefer X format" |
| ✅ Constraint revealed | "I don't have access to Y" |
| ✅ Decision made | "Let's use approach Z" |
| ✅ Pattern observed (2nd+ occurrence) | User corrects same thing again |
| ✅ Project context provided | "This project needs HIPAA compliance" |
| ❌ Already in memory | You retrieved this knowledge in Phase 1 |
| ❌ Temporary conversation state | "Let's discuss X next" |
| ❌ Acknowledgments | "Thanks, that's helpful" |

### Capture Decision Tree

```
Did I learn something NEW?
  ├─ NO → Done. No capture needed.
  └─ YES → Continue to routing
      │
      ├─ Size check: Is content >500 words?
      │   ├─ YES → Cold storage + hot reference
      │   └─ NO → Hot memory
      │
      ├─ Dual-write check: User-only or user+project?
      │   ├─ Personal observation → User memory ONLY
      │   └─ Project-helpful + public-safe → BOTH
      │
      └─ Domain routing: Where does this belong?
          ├─ projects/{name}/ - Project-specific knowledge
          ├─ professional/{area}/ - Domain expertise
          └─ personal/{area}/ - How user works
```

### Hot vs Cold Storage

| If... | Then... |
|-------|---------|
| Content ≤500 words (summary, core fact) | Hot memory: `information/{domain}/{topic}.md` |
| Content >500 words (full discussion, transcript) | Cold storage: `archive/{domain}/{date}-{topic}.md` + hot reference |

**Hot → Cold reference pattern:**
```markdown
# Hot item (information/projects/memory-system/architecture.md)
Memory system uses hot/cold tiers to keep search fast.

**See also:** archive/2026-02-18-architecture-discussion.md
```

### File Format

**Hot memory item:**
```markdown
---
id: info-{date}-{sequence}
created: 2026-02-18T23:00:00Z
modified: 2026-02-18T23:00:00Z
project: {project-name}
tags: [tag1, tag2, tag3]
keywords: [term1, term2, "multi word phrase", acronym]
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

**Size limit:** 200-500 words total.
```

**Cold storage item:**
```markdown
---
id: archive-{date}-{sequence}
created: 2026-02-18T23:00:00Z
referenced-by: [info-001]
tags: [tag1, tag2, tag3]
keywords: [same as hot reference]
visibility: private
---

# Title: Detailed Context

[Full content - no size limit]
```

### After Capture: Cross-Reference Cascade

**REQUIRED:** After capturing, check for secondary effects. See `_protocols/cross-reference-cascade.md`.

---

## Silent Operation

The user should NEVER see:
- "Searching memory..."
- "Capturing this to memory..."
- "I found X in your preferences..."
- "Added to knowledge graph..."

The loop is infrastructure. The user sees only your informed response.

---

## Gate Function

```
BEFORE responding to user message:
  1. CHECK: Inferred domain?
  2. CHECK: Extracted keywords?
  3. CHECK: Searched memory in domain?
  4. CHECK: Loaded relevant files?
  If ANY check fails: STOP. Complete RETRIEVE phase.

AFTER responding to user message:
  1. CHECK: Evaluated for new knowledge?
  2. CHECK: If new knowledge found, captured to appropriate location?
  3. CHECK: Cross-reference cascade completed?
  If ANY check fails: STOP. Complete CAPTURE phase.
```

---

## Three-Failure Escalation

If the Gate Function fails 3 times in the same session (e.g., repeatedly forgetting to search before responding, or learning new preferences without capturing them), STOP immediately.

1. State what you attempted and what failed each time
2. Ask the user for explicit guidance
3. Do not resume until the user provides direction

The RETRIEVE → RESPOND → CAPTURE loop is the foundation of the system. Repeated failures indicate the core mechanism is broken. Escalate rather than continue degrading service.

---

## Red Flags

If you catch yourself thinking:
- "I'll respond first, then search memory later"
- "This isn't important enough to capture"
- "The user will repeat this if it matters"
- "I don't need to search, I remember from earlier in the conversation"
- "I'll batch captures at the end of the session"

**All of these mean: STOP. Re-read the Mandatory Loop section above.**

---

## Common Rationalizations

| Excuse | Reality |
|--------|---------|
| "I remember this from earlier in conversation" | Conversation memory is ephemeral. Only persisted memory survives sessions. |
| "This isn't important enough to capture" | If the user stated it, it's important. Capture everything new. |
| "I'll capture it later" | Later means never. Capture immediately. |
| "I don't need to search, this is a simple question" | Simple questions often have nuanced preferences in memory. Always search. |
| "Searching will slow me down" | Domain-scoped search is <50ms. Forgetting preferences wastes minutes. |
| "The user will correct me if I get it wrong" | The user shouldn't have to repeat themselves. That's why we have memory. |

---

## Anti-Patterns

| Don't | Do |
|-------|-----|
| Skip RETRIEVE phase for "simple" questions | Search memory for EVERY message |
| Announce captures to user | Silent operation - just do it |
| Wait to batch captures | Capture immediately after responding |
| Store only "important" learnings | Capture all new knowledge |
| Search entire memory without domain inference | Infer domain first, search scoped |
| Use conversation context as memory source | Only persisted files survive sessions |

---

## Success Metrics

**You're doing this well when:**
- ✅ User never repeats preferences you already know
- ✅ Responses consistently reflect prior learnings
- ✅ Memory captures happen in real-time
- ✅ Search is fast (domain-scoped)
- ✅ User isn't aware of the capture process

**You need to improve when:**
- ❌ User says "I told you this before"
- ❌ You can't find knowledge you should have captured
- ❌ Batching captures at session end
- ❌ Searching all domains instead of inferring first
- ❌ User asks "did you save that?"
