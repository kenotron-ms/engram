# Knowledge Extraction Protocol

> **Use when:** Extracting structured knowledge from conversations, documents, or research for long-term storage.

```
NO KNOWLEDGE CAPTURED WITHOUT DOMAIN INFERENCE AND KEYWORD EXTRACTION
```

**Violating the letter of this protocol is violating the spirit of this protocol.**

---

## What is Knowledge Extraction?

Knowledge extraction transforms raw information (conversations, documents, research) into structured, retrievable knowledge items.

**Input:** Unstructured content (conversation, document, discussion)
**Output:** Structured memory item (hot or cold) with proper metadata

---

## When to Extract Knowledge

| Extract when... | Example |
|-----------------|---------|
| User teaches you domain knowledge | "HIPAA requires encryption at rest and in transit" |
| Technical pattern emerges | "We use X pattern for Y because Z" |
| Decision is made with rationale | "Chose hot/cold tiers for grep performance" |
| Constraint is revealed | "This project can't use external APIs" |
| Best practice is established | "Always infer domain before searching" |

**Don't extract:** Temporary conversation state, acknowledgments, or information already in memory.

---

## Extraction Steps

| Step | Action |
|------|--------|
| 1 | **Read source fully**: Understand complete context before extracting |
| 2 | **Infer domain**: Where does this belong? (projects/, professional/, personal/) |
| 3 | **Identify topic**: What is this about? (One clear focus per item) |
| 4 | **Check for existing item**: Does a file for this topic already exist? If yes, enrich it. If no, create new. |
| 5 | **Extract keywords**: Include natural variations, synonyms, acronyms |
| 6 | **Determine size**: ≤500 words → hot memory, >500 words → cold storage + hot reference |
| 7 | **Write structured item**: Follow hot or cold format |
| 8 | **Cross-reference**: Check for related knowledge (see cross-reference-cascade.md) |

---

## Quality Standards

Every extracted item MUST be:

| Dimension | Standard |
|-----------|----------|
| **Inductive structure** | State the crux first. Open with what matters. Supporting detail follows. |
| **Plain language** | Every technical term explained on first use. Reader NEVER needs to Google a term. |
| **Complete context** | Who, what, when, why provided. Future reader doesn't need to find original source. |
| **Proper keywords** | Natural variations included (singular/plural, synonyms, acronyms, multi-word phrases) |
| **Size-routed** | ≤500 words → hot, >500 words → cold with hot reference |
| **Domain-routed** | Correct domain based on relationship to content |

---

## Inductive Writing (Critical)

Write **inductively**: state the conclusion first, then build supporting detail below it.

| Deductive (don't) | Inductive (do) |
|-------------------|----------------|
| "HIPAA is a law passed in 1996 that created standards for..." → eventually → "...so you must encrypt data at rest" | "HIPAA requires encryption at rest and in transit for PHI. Here's why and what it means:" |
| Reader must read everything to find the point | Reader sees the point immediately, reads deeper only if needed |
| Optimized for writing (how you build knowledge) | Optimized for retrieval (how you look something up) |

**Apply at every level:**
- **Document level:** Opening section states what matters
- **Section level:** Section opens with takeaway, then supporting evidence
- **Paragraph level:** Lead sentence is conclusion; following sentences support it

---

## Hot vs Cold Decision

| If... | Then... |
|-------|---------|
| Content is summary, core fact, decision (≤500 words) | Hot memory: `information/{domain}/{topic}.md` |
| Content is detailed discussion, full transcript, research (>500 words) | Cold storage: `archive/{domain}/{date}-{topic}.md` + hot reference |

**Hot → Cold reference pattern:**
```markdown
# Hot item (information/professional/healthcare/hipaa-encryption.md)
HIPAA requires encryption at rest and in transit for PHI.
Applies to databases, backups, file storage, and network transmission.

**See also:** archive/2026-02-18-hipaa-deep-dive.md
```

---

## Enriching Existing Items

**Before creating a new file, check if one exists for this topic.**

If exists:
1. Read the existing item fully
2. Identify what's NEW in the source
3. Add new information to relevant section
4. Update `modified` timestamp
5. Add new keywords if needed
6. Increase confidence if pattern reinforced

**Don't duplicate.** Enrich existing items rather than creating near-duplicates.

---

## Plain Language Requirement

**Every technical term MUST be explained on first use.**

❌ "Use YAML frontmatter with keyword arrays for grep-based retrieval"
✅ "Use YAML frontmatter (metadata at top of file) with keyword arrays (lists of search terms) for grep-based retrieval (finding files by matching text patterns)"

**The reader in 6 months won't remember what these terms mean. Define everything.**

---

## Keyword Extraction

Keywords are MANDATORY. They enable grep-based retrieval.

**Include natural variations:**

| Type | Examples |
|------|----------|
| **Singular AND plural** | presentation, presentations |
| **Synonyms** | concise, brief, terse, succinct |
| **Common phrasings** | "bottom line", conclusion, summary, crux |
| **Acronyms AND full terms** | HIPAA, "health insurance portability", PHI, "protected health information" |
| **Product names AND variations** | Claude, "Claude Sonnet", claude-sonnet-4, Anthropic |
| **Technical terms AND plain language** | grep, search, "text matching", "pattern matching" |

**Use quotes for multi-word phrases:**
- "bottom line" (not bottom-line)
- "protected health information"
- "TLS 1.2"

---

## File Formats

### Hot Memory Format

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
- See also: archive/2026-02-18-detailed-discussion.md

**Size limit:** 200-500 words total.
```

### Cold Storage Format

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

Organize inductively: conclusion first, supporting detail follows.
Define all technical terms.
Include complete context.
```

---

## Domain Routing

See `scope-routing.md` for complete domain inference strategy.

**Quick reference:**

| Content Type | Domain |
|--------------|--------|
| Project-specific decision | `projects/{project-name}/` |
| Portable domain expertise | `professional/{area}/` |
| How user works | `personal/preferences/` |
| User constraints | `personal/constraints/` |

---

## Gate Function

```
BEFORE marking extraction complete:
  1. CHECK: Domain inferred correctly?
  2. CHECK: Existing item checked for (and enriched if found)?
  3. CHECK: Keywords include natural variations?
  4. CHECK: Technical terms defined in plain language?
  5. CHECK: Written inductively (conclusion first)?
  6. CHECK: Size-routed correctly (hot vs cold)?
  7. CHECK: Cross-references identified?
  If ANY check fails: Do not mark complete. List what's missing.
```

---

## Three-Failure Escalation

If the Gate Function fails 3 times in the same session for the same content, STOP immediately.

1. State what you attempted and what failed each time
2. Ask the user for explicit guidance
3. Do not resume until the user provides direction

Knowledge extraction requires judgment on domain routing, keyword selection, and quality standards. Repeated failures indicate a gap that guessing will not close. Escalate.

---

## Red Flags

If you catch yourself thinking:
- "I'll skip keyword extraction, the filename is enough"
- "This jargon is common knowledge"
- "Writing deductively is fine for detailed content"
- "I don't need to check for existing items"
- "This doesn't need plain language explanations"

**All of these mean: STOP. Re-read the Quality Standards section above.**

---

## Common Rationalizations

| Excuse | Reality |
|--------|---------|
| "This jargon is common knowledge" | The reader in 6 months won't remember. Define every term. |
| "I'll add keywords later" | Later means never. Extract keywords now. |
| "The filename is enough for search" | Grep searches content, not just filenames. Keywords are mandatory. |
| "Deductive writing is clearer for complex topics" | Inductive is ALWAYS clearer for retrieval. Conclusion first. |
| "I don't need to check for existing items" | Checking prevents duplicates. Always check. |
| "This is too detailed for hot memory" | If >500 words, use cold storage. Size routing is mandatory. |

---

## Anti-Patterns

| Don't | Do |
|-------|-----|
| Create new item without checking for existing | Search domain first, enrich existing if found |
| Skip keyword extraction | Always include keywords with natural variations |
| Write deductively (building to conclusion) | Write inductively (conclusion first) |
| Leave jargon unexplained | Define every technical term in plain language |
| Put 2000-word item in hot memory | Hot ≤500 words, cold >500 words |
| Use only exact terms in keywords | Include synonyms, plural forms, common phrasings |
| Skip domain inference | Infer domain before extraction |

---

## Success Metrics

**You're doing this well when:**
- ✅ Future searches find the knowledge easily
- ✅ Items are understandable 6 months later
- ✅ Keywords include natural variations
- ✅ Technical terms are explained
- ✅ Conclusion is visible immediately (inductive)
- ✅ No near-duplicate items

**You need to improve when:**
- ❌ Can't find knowledge you extracted
- ❌ Items require Googling terms to understand
- ❌ Missing keywords that would enable retrieval
- ❌ Must read entire item to find the point
- ❌ Multiple similar items exist for same topic
- ❌ Knowledge routed to wrong domain
