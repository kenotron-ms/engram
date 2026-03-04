# SPEC-STORAGE: Canvas Memory Storage Specification

**Version:** 0.1.0
**Status:** Draft
**Last Updated:** 2026-03-03

---

## 1. Overview

Canvas Memory uses SQLite as its sole persistence layer, extended with two virtual-table modules:

| Extension | Purpose | Version |
|-----------|---------|---------|
| **sqlite-vec** | KNN vector similarity search | 0.1.6+ |
| **FTS5** | Full-text keyword search (BM25) | Built-in (SQLite 3.9+) |

SQLite was chosen over client-server databases for three reasons:

1. **Zero infrastructure.** No daemon, no port, no auth. The database is a single file.
2. **Portability.** The file travels with the user (user DB) or the project (project DB).
3. **Concurrency model fits.** AI agent access is single-writer; SQLite's WAL mode handles concurrent reads from multiple tool invocations within a session.

---

## 2. Database Locations (Space Model)

Canvas Memory maintains two independent database files — one per "space":

| Space | Path | Scope | Lifetime |
|-------|------|-------|----------|
| `user` | `~/.engram/engram.db` | Personal knowledge, preferences, cross-project facts | Permanent — survives project deletion |
| `project` | `<project-root>/.engram/engram.db` | Project decisions, architecture, context | Lives with the repo; can be `.gitignore`d or shared |

### 2.1 Space Selection Rules

Every memory has a `space` column (`user` or `project`). The rules:

- **Personal preferences, bio, constraints, people** → `user` space, always.
- **Project decisions, patterns, context** → `project` space if a project DB is available, otherwise `user` space with the `project` column set.
- **Professional knowledge** (architecture patterns, engineering practices) → `user` space by default. Exception: project-specific patterns go to `project` space.

### 2.2 Directory Initialization

On first write to either space, the system:

```python
import os, sqlite3

def init_db(db_path: str) -> sqlite3.Connection:
    os.makedirs(os.path.dirname(db_path), exist_ok=True)
    conn = sqlite3.connect(db_path)
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("PRAGMA foreign_keys=ON")
    conn.execute("PRAGMA busy_timeout=5000")
    conn.enable_load_extension(True)
    conn.load_extension("vec0")  # sqlite-vec
    conn.enable_load_extension(False)
    _apply_schema(conn)
    return conn
```

### 2.3 Cross-Space Queries

Recall queries search **both** databases and merge results. The merge strategy:

1. Query user DB → scored result set A.
2. Query project DB → scored result set B.
3. Union A ∪ B, deduplicate by `memory_id`.
4. Re-rank the merged set by combined score.

Project-space results receive a **1.15× relevance boost** when the active working directory is inside the project root (the agent is "in context").

---

## 2b. MEMORY.md Index Files

Alongside each SQLite database, engram-lite maintains **plain-text Markdown index files** as a first-class storage artifact. These are engram-lite's own files — they are **not** Claude Code's native auto-memory (`CLAUDE.md`).

**Why both a DB and index files?**

1. **Zero-query session start.** MEMORY.md files are injected into the agent context by the engram-lite hook at session init. No DB connection, no embedding lookup, no latency — just `read_file`.
2. **Human-readable and human-editable.** Users and teammates can open, scan, and hand-edit these files in any text editor.
3. **Bridge pattern.** The files bridge the Engram "file-based memory" pattern (simple, portable, git-friendly) with the vector DB (powerful, queryable, semantic). Each MEMORY.md entry is a 1-line summary pointing into deeper DB content.

### 2b.1 File Locations

engram-lite produces three MEMORY.md files, one per scope:

| File | Scope | Committable? | Contents |
|------|-------|-------------|----------|
| `~/.engram/MEMORY.md` | User | Never (personal, cross-project) | `## You` + `## Now` |
| `<project>/.engram/MEMORY.md` | Project | Yes (team-shareable) | `## Project: {name}` + `## Now` |
| `<project>/.engram/MEMORY.local.md` | Local | No (gitignored, machine-specific) | `## Now` only (plus machine-specific overrides) |

**Rationale:** The three-file split mirrors the space model (Section 2) but adds a `local` scope for machine-specific context (paths, env vars, local tool versions) that shouldn't travel with the repo or the user profile.

### 2b.2 File Format

Every MEMORY.md file uses YAML frontmatter followed by three structured sections:

```markdown
---
scope: user              # user | project | local
updated: 2026-03-03T17:44:38Z
managed-by: engram-lite
db: ~/.engram/engram.db
entries: 12
---

# Memory

## You
<!-- Personal preferences, working style, constraints — apply across all projects.
     Added/updated by memory_capture(space='user'). Pruned when entries > 60. -->
- [pref] Prefers inductive writing (conclusion-first) for all output
- [constraint] macOS, Homebrew, VS Code; avoids Docker
- [domain] Healthcare/HIPAA domain familiarity
→ Deep search: memory_recall("user preferences") | memory_recall("working style")

## Project: {project-name}
<!-- Project-specific decisions, patterns, context.
     Added/updated by memory_capture(space='project'). Pruned when entries > 60. -->
- [arch] SQLite-vec + dual-route retrieval (Mnemis System-1/2)
- [decision] MCP for Claude Code tools; orchestrator:complete not response:complete
- [status] Specs complete, implementation pending
→ Deep search: memory_recall("project decisions") | memory_recall("{project-name}")

## Now
<!-- Current session focus — refreshed at session start from recent events in DB. -->
- Working on: MEMORY.md integration into engram-lite specs
- Context: canvas-memory directory
→ Recall anything: memory_recall("{your query}")
```

**Frontmatter fields:**

| Field | Purpose |
|-------|---------|
| `scope` | Which scope this file represents (`user`, `project`, `local`) |
| `updated` | ISO 8601 timestamp of last write by engram-lite |
| `managed-by` | Identifies the managing system (always `engram-lite`) |
| `db` | Path to the backing SQLite database for this scope |
| `entries` | Count of bullet entries across all sections (for quick budget checks) |

### 2b.3 Section Ownership

Not every section appears in every file:

| Section | `~/.engram/MEMORY.md` | `.engram/MEMORY.md` | `.engram/MEMORY.local.md` |
|---------|-----------------------|---------------------|--------------------------|
| `## You` | ✓ (primary home) | — | — |
| `## Project: {name}` | — | ✓ (primary home) | — |
| `## Now` | ✓ | ✓ | ✓ |

The `## Now` section appears in **all three files** and is refreshed at every session start from the last 5 `event`-type memories in the corresponding DB (see Section 2b.6).

### 2b.4 Entry Types

Each bullet in a MEMORY.md section follows the format: `- [type] Statement — optional context`

| Type | Meaning | Typical Section |
|------|---------|-----------------|
| `pref` | User preference or working style | `## You` |
| `constraint` | Environmental or personal constraint | `## You` |
| `domain` | Domain expertise or familiarity | `## You` |
| `skill` | Technical skill or tool proficiency | `## You` |
| `person` | Person/collaborator context | `## You` |
| `arch` | Architecture or design pattern | `## Project` |
| `decision` | Recorded decision with rationale | `## Project` |
| `pattern` | Code pattern or convention | `## Project` |
| `correction` | Corrected prior assumption | Any |
| `status` | Current project or task status | `## Project` / `## Now` |
| `event` | Timestamped occurrence | `## Now` |

### 2b.5 Line Budgets and Pruning

MEMORY.md files are injected into every session, so size discipline is critical.

**Soft limits (enforced by `memory_capture`):**

| Section | Max Entries | ~Lines (with headers) |
|---------|------------|-----------------------|
| `## You` | 60 | ~65 |
| `## Project: {name}` | 60 | ~65 |
| `## Now` | 10 | ~15 |
| **Total per file** | — | **≤ 100** |
| **Combined injection (user + project + local)** | — | **≤ 200** |

**Pruning algorithm** (triggered when a section exceeds its entry limit):

1. Compute `score = confidence × importance_weight` for each entry in the section, where `importance_weight` maps `critical=4, high=3, medium=2, low=1`.
2. Find the entry with the **lowest score**.
3. Remove that line from MEMORY.md. The underlying memory remains in the DB permanently — only its surface-level representation is pruned.
4. Write a pruning event to the `capture_log` table with `trigger = 'prune'`.

### 2b.6 `## Now` Section Refresh Algorithm

At session start, the engram-lite hook refreshes the `## Now` section in all three MEMORY.md files:

```python
def refresh_now_section(conn, memory_md_path: str):
    """Rebuild ## Now from the 5 most recent event-type memories."""
    rows = conn.execute("""
        SELECT summary, content_type, created_at
        FROM memories
        WHERE content_type = 'event'
          AND superseded_by IS NULL
          AND confidence > 0.30
        ORDER BY created_at DESC
        LIMIT 5
    """).fetchall()

    lines = ["## Now"]
    for row in rows:
        lines.append(f"- {row['summary']}")
    lines.append('→ Recall anything: memory_recall("{your query}")')

    _replace_section(memory_md_path, "## Now", lines)
    _update_frontmatter_timestamp(memory_md_path)
```

The `## You` and `## Project` sections are **not** refreshed at session start — they are only modified by `memory_capture` and `memory_forget` calls during a session.

### 2b.7 Relationship to the Database

A memory can exist in the DB only, in MEMORY.md only (not recommended), or in both:

| State | MEMORY.md | DB | How it happens |
|-------|-----------|-----|----------------|
| Normal captured memory | 1-line summary: `[type] Statement` | Full entry with embedding + metadata | `memory_capture()` writes to both |
| Pruned from surface | — | Full entry (unchanged) | Line budget exceeded; pruning removed the MEMORY.md line |
| Soft-forgotten | — | Full entry (`confidence > 0`) | `memory_forget(hard_delete=False)` |
| Hard-forgotten | — | — | `memory_forget(hard_delete=True)` |
| Human-edited entry | Hand-written line | May have no DB backing | User edited the file directly |

**Key invariant:** MEMORY.md is a **lossy projection** of the DB. The DB is the source of truth. If the two diverge, the DB wins on the next `memory_capture` or pruning cycle.

### 2b.8 `.gitignore` Guidance

For project-scoped `.engram/` directories:

```gitignore
# engram-lite: local-scope memory (machine-specific, not for team)
.engram/MEMORY.local.md

# engram-lite: never commit the database or its WAL/SHM files
.engram/engram.db
.engram/engram.db-wal
.engram/engram.db-shm

# engram-lite: commit these (project-shareable):
# .engram/MEMORY.md  ← intentionally NOT ignored
```

The user-scope `~/.engram/` directory is outside any repo and never committed.

---

## 3. Full Annotated Schema

### 3.1 Core Memory Table

```sql
CREATE TABLE memories (
    -- Primary key: UUID v4, generated client-side.
    -- UUIDs avoid coordination between user/project DBs and prevent
    -- collisions if databases are ever merged.
    id              TEXT PRIMARY KEY,

    -- The raw captured content, exactly as extracted from conversation.
    -- Never truncated. This is the source of truth.
    content         TEXT NOT NULL,

    -- Categorical type. Drives display, retrieval weighting, and
    -- auto-tagging behavior. Constrained to known set via CHECK.
    content_type    TEXT NOT NULL DEFAULT 'fact'
        CHECK (content_type IN (
            'fact', 'preference', 'event', 'skill',
            'entity', 'relationship', 'decision'
        )),

    -- Which database this memory belongs in. Denormalized here so
    -- a memory carries its space identity even if exported.
    space           TEXT NOT NULL DEFAULT 'user'
        CHECK (space IN ('user', 'project')),

    -- Hierarchical domain path from the taxonomy.
    -- e.g. 'professional/architecture', 'personal/preferences'
    -- Slash-separated. Always at least one level deep.
    domain          TEXT NOT NULL,

    -- HOT TIER: 1-2 sentence inductive summary.
    -- Written conclusion-first: "User prefers X because Y."
    -- This is what gets loaded into agent context by default.
    -- NULL only during initial capture before summarization completes.
    summary         TEXT,

    -- COLD TIER: Full elaboration, examples, nuance.
    -- Only loaded on explicit expansion or deep-dive queries.
    -- May contain markdown, code blocks, structured data.
    detail          TEXT,

    -- Epistemic confidence. See Section 7 for update formulas.
    confidence      REAL NOT NULL DEFAULT 0.7
        CHECK (confidence >= 0.0 AND confidence <= 1.0),

    -- Retrieval priority. 'critical' memories are always loaded
    -- into the agent's hot context at session start.
    importance      TEXT NOT NULL DEFAULT 'medium'
        CHECK (importance IN ('critical', 'high', 'medium', 'low')),

    -- Session ID that created this memory. Used for provenance
    -- tracking and confidence boosting (cross-session confirmation).
    source_session  TEXT,

    -- Project name, if applicable. Used when a project-scoped memory
    -- is stored in the user DB (no project DB available).
    project         TEXT,

    -- Timestamps: all ISO 8601 with timezone.
    -- created_at: immutable after insert.
    -- modified_at: updated on any content/metadata change.
    -- accessed_at: updated on every retrieval (read).
    created_at      TEXT NOT NULL,
    modified_at     TEXT NOT NULL,
    accessed_at     TEXT,

    -- Retrieval counter. Incremented on every recall hit.
    -- Used for importance inference and decay resistance.
    access_count    INTEGER NOT NULL DEFAULT 0,

    -- Temporal validity. NULL = permanent (no expiry).
    -- See Section 8 for TTL patterns per content type.
    expires_at      TEXT,

    -- Soft supersession pointer. When a memory is updated with
    -- substantially new information, the old version is kept but
    -- marked as superseded. The new memory's ID goes here.
    superseded_by   TEXT REFERENCES memories(id),

    -- Access control. 'private' = user only. 'project' = visible
    -- to project collaborators. 'public' = exportable.
    visibility      TEXT NOT NULL DEFAULT 'private'
        CHECK (visibility IN ('private', 'project', 'public'))
);

-- Indexes for common query patterns
CREATE INDEX idx_memories_space ON memories(space);
CREATE INDEX idx_memories_domain ON memories(domain);
CREATE INDEX idx_memories_content_type ON memories(content_type);
CREATE INDEX idx_memories_importance ON memories(importance);
CREATE INDEX idx_memories_project ON memories(project);
CREATE INDEX idx_memories_confidence ON memories(confidence);
CREATE INDEX idx_memories_created_at ON memories(created_at);
CREATE INDEX idx_memories_expires_at ON memories(expires_at)
    WHERE expires_at IS NOT NULL;
CREATE INDEX idx_memories_superseded ON memories(superseded_by)
    WHERE superseded_by IS NOT NULL;
```

**Design rationale:**

- `TEXT PRIMARY KEY` over `INTEGER PRIMARY KEY`: UUIDs enable cross-database operations without key collisions. The slight index overhead is negligible at expected scale (< 100K memories per DB).
- `content` vs `summary` vs `detail`: Three-tier information model. `summary` is the hot path — fast to scan, fits in agent context windows. `content` is the raw capture. `detail` is the expanded cold-tier explanation.
- `CHECK` constraints: SQLite enforces these on insert/update. Catches application bugs at the database level.
- Partial indexes on `expires_at` and `superseded_by`: Only index the non-NULL rows, which are the minority. Saves space and write overhead.

### 3.2 Tag Index

```sql
-- Tags are categorical labels attached to memories.
-- A memory can have many tags; a tag can apply to many memories.
-- Tags are normalized: lowercase, hyphenated, max 64 chars.
CREATE TABLE memory_tags (
    memory_id   TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    tag         TEXT NOT NULL
        CHECK (length(tag) <= 64),
    PRIMARY KEY (memory_id, tag)
);

-- Reverse index: find all memories with a given tag.
CREATE INDEX idx_tags_tag ON memory_tags(tag);
```

**Design rationale:**

- Composite primary key `(memory_id, tag)` prevents duplicate tags on the same memory and serves as the forward-lookup index.
- `ON DELETE CASCADE`: when a memory is deleted, its tags are automatically cleaned up.

### 3.3 Full-Text Search (FTS5)

```sql
-- FTS5 virtual table for BM25 keyword search.
-- Porter stemmer normalizes word forms: "running" → "run".
-- Unicode61 handles international text properly.
CREATE VIRTUAL TABLE memory_fts USING fts5(
    memory_id UNINDEXED,     -- stored but not indexed (join key only)
    content,                  -- raw content, fully indexed
    summary,                  -- hot-tier summary, fully indexed
    keywords,                 -- extracted keywords, space-separated
    tokenize = 'porter unicode61'
);
```

**Key details:**

| Column | Indexed? | Purpose |
|--------|----------|---------|
| `memory_id` | No (UNINDEXED) | Join back to `memories` table |
| `content` | Yes | Full-text search over raw content |
| `summary` | Yes | Full-text search over summaries |
| `keywords` | Yes | Boosted keyword vocabulary (synonyms, acronyms, plurals) |

**FTS5 BM25 weighting:**

```sql
-- Custom column weights: keywords 3×, summary 2×, content 1×
SELECT memory_id, bm25(memory_fts, 0.0, 1.0, 2.0, 3.0) AS score
FROM memory_fts
WHERE memory_fts MATCH ?
ORDER BY score;  -- bm25() returns negative values; lower = better match
```

The weight ordering matches the column ordering in the CREATE statement: `memory_id` (0.0 — unindexed), `content` (1.0), `summary` (2.0), `keywords` (3.0).

**FTS5 synchronization:** The FTS5 table must be manually kept in sync with the `memories` table. On every insert/update/delete to `memories`:

```python
def sync_fts(conn, memory_id: str, content: str, summary: str, keywords: str):
    """Insert or replace the FTS5 row for a memory."""
    conn.execute("DELETE FROM memory_fts WHERE memory_id = ?", (memory_id,))
    conn.execute(
        "INSERT INTO memory_fts (memory_id, content, summary, keywords) VALUES (?, ?, ?, ?)",
        (memory_id, content, summary or "", keywords)
    )

def delete_fts(conn, memory_id: str):
    conn.execute("DELETE FROM memory_fts WHERE memory_id = ?", (memory_id,))
```

### 3.4 Keyword Weights

```sql
-- Explicit keyword-weight pairs for hybrid search tuning.
-- Separate from FTS5 so we can store per-keyword weights
-- and use them in re-ranking without re-indexing.
CREATE TABLE memory_keywords (
    memory_id   TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    keyword     TEXT NOT NULL,
    weight      REAL NOT NULL DEFAULT 1.0
        CHECK (weight > 0.0 AND weight <= 5.0),
    PRIMARY KEY (memory_id, keyword)
);
CREATE INDEX idx_keywords_keyword ON memory_keywords(keyword);
```

**Weight scale:**

| Weight | Meaning | Example |
|--------|---------|---------|
| 1.0 | Standard relevance | General related term |
| 2.0 | Strong relevance | Synonym or core concept |
| 3.0 | Primary identifier | The exact subject of the memory |
| 4.0-5.0 | Critical match | Unique proper noun, project name |

### 3.5 Knowledge Graph Relations

```sql
-- Explicit typed edges between memories.
-- Forms a knowledge graph overlay on top of the flat memory store.
CREATE TABLE memory_relations (
    from_id       TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    to_id         TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    relation_type TEXT NOT NULL
        CHECK (relation_type IN (
            'relates-to', 'supports', 'contradicts', 'supersedes',
            'exemplifies', 'part-of', 'caused-by', 'decided-in', 'applies-to'
        )),
    -- Edge weight. 0.0 = weakest, 1.0 = strongest.
    -- Used in graph traversal to prune weak connections.
    strength      REAL NOT NULL DEFAULT 0.5
        CHECK (strength >= 0.0 AND strength <= 1.0),
    created_at    TEXT NOT NULL,
    -- Who created this edge. 'auto' = system inferred,
    -- 'user' = explicitly stated, 'ai' = LLM suggested.
    created_by    TEXT
        CHECK (created_by IN ('auto', 'user', 'ai')),
    PRIMARY KEY (from_id, to_id, relation_type)
);

CREATE INDEX idx_relations_to ON memory_relations(to_id);
CREATE INDEX idx_relations_type ON memory_relations(relation_type);
```

**Relation semantics:**

| Type | Direction | Meaning |
|------|-----------|---------|
| `relates-to` | Symmetric | General association |
| `supports` | Directed | from_id provides evidence for to_id |
| `contradicts` | Symmetric | from_id conflicts with to_id |
| `supersedes` | Directed | from_id replaces to_id |
| `exemplifies` | Directed | from_id is an example of to_id |
| `part-of` | Directed | from_id is a component of to_id |
| `caused-by` | Directed | from_id was caused by to_id |
| `decided-in` | Directed | from_id (decision) was made in to_id (event/context) |
| `applies-to` | Directed | from_id (pattern/skill) applies to to_id (project/domain) |

**Symmetric relations:** For `relates-to` and `contradicts`, the application layer inserts **both directions** `(A→B)` and `(B→A)` to simplify queries (no need for `OR` clauses on from/to).

### 3.6 Hierarchical Graph Nodes

```sql
-- Hierarchical taxonomy nodes (Mnemis System-2 inspired).
-- Organizes memories into a navigable tree structure.
-- Each node can hold an LLM-generated summary of its subtree.
CREATE TABLE graph_nodes (
    id          TEXT PRIMARY KEY,    -- UUID v4
    -- Unique path label. Slash-separated hierarchy.
    -- e.g. 'professional/architecture/distributed-systems'
    label       TEXT NOT NULL UNIQUE,
    -- Depth level: 0=root, 1=domain, 2=subdomain, 3=topic, 4=concept
    level       INTEGER NOT NULL
        CHECK (level >= 0 AND level <= 4),
    -- Parent pointer. NULL only for level-0 root nodes.
    parent_id   TEXT REFERENCES graph_nodes(id),
    -- LLM-generated summary of all memories in this subtree.
    -- Regenerated periodically as memories are added/removed.
    summary     TEXT,
    -- Denormalized counts for fast tree rendering.
    child_count  INTEGER NOT NULL DEFAULT 0,
    memory_count INTEGER NOT NULL DEFAULT 0,
    updated_at  TEXT NOT NULL
);

CREATE INDEX idx_graph_nodes_parent ON graph_nodes(parent_id);
CREATE INDEX idx_graph_nodes_level ON graph_nodes(level);
```

**Level semantics:**

| Level | Name | Example Label | Description |
|-------|------|---------------|-------------|
| 0 | Root | `professional` | Top-level domain category |
| 1 | Domain | `professional/architecture` | Primary knowledge area |
| 2 | Subdomain | `professional/architecture/distributed-systems` | Specific discipline |
| 3 | Topic | `professional/architecture/distributed-systems/consensus` | Concrete topic |
| 4 | Concept | `professional/architecture/distributed-systems/consensus/raft` | Individual concept |

### 3.7 Memory-to-Graph-Node Membership

```sql
-- Junction table: which memories belong to which graph nodes.
-- A memory can belong to multiple nodes (e.g. a decision about
-- microservices architecture that also applies to a specific project).
CREATE TABLE memory_graph_nodes (
    memory_id TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
    node_id   TEXT NOT NULL REFERENCES graph_nodes(id) ON DELETE CASCADE,
    PRIMARY KEY (memory_id, node_id)
);

CREATE INDEX idx_mgn_node ON memory_graph_nodes(node_id);
```

### 3.8 Vector Index (sqlite-vec)

```sql
-- sqlite-vec virtual table for KNN similarity search.
-- FLOAT[1536] stores 1536-dimensional float32 vectors.
-- vec0 module provides exact and approximate nearest-neighbor search.
CREATE VIRTUAL TABLE memory_vectors USING vec0(
    memory_id TEXT PRIMARY KEY,
    embedding FLOAT[1536]
);
```

**sqlite-vec integration details:**

| Aspect | Detail |
|--------|--------|
| Module | `vec0` (the primary virtual table type in sqlite-vec 0.1.6+) |
| Vector type | `FLOAT[1536]` — 32-bit IEEE 754 floats, 1536 dimensions |
| Storage | ~6 KB per vector (1536 × 4 bytes) |
| Distance metric | Cosine distance (default for `vec0`) |
| Index type | Exhaustive scan (exact KNN). No ANN index at expected scale. |

**Insertion:**

```python
import struct

def store_embedding(conn, memory_id: str, embedding: list[float]):
    """Store a vector in the vec0 virtual table.
    
    sqlite-vec expects vectors as serialized bytes (little-endian float32)
    or as JSON arrays. We use the binary format for efficiency.
    """
    vec_bytes = struct.pack(f"<{len(embedding)}f", *embedding)
    conn.execute(
        "INSERT INTO memory_vectors (memory_id, embedding) VALUES (?, ?)",
        (memory_id, vec_bytes)
    )

def update_embedding(conn, memory_id: str, embedding: list[float]):
    """Update requires delete + re-insert for vec0 virtual tables."""
    conn.execute("DELETE FROM memory_vectors WHERE memory_id = ?", (memory_id,))
    store_embedding(conn, memory_id, embedding)
```

**KNN Query:**

```sql
-- Find the 20 nearest neighbors to a query vector.
-- ? is the query vector as serialized bytes or JSON array.
SELECT memory_id, distance
FROM memory_vectors
WHERE embedding MATCH ?
  AND k = 20
ORDER BY distance;
```

**Version requirements:**

- sqlite-vec **0.1.6+** is required for `FLOAT[N]` syntax in `vec0` table definitions.
- Earlier versions used `float32[N]` syntax and had different query semantics.
- The extension is loaded at runtime via `conn.load_extension("vec0")`.
- On macOS (ARM): `pip install sqlite-vec` provides a pre-built wheel.
- On Linux: the wheel is available for x86_64 and aarch64.

### 3.9 Capture Provenance Log

```sql
-- Audit trail for memory captures.
-- Records when, why, and in what context each memory was created.
CREATE TABLE capture_log (
    id          TEXT PRIMARY KEY,     -- UUID v4
    memory_id   TEXT REFERENCES memories(id),
    session_id  TEXT,                 -- agent session that triggered capture
    captured_at TEXT NOT NULL,        -- ISO 8601
    -- What triggered the capture:
    --   'auto'     = agent auto-detected memorable content
    --   'explicit' = user said "remember this"
    --   'update'   = existing memory was updated/refined
    trigger     TEXT
        CHECK (trigger IN ('auto', 'explicit', 'update')),
    -- Brief context snippet (max ~200 chars) showing what
    -- conversation context led to the capture.
    raw_context TEXT
);

CREATE INDEX idx_capture_log_memory ON capture_log(memory_id);
CREATE INDEX idx_capture_log_session ON capture_log(session_id);
```

---

## 4. Hot/Cold Tier Model

Canvas Memory uses a two-tier information architecture to balance context window efficiency against information completeness.

### 4.1 Tier Definitions

| Tier | Column | Max Size | Loaded By Default | Purpose |
|------|--------|----------|-------------------|---------|
| **Hot** | `summary` | ~200 chars (1-2 sentences) | Yes | Quick-scan context; fits many memories in a prompt |
| **Cold** | `detail` | Unlimited | No | Full elaboration, examples, code blocks |
| **Raw** | `content` | Unlimited | No | Original captured text, unprocessed |

### 4.2 Summary Construction (Hot Tier)

Summaries are **inductive** — conclusion first, evidence second:

```
# Good (inductive):
"User prefers composition over inheritance in TypeScript because
it enables better testing and avoids deep class hierarchies."

# Bad (deductive):
"In a conversation about TypeScript patterns, the user mentioned
they like composition and don't like inheritance because..."
```

The summary is generated by the LLM at capture time using this pattern:

```
Given the captured content, write a 1-2 sentence summary.
Start with the conclusion or key fact, then briefly explain why.
Write in third person ("User prefers..." not "You prefer...").
```

### 4.3 Detail Construction (Cold Tier)

The `detail` column stores expanded information that doesn't fit in the summary:

- **Facts:** Supporting evidence, caveats, edge cases.
- **Preferences:** Context for when the preference applies, exceptions.
- **Decisions:** Alternatives considered, trade-offs, stakeholders.
- **Skills:** Full syntax examples, common patterns, gotchas.
- **Events:** Timeline, participants, outcomes.

### 4.4 Tier Transition Rules

```
Content captured
    │
    ├─ content (raw) is ALWAYS stored immediately
    │
    ├─ summary (hot) is generated within the same capture call
    │  (async LLM call, but completes before returning memory_id)
    │
    └─ detail (cold) is generated:
       ├─ Immediately, if content is > 500 chars (enough to elaborate)
       ├─ On first access, if content is ≤ 500 chars (lazy generation)
       └─ On update, if the memory is refined with new information
```

### 4.5 When to Load Cold Data

Cold-tier data (`detail`) is fetched when:

1. **Explicit expansion**: The agent requests full details for a specific memory.
2. **Deep-dive query**: The user asks for "more detail about X" or "explain the decision about Y."
3. **Conflict resolution**: When two memories may contradict each other, load full details to compare.
4. **High-confidence operations**: When the agent is about to take an action based on a memory (e.g., applying a pattern), load the detail to verify.

Cold data is **never** loaded in bulk recall operations. Standard recall returns only `(id, summary, confidence, importance, tags)`.

---

## 5. Data Lifecycle

### 5.1 Creation

```python
import uuid
from datetime import datetime, timezone

def create_memory(conn, content: str, content_type: str, domain: str, **kwargs) -> str:
    memory_id = str(uuid.uuid4())
    now = datetime.now(timezone.utc).isoformat()
    
    conn.execute("""
        INSERT INTO memories (
            id, content, content_type, space, domain,
            summary, detail, confidence, importance,
            source_session, project,
            created_at, modified_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    """, (
        memory_id, content, content_type,
        kwargs.get("space", "user"),
        domain,
        kwargs.get("summary"),      # generated async
        kwargs.get("detail"),       # may be None initially
        kwargs.get("confidence", 0.7),
        kwargs.get("importance", "medium"),
        kwargs.get("source_session"),
        kwargs.get("project"),
        now, now
    ))
    
    return memory_id
```

### 5.2 Updates

Updates follow an **append-or-supersede** model:

| Change Type | Action |
|-------------|--------|
| Minor refinement (same fact, better wording) | Update in-place. Bump `modified_at`. |
| Substantial new information | Create new memory. Set `superseded_by` on old one. Add `supersedes` relation. |
| Correction (user says "actually, no...") | Update in-place. Reset `confidence` to 0.60. Bump `modified_at`. |
| Contradiction from different source | Keep both. Add `contradicts` relation. Lower confidence on both by 0.15. |

```python
def update_memory(conn, memory_id: str, **changes):
    now = datetime.now(timezone.utc).isoformat()
    changes["modified_at"] = now
    
    set_clause = ", ".join(f"{k} = ?" for k in changes)
    values = list(changes.values()) + [memory_id]
    
    conn.execute(
        f"UPDATE memories SET {set_clause} WHERE id = ?",
        values
    )
```

### 5.3 Access Tracking

Every recall hit updates the memory's access metadata:

```python
def record_access(conn, memory_id: str):
    now = datetime.now(timezone.utc).isoformat()
    conn.execute("""
        UPDATE memories 
        SET accessed_at = ?, access_count = access_count + 1
        WHERE id = ?
    """, (now, memory_id))
```

Access data serves three purposes:

1. **Importance inference:** Frequently accessed memories may be candidates for promotion to `high` or `critical` importance.
2. **Decay resistance:** Accessed memories resist confidence decay (see Section 7).
3. **Cleanup candidates:** Memories never accessed after creation are candidates for pruning.

### 5.4 Deletion Model

Canvas Memory uses **soft delete** as the default, with hard delete available for explicit user requests.

**Soft delete:** Set `superseded_by` to a sentinel value and `confidence` to 0.0:

```python
SOFT_DELETED = "00000000-0000-0000-0000-000000000000"

def soft_delete(conn, memory_id: str):
    now = datetime.now(timezone.utc).isoformat()
    conn.execute("""
        UPDATE memories 
        SET superseded_by = ?, confidence = 0.0, modified_at = ?
        WHERE id = ?
    """, (SOFT_DELETED, now, memory_id))
```

Soft-deleted memories are excluded from recall queries by the filter `WHERE superseded_by IS NULL AND confidence > 0.0`.

**Hard delete:** Removes the memory and all associated data (cascading):

```python
def hard_delete(conn, memory_id: str):
    # CASCADE handles: memory_tags, memory_keywords, memory_relations,
    #                  memory_graph_nodes, capture_log
    # Must manually clean: memory_fts, memory_vectors
    conn.execute("DELETE FROM memory_fts WHERE memory_id = ?", (memory_id,))
    conn.execute("DELETE FROM memory_vectors WHERE memory_id = ?", (memory_id,))
    conn.execute("DELETE FROM memories WHERE id = ?", (memory_id,))
```

**Periodic cleanup:** A maintenance task runs hard-delete on soft-deleted memories older than 30 days:

```sql
DELETE FROM memories 
WHERE superseded_by = '00000000-0000-0000-0000-000000000000'
  AND modified_at < datetime('now', '-30 days');
```

---

## 6. SQLite Configuration

### 6.1 Required PRAGMAs

```sql
-- Write-Ahead Logging: enables concurrent reads during writes.
PRAGMA journal_mode = WAL;

-- Enforce foreign key constraints (OFF by default in SQLite).
PRAGMA foreign_keys = ON;

-- Wait up to 5 seconds for a write lock before returning SQLITE_BUSY.
PRAGMA busy_timeout = 5000;

-- Synchronous NORMAL: safe with WAL mode, better write performance
-- than FULL. Data is durable against application crashes. Only a
-- power loss during a WAL checkpoint could cause data loss.
PRAGMA synchronous = NORMAL;

-- 64 MB page cache. At 4096 bytes/page, this is 16384 pages.
-- Keeps hot data in memory for fast repeated queries.
PRAGMA cache_size = -65536;

-- 64 MB memory-mapped I/O. Improves read performance for the
-- vector table, which is read-heavy and benefits from mmap.
PRAGMA mmap_size = 67108864;
```

### 6.2 Connection Setup Sequence

```python
def connect(db_path: str) -> sqlite3.Connection:
    conn = sqlite3.connect(db_path)
    conn.row_factory = sqlite3.Row
    
    # PRAGMAs (must be set per-connection)
    conn.execute("PRAGMA journal_mode=WAL")
    conn.execute("PRAGMA foreign_keys=ON")
    conn.execute("PRAGMA busy_timeout=5000")
    conn.execute("PRAGMA synchronous=NORMAL")
    conn.execute("PRAGMA cache_size=-65536")
    conn.execute("PRAGMA mmap_size=67108864")
    
    # Load sqlite-vec extension
    conn.enable_load_extension(True)
    conn.load_extension("vec0")
    conn.enable_load_extension(False)
    
    return conn
```

---

## 7. Confidence Model

Confidence is a `REAL` value in `[0.0, 1.0]` representing how certain the system is that a memory is accurate and current.

### 7.1 Initial Values

| Trigger | Initial Confidence |
|---------|-------------------|
| Auto-captured (inferred from conversation) | 0.70 |
| Explicitly stated by user ("remember that I...") | 0.80 |
| Imported from verified source | 0.90 |

### 7.2 Confidence Update Formulas

**Cross-session confirmation:**

When a memory is confirmed (content re-stated or consistent with new information) in a different session:

```python
def confirm_memory(conn, memory_id: str, current_session: str):
    """Boost confidence when memory is confirmed in a new session."""
    row = conn.execute(
        "SELECT confidence, source_session FROM memories WHERE id = ?",
        (memory_id,)
    ).fetchone()
    
    if row["source_session"] == current_session:
        return  # Same session confirmation doesn't count
    
    old_conf = row["confidence"]
    
    # Asymptotic boost: approaches 1.0 but never reaches it.
    # Each confirmation adds diminishing returns.
    # First confirmation: 0.70 → 0.85
    # Second confirmation: 0.85 → 0.925
    # Third confirmation: 0.925 → 0.9625
    new_conf = old_conf + (1.0 - old_conf) * 0.5
    new_conf = min(new_conf, 0.99)  # Hard cap at 0.99
    
    update_memory(conn, memory_id, confidence=new_conf)
```

**User correction:**

```python
def correct_memory(conn, memory_id: str, new_content: str):
    """User explicitly corrects a memory. Reset confidence to re-learning state."""
    update_memory(conn, memory_id, 
                  content=new_content, 
                  confidence=0.60)
```

**Contradiction:**

```python
def mark_contradiction(conn, memory_a: str, memory_b: str):
    """Two memories contradict each other. Lower both."""
    for mid in (memory_a, memory_b):
        row = conn.execute(
            "SELECT confidence FROM memories WHERE id = ?", (mid,)
        ).fetchone()
        new_conf = max(row["confidence"] - 0.15, 0.10)
        update_memory(conn, mid, confidence=new_conf)
    
    # Add bidirectional contradiction edges
    now = datetime.now(timezone.utc).isoformat()
    for from_id, to_id in [(memory_a, memory_b), (memory_b, memory_a)]:
        conn.execute("""
            INSERT OR IGNORE INTO memory_relations 
            (from_id, to_id, relation_type, strength, created_at, created_by)
            VALUES (?, ?, 'contradicts', 0.8, ?, 'auto')
        """, (from_id, to_id, now))
```

### 7.3 Time-Based Confidence Decay

Memories that are not accessed decay slowly over time. The decay formula:

```python
import math
from datetime import datetime, timezone

def decayed_confidence(
    base_confidence: float,
    last_accessed: str | None,
    access_count: int,
    importance: str,
    now: datetime | None = None
) -> float:
    """Calculate effective confidence with time decay.
    
    Decay formula:
        effective = base × decay_factor
        decay_factor = exp(-λ × days_since_access)
    
    Where λ (decay rate) is modulated by:
        - importance (critical memories don't decay)
        - access_count (frequently accessed memories decay slower)
    """
    if importance == "critical":
        return base_confidence  # Critical memories never decay
    
    if last_accessed is None:
        return base_confidence  # Never accessed = no decay yet
    
    now = now or datetime.now(timezone.utc)
    last = datetime.fromisoformat(last_accessed)
    days = (now - last).total_seconds() / 86400
    
    # Base decay rates per importance level (per day)
    lambda_base = {
        "high": 0.0005,    # Half-life ≈ 1386 days (3.8 years)
        "medium": 0.002,   # Half-life ≈ 347 days (11.4 months)
        "low": 0.005,      # Half-life ≈ 139 days (4.6 months)
    }[importance]
    
    # Access count dampening: more accesses = slower decay
    # Each access halves the effective decay rate (up to 8× reduction)
    access_dampening = 1.0 / min(2 ** access_count, 8)
    lambda_effective = lambda_base * access_dampening
    
    decay_factor = math.exp(-lambda_effective * days)
    
    # Floor: confidence never decays below 0.10
    return max(base_confidence * decay_factor, 0.10)
```

**Decay examples (medium importance, no accesses):**

| Days Since Access | Decay Factor | Effective Confidence (base 0.85) |
|-------------------|-------------|----------------------------------|
| 0 | 1.000 | 0.850 |
| 30 | 0.942 | 0.800 |
| 90 | 0.835 | 0.710 |
| 180 | 0.697 | 0.593 |
| 365 | 0.482 | 0.410 |

### 7.4 Confidence Thresholds

| Threshold | Meaning | Action |
|-----------|---------|--------|
| ≥ 0.90 | High confidence | Present as established fact |
| 0.70–0.89 | Moderate confidence | Present normally |
| 0.50–0.69 | Low confidence | Present with hedging ("I believe...", "Previously you mentioned...") |
| 0.30–0.49 | Very low confidence | Only surface if directly relevant; flag as uncertain |
| < 0.30 | Near-expired | Candidate for cleanup; excluded from recall by default |

---

## 8. Temporal Validity

The `expires_at` column enables automatic expiration of time-sensitive memories.

### 8.1 TTL Patterns by Content Type

| Content Type | Default TTL | Rationale |
|-------------|------------|-----------|
| `fact` | `NULL` (permanent) | Facts persist until contradicted |
| `preference` | `NULL` (permanent) | Preferences persist until changed |
| `event` | `NULL` (permanent) | Historical events are permanent |
| `skill` | `NULL` (permanent) | Skills persist; may need version updates |
| `entity` | `NULL` (permanent) | Entity knowledge is long-lived |
| `relationship` | `NULL` (permanent) | Relationship knowledge is long-lived |
| `decision` | 365 days | Decisions may need revisiting annually |

### 8.2 Context-Based TTL Overrides

Some content should expire based on context:

```python
# Temporal patterns that trigger TTL assignment
TTL_PATTERNS = {
    "sprint": 14,           # Sprint-scoped decisions: 2 weeks
    "quarter": 90,          # Quarterly goals: 90 days
    "version": 180,         # Version-specific info: 6 months
    "current": 90,          # "Currently using X": 90 days
    "temporary": 30,        # Explicitly temporary: 30 days
    "workaround": 90,       # Workarounds: 90 days
    "deadline": None,       # Calculated from the deadline date
}
```

### 8.3 Expiration Processing

Expired memories are not immediately deleted. The cleanup process:

```sql
-- Mark expired memories as low-confidence (soft expiration)
UPDATE memories 
SET confidence = CASE 
        WHEN confidence > 0.30 THEN 0.25 
        ELSE confidence 
    END,
    importance = 'low'
WHERE expires_at IS NOT NULL 
  AND expires_at < datetime('now')
  AND superseded_by IS NULL;
```

A background maintenance task (or on-connect check) handles this. Expired memories remain searchable at low confidence — the user can still find them if they search specifically, but they won't surface in general recall.

---

## 9. Schema Migration Strategy

### 9.1 Version Tracking

Schema version is tracked using SQLite's built-in `user_version` pragma:

```python
def get_schema_version(conn) -> int:
    return conn.execute("PRAGMA user_version").fetchone()[0]

def set_schema_version(conn, version: int):
    conn.execute(f"PRAGMA user_version = {version}")
```

### 9.2 Migration Table

Migrations are stored as Python functions, keyed by version number:

```python
MIGRATIONS = {
    1: migrate_v0_to_v1,  # Initial schema
    2: migrate_v1_to_v2,  # Add graph_nodes
    3: migrate_v2_to_v3,  # Add capture_log
    # ...
}

def apply_migrations(conn):
    current = get_schema_version(conn)
    target = max(MIGRATIONS.keys())
    
    if current >= target:
        return
    
    for version in range(current + 1, target + 1):
        if version in MIGRATIONS:
            MIGRATIONS[version](conn)
            set_schema_version(conn, version)
    
    conn.commit()
```

### 9.3 Migration Rules

1. **Additive only:** Migrations should add columns, tables, and indexes. Never drop columns in production.
2. **Backwards-compatible defaults:** New columns must have `DEFAULT` values so old code doesn't break.
3. **Atomic:** Each migration runs in a single transaction.
4. **Idempotent checks:** Use `CREATE TABLE IF NOT EXISTS` and `CREATE INDEX IF NOT EXISTS`.
5. **Virtual table recreation:** FTS5 and vec0 tables cannot be `ALTER`ed. To change them, drop and recreate, then repopulate.

### 9.4 Virtual Table Migration

Migrating `memory_vectors` or `memory_fts` requires special handling since virtual tables don't support `ALTER TABLE`:

```python
def migrate_vectors_to_new_dimension(conn, old_dim: int, new_dim: int):
    """Example: migrating from 768 to 1536 dimensions."""
    # 1. Rename old table
    conn.execute("ALTER TABLE memory_vectors RENAME TO memory_vectors_old")
    
    # 2. Create new table with new dimension
    conn.execute(f"""
        CREATE VIRTUAL TABLE memory_vectors USING vec0(
            memory_id TEXT PRIMARY KEY,
            embedding FLOAT[{new_dim}]
        )
    """)
    
    # 3. Re-embed all memories (expensive, but unavoidable)
    rows = conn.execute("SELECT memory_id FROM memory_vectors_old").fetchall()
    # ... re-embed each memory and insert into new table ...
    
    # 4. Drop old table
    conn.execute("DROP TABLE memory_vectors_old")
```

---

## 10. Backup Considerations

### 10.1 Online Backup

SQLite's online backup API allows copying a database while it's in use:

```python
import sqlite3

def backup_database(source_path: str, backup_path: str):
    """Create a consistent backup of the database."""
    source = sqlite3.connect(source_path)
    dest = sqlite3.connect(backup_path)
    source.backup(dest)
    dest.close()
    source.close()
```

### 10.2 Backup Strategy

| Trigger | Action |
|---------|--------|
| Every 100 memory operations | Automatic backup to `~/.engram/backups/` |
| On schema migration | Pre-migration backup (mandatory) |
| User-initiated | `engram-lite backup` CLI command |
| On model change (re-embedding) | Pre-embedding backup |

### 10.3 Backup Retention

```
~/.engram/backups/
  canvas_20260303_164300.db     # timestamped backups
  canvas_20260302_120000.db
  canvas_pre_migrate_v3.db     # pre-migration snapshots
```

Retention policy:
- Keep daily backups for 7 days.
- Keep weekly backups for 4 weeks.
- Keep pre-migration backups permanently.

### 10.4 Export/Import

For portability, memories can be exported to JSON:

```python
def export_memories(conn, output_path: str):
    """Export all memories as a JSON file (without vectors)."""
    rows = conn.execute("SELECT * FROM memories").fetchall()
    # ... serialize to JSON ...

def import_memories(conn, input_path: str):
    """Import memories from JSON. Re-embeds all content."""
    # ... deserialize from JSON ...
    # ... insert into memories table ...
    # ... re-embed each memory (vectors are not portable across models) ...
```

Vectors are **never** exported — they are model-specific and must be regenerated on import.

---

## 11. Performance Characteristics

### 11.1 Expected Scale

| Metric | Expected Range |
|--------|---------------|
| Memories per user DB | 100 – 50,000 |
| Memories per project DB | 50 – 5,000 |
| Average memory size (content) | 200 – 2,000 chars |
| Vector table size | ~6 KB/row × N memories |
| Total DB size (10K memories) | ~80 MB |
| Total DB size (50K memories) | ~400 MB |

### 11.2 Query Performance Targets

All benchmarks assume a database with 10,000 memories, SQLite WAL mode, 64 MB page cache, running on Apple M-series or equivalent:

| Operation | Target | Method |
|-----------|--------|--------|
| KNN vector search (k=20) | < 10 ms | `vec0` exhaustive scan over 10K vectors |
| FTS5 BM25 search | < 5 ms | FTS5 inverted index lookup |
| Hybrid search (KNN + BM25 + re-rank) | < 50 ms | Parallel queries, merge, re-rank |
| Single memory insert (with embedding) | < 300 ms | Dominated by embedding API latency |
| Single memory insert (without embedding) | < 1 ms | Pure SQLite insert |
| Tag filter query | < 2 ms | Index scan on `memory_tags.tag` |
| Graph traversal (2 hops) | < 5 ms | Index scan on `memory_relations` |
| Hot context load (critical memories) | < 20 ms | Index scan on `importance = 'critical'` |

### 11.3 Scaling Considerations

At 50K+ memories, the exhaustive KNN scan in `vec0` may exceed 50 ms. Mitigation strategies:

1. **Pre-filter by domain/space:** Reduce the candidate set before KNN search.
2. **Partitioned vector tables:** Split vectors by space or domain into separate `vec0` tables.
3. **Quantization:** Use `INT8[1536]` instead of `FLOAT[1536]` for 4× storage reduction and faster scans (supported in sqlite-vec 0.1.6+).
4. **Dimensionality reduction:** Use the 512-dim variant of text-embedding-3-small via OpenAI's `dimensions` parameter.

### 11.4 Write Amplification

Each memory capture triggers writes to multiple tables:

| Table | Writes per capture |
|-------|-------------------|
| `memories` | 1 INSERT |
| `memory_tags` | 3–8 INSERTs |
| `memory_keywords` | 5–15 INSERTs |
| `memory_fts` | 1 INSERT |
| `memory_vectors` | 1 INSERT |
| `memory_graph_nodes` | 1–3 INSERTs |
| `memory_relations` | 0–4 INSERTs |
| `capture_log` | 1 INSERT |

Total: ~15–35 row writes per memory capture. At SQLite's typical throughput of 50,000+ writes/second (WAL mode), this is negligible. The bottleneck is always the embedding API call.

---

## 12. Transaction Boundaries

### 12.1 Capture Transaction

A memory capture is atomic — all-or-nothing:

```python
def capture_memory(conn, content: str, metadata: dict, embedding: list[float]) -> str:
    """Full capture pipeline, atomic via transaction."""
    with conn:  # implicit BEGIN / COMMIT
        memory_id = create_memory(conn, content, **metadata)
        store_tags(conn, memory_id, metadata["tags"])
        store_keywords(conn, memory_id, metadata["keywords"])
        sync_fts(conn, memory_id, content, metadata.get("summary"), 
                 " ".join(k for k, _ in metadata["keywords"]))
        store_embedding(conn, memory_id, embedding)
        assign_graph_nodes(conn, memory_id, metadata["domain"])
        create_relations(conn, memory_id, metadata.get("relations", []))
        log_capture(conn, memory_id, metadata.get("session_id"), "auto", content[:200])
    return memory_id
```

### 12.2 Read Transactions

Recall queries use SQLite's implicit read transactions. In WAL mode, reads never block writes and vice versa. No explicit transaction management is needed for reads.

---

## Appendix A: Complete Schema DDL

For a copy-pasteable complete schema, concatenate all `CREATE` statements from Sections 3.1–3.9 above. The schema is applied via the migration system (Section 9) starting from `user_version = 0` (empty database) to `user_version = 1` (full schema).

## Appendix B: sqlite-vec Loading

```python
import platform
import sqlite3

def load_sqlite_vec(conn: sqlite3.Connection):
    """Load the sqlite-vec extension, handling platform differences."""
    conn.enable_load_extension(True)
    try:
        # Try the standard module name first
        conn.load_extension("vec0")
    except OSError:
        # Fallback: try the full shared library name
        system = platform.system()
        if system == "Darwin":
            conn.load_extension("vec0.dylib")
        elif system == "Linux":
            conn.load_extension("vec0.so")
        elif system == "Windows":
            conn.load_extension("vec0.dll")
        else:
            raise RuntimeError(f"Unsupported platform: {system}")
    finally:
        conn.enable_load_extension(False)
```
