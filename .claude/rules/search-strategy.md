# Memory Search Strategy

Domain inference and keyword extraction for efficient retrieval.

## Domain Inference

**Before searching, determine the relevant domain(s).**

### Common Signals

| Signal in Conversation | Likely Domain |
|------------------------|---------------|
| Memory system, Engram | `projects/engram/` |
| Personal preference, work style | `personal/preferences/` |
| Schedule, habits, routines | `personal/work-patterns/` |
| System design, architecture | `professional/architecture/` |
| HIPAA, compliance, medical | `professional/healthcare/` |
| Current project name mentioned | `projects/{project-name}/` |

### Multi-Domain Search

Often relevant to search MULTIPLE domains:

Example: User asks about memory system architecture
- Domain 1: `projects/memory-system/`
- Domain 2: `professional/architecture/`

Search both, merge results.

## Keyword Extraction

**Extract precise, searchable terms with natural variations.**

### Include All Forms

| Category | Example |
|----------|---------|
| **Singular + Plural** | presentation, presentations |
| **Synonyms** | concise, brief, terse, succinct |
| **Common phrases** | "bottom line", conclusion, summary, takeaway |
| **Acronyms** | HIPAA, PHI, "protected health information" |
| **Product names** | Claude, "Claude Sonnet", claude-sonnet-4, Anthropic |
| **Technical terms** | hot-memory, cold-storage, grep, YAML, frontmatter |

### Quote Multi-Word Phrases

```
✅ "bottom line"
✅ "hot memory"
✅ "dual write"

❌ bottom-line (searches for hyphenated only)
❌ hot memory (searches for 'hot' AND 'memory' separately)
```

## Search Tool Usage

**YAML-aware search** (handles multi-line arrays properly):
```bash
python scripts/canvas-memory-search.py \
  --keyword "term1,term2,term3" \
  --domain "domain/" \
  --base ~/.canvas/memory
```

**For project memory**, change base:
```bash
python scripts/canvas-memory-search.py \
  --keyword "architecture,design" \
  --domain "knowledge/" \
  --base .canvas/memory
```

## Optimization: Domain-Scoped Search

**Critical performance optimization:**

| Without Domain Scoping | With Domain Scoping |
|------------------------|---------------------|
| Search all 1000 items | Search 20-50 items in domain |
| Get 50+ matches | Get 3-5 matches |
| Need ranking algorithm | Directly relevant results |

**The folder structure IS the optimization.** Always infer domain first.

## Search Workflow

```
1. USER MESSAGE arrives
   ↓
2. Infer domain(s) from context
   ↓
3. Extract keywords with variations
   ↓
4. Search user memory in domain(s)
   ↓
5. Search project memory in domain(s)
   ↓
6. Load 2-3 most relevant files from each
   ↓
7. Apply knowledge in response
```

## When Search Finds Nothing

If search returns no results:
1. ✅ Continue with general knowledge (no memory for this topic yet)
2. ✅ Respond to user's question
3. ✅ Consider capturing the new knowledge after responding

**Do NOT mention** that memory search found nothing.
