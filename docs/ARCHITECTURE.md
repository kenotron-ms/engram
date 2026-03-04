# engram-lite: Architecture Document

**Version:** 0.1.0
**Date:** 2026-03-03
**Status:** Draft
**Authors:** Ken (design lead)

---

## 1. System Overview

```
+-----------------------------------------------------------------------------------+
|                              AI Agent (Claude Code / Amplifier)                    |
|                                                                                   |
|   +------------------+    +-------------------+    +--------------------------+   |
|   | Hook: on submit  |--->| Agent LLM Context |<---| Hook: on response        |   |
|   | (recall reminder)|    |                   |    | (capture reminder)       |   |
|   +------------------+    +--------+----------+    +--------------------------+   |
|                                    |                                              |
|                           Tool calls (function calling)                           |
+------------------------------------|--------------------------------------------- +
                                     |
                    +----------------v-----------------+
                    |        engram-lite Engine       |
                    |                                   |
                    |  +-----------------------------+  |
                    |  |       Tool Router            |  |
                    |  |  capture | recall | search   |  |
                    |  |  update  | relate | forget   |  |
                    |  |  graph_explore | stats       |  |
                    |  +-------------+---------------+  |
                    |                |                   |
                    |       +--------v--------+         |
                    |       | Retrieval Engine |         |
                    |       |                 |         |
                    |  +----v----+  +----v----+         |
                    |  |System-1 |  |System-2 |         |
                    |  |Vec+BM25 |  | Graph   |         |
                    |  | + RRF   |  |Traversal|         |
                    |  +----+----+  +----+----+         |
                    |       |            |              |
                    |       +-----+------+              |
                    |             |                     |
                    |       +-----v------+              |
                    |       | RRF Merger |              |
                    |       +-----+------+              |
                    |             |                     |
                    |  +----------v-----------+         |
                    |  |   Capture Pipeline   |         |
                    |  | embed | classify |   |         |
                    |  | summarize | xref  |  |         |
                    |  +----------+----------+          |
                    |             |                     |
                    +-------------|---------------------+
                                  |
                 +----------------v-----------------+
                 |         Storage Layer             |
                 |                                   |
                 |  +-------------+ +-------------+  |
                 |  | User Space  | |Project Space|  |
                 |  | ~/.engram/  | | .engram/    |  |
                 |  | memory/     | | memory/     |  |
                 |  |             | |             |  |
                 |  | memory.db   | | memory.db   |  |
                 |  | (SQLite +   | | (SQLite +   |  |
                 |  |  sqlite-vec)| |  sqlite-vec)|  |
                 |  +-------------+ +-------------+  |
                 +-----------------------------------+
```

---

## 2. Core Principles

### P1: Silent by Default

Memory is infrastructure, not interface. The user should never see "Searching memory...", "Saving to memory...", or any acknowledgment of memory operations. The AI simply knows things. This mirrors how human memory works — you don't announce "accessing long-term memory" before answering a question. All memory tool calls are made silently; their results inform the response but are never narrated.

### P2: Tools-First, Not File-First

Memory operations are exposed as real function-calling tools (`memory_capture`, `memory_recall`, etc.), not as file reads/writes that the AI must construct manually. This gives the AI's native tool-calling machinery full control over when and how memory is used, enables structured input validation and output formatting, and keeps the memory protocol decoupled from any specific file format.

### P3: Dual-Route Retrieval for Completeness

No single retrieval method covers all query types. Vector similarity excels at finding the needle ("What's Ken's preferred test framework?") but fails at the haystack ("What are all the security decisions we've made?"). Hierarchical graph traversal covers structural breadth but may miss semantically similar items that aren't co-located in the taxonomy. Combining both routes — and letting the system choose the right blend per query — ensures both precision and coverage.

### P4: Privacy as Architecture, Not Policy

The user-private and project-shared spaces are not merely conventions — they are separate database files in separate filesystem locations with separate access paths. There is no code path that reads from user-space when operating in project-space context, and no query that spans both without explicit intent. The "README test" is enforced at the capture boundary, not as a post-hoc audit.

### P5: Graceful Degradation

Every component can fail independently without cascading. If the embedding API is unreachable, memories are captured without vectors and backfilled later. If the graph is corrupted, System-1 still works. If the entire memory system is down, the AI continues as a normal stateless agent. The user never sees an error caused by memory infrastructure.

### P6: Local-Only, Zero Network Dependency for Storage

All memory data lives on local disk as SQLite files. The only network call is to the embedding API (OpenAI `text-embedding-3-small`), and even that is optional — the system functions (with degraded retrieval quality) using BM25-only search when embeddings are unavailable. No memory content is ever transmitted to a remote service for storage, indexing, or analytics.

---

## 3. Component Architecture

### 3.1 Tool Router

The entry point for all memory operations. Receives tool calls from the AI agent, validates parameters, resolves the target space (user vs project), and dispatches to the appropriate subsystem.

```
Tool Router
├── Input validation (schema enforcement, type coercion)
├── Space resolution (explicit space param > heuristic: git project → project, else → user)
├── Dispatch to:
│   ├── Capture Pipeline    (memory_capture)
│   ├── Retrieval Engine    (memory_recall, memory_search)
│   ├── Update Handler      (memory_update)
│   ├── Relation Manager    (memory_relate)
│   ├── Forget Handler      (memory_forget)
│   ├── Graph Explorer      (memory_graph_explore)
│   └── Stats Collector     (memory_stats)
└── Response formatting (structured JSON for agent consumption)
```

**Key design decisions:**
- Space resolution is performed once at the router level, not repeated in each subsystem.
- Cross-space queries (e.g., recall from both user and project) are dispatched as two parallel queries and merged at the router level using interleaved RRF.
- All tool calls return a uniform envelope: `{ success: bool, data: any, error?: string }`.

### 3.2 Capture Pipeline

Handles the full lifecycle of creating a new memory, from raw content to indexed, embedded, graph-linked record.

```
Capture Pipeline
│
├── 1. Deduplication Check
│   └── Embed query content → cosine similarity scan → if >0.95 match, route to Update
│
├── 2. Content Classification
│   ├── Assign content_type if not provided (fact|preference|event|skill|entity|relationship|decision)
│   └── Assign domain path from taxonomy if not provided
│
├── 3. Summary Generation
│   └── If content > 500 words, generate 200-500 word summary for hot tier
│   └── Store full content in detail field (cold tier)
│
├── 4. Embedding Generation
│   ├── Format: "{content_type}: {summary}\n\n{content[:512]}"
│   ├── Call text-embedding-3-small → 1536-dim vector
│   └── On API failure: mark as pending_embedding, continue without vector
│
├── 5. Keyword Extraction
│   └── Extract weighted keywords (synonyms, acronyms, phrases) for BM25 index
│
├── 6. Storage Write (atomic transaction)
│   ├── INSERT into memories
│   ├── INSERT into memory_vectors (sqlite-vec)
│   ├── INSERT into memory_tags
│   ├── INSERT into memory_keywords
│   └── INSERT into memory_graph_nodes (link to taxonomy)
│
├── 7. Graph Node Assignment
│   ├── Resolve domain path to graph_nodes hierarchy
│   ├── Create missing intermediate nodes
│   └── Update child_count on parent nodes
│
└── 8. Cross-Reference Cascade (async, post-commit)
    ├── Find top-K similar existing memories (K=10)
    ├── Score relation candidates by type heuristics
    ├── Create memory_relations edges where strength > threshold
    ├── Flag contradictions (content_type=decision with opposing conclusions)
    └── Detect potential supersessions (same domain + newer + overlapping keywords)
```

### 3.3 Retrieval Engine

Dual-route retrieval system inspired by Mnemis. Selects and executes the appropriate retrieval strategy based on query characteristics.

```
Retrieval Engine
│
├── Route Selector
│   ├── Explicit route override (route="system1"|"system2"|"hybrid")
│   └── Automatic classification:
│       ├── Specific/factual query → System-1 only
│       ├── Broad/enumerative query → System-1 + System-2 (hybrid)
│       └── Exploratory/structural query → System-2 primary, System-1 supplemental
│
├── System-1: Similarity-Based (Fast Path)
│   ├── Vector KNN: embed query → top-K nearest in memory_vectors (sqlite-vec)
│   ├── BM25 Full-Text: query against memory content + keywords (FTS5)
│   └── RRF Fusion: merge vector and BM25 ranked lists
│       score(d) = Σ 1 / (k + rank_i(d))  where k=60
│
├── System-2: Hierarchical Graph (Deliberate Path)
│   ├── Entry point resolution: query → matching graph_nodes (by label similarity)
│   ├── Upward walk: find common ancestors for structural context
│   ├── Downward walk: collect all descendant memories under matched nodes
│   ├── Sibling expansion: include memories under sibling nodes for breadth
│   └── Rank by: node relevance × memory importance × recency
│
├── Result Merger
│   ├── When both routes fire: RRF across System-1 and System-2 result lists
│   ├── Apply post-filters: domain, space, content_type, temporal range
│   ├── Apply boosting: importance weight, recency decay, access frequency
│   ├── Deprioritize: superseded memories (move to end unless explicitly requested)
│   └── Attach: related memories via memory_relations (strength > 0.5)
│
└── Response Assembly
    ├── Return hot tier (summary) by default
    ├── Include cold tier (detail) only if explicitly requested or summary is insufficient
    └── Include relation metadata (what relates to what, and how)
```

### 3.4 Graph Manager

Maintains the hierarchical graph structure that enables System-2 retrieval. The graph mirrors the domain taxonomy but grows dynamically as new memories are captured.

```
graph_nodes table:
┌──────────────────────────────────────────────────┐
│  level 0 (roots)                                 │
│  ├── personal/          ├── professional/        │
│  │   ├── preferences/   │   ├── architecture/    │
│  │   ├── constraints/   │   ├── engineering/     │
│  │   ├── workflow/      │   ├── security/        │
│  │   └── bio/           │   ├── data/            │
│  │                      │   └── domain-specific/ │
│  ├── projects/          ├── people/              │
│  │   └── {name}/        │   └── {name}/          │
│  │       ├── decisions/ │                        │
│  │       ├── context/   │                        │
│  │       └── patterns/  │                        │
└──────────────────────────────────────────────────┘

Each node stores:
- id: unique identifier
- label: human-readable name ("architecture", "security")
- level: depth in hierarchy (0 = root)
- parent_id: FK to parent node (null for roots)
- summary: auto-generated digest of child memories
- child_count: number of direct children (nodes + memories)
- updated_at: last modification timestamp
```

**Dynamic growth:** When a memory is captured with `domain="projects/engram-lite/decisions"`, the graph manager ensures all intermediate nodes exist (`projects/` → `projects/engram-lite/` → `projects/engram-lite/decisions/`), creating them as needed.

**Node summaries:** Each graph node maintains a rolling summary of its children. This enables System-2 to make traversal decisions without loading individual memories — "Does the `security/` subtree contain anything relevant to my query?" can be answered by reading the node summary.

### 3.5 Embedding Service

Abstraction layer over embedding model access. Handles batching, caching, retry logic, and graceful fallback.

```
Embedding Service
├── Provider: OpenAI text-embedding-3-small (default)
├── Dimensions: 1536 (native output)
├── Input format: "{content_type}: {summary}\n\n{content[:512]}"
├── Batching: up to 100 texts per API call
├── Cache: LRU in-memory cache (256 entries) to avoid re-embedding identical content
├── Retry: exponential backoff (3 attempts, 1s/2s/4s)
├── Fallback: on persistent failure, return None (memory stored without embedding)
└── Extension point: swap provider by implementing EmbeddingProvider interface
```

**Why text-embedding-3-small:**
- 1536 dimensions balance quality with storage efficiency (~6KB per vector).
- Strong performance on retrieval benchmarks (MTEB).
- Supports Matryoshka Representation Learning (MRL) for future dimension reduction.
- Cost-effective for high-volume embedding at ~$0.02 / 1M tokens.

### 3.6 Hook Manager

Manages the behavioral injection points that enforce the RETRIEVE-RESPOND-CAPTURE loop without requiring the AI to be explicitly instructed each turn.

```
Hook Manager
│
├── Session Start Hook
│   ├── Trigger: new session detected (first tool call or explicit init)
│   ├── Action: load top-K user preferences + recent project memories
│   ├── Injection: prepend to system context (~500 token budget)
│   └── Content: "You have memory of this user. Key context: [...]"
│
├── Prompt Submit Hook
│   ├── Trigger: prompt:submit (Amplifier) / UserPromptSubmit (Claude Code)
│   ├── Action: inject compact recall reminder into agent context
│   ├── Budget: ~100 tokens
│   └── Content: "[Memory: consider recalling relevant context for this query]"
│
└── Response Complete Hook
    ├── Trigger: response:complete (Amplifier) / Stop (Claude Code)
    ├── Action: inject capture reminder into agent context
    ├── Budget: ~100 tokens
    └── Content: "[Memory: evaluate if anything from this exchange should be remembered]"
```

**Critical constraint:** Hook injection must be invisible to the user. The injected text appears only in the system/tool context, never in the visible conversation. The AI acts on the injected guidance but does not reference it.

---

## 4. Data Flow

### 4.1 RETRIEVE-RESPOND-CAPTURE Loop

The core behavioral loop operates on every conversational turn:

```
                         ┌─────────────────────────────────────┐
                         │           SESSION START              │
                         │  Hook loads: preferences, recents    │
                         │  Injects ~500 tok of user context    │
                         └──────────────┬──────────────────────┘
                                        │
                                        v
┌───────────────────────────────────────────────────────────────────────────┐
│                              TURN LOOP                                    │
│                                                                           │
│  ┌─────────┐     ┌──────────────────────────┐     ┌────────────────────┐ │
│  │  USER    │     │       RETRIEVE           │     │     RESPOND        │ │
│  │  PROMPT  │────>│                          │────>│                    │ │
│  │          │     │ 1. Hook injects recall   │     │ Agent generates    │ │
│  └─────────┘     │    reminder              │     │ response informed  │ │
│                   │ 2. Agent calls           │     │ by retrieved       │ │
│                   │    memory_recall()       │     │ memories           │ │
│                   │ 3. System-1 + System-2   │     │                    │ │
│                   │    execute               │     │ (memories never    │ │
│                   │ 4. Results injected      │     │  mentioned to user)│ │
│                   │    into context           │     │                    │ │
│                   └──────────────────────────┘     └────────┬───────────┘ │
│                                                             │             │
│                                                             v             │
│                                              ┌──────────────────────────┐ │
│                                              │        CAPTURE           │ │
│                                              │                          │ │
│                                              │ 1. Hook injects capture  │ │
│                                              │    reminder              │ │
│                                              │ 2. Agent evaluates:      │ │
│                                              │    - New fact learned?    │ │
│                                              │    - Preference stated?   │ │
│                                              │    - Decision made?       │ │
│                                              │    - Correction issued?   │ │
│                                              │ 3. Agent calls            │ │
│                                              │    memory_capture() if    │ │
│                                              │    worthwhile             │ │
│                                              │ 4. Cross-reference        │ │
│                                              │    cascade runs           │ │
│                                              └──────────────────────────┘ │
│                                                                           │
└───────────────────────────────────────────────────────────────────────────┘
```

### 4.2 Capture Data Flow (Detail)

```
User says: "Actually, let's use Vitest instead of Jest for this project."

Agent decides to capture:
│
├── memory_capture(
│     content="User decided to switch from Jest to Vitest for testing in this project. 
│              Reasons: faster execution, native ESM support, better DX with HMR.",
│     type="decision",
│     tags=["testing", "vitest", "jest", "migration"],
│     domain="projects/engram-lite/decisions",
│     space="project",
│     importance=7
│   )
│
├── Capture Pipeline executes:
│   ├── Dedup check: no existing memory with >0.95 cosine similarity → proceed
│   ├── Summary: content is <500 words → summary = content (no compression needed)
│   ├── Embed: "decision: User decided to switch from Jest to Vitest..." → [0.012, -0.034, ...]
│   ├── Keywords: ["vitest", "jest", "testing", "esm", "migration", "hmr"] with weights
│   ├── Graph: assign to projects/engram-lite/decisions node
│   └── Write: atomic INSERT across 5 tables
│
├── Cross-reference cascade:
│   ├── Finds existing memory: "Project uses Jest for unit testing" (similarity=0.82)
│   ├── Creates relation: new_memory --supersedes--> jest_memory (strength=0.9)
│   ├── Sets jest_memory.superseded_by = new_memory.id
│   ├── Finds existing memory: "User prefers fast test feedback loops" (similarity=0.71)
│   └── Creates relation: new_memory --supports--> fast_feedback_preference (strength=0.7)
│
└── Returns: { success: true, data: { memory_id: "mem_a3f2..." } }
```

### 4.3 Retrieval Data Flow (Detail)

```
User asks: "What testing setup are we using?"

Agent calls: memory_recall(query="testing setup for this project", domain="projects/")
│
├── Route Selector: specific/factual query → System-1 only
│
├── System-1 executes:
│   ├── Vector KNN (sqlite-vec):
│   │   ├── Embed query → [0.008, -0.029, ...]
│   │   ├── SELECT from memory_vectors ORDER BY distance LIMIT 20
│   │   └── Results: [mem_a3f2 (0.87), mem_b1c4 (0.74), mem_d5e6 (0.71), ...]
│   │
│   ├── BM25 Full-Text (FTS5):
│   │   ├── Query: "testing setup project"
│   │   ├── Match against content + keywords
│   │   └── Results: [mem_a3f2 (12.3), mem_f7g8 (8.1), mem_b1c4 (6.4), ...]
│   │
│   └── RRF Fusion (k=60):
│       ├── mem_a3f2: 1/(60+1) + 1/(60+1) = 0.0328  (top in both)
│       ├── mem_b1c4: 1/(60+2) + 1/(60+3) = 0.0321
│       ├── mem_f7g8: 1/(60+5) + 1/(60+2) = 0.0315
│       └── ... ranked list continues
│
├── Post-processing:
│   ├── mem_b1c4 (old Jest memory) is superseded → deprioritize
│   ├── Apply importance boost: mem_a3f2 importance=7 → +0.003
│   ├── Apply recency boost: mem_a3f2 created today → +0.002
│   └── Attach relations: mem_a3f2 --supersedes--> mem_b1c4, --supports--> mem_e9f0
│
└── Returns: {
      success: true,
      data: {
        memories: [
          { id: "mem_a3f2", summary: "Switched to Vitest...", type: "decision",
            importance: 7, relations: [...] },
          ...
        ],
        route: "system1",
        total: 8
      }
    }
```

---

## 5. Dual-Route Retrieval (Detail)

### 5.1 System-1: Similarity-Based (Fast Path)

System-1 is the default retrieval path. It combines two complementary ranking signals and fuses them into a single ranked list.

#### Vector KNN (Semantic Similarity)

```
Query → Embedding → sqlite-vec KNN search

SELECT m.id, m.summary, m.content_type, v.distance
FROM memory_vectors v
JOIN memories m ON m.id = v.rowid
WHERE m.space = ?
ORDER BY v.distance
LIMIT ?
```

- Uses sqlite-vec's built-in approximate nearest neighbor search.
- Distance metric: cosine distance (1 - cosine_similarity).
- Returns top-K candidates ranked by semantic similarity to the query embedding.

**Strengths:** Handles paraphrasing, synonyms, conceptual similarity. "What test runner do we use?" matches a memory about "Vitest for unit testing" even without exact keyword overlap.

**Weaknesses:** Cannot distinguish structural context. A memory about "testing in production" and "testing in CI" have similar embeddings but very different domain relevance.

#### BM25 Full-Text Search (Lexical Matching)

```
Query → Tokenize → FTS5 search

SELECT m.id, m.summary, m.content_type, rank
FROM memories_fts
JOIN memories m ON m.id = memories_fts.rowid
WHERE memories_fts MATCH ?
AND m.space = ?
ORDER BY rank
LIMIT ?
```

- Uses SQLite FTS5 extension for full-text search.
- Index covers: `content`, `summary`, and `keywords` (from `memory_keywords`).
- Weighted keywords from `memory_keywords` are injected into the FTS index with boosted term frequency.

**Strengths:** Exact keyword matching, handles acronyms and proper nouns precisely. "HIPAA compliance" matches exactly, not approximately.

**Weaknesses:** Fails on paraphrasing. "Health data regulations" would not match "HIPAA compliance" without synonym expansion.

#### Reciprocal Rank Fusion (RRF)

The two ranked lists are combined using RRF, a parameter-light fusion method:

```
RRF_score(d) = Σ  1 / (k + rank_i(d))
               i∈{vec, bm25}

where k = 60 (standard constant that controls rank vs score emphasis)
```

| Document | Vec Rank | BM25 Rank | RRF Score |
|----------|----------|-----------|-----------|
| mem_a3f2 | 1 | 1 | 1/61 + 1/61 = 0.0328 |
| mem_b1c4 | 2 | 3 | 1/62 + 1/63 = 0.0320 |
| mem_f7g8 | 5 | 2 | 1/65 + 1/62 = 0.0315 |
| mem_c3d4 | 3 | 8 | 1/63 + 1/68 = 0.0306 |

**Why RRF over learned fusion:** RRF requires no training data, no parameter tuning, and produces stable results across diverse query types. It is the standard fusion method in production hybrid search systems (used by Elasticsearch, Vespa, and others).

### 5.2 System-2: Hierarchical Graph (Deliberate Path)

System-2 activates for queries that require breadth, coverage, or structural reasoning. It traverses the hierarchical graph to find memories that are structurally related to the query, even if they are not the nearest vector neighbors.

#### When System-2 Fires

The route selector classifies queries using lightweight heuristics:

| Signal | Example | Route |
|--------|---------|-------|
| Specific entity or fact | "What's Ken's email?" | System-1 only |
| Question with singular answer | "Which ORM do we use?" | System-1 only |
| Enumerative ("all", "every", "list") | "List all security decisions" | Hybrid (S1 + S2) |
| Comparative ("how does X compare to") | "How does our auth compare to..." | Hybrid (S1 + S2) |
| Broad domain question | "What do we know about the data layer?" | System-2 primary |
| Exploratory / open-ended | "What patterns have we established?" | System-2 primary |

#### Graph Traversal Algorithm

```
System-2 Retrieval(query, graph):

1. ENTRY POINT RESOLUTION
   - Embed query → find top-3 graph_nodes by label similarity
   - Also match via keyword overlap with node labels
   - Result: entry_nodes = [{node_id, relevance_score}, ...]

2. UPWARD WALK (find structural context)
   - For each entry_node, walk parent_id chain to root
   - Identify the Lowest Common Ancestor (LCA) of all entry nodes
   - LCA becomes the "scope root" for downward traversal

3. DOWNWARD WALK (collect candidate memories)
   - From scope root, BFS through children
   - At each node, read node.summary to decide relevance
   - If relevant: collect all memories linked via memory_graph_nodes
   - If not relevant: prune subtree (skip children)
   - Depth limit: 4 levels from scope root

4. SIBLING EXPANSION (add breadth)
   - For each entry_node, include memories from sibling nodes
   - Siblings share the same parent → structurally adjacent knowledge

5. RANKING
   - Score = node_relevance × memory_importance × recency_decay
   - Return ranked list of memories with graph path metadata
```

**Example:** Query = "What are all the security considerations across our services?"

```
Step 1: Entry nodes → [professional/security (0.92), projects/auth-service/context (0.65)]
Step 2: LCA → root (too broad) → fall back to union of subtrees
Step 3: Walk professional/security/* → collect 12 memories
        Walk projects/auth-service/* → collect 8 memories  
        Walk projects/api-gateway/security-context → collect 3 memories (via node summary match)
Step 4: Sibling expansion: professional/architecture (sibling of security) → 2 relevant memories
Step 5: Rank 25 candidates → return top-K
```

### 5.3 Route Combination

When both routes fire (hybrid mode), their results are merged via a second RRF pass:

```
Final_score(d) = α × RRF_system1(d) + β × RRF_system2(d)

where:
  α = 0.6 (System-1 weight — precision-oriented)
  β = 0.4 (System-2 weight — coverage-oriented)
```

These weights are tunable. For exploratory queries where System-2 is primary, the weights invert (α=0.3, β=0.7).

The final ranked list is post-processed with:
- **Domain filter**: only memories matching the requested domain (if specified)
- **Space filter**: only memories from the requested space
- **Supersession filter**: superseded memories moved to end of list
- **Importance boost**: `final_score += importance / 100`
- **Recency boost**: `final_score += 1 / (1 + days_since_access) × 0.01`
- **Relation attachment**: for each top-K result, attach related memories (via `memory_relations` with strength > 0.5)

---

## 6. Storage Architecture

### 6.1 SQLite + sqlite-vec Rationale

| Requirement | SQLite + sqlite-vec | Alternatives Considered |
|-------------|---------------------|------------------------|
| Local-only, no server | Single file, zero config | Postgres/pgvector requires a running server |
| Atomic transactions | WAL mode, full ACID | File-based (JSON/YAML) has no atomicity |
| Vector similarity search | sqlite-vec provides KNN on virtual tables | FAISS requires separate index management |
| Full-text search | FTS5 built into SQLite | Separate Lucene/Tantivy index adds complexity |
| Portability | Single .db file, cross-platform | Most alternatives require installation |
| Concurrent reads | WAL mode allows concurrent readers | File-based systems need external locking |
| Ecosystem | Python sqlite3 in stdlib, sqlite-vec via pip | Minimal dependency footprint |

### 6.2 Schema

```sql
-- Core memory storage
CREATE TABLE memories (
    id              TEXT PRIMARY KEY,        -- ULID for time-sortable unique IDs
    content         TEXT NOT NULL,           -- Full memory content
    content_type    TEXT NOT NULL,           -- fact|preference|event|skill|entity|relationship|decision
    space           TEXT NOT NULL,           -- 'user' or 'project'
    domain          TEXT,                    -- Taxonomy path: 'professional/architecture'
    summary         TEXT,                    -- Hot tier: 200-500 word summary
    detail          TEXT,                    -- Cold tier: full detail (if content was summarized)
    confidence      REAL DEFAULT 1.0,        -- 0.0-1.0, may decay over time
    importance      INTEGER DEFAULT 5,       -- 1-10, user or agent assigned
    source_session  TEXT,                    -- Session ID that created this memory
    project         TEXT,                    -- Project identifier (directory name or explicit)
    visibility      TEXT DEFAULT 'normal',   -- 'normal', 'archived', 'deleted'
    created_at      TEXT NOT NULL,           -- ISO 8601 timestamp
    modified_at     TEXT NOT NULL,           -- ISO 8601 timestamp
    accessed_at     TEXT,                    -- ISO 8601, updated on retrieval
    access_count    INTEGER DEFAULT 0,       -- Incremented on retrieval
    expires_at      TEXT,                    -- ISO 8601, NULL = never expires
    superseded_by   TEXT REFERENCES memories(id)  -- FK to newer memory, if superseded
);

-- Full-text search index
CREATE VIRTUAL TABLE memories_fts USING fts5(
    content, summary, 
    content='memories', content_rowid='rowid'
);

-- Vector embeddings (sqlite-vec virtual table)
CREATE VIRTUAL TABLE memory_vectors USING vec0(
    embedding FLOAT[1536]
);

-- Tags (many-to-many)
CREATE TABLE memory_tags (
    memory_id   TEXT NOT NULL REFERENCES memories(id),
    tag         TEXT NOT NULL,
    PRIMARY KEY (memory_id, tag)
);

-- Weighted keywords for BM25 boosting
CREATE TABLE memory_keywords (
    memory_id   TEXT NOT NULL REFERENCES memories(id),
    keyword     TEXT NOT NULL,
    weight      REAL DEFAULT 1.0,       -- Higher weight = more important for this memory
    PRIMARY KEY (memory_id, keyword)
);

-- Inter-memory relations
CREATE TABLE memory_relations (
    from_id         TEXT NOT NULL REFERENCES memories(id),
    to_id           TEXT NOT NULL REFERENCES memories(id),
    relation_type   TEXT NOT NULL,       -- relates-to|supports|contradicts|supersedes|...
    strength        REAL DEFAULT 0.5,    -- 0.0-1.0
    created_at      TEXT NOT NULL,
    PRIMARY KEY (from_id, to_id, relation_type)
);

-- Hierarchical graph for System-2 retrieval
CREATE TABLE graph_nodes (
    id          TEXT PRIMARY KEY,
    label       TEXT NOT NULL,           -- Human-readable: "architecture", "security"
    level       INTEGER NOT NULL,        -- Depth: 0=root, 1=category, 2=subcategory
    parent_id   TEXT REFERENCES graph_nodes(id),
    summary     TEXT,                    -- Rolling digest of child memories
    child_count INTEGER DEFAULT 0,       -- Direct children (nodes + linked memories)
    updated_at  TEXT NOT NULL
);

-- Junction: memories ↔ graph nodes
CREATE TABLE memory_graph_nodes (
    memory_id   TEXT NOT NULL REFERENCES memories(id),
    node_id     TEXT NOT NULL REFERENCES graph_nodes(id),
    PRIMARY KEY (memory_id, node_id)
);

-- Schema versioning
CREATE TABLE schema_version (
    version     INTEGER PRIMARY KEY,
    applied_at  TEXT NOT NULL,
    description TEXT
);

-- Indexes for common query patterns
CREATE INDEX idx_memories_space ON memories(space);
CREATE INDEX idx_memories_domain ON memories(domain);
CREATE INDEX idx_memories_content_type ON memories(content_type);
CREATE INDEX idx_memories_project ON memories(project);
CREATE INDEX idx_memories_created_at ON memories(created_at);
CREATE INDEX idx_memories_accessed_at ON memories(accessed_at);
CREATE INDEX idx_memories_visibility ON memories(visibility);
CREATE INDEX idx_memory_tags_tag ON memory_tags(tag);
CREATE INDEX idx_memory_keywords_keyword ON memory_keywords(keyword);
CREATE INDEX idx_memory_relations_to ON memory_relations(to_id);
CREATE INDEX idx_graph_nodes_parent ON graph_nodes(parent_id);
CREATE INDEX idx_graph_nodes_level ON graph_nodes(level);
```

### 6.3 ID Strategy

Memories use **ULIDs** (Universally Unique Lexicographically Sortable Identifiers):
- Time-sortable: prefix encodes millisecond timestamp, so `ORDER BY id` = chronological order.
- Unique: 80 bits of randomness per millisecond — no collisions in practice.
- String-friendly: 26-character Crockford Base32, safe for URLs, filenames, and JSON.
- No coordination: generated locally, no sequence server needed.

### 6.4 Database File Layout

```
~/.engram/
└── memory/
    ├── memory.db           # User-private memory (SQLite + sqlite-vec)
    ├── memory.db-wal       # Write-ahead log
    ├── memory.db-shm       # Shared memory file
    └── config.yaml         # Memory system configuration

<project-root>/
└── .engram/
    └── memory/
        ├── memory.db       # Project-shared memory
        ├── memory.db-wal   # (gitignored)
        ├── memory.db-shm   # (gitignored)
        ├── .gitignore      # Ignores WAL, SHM, and temp files
        └── README.md       # Explains memory format for collaborators
```

---

## 7. Integration Architecture

### 7.1 Amplifier Integration

Amplifier exposes a hook-based extension API that engram-lite plugs into:

```
┌─────────────────────────────────────────────────────────┐
│                    Amplifier Runtime                      │
│                                                          │
│  prompt:submit ──────> engram-lite hook                │
│                        ├── Recall relevant memories       │
│                        ├── Format as compact context      │
│                        └── Inject into prompt context     │
│                                                          │
│  [Agent processes with memory-augmented context]         │
│                                                          │
│  response:complete ──> engram-lite hook                │
│                        ├── Inject capture reminder        │
│                        └── Agent evaluates + captures     │
│                                                          │
│  Tool calls ──────────> engram-lite tool server        │
│                         (MCP or native function calls)   │
└─────────────────────────────────────────────────────────┘
```

**Tool registration:** engram-lite registers its 8 tools with Amplifier's tool registry. The AI agent sees them as native function-calling tools alongside other available tools (file operations, bash, etc.).

**Hook registration:**
```yaml
# .amplifier/hooks.yaml
hooks:
  - event: prompt:submit
    handler: engram-lite:on_prompt_submit
  - event: response:complete
    handler: engram-lite:on_response_complete
```

### 7.2 Claude Code Integration

Claude Code uses a similar but distinct hook mechanism:

```
┌─────────────────────────────────────────────────────────┐
│                    Claude Code Runtime                    │
│                                                          │
│  UserPromptSubmit ────> engram-lite hook               │
│                         (same logic as Amplifier)        │
│                                                          │
│  Stop ────────────────> engram-lite hook               │
│                         (same logic as Amplifier)        │
│                                                          │
│  Tool calls ──────────> engram-lite MCP server         │
│                         (Model Context Protocol)         │
└─────────────────────────────────────────────────────────┘
```

**MCP server:** engram-lite exposes its tools as an MCP (Model Context Protocol) server that Claude Code connects to. The MCP transport is stdio-based (local process), not HTTP.

### 7.3 Shared Core, Platform Adapters

```
engram-lite/
├── core/                   # Platform-independent
│   ├── engine.py           # Main memory engine
│   ├── capture.py          # Capture pipeline
│   ├── retrieval.py        # Dual-route retrieval engine
│   ├── graph.py            # Graph manager
│   ├── storage.py          # SQLite + sqlite-vec abstraction
│   ├── embedding.py        # Embedding service
│   ├── schema.py           # Database schema and migrations
│   └── types.py            # Shared type definitions
│
├── adapters/               # Platform-specific
│   ├── amplifier/          # Amplifier hook + tool adapter
│   │   ├── hooks.py        # prompt:submit, response:complete handlers
│   │   └── tools.py        # Tool registration for Amplifier
│   │
│   └── claude_code/        # Claude Code hook + MCP adapter
│       ├── hooks.py        # UserPromptSubmit, Stop handlers
│       └── mcp_server.py   # MCP tool server (stdio transport)
│
└── cli/                    # Debug/admin CLI
    └── main.py             # memory stats, memory export, memory import
```

---

## 8. Privacy & Security Model

### 8.1 Dual-Space Isolation

The two memory spaces are architecturally isolated — they are separate SQLite database files in separate filesystem locations. No code path crosses the boundary unintentionally.

```
┌─────────────────────────────────┐  ┌─────────────────────────────────┐
│         USER SPACE               │  │       PROJECT SPACE              │
│   ~/.engram/memory.db     │  │  .engram/memory.db       │
│                                  │  │                                  │
│  - Personal preferences          │  │  - Architectural decisions       │
│  - Cross-project knowledge       │  │  - Project-specific patterns     │
│  - Private opinions              │  │  - Team conventions              │
│  - Credentials / sensitive data  │  │  - Shareable context             │
│  - Communication style           │  │  - Domain knowledge              │
│                                  │  │                                  │
│  NEVER committed to git          │  │  Committable to git              │
│  NEVER readable by project tools │  │  Readable by all project users   │
│  NEVER referenced in project ctx │  │  Passes README test              │
└─────────────────────────────────┘  └─────────────────────────────────┘
```

### 8.2 The README Test

Before writing to project space, content must pass this heuristic:

> **Would this content be appropriate in a public README file?**

Content that fails:
- Personally identifiable information (real names in private context, email, phone)
- Credentials, API keys, tokens
- Private opinions about colleagues ("Ken thinks Bob's code is sloppy")
- Medical, financial, or legal information about individuals
- Content marked as confidential

Implementation: The capture pipeline runs a lightweight content classifier before project-space writes. On failure, the memory is either:
1. Redirected to user-space with a note, or
2. Rejected with an error message to the agent (not the user).

### 8.3 Threat Model

| Threat | Mitigation |
|--------|------------|
| Memory content leaks into project git repo | Project DB is committable by design; WAL/SHM are gitignored; README test prevents sensitive content |
| User-space memory accessed by project code | Separate DB files; no cross-space code path; space parameter validated at router level |
| Memory content sent to network | Storage is local-only; only embedding API calls transmit content (summary + first 512 chars) |
| Stale/incorrect memories cause bad AI behavior | Confidence decay, supersession tracking, `memory_forget`, contradiction detection |
| Embedding API receives private content | Embed format is `"{type}: {summary}\n\n{content[:512]}"` — user-space content goes to OpenAI's embedding API; users should be aware of this |
| Disk-level access to memory DB | No encryption at rest; relies on OS file permissions; user's responsibility to protect `~/.engram/` |
| Memory poisoning (malicious memory injection) | Project-space DB could be poisoned via git; `source_session` tracking + confidence scoring mitigates; future: content signing |

### 8.4 Data Lifecycle

```
CAPTURE ──> ACTIVE ──> ACCESSED (access_count++) ──> STALE (confidence decays)
                                                          │
                                    ┌─────────────────────┤
                                    v                     v
                               SUPERSEDED            EXPIRED
                            (superseded_by set)    (expires_at passed)
                                    │                     │
                                    v                     v
                               DEPRIORITIZED         DEPRIORITIZED
                            (still queryable)       (still queryable)
                                    │                     │
                                    v                     v
                              SOFT-DELETED ──────> HARD-DELETED
                           (visibility=deleted)   (all data removed)
```

---

## 9. Extension Points

### 9.1 Embedding Model Swapping

The embedding service uses a provider interface that can be swapped without changing any other component:

```python
class EmbeddingProvider(Protocol):
    """Interface for embedding model providers."""
    
    @property
    def dimensions(self) -> int:
        """Embedding vector dimensions."""
        ...
    
    async def embed(self, texts: list[str]) -> list[list[float]]:
        """Embed a batch of texts. Returns list of vectors."""
        ...
    
    async def embed_query(self, query: str) -> list[float]:
        """Embed a single query. May use different prompt template."""
        ...
```

**Swappable providers:**

| Provider | Dimensions | Notes |
|----------|------------|-------|
| `text-embedding-3-small` (default) | 1536 | Best cost/quality ratio |
| `text-embedding-3-large` | 3072 → 1536 (MRL) | Higher quality, truncated via Matryoshka |
| Local model (e.g., `nomic-embed-text`) | 768 | Zero network dependency, lower quality |
| No-op provider | 0 | BM25-only mode, no vectors |

**Migration path:** When switching embedding models, existing vectors must be re-embedded. The system stores `embedding_model` metadata per-vector to detect mismatches and trigger backfill.

### 9.2 Domain Taxonomy Extension

The domain taxonomy is stored as graph nodes, not as code constants. New domains are created dynamically:

```python
# Capturing a memory with a new domain auto-creates the graph path
memory_capture(
    content="HIPAA requires encryption at rest for PHI",
    domain="professional/domain-specific/healthcare",
    type="fact"
)
# This creates graph nodes: professional/ → domain-specific/ → healthcare/
# if they don't already exist
```

Custom project taxonomies emerge naturally from usage. A healthcare project might develop:
```
projects/patient-portal/
├── decisions/
├── context/
├── patterns/
└── compliance/          # Custom — created by first memory using this domain
    ├── hipaa/
    └── audit/
```

### 9.3 Relation Type Extension

New relation types can be added without schema changes — `relation_type` is a TEXT field, not an enum. The cross-reference cascade uses a pluggable set of relation detectors:

```python
class RelationDetector(Protocol):
    """Detects potential relations between a new memory and existing memories."""
    
    def detect(
        self, 
        new_memory: Memory, 
        candidates: list[Memory]
    ) -> list[RelationCandidate]:
        """Returns potential relations with confidence scores."""
        ...
```

Built-in detectors:
- **SupersessionDetector**: Same domain + overlapping keywords + newer → `supersedes`
- **ContradictionDetector**: Same topic + opposing conclusions → `contradicts`
- **SupportDetector**: Same domain + reinforcing content → `supports`
- **PartOfDetector**: Entity/fact that belongs to a larger entity → `part-of`

Custom detectors can be registered for project-specific relation types.

### 9.4 Storage Backend Extension

While SQLite + sqlite-vec is the only supported backend, the storage layer uses an abstract interface:

```python
class MemoryStore(Protocol):
    """Abstract storage backend for memories."""
    
    async def insert(self, memory: Memory) -> str: ...
    async def get(self, memory_id: str) -> Memory | None: ...
    async def update(self, memory_id: str, **fields) -> bool: ...
    async def delete(self, memory_id: str, hard: bool = False) -> bool: ...
    async def vector_search(self, vector: list[float], limit: int) -> list[ScoredMemory]: ...
    async def text_search(self, query: str, limit: int) -> list[ScoredMemory]: ...
    async def get_relations(self, memory_id: str) -> list[Relation]: ...
```

This enables future exploration of alternative backends (DuckDB, LanceDB, etc.) without rewriting the engine.

### 9.5 Hook Customization

Hook behavior is configurable via `config.yaml`:

```yaml
# ~/.engram/config.yaml
hooks:
  session_start:
    enabled: true
    max_tokens: 500
    include:
      - preferences          # Top user preferences
      - recent_memories: 5   # Last 5 accessed memories
      - active_decisions: 3  # Most recent decisions
  
  prompt_submit:
    enabled: true
    max_tokens: 100
    
  response_complete:
    enabled: true
    max_tokens: 100

retrieval:
  default_route: auto       # auto | system1 | system2 | hybrid
  system1_weight: 0.6
  system2_weight: 0.4
  rrf_k: 60
  
embedding:
  provider: openai
  model: text-embedding-3-small
  dimensions: 1536
  batch_size: 100
  
storage:
  user_path: ~/.engram/
  project_path: .engram/
  wal_mode: true
```

---

## Appendix A: Technology Choices Summary

| Component | Technology | Rationale |
|-----------|-----------|-----------|
| Database | SQLite 3.41+ | Local-only, zero-config, ACID, universal |
| Vector search | sqlite-vec | Native SQLite extension, KNN on virtual tables |
| Full-text search | SQLite FTS5 | Built into SQLite, BM25 scoring |
| Embeddings | OpenAI text-embedding-3-small | Best cost/quality, MRL-compatible, 1536 dims |
| Rank fusion | Reciprocal Rank Fusion (RRF) | Parameter-free, stable, production-proven |
| IDs | ULID | Time-sortable, unique, string-safe |
| Language | Python 3.11+ | Target platform ecosystem (Amplifier, Claude Code) |
| MCP transport | stdio | Local process, no HTTP overhead |
| Configuration | YAML | Human-readable, standard in the ecosystem |

## Appendix B: Performance Budget

```
Total per-turn overhead budget: < 600ms

Breakdown:
├── Hook injection (recall reminder):      ~10ms
├── Retrieval (System-1):                  ~80ms
│   ├── Embedding query:                   ~300ms (API, cached if repeat)
│   ├── Vector KNN:                        ~30ms
│   ├── BM25 search:                       ~20ms
│   └── RRF fusion:                        ~5ms
├── Retrieval (System-2, when active):     ~200ms
│   ├── Graph node matching:               ~30ms
│   ├── Tree traversal:                    ~100ms
│   └── Memory collection + ranking:       ~70ms
├── Result post-processing:                ~20ms
├── Capture (when triggered):              ~400ms
│   ├── Embedding generation:              ~300ms (API)
│   ├── DB write (atomic):                 ~30ms
│   ├── Graph assignment:                  ~20ms
│   └── Cross-reference cascade:           ~50ms
└── Hook injection (capture reminder):     ~10ms

Note: Retrieval and capture don't both occur on every turn.
      Embedding API latency dominates but is parallelizable.
      BM25-only fallback eliminates embedding latency entirely.
```
