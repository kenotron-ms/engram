# SPEC-RETRIEVAL: Dual-Route Retrieval Specification

> engram-lite retrieval engine — System-1 + System-2 architecture
> Version: 0.1.0 | Status: Draft

---

## Table of Contents

1. [Overview](#1-overview)
2. [Query Analysis & Auto-Routing](#2-query-analysis--auto-routing)
3. [System-1: Fast Similarity Retrieval](#3-system-1-fast-similarity-retrieval)
4. [System-2: Hierarchical Graph Traversal](#4-system-2-hierarchical-graph-traversal)
5. [Hybrid Mode](#5-hybrid-mode)
6. [Keyword Route (BM25-Only)](#6-keyword-route-bm25-only)
7. [Re-Ranking](#7-re-ranking)
8. [Asymmetric Embedding](#8-asymmetric-embedding)
9. [Domain Filtering](#9-domain-filtering)
10. [Hot/Cold Tier Retrieval](#10-hotcold-tier-retrieval)
11. [Context Loading & Injection](#11-context-loading--injection)
12. [Session-Start Pre-Loading](#12-session-start-pre-loading)
13. [Cross-Reference Graph Expansion](#13-cross-reference-graph-expansion)
14. [SQL Reference](#14-sql-reference)
15. [Performance Targets](#15-performance-targets)

---

## 1. Overview

engram-lite implements **dual-route retrieval** inspired by the Mnemis architecture (arXiv:2602.15313). The core insight is that similarity-based retrieval alone cannot handle the full spectrum of memory queries:

| Query Type | Example | Failure Mode of Vector-Only |
|---|---|---|
| Specific lookup | "what port does the API run on?" | Works well |
| Broad domain sweep | "everything I know about this project's auth system" | Misses memories that are semantically distant but topically related |
| Comprehensive recall | "summarize all decisions we've made" | Returns only the most similar cluster, missing breadth |
| Exact term match | "HIPAA compliance requirements" | Approximate match may miss exact terms |

The dual-route architecture addresses this by combining:

- **System-1 (Fast Path):** KNN vector similarity + BM25 full-text search, fused via Reciprocal Rank Fusion. Handles "what do I know about X?" — specific, focused queries.
- **System-2 (Deliberate Path):** Top-down traversal of a hierarchical semantic graph. Handles "what do I know broadly about this domain?" — comprehensive, structural queries.

### Route Selection

| Route | Value | Behavior |
|---|---|---|
| `auto` | Default | Query analysis selects the best route |
| `vector` | Force System-1 | KNN + BM25 with RRF fusion |
| `graph` | Force System-2 | Hierarchical graph traversal only |
| `hybrid` | Both parallel | Run System-1 and System-2 in parallel, fuse results |
| `keyword` | BM25 only | FTS5 full-text search, no vector component |

### Architecture Diagram

```
                         ┌──────────────┐
                         │  memory_recall│
                         │  (query, route)│
                         └──────┬───────┘
                                │
                         ┌──────▼───────┐
                         │ Query Analyzer │
                         │ (auto-routing) │
                         └──────┬───────┘
                                │
              ┌─────────────────┼─────────────────┐
              │                 │                  │
       ┌──────▼──────┐  ┌──────▼──────┐  ┌───────▼──────┐
       │  System-1    │  │  System-2    │  │  Keyword     │
       │  (Vector+BM25)│  │  (Graph)    │  │  (FTS5)      │
       └──────┬──────┘  └──────┬──────┘  └───────┬──────┘
              │                 │                  │
              └─────────────────┼──────────────────┘
                                │
                         ┌──────▼───────┐
                         │  RRF Fusion   │
                         │  + Dedup      │
                         └──────┬───────┘
                                │
                         ┌──────▼───────┐
                         │  Re-Ranker    │
                         │  + Filters    │
                         └──────┬───────┘
                                │
                         ┌──────▼───────┐
                         │  Cross-Ref    │
                         │  Expansion    │
                         └──────┬───────┘
                                │
                         ┌──────▼───────┐
                         │  Format &     │
                         │  Return       │
                         └──────────────┘
```

---

## 2. Query Analysis & Auto-Routing

When `route='auto'` (the default), the query analyzer classifies the query and selects the optimal retrieval route.

### Classification Algorithm

```python
import re

def analyze_query(query: str) -> str:
    """Classify a query and return the optimal route.

    Returns one of: 'vector', 'graph', 'hybrid', 'keyword'
    """
    q = query.strip().lower()
    tokens = q.split()
    token_count = len(tokens)

    # ── Rule 1: Exact term / acronym lookup → keyword ──
    # All-caps tokens, quoted phrases, or very short specific terms
    if re.search(r'\b[A-Z]{2,}\b', query):  # case-sensitive check on original
        return 'keyword'
    if re.search(r'["\'].*?["\']', query):
        return 'keyword'

    # ── Rule 2: Broad / comprehensive queries → graph ──
    broad_signals = [
        r'\beverything\b', r'\ball\b', r'\bsummarize\b', r'\boverview\b',
        r'\bbroad(ly)?\b', r'\bcomprehensive\b', r'\bentire\b',
        r'\bwhat do (i|we) know\b', r'\btell me about\b',
        r'\bwhat have (i|we)\b', r'\bhistory of\b',
        r'\brelated to\b.*\bdomain\b',
    ]
    for pattern in broad_signals:
        if re.search(pattern, q):
            return 'graph'

    # ── Rule 3: Domain-scoped broad queries → graph ──
    # "about the auth system", "regarding deployment"
    if re.search(r'\babout (the |our |this )', q) and token_count > 4:
        return 'graph'

    # ── Rule 4: Short specific queries → vector ──
    if token_count <= 4:
        return 'vector'

    # ── Rule 5: Question-form queries → hybrid ──
    question_signals = [
        r'^(what|how|where|when|why|which|who)\b',
        r'\?$',
    ]
    for pattern in question_signals:
        if re.search(pattern, q):
            return 'hybrid'

    # ── Rule 6: Medium-length declarative → vector ──
    if token_count <= 8:
        return 'vector'

    # ── Default: hybrid for anything complex ──
    return 'hybrid'
```

### Routing Examples

| Query | Tokens | Signals Detected | Route |
|---|---|---|---|
| `"kubernetes timeout setting"` | 3 | Short specific | `vector` |
| `"everything about this project"` | 4 | "everything" | `graph` |
| `"HIPAA"` | 1 | All-caps acronym | `keyword` |
| `"what do I know about the auth system?"` | 9 | "what do I know" | `graph` |
| `"how does the API handle rate limiting?"` | 8 | Question-form | `hybrid` |
| `"summarize all decisions we've made"` | 5 | "summarize", "all" | `graph` |
| `"redis connection pool"` | 3 | Short specific | `vector` |
| `"what port does the staging server use?"` | 7 | Question-form | `hybrid` |
| `'"error_handler" function'` | 2 | Quoted phrase | `keyword` |

### Override Behavior

When a caller provides an explicit `route` parameter (not `'auto'`), query analysis is skipped entirely and the specified route executes directly. This allows deterministic control when the caller knows what they want.

---

## 3. System-1: Fast Similarity Retrieval

System-1 combines two complementary retrieval strategies and fuses their results.

### 3.1 Vector KNN Search (sqlite-vec)

**Purpose:** Find memories whose semantic meaning is closest to the query.

**Pipeline:**

1. **Embed the query** using asymmetric embedding (see [Section 8](#8-asymmetric-embedding))
2. **Execute KNN search** via sqlite-vec's `vec_distance_cosine` virtual table
3. **Return top-k candidates** (default k=20)

**SQL Query:**

```sql
-- KNN vector search via sqlite-vec
SELECT
    m.memory_id,
    m.summary,
    m.content_type,
    m.domain,
    m.importance,
    m.confidence,
    m.created_at,
    m.accessed_at,
    v.distance
FROM memory_vectors AS v
INNER JOIN memories AS m ON m.memory_id = v.memory_id
WHERE v.embedding MATCH :query_embedding
    AND k = :k
ORDER BY v.distance ASC;
```

The `MATCH` syntax is sqlite-vec's KNN operator. The `distance` column contains the cosine distance (0 = identical, 2 = opposite). We convert to similarity: `similarity = 1.0 - distance`.

**Filtering applied post-KNN** (sqlite-vec does not support pre-filtering):

```sql
-- Post-filter wrapper
SELECT * FROM (
    SELECT
        m.memory_id,
        m.summary,
        m.content_type,
        m.domain,
        m.importance,
        m.confidence,
        m.created_at,
        m.accessed_at,
        (1.0 - v.distance) AS similarity
    FROM memory_vectors AS v
    INNER JOIN memories AS m ON m.memory_id = v.memory_id
    WHERE v.embedding MATCH :query_embedding
        AND k = :k_oversample  -- oversample to compensate for post-filtering
) AS candidates
WHERE (:domain IS NULL OR domain LIKE :domain_prefix || '%')
    AND (:space IS NULL OR space = :space)
    AND confidence >= :min_confidence
    AND (:content_types_empty OR content_type IN (:content_types))
    AND (deleted_at IS NULL)
ORDER BY similarity DESC
LIMIT :limit;
```

**Oversampling strategy:** When filters are applied, we oversample the KNN search by a factor of 3x (e.g., request k=60 to get 20 post-filter results) to ensure enough candidates survive filtering.

### 3.2 BM25 Full-Text Search (FTS5)

**Purpose:** Find memories that contain the exact terms from the query. Complements vector search by catching literal matches that might be semantically distant.

**FTS5 Table Definition:**

```sql
CREATE VIRTUAL TABLE memory_fts USING fts5(
    memory_id UNINDEXED,
    summary,
    keywords,
    tags,
    content='memories',
    content_rowid='rowid',
    tokenize='porter unicode61'
);
```

**BM25 Search Query:**

```sql
-- BM25 full-text search
SELECT
    mf.memory_id,
    m.summary,
    m.content_type,
    m.domain,
    m.importance,
    m.confidence,
    m.created_at,
    m.accessed_at,
    rank AS bm25_score
FROM memory_fts AS mf
INNER JOIN memories AS m ON m.memory_id = mf.memory_id
WHERE memory_fts MATCH :query_fts
    AND (:domain IS NULL OR m.domain LIKE :domain_prefix || '%')
    AND (:space IS NULL OR m.space = :space)
    AND m.confidence >= :min_confidence
    AND (:content_types_empty OR m.content_type IN (:content_types))
    AND m.deleted_at IS NULL
ORDER BY rank
LIMIT :k;
```

**Query preprocessing for FTS5:**

```python
def prepare_fts_query(query: str) -> str:
    """Convert natural language query to FTS5 query syntax.

    Applies porter stemming via the tokenizer, but we also:
    - Remove stop words that add noise
    - Quote multi-word phrases if detected
    - Use OR between terms for broad matching
    """
    stop_words = {'the', 'a', 'an', 'is', 'are', 'was', 'were', 'do', 'does',
                  'did', 'i', 'we', 'you', 'it', 'my', 'our', 'this', 'that',
                  'what', 'how', 'when', 'where', 'about', 'know'}
    tokens = [t for t in query.lower().split() if t not in stop_words]
    if not tokens:
        tokens = query.lower().split()[:5]  # fallback: use raw tokens
    return ' OR '.join(tokens)
```

### 3.3 Reciprocal Rank Fusion (RRF)

After obtaining ranked lists from both KNN and BM25, we fuse them using RRF.

**Formula:**

```
RRF_score(d) = Σ  1 / (k + rank_i(d) + 1)
               i∈{knn, bm25}
```

Where `k = 60` (the smoothing constant) and `rank_i(d)` is the 0-based rank of document `d` in ranking `i`. Documents not present in a ranking receive no contribution from that ranking.

**Implementation:**

```python
def reciprocal_rank_fusion(
    rankings: list[list[str]],
    k: int = 60,
) -> dict[str, float]:
    """Fuse multiple ranked lists into a single score dict.

    Args:
        rankings: List of ranked lists, each containing memory_ids
                  ordered by decreasing relevance.
        k: Smoothing constant. Higher k reduces the influence of
           high ranks. k=60 is standard (from Cormack et al. 2009).

    Returns:
        Dict mapping memory_id → fused RRF score.
    """
    scores: dict[str, float] = {}
    for ranking in rankings:
        for rank, memory_id in enumerate(ranking):
            scores[memory_id] = scores.get(memory_id, 0.0) + 1.0 / (k + rank + 1)
    return scores
```

**Score interpretation:**

| Scenario | RRF Score | Meaning |
|---|---|---|
| Rank 1 in both lists | `1/61 + 1/61 = 0.0328` | Strongest possible signal |
| Rank 1 in one list only | `1/61 = 0.0164` | Strong in one modality |
| Rank 10 in both lists | `1/71 + 1/71 = 0.0282` | Consistent mid-rank |
| Rank 1 in one, rank 20 in other | `1/61 + 1/81 = 0.0287` | Good but uneven |

**Why RRF over alternatives:**

- **No score normalization needed:** BM25 scores and cosine distances have different scales. RRF only uses rank positions, making it scale-invariant.
- **Robust to outliers:** The `k` constant prevents a single very high rank from dominating.
- **Simple and fast:** O(n) computation, no hyperparameters to tune beyond `k`.

### 3.4 Complete System-1 Pipeline

```python
async def system1_retrieve(
    query: str,
    limit: int = 5,
    domain: str | None = None,
    space: str | None = None,
    min_confidence: float = 0.5,
    content_types: list[str] = [],
) -> list[ScoredMemory]:
    """System-1 fast retrieval: vector KNN + BM25, fused via RRF."""

    # 1. Embed query (asymmetric)
    query_embedding = embed_query(query)  # adds instruction prefix

    # 2. KNN search (oversample 3x for post-filtering headroom)
    k_oversample = limit * 3 if (domain or space or content_types) else 20
    knn_results = knn_search(
        embedding=query_embedding,
        k=max(k_oversample, 20),
        domain=domain, space=space,
        min_confidence=min_confidence,
        content_types=content_types,
    )

    # 3. BM25 search
    fts_query = prepare_fts_query(query)
    bm25_results = bm25_search(
        query=fts_query,
        k=20,
        domain=domain, space=space,
        min_confidence=min_confidence,
        content_types=content_types,
    )

    # 4. RRF fusion
    knn_ranking = [r.memory_id for r in knn_results]
    bm25_ranking = [r.memory_id for r in bm25_results]
    rrf_scores = reciprocal_rank_fusion([knn_ranking, bm25_ranking], k=60)

    # 5. Sort by RRF score, return top-N
    all_results = {r.memory_id: r for r in knn_results + bm25_results}
    scored = []
    for memory_id, rrf_score in sorted(rrf_scores.items(), key=lambda x: -x[1]):
        memory = all_results[memory_id]
        memory.score = rrf_score
        scored.append(memory)

    return scored[:limit]
```

---

## 4. System-2: Hierarchical Graph Traversal

System-2 retrieval provides **comprehensive, structured recall** by traversing the semantic knowledge graph.

### 4.1 Graph Structure

The graph is a hierarchical tree where:

- **Root nodes** represent top-level domains (e.g., "authentication", "deployment", "user-preferences")
- **Interior nodes** represent sub-domains or topic clusters
- **Leaf nodes** link to individual memories

```
ROOT: "project/backend"
├── "authentication"
│   ├── "oauth2-flow"
│   │   ├── memory_001 (token refresh logic)
│   │   └── memory_002 (scope definitions)
│   └── "session-management"
│       ├── memory_003 (session timeout = 30min)
│       └── memory_004 (redis session store decision)
├── "deployment"
│   ├── "kubernetes"
│   │   ├── memory_005 (pod resource limits)
│   │   └── memory_006 (helm chart structure)
│   └── "ci-cd"
│       └── memory_007 (GitHub Actions workflow)
└── "database"
    ├── "schema"
    │   └── memory_008 (users table definition)
    └── "migrations"
        └── memory_009 (alembic configuration)
```

**Graph tables:**

```sql
CREATE TABLE graph_nodes (
    node_id       TEXT PRIMARY KEY,
    label         TEXT NOT NULL,           -- human-readable name
    level         INTEGER NOT NULL,        -- 0 = root, 1 = domain, 2 = sub-domain, ...
    parent_id     TEXT REFERENCES graph_nodes(node_id),
    summary       TEXT,                    -- LLM-generated summary of this subtree
    memory_count  INTEGER DEFAULT 0,       -- cached count of memories in subtree
    created_at    TEXT DEFAULT (datetime('now')),
    updated_at    TEXT DEFAULT (datetime('now'))
);

CREATE TABLE graph_node_vectors (
    node_id    TEXT PRIMARY KEY REFERENCES graph_nodes(node_id),
    embedding  FLOAT[768]                  -- embedding of the node summary
);

CREATE TABLE graph_node_memories (
    node_id    TEXT REFERENCES graph_nodes(node_id),
    memory_id  TEXT REFERENCES memories(memory_id),
    PRIMARY KEY (node_id, memory_id)
);
```

### 4.2 System-2 Pipeline

```python
async def system2_retrieve(
    query: str,
    limit: int = 5,
    domain: str | None = None,
    space: str | None = None,
    min_confidence: float = 0.5,
    content_types: list[str] = [],
) -> list[ScoredMemory]:
    """System-2 deliberate retrieval: hierarchical graph traversal."""

    # 1. Embed query
    query_embedding = embed_query(query)

    # 2. Find best-matching graph nodes
    matching_nodes = find_matching_nodes(
        embedding=query_embedding,
        domain=domain,
        top_k=3,  # start from up to 3 entry points
    )

    # 3. Expand to full subtrees (parents + siblings + children)
    expanded_node_ids = set()
    for node in matching_nodes:
        subtree = get_subtree(node.node_id, include_parents=True, include_siblings=True)
        expanded_node_ids.update(subtree)

    # 4. Collect all memory_ids from selected nodes
    memory_ids = collect_memories_from_nodes(expanded_node_ids)

    # 5. Load and score memories
    memories = load_memories(memory_ids)
    scored = []
    for m in memories:
        if m.confidence < min_confidence:
            continue
        if content_types and m.content_type not in content_types:
            continue
        if space and m.space != space:
            continue

        score = compute_graph_score(m)
        scored.append(ScoredMemory(memory=m, score=score))

    # 6. Sort by score, return top-N
    scored.sort(key=lambda x: -x.score)
    return scored[:limit]
```

### 4.3 Node Matching Query

```sql
-- Find graph nodes most similar to the query
SELECT
    gn.node_id,
    gn.label,
    gn.level,
    gn.summary,
    gn.memory_count,
    (1.0 - gnv.distance) AS similarity
FROM graph_node_vectors AS gnv
INNER JOIN graph_nodes AS gn ON gn.node_id = gnv.node_id
WHERE gnv.embedding MATCH :query_embedding
    AND k = :top_k
    AND (:domain IS NULL OR gn.label LIKE :domain_prefix || '%'
         OR gn.node_id IN (
             SELECT node_id FROM graph_nodes
             WHERE label LIKE :domain_prefix || '%'
         ))
ORDER BY gnv.distance ASC;
```

### 4.4 Subtree Traversal (Recursive CTE)

```sql
-- Traverse the graph: from matched nodes, get full subtrees
-- including parent chain and sibling nodes
WITH RECURSIVE subtree AS (
    -- Base case: the matched nodes
    SELECT node_id, parent_id, label, level, 0 AS depth
    FROM graph_nodes
    WHERE node_id IN (:matched_node_ids)

    UNION ALL

    -- Recurse downward: children of current nodes
    SELECT gn.node_id, gn.parent_id, gn.label, gn.level, st.depth + 1
    FROM graph_nodes AS gn
    INNER JOIN subtree AS st ON gn.parent_id = st.node_id
    WHERE st.depth < :max_depth  -- limit traversal depth
),
-- Also traverse upward to include parent chain
parent_chain AS (
    SELECT node_id, parent_id, label, level
    FROM graph_nodes
    WHERE node_id IN (:matched_node_ids)

    UNION ALL

    SELECT gn.node_id, gn.parent_id, gn.label, gn.level
    FROM graph_nodes AS gn
    INNER JOIN parent_chain AS pc ON gn.node_id = pc.parent_id
),
-- Include siblings of matched nodes
siblings AS (
    SELECT gn.node_id, gn.parent_id, gn.label, gn.level
    FROM graph_nodes AS gn
    WHERE gn.parent_id IN (
        SELECT parent_id FROM graph_nodes
        WHERE node_id IN (:matched_node_ids)
    )
),
-- Union all discovered nodes
all_nodes AS (
    SELECT node_id FROM subtree
    UNION
    SELECT node_id FROM parent_chain
    UNION
    SELECT node_id FROM siblings
)
-- Collect all memories linked to discovered nodes
SELECT DISTINCT gnm.memory_id
FROM graph_node_memories AS gnm
WHERE gnm.node_id IN (SELECT node_id FROM all_nodes);
```

### 4.5 Graph Node Scoring

Memories retrieved via System-2 are scored using a composite formula:

```python
import math
from datetime import datetime, timezone

IMPORTANCE_WEIGHT = {
    'critical': 1.0,
    'high':     0.8,
    'medium':   0.5,
    'low':      0.2,
}

RECENCY_HALF_LIFE_DAYS = 90  # memories lose half their recency score in 90 days

def compute_graph_score(memory) -> float:
    """Score a memory retrieved via graph traversal.

    Components:
        confidence:       0.0 - 1.0 (how certain we are this memory is accurate)
        importance_weight: mapped from importance level
        recency_decay:    exponential decay based on last access time

    Formula:
        score = confidence * importance_weight * recency_decay
    """
    # Importance weight
    iw = IMPORTANCE_WEIGHT.get(memory.importance, 0.5)

    # Recency decay: exp(-days / half_life)
    now = datetime.now(timezone.utc)
    last_access = memory.accessed_at or memory.created_at
    days_since = (now - last_access).total_seconds() / 86400.0
    recency = math.exp(-days_since / RECENCY_HALF_LIFE_DAYS)

    return memory.confidence * iw * recency
```

**Score ranges by scenario:**

| Scenario | Confidence | Importance | Days Since | Score |
|---|---|---|---|---|
| Fresh critical fact | 1.0 | critical (1.0) | 0 | 1.000 |
| Week-old high-importance | 0.9 | high (0.8) | 7 | 0.666 |
| Month-old medium fact | 0.8 | medium (0.5) | 30 | 0.286 |
| 6-month-old low preference | 0.7 | low (0.2) | 180 | 0.019 |

---

## 5. Hybrid Mode

Hybrid mode runs System-1 and System-2 **in parallel** and fuses their results for maximum recall.

### 5.1 Parallel Execution

```python
import asyncio

async def hybrid_retrieve(
    query: str,
    limit: int = 5,
    **filters,
) -> list[ScoredMemory]:
    """Hybrid retrieval: System-1 + System-2 in parallel, fused via RRF."""

    # Run both systems in parallel
    s1_task = asyncio.create_task(
        system1_retrieve(query, limit=limit * 2, **filters)
    )
    s2_task = asyncio.create_task(
        system2_retrieve(query, limit=limit * 2, **filters)
    )

    s1_results, s2_results = await asyncio.gather(s1_task, s2_task)

    # Fuse via RRF
    s1_ranking = [r.memory_id for r in s1_results]
    s2_ranking = [r.memory_id for r in s2_results]
    fused_scores = reciprocal_rank_fusion([s1_ranking, s2_ranking], k=60)

    # Deduplicate: merge metadata from both result sets
    all_results = {}
    for r in s1_results + s2_results:
        if r.memory_id not in all_results:
            all_results[r.memory_id] = r

    # Sort by fused score
    scored = []
    for memory_id, rrf_score in sorted(fused_scores.items(), key=lambda x: -x[1]):
        memory = all_results[memory_id]
        memory.score = rrf_score
        scored.append(memory)

    return scored[:limit]
```

### 5.2 Result Deduplication

When the same memory appears in both System-1 and System-2 results:

1. **Memory-level dedup:** Same `memory_id` — keep the higher score, merge any metadata differences.
2. **Content-level dedup:** Different `memory_id` but highly overlapping content — detected by comparing summaries with Jaccard similarity > 0.8. Keep the higher-confidence version and boost its score by 10%.
3. **Supersession dedup:** If memory A `supersedes` memory B (via relation), drop B unless B contains unique detail not in A.

---

## 6. Keyword Route (BM25-Only)

The keyword route bypasses vector search entirely and uses only FTS5 for precise term matching.

### When to Use

- Exact acronyms: `"HIPAA"`, `"GDPR"`, `"RFC 7519"`
- Specific identifiers: `"error_handler"`, `"UserAuthService"`
- Quoted phrases: `"connection pool timeout"`

### Implementation

```python
async def keyword_retrieve(
    query: str,
    limit: int = 5,
    **filters,
) -> list[ScoredMemory]:
    """Keyword-only retrieval via FTS5 BM25."""
    fts_query = prepare_fts_query(query)
    return bm25_search(query=fts_query, k=limit, **filters)
```

The keyword route uses the same BM25 search as System-1 but without the vector component or RRF fusion. Results are ranked purely by BM25 relevance score.

---

## 7. Re-Ranking

After retrieval (regardless of route), a final re-ranking pass produces the definitive ordering.

### 7.1 Final Scoring Formula

```python
def final_rerank(
    results: list[ScoredMemory],
    query_embedding: list[float],
) -> list[ScoredMemory]:
    """Apply final re-ranking to retrieval results.

    Final score combines four signals:
        1. query_match:  cosine similarity between query and memory embedding
        2. confidence:   memory's confidence score (0.0 - 1.0)
        3. importance:   mapped importance weight
        4. recency:      exponential decay from last access
    """
    for result in results:
        query_match = cosine_similarity(query_embedding, result.embedding)
        confidence = result.confidence
        importance = IMPORTANCE_WEIGHT.get(result.importance, 0.5)
        recency = compute_recency_decay(result.accessed_at)

        result.final_score = (
            0.40 * query_match +
            0.25 * confidence +
            0.20 * importance +
            0.15 * recency
        )

    results.sort(key=lambda r: -r.final_score)
    return results
```

### 7.2 Weight Rationale

| Component | Weight | Rationale |
|---|---|---|
| `query_match` | 0.40 | Relevance to the current query is the primary signal |
| `confidence` | 0.25 | Higher-confidence memories are more trustworthy |
| `importance` | 0.20 | Critical/high memories should surface more often |
| `recency` | 0.15 | Recent memories are more likely to be contextually relevant |

### 7.3 Boost Factors

In addition to the base formula, certain conditions apply multiplicative boosts:

| Condition | Boost | Rationale |
|---|---|---|
| Memory appeared in multiple routes | ×1.15 | Cross-route agreement is a strong signal |
| Memory has `contradicts` relation to another result | ×1.10 | Contradictions are important to surface |
| Memory is `critical` importance and <7 days old | ×1.20 | Fresh critical information takes priority |
| Memory's domain exactly matches query domain filter | ×1.05 | Exact domain match is a relevance signal |

---

## 8. Asymmetric Embedding

engram-lite uses **asymmetric embedding** to improve retrieval quality. The key insight: documents and queries have different semantic roles.

### The Problem with Symmetric Embedding

If we embed both "The API runs on port 8080" (memory) and "What port does the API run on?" (query) the same way, the cosine similarity may be lower than expected because the surface forms differ significantly.

### Asymmetric Strategy

**Document embedding (at capture time):**
```python
def embed_document(content: str) -> list[float]:
    """Embed a memory's content for storage.

    Documents are embedded as-is (or with a passage prefix for models
    that support it). The content represents a statement of fact.
    """
    # For models with instruction support (e.g., nomic-embed-text):
    prefix = "search_document: "
    return embedding_model.encode(prefix + content)
```

**Query embedding (at retrieval time):**
```python
def embed_query(query: str) -> list[float]:
    """Embed a query for retrieval.

    Queries are wrapped with a question/search instruction prefix to
    shift the embedding into the 'question space' that the model was
    trained to align with document embeddings.
    """
    # For models with instruction support (e.g., nomic-embed-text):
    prefix = "search_query: "
    return embedding_model.encode(prefix + query)
```

### Model-Specific Prefixes

| Model | Document Prefix | Query Prefix |
|---|---|---|
| `nomic-embed-text` | `"search_document: "` | `"search_query: "` |
| `text-embedding-3-small` | (none) | (none — symmetric model) |
| `bge-small-en-v1.5` | `"Represent this document: "` | `"Represent this query: "` |
| `e5-small-v2` | `"passage: "` | `"query: "` |

The embedding model and prefix strategy are configurable. The default is `nomic-embed-text` (768 dimensions) via Ollama for local inference.

---

## 9. Domain Filtering

Domain filtering restricts retrieval to a subtree of the knowledge hierarchy.

### Domain Path Format

Domains use a hierarchical path format with `/` separators:

```
project/backend/authentication
project/backend/database
project/frontend/components
user/preferences/editor
user/workflow/git
```

### Effect on Each Route

**System-1 (vector + BM25):**
- Domain filter is applied as a `LIKE` prefix match on the `domain` column
- `domain='project/backend'` matches `project/backend`, `project/backend/auth`, etc.
- Oversampling factor increases to 4x when domain filter is active

```sql
WHERE m.domain LIKE :domain || '/%' OR m.domain = :domain
```

**System-2 (graph):**
- Graph traversal is **rooted** at the matching domain node instead of searching all nodes
- Only the subtree under the domain node is traversed
- This dramatically reduces traversal time for large graphs

```sql
-- Find the domain's graph node as traversal root
SELECT node_id FROM graph_nodes
WHERE label = :domain OR label LIKE :domain || '/%'
ORDER BY level ASC
LIMIT 1;
```

**Keyword (BM25):**
- Same prefix match filter as System-1

### Domain Inference

When no domain is specified by the caller, the system infers it from the query:

```python
def infer_domain(query: str, available_domains: list[str]) -> str | None:
    """Attempt to infer the most likely domain from the query.

    Returns None if no clear domain match is found (search all).
    """
    query_embedding = embed_query(query)
    domain_embeddings = {d: embed_document(d) for d in available_domains}

    best_domain = None
    best_similarity = 0.0
    for domain, emb in domain_embeddings.items():
        sim = cosine_similarity(query_embedding, emb)
        if sim > best_similarity:
            best_similarity = sim
            best_domain = domain

    # Only use inferred domain if confidence is high
    return best_domain if best_similarity > 0.6 else None
```

---

## 10. Hot/Cold Tier Retrieval

Memories have two content tiers:

| Tier | Field | Loaded When | Typical Size |
|---|---|---|---|
| **Hot** | `summary` | Always (included in all retrieval results) | 1-3 sentences, ~50-150 tokens |
| **Cold** | `detail` | Only when `include_detail=True` in `memory_recall` | Full content, up to ~2000 tokens |

### Hot Tier (Default)

Every retrieval result includes the `summary` field. This is a concise, LLM-generated summary designed for efficient context loading:

```json
{
    "memory_id": "mem_a1b2c3",
    "summary": "API rate limiting uses token bucket algorithm with 100 req/min per user",
    "domain": "project/backend/api",
    "tags": ["rate-limiting", "api"],
    "confidence": 0.9,
    "importance": "high",
    "score": 0.0312
}
```

### Cold Tier (On-Demand)

The `detail` field contains the full original content. It is loaded separately via a second query only when requested:

```sql
-- Cold tier: load full detail for specific memories
SELECT memory_id, detail
FROM memory_details
WHERE memory_id IN (:memory_ids);
```

The `detail` field is stored in a **separate table** (`memory_details`) to keep the hot path scan efficient:

```sql
CREATE TABLE memory_details (
    memory_id  TEXT PRIMARY KEY REFERENCES memories(memory_id),
    detail     TEXT NOT NULL,
    byte_size  INTEGER NOT NULL
);
```

### When to Load Detail

The AI agent should request `include_detail=True` when:

1. The summary alone is insufficient to answer the user's question
2. The user asks for exact syntax, code snippets, or precise specifications
3. A memory's summary mentions "see detail for..." or similar

The context loading strategy (Section 11) determines how much detail to inject.

---

## 11. Context Loading & Injection

Retrieved memories must be formatted for injection into the LLM's context window.

### 11.1 Formatting Template

```xml
<memories source="engram-lite" count="5" route="hybrid" query="auth token refresh">
  <memory id="mem_a1b2c3" type="decision" domain="project/backend/auth"
          importance="high" confidence="0.95" score="0.031">
    OAuth2 refresh tokens are rotated on each use with a 7-day absolute expiry.
    Decided 2025-01-15 based on OWASP recommendations.
  </memory>
  <memory id="mem_d4e5f6" type="fact" domain="project/backend/auth"
          importance="high" confidence="0.90" score="0.028">
    Token refresh endpoint is POST /api/v2/auth/refresh. Returns new
    access_token (15min) and refresh_token (7d).
  </memory>
  <memory id="mem_g7h8i9" type="skill" domain="project/backend/auth"
          importance="medium" confidence="0.85" score="0.024">
    When debugging token issues: check Redis session store first,
    then verify JWT signature with the rotated signing key.
  </memory>
</memories>
```

### 11.2 Context Budget

The injected memory context must not exceed a configurable token budget:

| Context | Default Budget | Configurable |
|---|---|---|
| Session-start pre-load | 2,000 tokens | `hot_context_limit` |
| Per-query recall | 1,500 tokens | `recall_context_limit` |
| Detail expansion | 3,000 tokens | `detail_context_limit` |

**Budget allocation strategy:**

```python
def allocate_context_budget(
    results: list[ScoredMemory],
    budget_tokens: int,
    include_detail: bool = False,
) -> list[FormattedMemory]:
    """Greedily allocate context budget to highest-scoring memories.

    Each memory's token cost is estimated as:
        summary_tokens ≈ len(summary.split()) * 1.3
        detail_tokens  ≈ len(detail.split()) * 1.3 (if include_detail)
        overhead       ≈ 40 tokens (XML tags, attributes)
    """
    allocated = []
    remaining = budget_tokens

    for result in results:  # already sorted by score descending
        cost = estimate_tokens(result.summary) + 40  # summary + overhead
        if include_detail and result.detail:
            cost += estimate_tokens(result.detail)

        if cost > remaining:
            # Try summary-only even if detail was requested
            summary_cost = estimate_tokens(result.summary) + 40
            if summary_cost <= remaining:
                allocated.append(format_memory(result, include_detail=False))
                remaining -= summary_cost
            continue

        allocated.append(format_memory(result, include_detail=include_detail))
        remaining -= cost

    return allocated
```

---

## 12. Session-Start Pre-Loading

At the beginning of each agent session, engram-lite pre-loads a context summary.

### What Gets Loaded

1. **Critical importance memories** from all domains (unfiltered — these always matter)
2. **High importance memories** from domains relevant to the current project/workspace
3. **Recently accessed memories** (last 24 hours, any importance)

### Pre-Load Query

```sql
-- Session-start pre-load: critical + high importance + recent
SELECT
    m.memory_id,
    m.summary,
    m.content_type,
    m.domain,
    m.importance,
    m.confidence,
    m.created_at,
    m.accessed_at
FROM memories AS m
WHERE m.deleted_at IS NULL
    AND m.confidence >= 0.5
    AND (
        -- Always load critical memories
        m.importance = 'critical'
        -- Load high-importance from relevant domains
        OR (m.importance = 'high' AND (
            :space IS NULL OR m.space = :space
        ))
        -- Load recently accessed memories
        OR m.accessed_at >= datetime('now', '-1 day')
    )
ORDER BY
    CASE m.importance
        WHEN 'critical' THEN 1
        WHEN 'high' THEN 2
        WHEN 'medium' THEN 3
        WHEN 'low' THEN 4
    END ASC,
    m.accessed_at DESC
LIMIT :max_preload;  -- default: 20
```

### Pre-Load Budget

The pre-loaded context is subject to the `hot_context_limit` budget (default: 2,000 tokens). The allocation strategy from Section 11.2 applies — highest-importance memories are loaded first, with graceful degradation when the budget is exhausted.

### Pre-Load Formatting

```xml
<system-reminder source="engram-lite">
<context type="session-start" memories="12" budget="2000t">
  You have persistent memory. Here is your current context:

  [CRITICAL]
  - User prefers TypeScript over JavaScript for all new code (confidence: 0.95)
  - Project uses PostgreSQL 16, NOT MySQL (confidence: 1.0)
  - Never suggest using `any` type — user considers this a code smell (confidence: 0.9)

  [HIGH IMPORTANCE - project/backend]
  - API follows REST conventions with /api/v2/ prefix
  - Authentication uses OAuth2 with JWT tokens (15min access, 7d refresh)
  - Rate limiting: token bucket, 100 req/min per user

  [RECENT]
  - Yesterday: discussed migrating from Express to Fastify (decision pending)
  - Yesterday: user encountered CORS issue with staging environment

  Use memory_recall to retrieve additional context when relevant.
  Use memory_capture to save new information worth remembering.
  NEVER announce memory operations to the user.
</context>
</system-reminder>
```

---

## 13. Cross-Reference Graph Expansion

After the primary retrieval pass, an optional expansion step follows relation edges to pull in contextually important linked memories.

### Expansion Algorithm

```python
async def expand_via_relations(
    results: list[ScoredMemory],
    max_expansion: int = 3,
) -> list[ScoredMemory]:
    """Expand retrieval results by following relation edges.

    For each result, check for important relations:
    - 'contradicts': ALWAYS include (critical for correctness)
    - 'supersedes':  Replace old with new
    - 'supports':    Include if budget allows
    - 'part-of':     Include parent memory
    """
    result_ids = {r.memory_id for r in results}
    expansions = []

    for result in results:
        relations = get_relations(result.memory_id)
        for rel in relations:
            other_id = rel.to_id if rel.from_id == result.memory_id else rel.from_id

            if other_id in result_ids:
                continue  # already in results

            if rel.relation_type == 'contradicts':
                # Always include contradictions
                other = load_memory(other_id)
                other.score = result.score * 0.95  # slightly below the source
                other._expansion_reason = f"contradicts {result.memory_id}"
                expansions.append(other)

            elif rel.relation_type == 'supersedes':
                # If result supersedes another, drop the old one
                # If result IS superseded, swap in the newer version
                if rel.from_id == result.memory_id:
                    pass  # result is newer, keep it
                else:
                    newer = load_memory(rel.from_id)
                    newer.score = result.score * 1.05
                    newer._expansion_reason = f"supersedes {result.memory_id}"
                    expansions.append(newer)

            elif rel.relation_type in ('supports', 'part-of') and len(expansions) < max_expansion:
                other = load_memory(other_id)
                other.score = result.score * 0.7 * rel.strength
                other._expansion_reason = f"{rel.relation_type} {result.memory_id}"
                expansions.append(other)

    # Deduplicate and merge
    all_results = {r.memory_id: r for r in results}
    for exp in expansions:
        if exp.memory_id not in all_results:
            all_results[exp.memory_id] = exp

    # Re-sort
    final = sorted(all_results.values(), key=lambda r: -r.score)
    return final
```

### Relation Expansion Query

```sql
-- Find all relations for a set of memory IDs
SELECT
    r.relation_id,
    r.from_id,
    r.to_id,
    r.relation_type,
    r.strength,
    r.created_at
FROM memory_relations AS r
WHERE r.from_id IN (:memory_ids) OR r.to_id IN (:memory_ids)
ORDER BY
    CASE r.relation_type
        WHEN 'contradicts' THEN 1
        WHEN 'supersedes' THEN 2
        WHEN 'supports' THEN 3
        WHEN 'part-of' THEN 4
        ELSE 5
    END ASC,
    r.strength DESC;
```

---

## 14. SQL Reference

### Complete Table Schema

```sql
-- Core memories table (hot tier)
CREATE TABLE memories (
    memory_id     TEXT PRIMARY KEY,
    summary       TEXT NOT NULL,
    content_type  TEXT NOT NULL CHECK(content_type IN
                    ('fact','preference','event','skill','entity','relationship','decision')),
    domain        TEXT,
    space         TEXT NOT NULL CHECK(space IN ('user', 'project')),
    importance    TEXT NOT NULL DEFAULT 'medium' CHECK(importance IN
                    ('critical','high','medium','low')),
    confidence    FLOAT NOT NULL DEFAULT 0.8 CHECK(confidence >= 0.0 AND confidence <= 1.0),
    tags          TEXT,        -- JSON array: '["tag1", "tag2"]'
    keywords      TEXT,        -- space-separated keywords for FTS
    expires_at    TEXT,        -- ISO 8601 datetime, NULL = permanent
    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at    TEXT NOT NULL DEFAULT (datetime('now')),
    accessed_at   TEXT NOT NULL DEFAULT (datetime('now')),
    deleted_at    TEXT,        -- soft delete timestamp
    deleted_reason TEXT        -- why it was deleted
);

-- Vector embeddings (sqlite-vec virtual table)
CREATE VIRTUAL TABLE memory_vectors USING vec0(
    memory_id TEXT PRIMARY KEY,
    embedding FLOAT[768]
);

-- Full-text search index
CREATE VIRTUAL TABLE memory_fts USING fts5(
    memory_id UNINDEXED,
    summary,
    keywords,
    tags,
    content='memories',
    content_rowid='rowid',
    tokenize='porter unicode61'
);

-- Cold tier detail storage
CREATE TABLE memory_details (
    memory_id  TEXT PRIMARY KEY REFERENCES memories(memory_id),
    detail     TEXT NOT NULL,
    byte_size  INTEGER NOT NULL
);

-- Relations between memories
CREATE TABLE memory_relations (
    relation_id   TEXT PRIMARY KEY,
    from_id       TEXT NOT NULL REFERENCES memories(memory_id),
    to_id         TEXT NOT NULL REFERENCES memories(memory_id),
    relation_type TEXT NOT NULL CHECK(relation_type IN
                    ('relates-to','supports','contradicts','supersedes',
                     'exemplifies','part-of','caused-by','decided-in','applies-to')),
    strength      FLOAT NOT NULL DEFAULT 0.5 CHECK(strength >= 0.0 AND strength <= 1.0),
    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(from_id, to_id, relation_type)
);

-- Indexes
CREATE INDEX idx_memories_domain ON memories(domain);
CREATE INDEX idx_memories_space ON memories(space);
CREATE INDEX idx_memories_importance ON memories(importance);
CREATE INDEX idx_memories_content_type ON memories(content_type);
CREATE INDEX idx_memories_accessed_at ON memories(accessed_at);
CREATE INDEX idx_memories_deleted_at ON memories(deleted_at);
CREATE INDEX idx_relations_from ON memory_relations(from_id);
CREATE INDEX idx_relations_to ON memory_relations(to_id);
CREATE INDEX idx_relations_type ON memory_relations(relation_type);
```

### Key Queries Summary

| Operation | Query | Expected Time |
|---|---|---|
| KNN search (k=20) | `vec_distance_cosine MATCH` on `memory_vectors` | <20ms @ 10k |
| BM25 search (k=20) | `memory_fts MATCH` ranked query | <10ms @ 10k |
| RRF fusion | In-memory Python computation | <1ms |
| Graph node match | `vec_distance_cosine MATCH` on `graph_node_vectors` | <10ms |
| Subtree traversal | Recursive CTE on `graph_nodes` | <50ms |
| Memory collection | `IN` query on `graph_node_memories` | <20ms |
| Relation expansion | `IN` query on `memory_relations` | <5ms |
| Pre-load query | Filtered scan on `memories` | <30ms |

---

## 15. Performance Targets

### Latency Budgets

| Route | Target (10k memories) | Target (100k memories) | Breakdown |
|---|---|---|---|
| **System-1 (vector)** | <50ms | <150ms | embed: 10ms, KNN: 20ms, BM25: 10ms, RRF: 1ms, rerank: 5ms |
| **System-2 (graph)** | <200ms | <500ms | embed: 10ms, node match: 10ms, CTE: 50ms, collect: 20ms, score: 10ms, rerank: 5ms |
| **Hybrid** | <200ms | <500ms | parallel(S1, S2), fuse: 2ms, dedup: 5ms, rerank: 5ms |
| **Keyword** | <30ms | <100ms | FTS5 query: 15ms, rerank: 5ms |
| **Session pre-load** | <100ms | <200ms | filtered scan: 50ms, format: 20ms |

### Throughput

- Single-query throughput: ≥50 queries/sec for System-1, ≥10 queries/sec for System-2
- Concurrent sessions: database supports WAL mode for concurrent reads

### Memory Overhead

- sqlite-vec index: ~3KB per memory (768 × float32 = 3072 bytes + overhead)
- FTS5 index: ~500 bytes per memory
- Graph node vectors: ~3KB per node
- Total for 10k memories: ~35MB on disk, ~50MB with WAL

### Scaling Limits

| Scale | Supported | Notes |
|---|---|---|
| 1k memories | Fully | All routes <50ms |
| 10k memories | Fully | System-1 <50ms, System-2 <200ms |
| 100k memories | Supported | May need indexed domain filtering, graph pruning |
| 1M memories | Not targeted | Would require sharding, approximate graph, HNSW |

---

*End of SPEC-RETRIEVAL.md*
