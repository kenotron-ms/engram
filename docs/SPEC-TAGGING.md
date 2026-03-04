# SPEC-TAGGING: Canvas Memory Tagging & Taxonomy Specification

**Version:** 0.1.0
**Status:** Draft
**Last Updated:** 2026-03-03

---

## 1. Overview

Canvas Memory uses a multi-layered tagging system to organize memories for fast filtered retrieval. Tags work alongside vector embeddings and full-text search — they provide hard categorical filters that narrow the search space before semantic ranking takes over.

The tagging system has three complementary components:

| Component | Table | Purpose |
|-----------|-------|---------|
| **Tags** | `memory_tags` | Categorical labels for filtering (domain, type, importance) |
| **Keywords** | `memory_keywords` | Recall vocabulary for hybrid search (synonyms, plurals, acronyms) |
| **Graph nodes** | `graph_nodes` + `memory_graph_nodes` | Hierarchical taxonomy for navigation and inheritance |

Tags answer "what category is this?" Keywords answer "what words should find this?" Graph nodes answer "where does this fit in the knowledge tree?"

---

## 2. Tag Types

Every tag on a memory serves one of six functional roles. Tags are stored as flat strings in `memory_tags`, but follow naming conventions that encode their role.

### 2.1 Domain Tags

Hierarchical path tags that place a memory in the knowledge taxonomy.

| Format | Example | Description |
|--------|---------|-------------|
| `domain:{path}` | `domain:professional/architecture` | Primary domain placement |
| | `domain:personal/preferences` | |
| | `domain:projects/engram-lite/decisions` | |

Domain tags mirror the `domain` column on `memories` but are also stored as tags to enable multi-domain membership (a memory's `domain` column is its _primary_ domain; it can have additional domain tags for secondary placements).

### 2.2 Content-Type Tags

The seven canonical content types, stored as tags for uniform filtering.

| Tag | Description |
|-----|-------------|
| `type:fact` | Objective information |
| `type:preference` | Subjective user preference |
| `type:event` | Something that happened (time-bound) |
| `type:skill` | A capability or expertise |
| `type:entity` | A thing (tool, service, framework) |
| `type:relationship` | Connection between entities/people |
| `type:decision` | A choice that was made with rationale |

### 2.3 Semantic Tags

Subject-matter tags that describe what the memory is about. These are the most numerous and varied.

| Example | Description |
|---------|-------------|
| `distributed-systems` | Technical topic |
| `kubernetes` | Specific technology |
| `hipaa` | Compliance framework |
| `team-communication` | Soft skill area |
| `python` | Programming language |
| `api-design` | Design discipline |
| `cost-optimization` | Business concern |

Semantic tags use plain hyphenated lowercase strings with no prefix.

### 2.4 Temporal Tags

Tags that describe the memory's relationship to time.

| Tag | Meaning | Typical Content Types |
|-----|---------|----------------------|
| `temporal:current` | Reflects the present state of affairs | fact, preference, entity |
| `temporal:historical` | Describes a past state that may have changed | event, decision |
| `temporal:recurring` | A pattern that repeats | preference, skill |
| `temporal:time-sensitive` | Will become stale; pairs with `expires_at` | fact, decision |

### 2.5 Importance Tags

Mirror the `importance` column as tags for uniform tag-based filtering.

| Tag | When Applied |
|-----|-------------|
| `importance:critical` | Always loaded into hot context |
| `importance:high` | Loaded when relevant domain is active |
| `importance:medium` | Loaded on recall match |
| `importance:low` | Loaded only on direct search |

### 2.6 Provenance Tags

Describe how the memory was acquired and how much to trust it.

| Tag | Meaning | Default Confidence Effect |
|-----|---------|---------------------------|
| `provenance:user-stated` | User explicitly told the agent | +0.10 (starts at 0.80) |
| `provenance:observed` | Agent inferred from user behavior | Neutral (starts at 0.70) |
| `provenance:inferred` | Agent reasoned from indirect evidence | −0.05 (starts at 0.65) |
| `provenance:verified` | Confirmed across multiple sessions | Sets floor at 0.85 |

---

## 3. Tag Normalization Rules

All tags are normalized before storage to ensure consistent matching.

### 3.1 Normalization Algorithm

```python
import re
import unicodedata

MAX_TAG_LENGTH = 64

def normalize_tag(raw: str) -> str:
    """Normalize a tag string for storage.
    
    Rules:
    1. Unicode NFC normalization
    2. Lowercase
    3. Replace spaces and underscores with hyphens
    4. Remove all characters except [a-z0-9\-/:]
    5. Collapse multiple hyphens into one
    6. Strip leading/trailing hyphens
    7. Truncate to 64 characters
    
    The colon (:) is preserved for prefixed tags (type:, domain:, etc.)
    The slash (/) is preserved for hierarchical paths (domain:professional/architecture)
    """
    # Step 1: Unicode normalization
    tag = unicodedata.normalize("NFC", raw)
    
    # Step 2: Lowercase
    tag = tag.lower()
    
    # Step 3: Replace spaces and underscores with hyphens
    tag = tag.replace(" ", "-").replace("_", "-")
    
    # Step 4: Remove disallowed characters
    tag = re.sub(r"[^a-z0-9\-/:]", "", tag)
    
    # Step 5: Collapse multiple hyphens
    tag = re.sub(r"-{2,}", "-", tag)
    
    # Step 6: Strip leading/trailing hyphens
    tag = tag.strip("-")
    
    # Step 7: Truncate
    tag = tag[:MAX_TAG_LENGTH]
    
    return tag
```

### 3.2 Normalization Examples

| Raw Input | Normalized Output |
|-----------|-------------------|
| `Distributed Systems` | `distributed-systems` |
| `HIPAA_Compliance` | `hipaa-compliance` |
| `type: Fact` | `type:fact` |
| `domain:Professional/Architecture` | `domain:professional/architecture` |
| `C++` | `c` (note: `++` stripped — use `cpp` instead) |
| `node.js` | `nodejs` (note: `.` stripped) |
| `---test---` | `test` |
| `résumé` | `resume` (NFC then stripped) |

### 3.3 Reserved Prefixes

| Prefix | Owner | Examples |
|--------|-------|---------|
| `type:` | System | `type:fact`, `type:decision` |
| `domain:` | System | `domain:professional/architecture` |
| `temporal:` | System | `temporal:current` |
| `importance:` | System | `importance:high` |
| `provenance:` | System | `provenance:verified` |
| (none) | Semantic | `kubernetes`, `api-design` |

Tags without a prefix are always semantic tags. The system never generates unprefixed tags that could collide with reserved prefixes.

---

## 4. Auto-Tagging During Capture

When a memory is captured, an LLM generates its tag set. The prompt explicitly requests each tag type.

### 4.1 Auto-Tag Prompt Template

```
You are a memory-tagging system. Given the captured content and its metadata, generate a comprehensive tag set.

## Input

Content type: {content_type}
Domain: {domain}
Content:
{content}

Summary:
{summary}

## Instructions

Generate tags in each of the following categories. Return ONLY a JSON object with the keys shown below.

### semantic_tags (3-8 tags)
Subject-matter tags describing what this memory is about.
- Use lowercase hyphenated strings
- Include specific technologies, concepts, methodologies
- Include both broad and narrow terms (e.g., "databases" AND "postgresql")
- Do NOT include tags that duplicate the content_type or domain

### temporal_tag (exactly 1)
One of: "current", "historical", "recurring", "time-sensitive"
- "current": reflects present state ("I use vim", "We're on AWS")
- "historical": past state that may have changed ("Last year we used Jenkins")
- "recurring": repeated pattern ("Every sprint we do retros")
- "time-sensitive": will become stale ("The deadline is March 15th")

### provenance_tag (exactly 1)
One of: "user-stated", "observed", "inferred"
- "user-stated": the user explicitly told the agent this fact
- "observed": the agent saw the user do this (code patterns, tool usage)
- "inferred": the agent deduced this from indirect evidence

## Output Format

Return a JSON object. No markdown fencing. No explanation.

{
  "semantic_tags": ["tag1", "tag2", "tag3"],
  "temporal_tag": "current",
  "provenance_tag": "user-stated"
}
```

### 4.2 Auto-Tag Post-Processing

After the LLM returns the tag JSON, the system:

1. Parses the JSON (with fallback regex extraction on parse failure).
2. Normalizes every tag through `normalize_tag()`.
3. Adds the system-generated tags:
   - `type:{content_type}`
   - `domain:{domain}`
   - `importance:{importance}`
   - `temporal:{temporal_tag}`
   - `provenance:{provenance_tag}`
4. Deduplicates the full tag set.
5. Inserts all tags into `memory_tags`.

```python
def build_full_tag_set(
    content_type: str,
    domain: str,
    importance: str,
    llm_tags: dict,
) -> set[str]:
    """Build the complete normalized tag set for a memory."""
    tags = set()
    
    # System-generated tags
    tags.add(normalize_tag(f"type:{content_type}"))
    tags.add(normalize_tag(f"domain:{domain}"))
    tags.add(normalize_tag(f"importance:{importance}"))
    
    # LLM-generated tags
    for tag in llm_tags.get("semantic_tags", []):
        tags.add(normalize_tag(tag))
    
    temporal = llm_tags.get("temporal_tag", "current")
    tags.add(normalize_tag(f"temporal:{temporal}"))
    
    provenance = llm_tags.get("provenance_tag", "observed")
    tags.add(normalize_tag(f"provenance:{provenance}"))
    
    return tags
```

### 4.3 Example: Full Auto-Tag Pipeline

**Input:**
```
Content type: decision
Domain: projects/engram-lite/decisions
Importance: high
Content: "We decided to use SQLite with sqlite-vec for the memory storage
layer instead of PostgreSQL with pgvector. Key reasons: zero infrastructure
overhead, single-file portability, and sufficient performance for the
expected scale of < 100K memories per database."
```

**LLM returns:**
```json
{
  "semantic_tags": [
    "sqlite",
    "database-selection",
    "sqlite-vec",
    "vector-search",
    "infrastructure",
    "pgvector",
    "postgresql"
  ],
  "temporal_tag": "current",
  "provenance_tag": "user-stated"
}
```

**Final tag set (12 tags):**
```
type:decision
domain:projects/engram-lite/decisions
importance:high
temporal:current
provenance:user-stated
sqlite
database-selection
sqlite-vec
vector-search
infrastructure
pgvector
postgresql
```

---

## 5. Keyword Extraction

### 5.1 Tags vs. Keywords

Tags and keywords serve different purposes and are stored in different tables:

| Aspect | Tags (`memory_tags`) | Keywords (`memory_keywords`) |
|--------|----------------------|------------------------------|
| **Purpose** | Categorical filtering (narrow the search space) | Recall vocabulary (find by any related term) |
| **Quantity** | 5–15 per memory | 10–40 per memory |
| **Form** | Normalized, hyphenated, categorical | Natural language forms: plural, singular, synonyms |
| **Used in** | Pre-filter before search | BM25 ranking via FTS5 `keywords` column |
| **Weighted?** | No (present or absent) | Yes (1.0–5.0 weight per keyword) |

### 5.2 Keyword Construction Rules

Keywords must comprehensively cover the vocabulary space around a memory so that users (or agents) can find it regardless of the specific phrasing they use.

#### Rule 1: Singular AND Plural Forms

Every noun keyword must include both forms:

| Singular | Plural |
|----------|--------|
| `microservice` | `microservices` |
| `database` | `databases` |
| `policy` | `policies` |
| `index` | `indexes` / `indices` |

#### Rule 2: Synonyms and Common Phrasings

Include alternative ways to express the same concept:

| Primary Term | Synonyms |
|-------------|----------|
| `authentication` | `auth`, `login`, `sign-in`, `sign in` |
| `configuration` | `config`, `settings`, `setup` |
| `container` | `docker`, `containerized`, `containerization` |
| `deployment` | `deploy`, `ship`, `release`, `rollout` |
| `infrastructure` | `infra`, `platform`, `ops` |

#### Rule 3: Acronyms AND Full Expansions

| Acronym | Expansion |
|---------|-----------|
| `k8s` | `kubernetes` |
| `HIPAA` | `health insurance portability accountability act` |
| `CI/CD` | `continuous integration`, `continuous delivery`, `continuous deployment` |
| `API` | `application programming interface` |
| `DB` | `database` |
| `IAM` | `identity access management` |
| `VPC` | `virtual private cloud` |

#### Rule 4: Multi-Word Phrases

Stored as space-separated strings. FTS5's porter tokenizer handles them correctly:

```
# These are all valid keywords stored in memory_keywords.keyword
"distributed systems"
"event driven architecture"
"blue green deployment"
"service mesh"
"rate limiting"
```

#### Rule 5: Technical AND Plain-Language Equivalents

| Technical | Plain Language |
|-----------|---------------|
| `idempotent` | `safe to retry`, `repeatable` |
| `eventual consistency` | `delayed sync`, `not immediately consistent` |
| `load balancer` | `traffic distributor`, `request router` |
| `schema migration` | `database update`, `table change` |
| `observability` | `monitoring`, `logging`, `tracing` |

#### Rule 6: Related Terms

Include terms that are conceptually adjacent, not just synonyms:

| Core Concept | Related Terms |
|-------------|---------------|
| `postgresql` | `postgres`, `pg`, `relational`, `sql`, `rdbms` |
| `kubernetes` | `k8s`, `container orchestration`, `pods`, `helm`, `kubectl` |
| `react` | `jsx`, `hooks`, `components`, `frontend`, `ui` |
| `terraform` | `infrastructure as code`, `iac`, `hcl`, `provisioning` |

### 5.3 Keyword Extraction Prompt

```
You are a keyword extraction system. Given a memory's content, generate a comprehensive keyword list for search recall.

## Input

Content type: {content_type}
Content:
{content}

Summary:
{summary}

## Instructions

Generate keywords following these rules:
1. Include BOTH singular and plural forms of every noun
2. Include synonyms and common alternative phrasings
3. Include acronyms AND their full expansions
4. Include multi-word phrases (as space-separated strings)
5. Include technical AND plain-language equivalents
6. Include related terms (not just exact matches)

For each keyword, assign a weight:
- 3.0: Primary subject (the main thing this memory is about)
- 2.0: Important related term or strong synonym
- 1.0: Peripheral related term, alternative phrasing

## Output Format

Return a JSON array of [keyword, weight] pairs. No markdown fencing.

[
  ["keyword1", 3.0],
  ["keyword2", 2.0],
  ["keyword3", 1.0]
]
```

### 5.4 Keyword Example

**Memory content:** "User prefers composition over inheritance in TypeScript for better testing."

**Extracted keywords:**

| Keyword | Weight | Rule Applied |
|---------|--------|--------------|
| `composition` | 3.0 | Primary subject |
| `inheritance` | 3.0 | Primary subject |
| `typescript` | 3.0 | Primary subject |
| `ts` | 2.0 | Acronym |
| `design pattern` | 2.0 | Related concept |
| `design patterns` | 2.0 | Plural form |
| `testing` | 2.0 | Key reason |
| `test` | 2.0 | Singular form |
| `tests` | 2.0 | Plural form |
| `testability` | 2.0 | Related concept |
| `composition over inheritance` | 2.0 | Multi-word phrase |
| `favor composition` | 1.0 | Alternative phrasing |
| `object oriented` | 1.0 | Related concept |
| `oop` | 1.0 | Acronym |
| `class hierarchy` | 1.0 | Related concept |
| `class hierarchies` | 1.0 | Plural form |
| `interface` | 1.0 | Related concept |
| `interfaces` | 1.0 | Plural form |
| `mixin` | 1.0 | Related pattern |
| `mixins` | 1.0 | Plural form |
| `dependency injection` | 1.0 | Related pattern |
| `di` | 1.0 | Acronym |
| `code reuse` | 1.0 | Plain-language equivalent |

### 5.5 Keyword-to-FTS5 Synchronization

Keywords are stored in two places:

1. **`memory_keywords` table** — with per-keyword weights for re-ranking.
2. **`memory_fts.keywords` column** — as a single space-separated string for BM25 search.

```python
def sync_keywords(conn, memory_id: str, keywords: list[tuple[str, float]]):
    """Store keywords in both the keywords table and FTS5."""
    # 1. Store in memory_keywords with weights
    conn.execute("DELETE FROM memory_keywords WHERE memory_id = ?", (memory_id,))
    conn.executemany(
        "INSERT INTO memory_keywords (memory_id, keyword, weight) VALUES (?, ?, ?)",
        [(memory_id, kw, weight) for kw, weight in keywords]
    )
    
    # 2. Build the FTS5 keyword string (weights not included — FTS5 uses term frequency)
    # Repeat high-weight keywords to boost their BM25 score
    fts_parts = []
    for kw, weight in keywords:
        repeat = max(1, int(weight))  # weight 3.0 → repeat 3 times
        fts_parts.extend([kw] * repeat)
    
    keyword_string = " ".join(fts_parts)
    return keyword_string  # Caller passes this to sync_fts()
```

---

## 6. Domain Inference Algorithm

When a new memory is captured, the system must determine the correct domain path. This is a four-step process performed by the LLM during capture.

### 6.1 Step 1: Relationship Classification

**Question:** What is the user's RELATIONSHIP to this content?

| Relationship | Indicator | Maps To |
|-------------|-----------|---------|
| **Personal experience** | "I prefer...", "I always...", "My approach..." | `personal/` |
| **Professional knowledge** | Architecture patterns, engineering practices, domain expertise | `professional/` |
| **Project-specific** | "In this project...", "Our service...", specific repo/system names | `projects/{name}/` |
| **About a person** | "Alice mentioned...", "The tech lead prefers..." | `people/{name}/` |
| **General knowledge** | Objective facts without personal connection | `professional/` (default) |

### 6.2 Step 2: Primary Domain Selection

```
personal/         → User's own preferences, constraints, workflow, bio
professional/     → Technical knowledge, patterns, practices
projects/{name}/  → Specific project context, decisions
people/{name}/    → Knowledge about collaborators
```

Decision tree:

```
Is this about the user's personal style, preference, or constraint?
  YES → personal/
  NO  ↓
Is this about a specific project, repo, or system?
  YES → projects/{project_name}/
  NO  ↓
Is this about a specific person (not the user)?
  YES → people/{person_name}/
  NO  → professional/
```

### 6.3 Step 3: Subdomain Selection

The subdomain should match an **existing** graph node when possible. Creating new nodes is a last resort.

```python
def infer_subdomain(
    conn,
    content: str,
    primary_domain: str,
    content_type: str,
) -> str:
    """Determine the subdomain for a memory.
    
    Strategy:
    1. Fetch existing graph nodes under the primary domain.
    2. Ask the LLM to classify the content into an existing node.
    3. If no existing node fits, propose a new subdomain.
    """
    # Fetch existing nodes under this domain
    existing_nodes = conn.execute("""
        SELECT label, summary, memory_count
        FROM graph_nodes
        WHERE label LIKE ? || '/%'
        ORDER BY memory_count DESC
    """, (primary_domain,)).fetchall()
    
    if not existing_nodes:
        # No existing nodes — use LLM to propose initial subdomain
        return llm_propose_subdomain(content, primary_domain, content_type)
    
    # Format existing nodes for LLM selection
    node_list = "\n".join(
        f"- {n['label']} ({n['memory_count']} memories): {n['summary'] or 'No summary'}"
        for n in existing_nodes
    )
    
    return llm_select_subdomain(content, primary_domain, node_list, content_type)
```

**LLM prompt for subdomain selection:**

```
Given this memory content and the existing taxonomy nodes, select the best-fit subdomain.

Content type: {content_type}
Content: {content}

Primary domain: {primary_domain}

Existing nodes under {primary_domain}:
{node_list}

Instructions:
1. If the content fits an existing node, return that node's label.
2. If no existing node fits, propose a new subdomain label.
3. New labels should be lowercase, hyphenated, and descriptive.
4. Prefer reusing existing nodes over creating new ones.
5. Depth should match specificity: broad topics → level 1-2, specific topics → level 3-4.

Return ONLY the full domain path (e.g., "professional/architecture/microservices").
No explanation.
```

### 6.4 Step 4: Validation

After domain inference, validate the assignment with a simple heuristic:

**Question:** If the user searched for content under this domain path, would they expect to find this memory?

```python
def validate_domain_assignment(content: str, domain: str, summary: str) -> bool:
    """Sanity check: would this content reappear naturally under this path?
    
    The validation asks: if the user browses to this domain node and
    sees a list of memory summaries, would this summary look out of place?
    """
    # Quick heuristic checks
    parts = domain.split("/")
    
    # Check 1: The domain path should have at least 2 levels
    if len(parts) < 2:
        return False
    
    # Check 2: The content should contain at least one term related
    # to the leaf node of the domain path
    leaf = parts[-1].replace("-", " ")
    content_lower = content.lower()
    if leaf not in content_lower and not any(
        word in content_lower for word in leaf.split()
    ):
        # Weak signal — not a hard failure, but flag for review
        pass
    
    return True
```

### 6.5 Domain Inference Examples

| Content | Step 1 | Step 2 | Step 3 | Final Domain |
|---------|--------|--------|--------|-------------|
| "I always use 4-space indentation in Python" | Personal preference | `personal/` | `preferences` | `personal/preferences` |
| "The payment service uses event sourcing for audit trails" | Project-specific | `projects/payment-service/` | `patterns` | `projects/payment-service/patterns` |
| "CQRS separates read and write models for scalability" | Professional knowledge | `professional/` | `architecture` | `professional/architecture` |
| "Alice prefers to review PRs in the morning" | About a person | `people/alice/` | (root) | `people/alice` |
| "We decided to use gRPC over REST for internal services" | Project decision | `projects/{active}/` | `decisions` | `projects/{active}/decisions` |
| "I have HIPAA certification and 5 years healthcare IT experience" | Personal bio | `personal/` | `bio` | `personal/bio` |

---

## 7. Tag Inheritance from Graph Nodes

When a memory is assigned to a graph node, it inherits tags from all ancestor nodes in the hierarchy.

### 7.1 Inheritance Rule

A memory placed at `professional/architecture/microservices` automatically receives:

```
domain:professional
domain:professional/architecture
domain:professional/architecture/microservices
```

This ensures that a query filtered by `domain:professional` will find memories at any depth under that domain.

### 7.2 Implementation

```python
def inherit_domain_tags(domain: str) -> list[str]:
    """Generate inherited domain tags for all ancestor paths.
    
    Input:  "professional/architecture/microservices"
    Output: [
        "domain:professional",
        "domain:professional/architecture",
        "domain:professional/architecture/microservices"
    ]
    """
    parts = domain.split("/")
    tags = []
    for i in range(1, len(parts) + 1):
        path = "/".join(parts[:i])
        tags.append(f"domain:{path}")
    return tags
```

### 7.3 Cascading Updates

When a graph node is renamed or moved, all memories assigned to that node need their domain tags updated:

```python
def rename_graph_node(conn, old_label: str, new_label: str):
    """Rename a graph node and update all affected memory tags."""
    # 1. Update the graph node itself
    conn.execute(
        "UPDATE graph_nodes SET label = ? WHERE label = ?",
        (new_label, old_label)
    )
    
    # 2. Find all affected memories
    node_id = conn.execute(
        "SELECT id FROM graph_nodes WHERE label = ?", (new_label,)
    ).fetchone()["id"]
    
    memory_ids = conn.execute(
        "SELECT memory_id FROM memory_graph_nodes WHERE node_id = ?",
        (node_id,)
    ).fetchall()
    
    # 3. Update domain tags on each affected memory
    old_domain_tag = f"domain:{old_label}"
    new_domain_tag = f"domain:{new_label}"
    
    for row in memory_ids:
        mid = row["memory_id"]
        # Remove old domain tag
        conn.execute(
            "DELETE FROM memory_tags WHERE memory_id = ? AND tag = ?",
            (mid, old_domain_tag)
        )
        # Add new domain tag
        conn.execute(
            "INSERT OR IGNORE INTO memory_tags (memory_id, tag) VALUES (?, ?)",
            (mid, new_domain_tag)
        )
    
    # 4. Update all child node labels (cascade the rename)
    children = conn.execute(
        "SELECT label FROM graph_nodes WHERE label LIKE ? || '/%'",
        (old_label,)
    ).fetchall()
    
    for child in children:
        child_new = child["label"].replace(old_label, new_label, 1)
        rename_graph_node(conn, child["label"], child_new)
```

---

## 8. Tag-Based Retrieval

### 8.1 Tag Filtering in Recall

Tags are used as pre-filters before vector and FTS search. The recall pipeline:

```
Query context
    │
    ├─ Extract tag filters from query
    │  (e.g., "What Python preferences?" → filter: type:preference, python)
    │
    ├─ Build candidate set:
    │  SELECT DISTINCT m.id FROM memories m
    │  JOIN memory_tags t ON m.id = t.memory_id
    │  WHERE t.tag IN (?, ?, ?)        -- tag filter
    │    AND m.confidence > 0.30       -- confidence floor
    │    AND m.superseded_by IS NULL   -- not superseded
    │
    ├─ Vector search within candidate set (or full set if no tags)
    ├─ FTS5 search within candidate set
    └─ Merge and re-rank
```

### 8.2 Tag Filter Queries

**Single tag filter (AND with search):**

```sql
SELECT m.id, m.summary, m.confidence
FROM memories m
JOIN memory_tags t ON m.id = t.memory_id
WHERE t.tag = 'python'
  AND m.superseded_by IS NULL
  AND m.confidence > 0.30
ORDER BY m.confidence DESC;
```

**Multiple tags (ANY match):**

```sql
SELECT m.id, m.summary, m.confidence, COUNT(*) AS tag_matches
FROM memories m
JOIN memory_tags t ON m.id = t.memory_id
WHERE t.tag IN ('python', 'type:preference', 'domain:personal/preferences')
  AND m.superseded_by IS NULL
  AND m.confidence > 0.30
GROUP BY m.id
ORDER BY tag_matches DESC, m.confidence DESC;
```

**Multiple tags (ALL must match):**

```sql
SELECT m.id, m.summary, m.confidence
FROM memories m
WHERE m.superseded_by IS NULL
  AND m.confidence > 0.30
  AND m.id IN (
    SELECT memory_id FROM memory_tags WHERE tag = 'python'
  )
  AND m.id IN (
    SELECT memory_id FROM memory_tags WHERE tag = 'type:preference'
  )
ORDER BY m.confidence DESC;
```

**Domain hierarchy filter (all memories under a domain):**

```sql
-- Uses the inherited domain tags
SELECT m.id, m.summary, m.confidence
FROM memories m
JOIN memory_tags t ON m.id = t.memory_id
WHERE t.tag = 'domain:professional'  -- catches all subdomain memories
  AND m.superseded_by IS NULL
  AND m.confidence > 0.30
ORDER BY m.confidence DESC;
```

### 8.3 Tag Extraction from Query

The system infers tag filters from natural-language queries:

| Query Fragment | Inferred Tag Filter |
|---------------|---------------------|
| "Python preferences" | `python`, `type:preference` |
| "architecture decisions" | `domain:professional/architecture`, `type:decision` |
| "what does Alice think about..." | `domain:people/alice` |
| "project engram-lite" | `domain:projects/engram-lite` |
| "critical items" | `importance:critical` |
| "recent events" | `type:event`, `temporal:current` |

---

## 9. Tag Management

### 9.1 Merging Tags

When two tags should be combined (e.g., `k8s` and `kubernetes` were both used as semantic tags):

```python
def merge_tags(conn, old_tag: str, new_tag: str):
    """Merge old_tag into new_tag across all memories.
    
    All memories tagged with old_tag will be tagged with new_tag.
    old_tag entries are removed.
    """
    # Find memories that have old_tag but not new_tag
    conn.execute("""
        INSERT OR IGNORE INTO memory_tags (memory_id, tag)
        SELECT memory_id, ? FROM memory_tags WHERE tag = ?
    """, (new_tag, old_tag))
    
    # Remove old_tag
    conn.execute("DELETE FROM memory_tags WHERE tag = ?", (old_tag,))
```

### 9.2 Splitting Tags

When a tag is too broad and should be split into more specific tags:

```python
def split_tag(conn, old_tag: str, mapping: dict[str, list[str]]):
    """Split a tag based on content analysis.
    
    mapping: {memory_id: [new_tag1, new_tag2]} — per-memory tag assignment.
    Memories not in the mapping keep the old tag.
    """
    for memory_id, new_tags in mapping.items():
        # Remove old tag from this memory
        conn.execute(
            "DELETE FROM memory_tags WHERE memory_id = ? AND tag = ?",
            (memory_id, old_tag)
        )
        # Add new tags
        conn.executemany(
            "INSERT OR IGNORE INTO memory_tags (memory_id, tag) VALUES (?, ?)",
            [(memory_id, t) for t in new_tags]
        )
```

### 9.3 Renaming Tags

```python
def rename_tag(conn, old_tag: str, new_tag: str):
    """Rename a tag across all memories.
    
    This is equivalent to merge_tags, but semantically represents
    a rename rather than a merge (the old tag was "wrong").
    """
    new_tag = normalize_tag(new_tag)
    
    # Update all occurrences
    conn.execute("""
        UPDATE OR IGNORE memory_tags SET tag = ? WHERE tag = ?
    """, (new_tag, old_tag))
    
    # Clean up any duplicates created by the update
    # (if a memory already had new_tag, the UPDATE OR IGNORE skips it,
    #  but the old_tag row remains)
    conn.execute("""
        DELETE FROM memory_tags 
        WHERE tag = ? 
          AND memory_id IN (
            SELECT memory_id FROM memory_tags WHERE tag = ?
          )
    """, (old_tag, new_tag))
```

### 9.4 Tag Statistics

```sql
-- Most used tags
SELECT tag, COUNT(*) as usage_count
FROM memory_tags
GROUP BY tag
ORDER BY usage_count DESC
LIMIT 50;

-- Tags used only once (candidates for cleanup)
SELECT tag
FROM memory_tags
GROUP BY tag
HAVING COUNT(*) = 1;

-- Tags per memory (identify over/under-tagged memories)
SELECT memory_id, COUNT(*) as tag_count
FROM memory_tags
GROUP BY memory_id
ORDER BY tag_count DESC;

-- Orphaned semantic tags (no memories using them)
-- Not directly queryable since tags don't have their own table,
-- but you can find memories with very few tags:
SELECT m.id, m.summary, COUNT(t.tag) as tag_count
FROM memories m
LEFT JOIN memory_tags t ON m.id = t.memory_id
WHERE m.superseded_by IS NULL
GROUP BY m.id
HAVING tag_count < 3
ORDER BY m.created_at DESC;
```

---

## 10. Example Tag Sets by Content Type

### 10.1 Fact

**Content:** "The engram-lite project uses SQLite with sqlite-vec for vector storage."

```
type:fact
domain:projects/engram-lite/context
importance:high
temporal:current
provenance:user-stated
sqlite
sqlite-vec
vector-storage
database
persistence
```

**Keywords (12):**
```
sqlite (3.0), sqlite-vec (3.0), vector storage (3.0),
database (2.0), databases (2.0), persistence (2.0),
vector search (2.0), embeddings (1.0), knn (1.0),
engram-lite (2.0), vec0 (1.0), vector database (1.0)
```

### 10.2 Preference

**Content:** "I prefer dark mode in all editors and terminals. Light themes cause eye strain."

```
type:preference
domain:personal/preferences
importance:medium
temporal:recurring
provenance:user-stated
dark-mode
editor
terminal
visual-preferences
accessibility
```

**Keywords (15):**
```
dark mode (3.0), dark theme (3.0), editor (2.0), editors (2.0),
terminal (2.0), terminals (2.0), light mode (2.0), light theme (2.0),
eye strain (2.0), color scheme (1.0), color schemes (1.0),
theme (1.0), themes (1.0), visual preference (1.0), accessibility (1.0)
```

### 10.3 Event

**Content:** "On 2026-02-15, we migrated the payment service from MongoDB to PostgreSQL. Took 3 weeks, zero downtime."

```
type:event
domain:projects/payment-service/context
importance:high
temporal:historical
provenance:user-stated
database-migration
mongodb
postgresql
zero-downtime
```

**Keywords (20):**
```
database migration (3.0), mongodb (3.0), mongo (2.0), postgresql (3.0),
postgres (2.0), pg (1.0), payment service (3.0), migration (2.0),
migrations (2.0), zero downtime (2.0), zero-downtime migration (2.0),
downtime (1.0), data migration (1.0), schema migration (1.0),
february 2026 (1.0), nosql (1.0), relational (1.0), rdbms (1.0),
cutover (1.0), database switch (1.0)
```

### 10.4 Skill

**Content:** "User is proficient with Kubernetes, regularly deploying multi-cluster setups with Istio service mesh."

```
type:skill
domain:professional/engineering
importance:medium
temporal:current
provenance:observed
kubernetes
istio
service-mesh
multi-cluster
container-orchestration
devops
```

**Keywords (22):**
```
kubernetes (3.0), k8s (3.0), istio (3.0), service mesh (3.0),
multi-cluster (2.0), container orchestration (2.0), deployment (2.0),
deployments (2.0), deploy (2.0), cluster (2.0), clusters (2.0),
pods (1.0), helm (1.0), kubectl (1.0), devops (1.0),
infrastructure (1.0), infra (1.0), containers (1.0), docker (1.0),
microservices (1.0), sidecar proxy (1.0), envoy (1.0)
```

### 10.5 Entity

**Content:** "Datadog is our primary observability platform. We use it for metrics, traces, and logs across all services."

```
type:entity
domain:professional/engineering
importance:high
temporal:current
provenance:user-stated
datadog
observability
monitoring
apm
logging
```

**Keywords (18):**
```
datadog (3.0), observability (3.0), monitoring (3.0), metrics (2.0),
traces (2.0), tracing (2.0), logs (2.0), logging (2.0),
apm (2.0), application performance monitoring (2.0),
observability platform (2.0), dashboards (1.0), alerts (1.0),
alerting (1.0), telemetry (1.0), open telemetry (1.0),
otel (1.0), distributed tracing (1.0)
```

### 10.6 Relationship

**Content:** "The auth service depends on Redis for session storage and PostgreSQL for user accounts."

```
type:relationship
domain:projects/auth-service/context
importance:high
temporal:current
provenance:observed
redis
postgresql
session-storage
service-dependency
auth-service
```

**Keywords (18):**
```
auth service (3.0), authentication service (2.0), redis (3.0),
session storage (3.0), sessions (2.0), postgresql (3.0),
postgres (2.0), user accounts (2.0), accounts (2.0),
dependency (2.0), dependencies (2.0), service dependency (2.0),
cache (1.0), caching (1.0), database (1.0), databases (1.0),
data store (1.0), persistence (1.0)
```

### 10.7 Decision

**Content:** "We chose gRPC over REST for internal service communication because of type safety via protobuf, streaming support, and 3× throughput improvement in benchmarks."

```
type:decision
domain:professional/architecture
importance:high
temporal:current
provenance:user-stated
grpc
rest
api-design
service-communication
protobuf
performance
```

**Keywords (24):**
```
grpc (3.0), rest (3.0), restful (2.0), api design (2.0),
service communication (3.0), inter-service (2.0),
protobuf (3.0), protocol buffers (2.0), proto (2.0),
type safety (2.0), type-safe (2.0), streaming (2.0),
throughput (2.0), performance (2.0), benchmark (1.0),
benchmarks (1.0), rpc (1.0), remote procedure call (1.0),
http2 (1.0), http/2 (1.0), api (1.0), apis (1.0),
serialization (1.0), microservice communication (1.0)
```

---

## 11. Graph Node Management

### 11.1 Node Creation

New graph nodes are created on-demand during domain inference:

```python
import uuid
from datetime import datetime, timezone

def ensure_graph_path(conn, domain: str) -> str:
    """Ensure all graph nodes exist for a domain path.
    
    Input: "professional/architecture/microservices"
    Creates nodes (if missing):
      - "professional" (level 0)
      - "professional/architecture" (level 1)
      - "professional/architecture/microservices" (level 2)
    
    Returns the ID of the leaf node.
    """
    parts = domain.split("/")
    parent_id = None
    leaf_id = None
    
    for i, part in enumerate(parts):
        label = "/".join(parts[:i + 1])
        level = i
        
        existing = conn.execute(
            "SELECT id FROM graph_nodes WHERE label = ?", (label,)
        ).fetchone()
        
        if existing:
            parent_id = existing["id"]
            leaf_id = existing["id"]
        else:
            node_id = str(uuid.uuid4())
            now = datetime.now(timezone.utc).isoformat()
            conn.execute("""
                INSERT INTO graph_nodes (id, label, level, parent_id, child_count, memory_count, updated_at)
                VALUES (?, ?, ?, ?, 0, 0, ?)
            """, (node_id, label, level, parent_id, now))
            
            # Increment parent's child_count
            if parent_id:
                conn.execute(
                    "UPDATE graph_nodes SET child_count = child_count + 1 WHERE id = ?",
                    (parent_id,)
                )
            
            parent_id = node_id
            leaf_id = node_id
    
    return leaf_id
```

### 11.2 Node Summaries

Graph node summaries are LLM-generated aggregations of the memories under that node. They are regenerated periodically:

```python
def regenerate_node_summary(conn, node_id: str, llm_client):
    """Regenerate the summary for a graph node based on its memories."""
    # Fetch the node's label and all direct memory summaries
    node = conn.execute(
        "SELECT label FROM graph_nodes WHERE id = ?", (node_id,)
    ).fetchone()
    
    memories = conn.execute("""
        SELECT m.summary, m.content_type, m.confidence
        FROM memories m
        JOIN memory_graph_nodes mg ON m.id = mg.memory_id
        WHERE mg.node_id = ?
          AND m.superseded_by IS NULL
          AND m.confidence > 0.30
        ORDER BY m.confidence DESC
        LIMIT 20
    """, (node_id,)).fetchall()
    
    if not memories:
        return
    
    # Generate summary
    memory_list = "\n".join(
        f"- [{m['content_type']}] {m['summary']}" for m in memories
    )
    
    prompt = f"""Summarize the knowledge contained in the "{node['label']}" 
category based on these memories:

{memory_list}

Write a 2-3 sentence summary of the key themes and facts. Be specific."""
    
    summary = llm_client.complete(prompt)
    
    conn.execute(
        "UPDATE graph_nodes SET summary = ?, updated_at = ? WHERE id = ?",
        (summary, datetime.now(timezone.utc).isoformat(), node_id)
    )
```

### 11.3 Memory Count Maintenance

The `memory_count` on graph nodes is denormalized for performance. It must be maintained on insert/delete:

```python
def increment_memory_count(conn, node_id: str):
    """Increment memory count for a node and all ancestors."""
    conn.execute(
        "UPDATE graph_nodes SET memory_count = memory_count + 1 WHERE id = ?",
        (node_id,)
    )
    # Walk up the tree
    parent = conn.execute(
        "SELECT parent_id FROM graph_nodes WHERE id = ?", (node_id,)
    ).fetchone()
    if parent and parent["parent_id"]:
        increment_memory_count(conn, parent["parent_id"])
```

---

## Appendix A: Complete Tag Taxonomy Reference

```
# System tags (auto-generated, prefixed)
type:fact
type:preference
type:event
type:skill
type:entity
type:relationship
type:decision

domain:personal
domain:personal/preferences
domain:personal/constraints
domain:personal/workflow
domain:personal/bio
domain:professional
domain:professional/architecture
domain:professional/engineering
domain:professional/security
domain:professional/data
domain:professional/domain-specific
domain:projects/{name}
domain:projects/{name}/decisions
domain:projects/{name}/context
domain:projects/{name}/patterns
domain:people/{name}

temporal:current
temporal:historical
temporal:recurring
temporal:time-sensitive

importance:critical
importance:high
importance:medium
importance:low

provenance:user-stated
provenance:observed
provenance:inferred
provenance:verified

# Semantic tags (LLM-generated, unprefixed) — examples
python
typescript
kubernetes
docker
postgresql
redis
aws
gcp
microservices
distributed-systems
api-design
testing
security
authentication
deployment
ci-cd
monitoring
observability
performance
scalability
```

## Appendix B: Tag Query Cheat Sheet

| Goal | Query Pattern |
|------|--------------|
| All preferences | `WHERE tag = 'type:preference'` |
| All critical items | `WHERE tag = 'importance:critical'` |
| Everything about Python | `WHERE tag = 'python'` |
| Python preferences only | `WHERE tag IN ('python', 'type:preference') GROUP BY ... HAVING COUNT(*) = 2` |
| All project decisions | `WHERE tag LIKE 'domain:projects/%' INTERSECT WHERE tag = 'type:decision'` |
| Everything under professional/ | `WHERE tag = 'domain:professional'` |
| User-stated facts | `WHERE tag IN ('type:fact', 'provenance:user-stated') GROUP BY ... HAVING COUNT(*) = 2` |
| Current, high-importance | `WHERE tag IN ('temporal:current', 'importance:high') GROUP BY ... HAVING COUNT(*) = 2` |
| Tags on a specific memory | `WHERE memory_id = ?` |
| Memories sharing ≥ 3 tags with memory X | Complex: join memory_tags to itself, count overlaps |
