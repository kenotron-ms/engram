# engram-lite: Product Requirements Document

**Version:** 0.1.0
**Date:** 2026-03-03
**Status:** Draft
**Authors:** Ken (design lead)

---

## 1. Executive Summary

AI coding assistants lose all context the moment a session ends, forcing users to re-explain preferences, decisions, and project history every time they start a new conversation. **engram-lite** is a SQLite-vec based persistent memory system that gives AI agents (Claude Code and Amplifier) durable, searchable, cross-session recall — so the AI remembers who you are, what you've decided, and how you work. It operates silently within the agent's behavioral loop, never surfacing memory operations to the user, making the AI feel genuinely continuous rather than stateless.

---

## 2. Problem Statement

Today's AI coding agents are **stateless by default**. Each session begins as a blank slate. This creates compounding friction:

- **Repetition tax**: Users re-explain architectural decisions, coding preferences, team conventions, and project context session after session. A developer who has told their AI assistant "I prefer composition over inheritance" twenty times eventually stops correcting it.
- **Lost institutional knowledge**: Decisions made three months ago — why we chose SQLite over Postgres, why the auth module uses that specific pattern — evaporate when the session closes. The AI cannot learn from its own history with a user.
- **No progressive relationship**: Human collaborators build shared understanding over time. Without memory, AI agents are perpetual strangers. They cannot anticipate, cannot build on prior work, and cannot adapt to a user's evolving expertise.
- **Context window as a bottleneck**: Even within a single session, relevant past knowledge competes with new content for limited context window space. There is no mechanism to compress, rank, or selectively surface the right information at the right time.

**engram-lite** solves these problems by providing a structured, dual-route persistent memory that integrates directly into the agent's tool-calling interface, with automatic behavioral hooks that enforce a silent RETRIEVE-RESPOND-CAPTURE loop.

---

## 3. Goals & Non-Goals

### Goals

| ID | Goal | Rationale |
|----|------|-----------|
| G1 | Provide persistent, structured memory across AI sessions | Core value proposition — continuity |
| G2 | Operate silently within the agent loop | Memory ops should be invisible to the user; the AI just "knows" |
| G3 | Support dual-route retrieval (System-1 + System-2) | Fast vector similarity for exact recall; hierarchical graph for broad reasoning |
| G4 | Enforce dual-space privacy model | User-private memory (`~/.engram/`) must never leak into project-shared memory (`.engram/`) |
| G5 | Integrate with both Amplifier and Claude Code | Two target platforms, one memory system |
| G6 | Expose memory as real function-calling tools | Tools-first design — AI decides when and how to use memory via native tool calls |
| G7 | Support hot/cold storage tiers | Summaries for fast context injection; full detail available on-demand |
| G8 | Enable cross-reference and relationship graphs | Memories are not isolated — they connect, support, contradict, and supersede each other |

### Non-Goals

| ID | Non-Goal | Rationale |
|----|----------|-----------|
| NG1 | Real-time multi-user collaboration on shared memory | Single-user system; project space is shared via version control, not live sync |
| NG2 | Cloud-hosted memory service | All storage is local (SQLite files on disk); no network dependency |
| NG3 | General-purpose knowledge base or RAG system | Purpose-built for AI agent memory, not document retrieval |
| NG4 | Training or fine-tuning models on memory content | Memory is retrieved at inference time, never used for model training |
| NG5 | Replacing the AI's built-in knowledge | Memory augments, not replaces, the model's parametric knowledge |
| NG6 | Supporting non-Claude AI models | Designed for Claude Code and Amplifier; other models are not a target |

---

## 4. User Stories

### US-1: Remembering Preferences Across Sessions
> **As** a developer using Claude Code daily,
> **I want** the AI to remember my coding style preferences (e.g., "I prefer functional patterns," "always use early returns"),
> **so that** I don't have to re-state them at the start of every session.

**Acceptance:** After stating a preference once, the AI applies it in subsequent sessions without being reminded. The user never sees a "Loading your preferences..." message.

### US-2: Recalling Architectural Decisions
> **As** a tech lead working on a long-running project,
> **I want** the AI to recall past architectural decisions and their rationale,
> **so that** when I'm making new decisions, it can reference why we made prior choices and flag potential contradictions.

**Acceptance:** When the user asks "Why did we choose X?", the AI retrieves the decision memory and its linked rationale. When a new proposal contradicts a prior decision, the AI proactively surfaces the conflict.

### US-3: Project Context on Reconnect
> **As** a developer returning to a project after a week away,
> **I want** the AI to have context about where I left off — what I was working on, what was blocked, what I decided,
> **so that** I can resume productive work without a 15-minute context dump.

**Acceptance:** On session start, the agent silently loads recent project memories and can answer "What was I working on last?" accurately.

### US-4: Private vs Shared Knowledge
> **As** a developer who works on both personal and team projects,
> **I want** my personal preferences (communication style, workflow habits) to persist across all projects in my private space,
> **while** project-specific decisions stay in the project directory and can be shared with teammates via git,
> **so that** I have continuity without privacy leakage.

**Acceptance:** `memory_capture(space="user")` writes to `~/.engram/`; `memory_capture(space="project")` writes to `.engram/`. Project memories committed to git are usable by teammates. Personal preferences never appear in project memory.

### US-5: Correcting Stale Knowledge
> **As** a user whose tech stack has evolved,
> **I want** to update or supersede outdated memories (e.g., "we migrated from Express to Fastify"),
> **so that** the AI doesn't act on obsolete information.

**Acceptance:** `memory_update` modifies existing memories. `memory_relate(from_id, to_id, "supersedes")` marks old decisions as superseded. Superseded memories are deprioritized in recall but remain accessible for historical context.

### US-6: Broad Reasoning Over Many Memories
> **As** a developer asking a high-level question like "What are all the security considerations across our services?",
> **I want** the AI to traverse its memory graph hierarchically — not just find the top-5 vector matches,
> **so that** I get comprehensive coverage, not just the most similar snippets.

**Acceptance:** System-2 retrieval activates for broad queries, traversing the hierarchical graph through `security/` domain nodes and aggregating related memories. The response covers memories that vector search alone would miss.

### US-7: Silent Operation
> **As** a user,
> **I want** memory operations to be completely invisible in the conversation,
> **so that** I experience a naturally knowledgeable assistant rather than a system that announces "Searching memory..." or "Saving to memory..."

**Acceptance:** No memory-related status messages, acknowledgments, or explanations appear in the AI's responses. Memory tool calls happen but their execution is never narrated.

### US-8: Forgetting on Request
> **As** a user who shared sensitive information in a session,
> **I want** to ask the AI to forget specific memories,
> **so that** I maintain control over what persists.

**Acceptance:** `memory_forget(memory_id, reason)` soft-deletes the memory. The user can also request deletion by description ("forget what I told you about the AWS credentials"), and the AI resolves the intent to the correct memory ID.

### US-9: Cross-Referencing Related Knowledge
> **As** a developer making a design decision,
> **I want** the AI to surface related memories — prior decisions in the same domain, relevant preferences, applicable patterns,
> **so that** my decisions are informed by the full context of what the AI knows.

**Acceptance:** After capturing a new decision memory, the system creates `relates-to`, `supports`, or `contradicts` relations to existing memories. On recall, related memories are returned alongside the primary result.

### US-10: Temporal Awareness
> **As** a user,
> **I want** the AI to understand that memories have temporal relevance — recent events matter more, old preferences may have changed,
> **so that** recall is weighted appropriately and stale information is flagged.

**Acceptance:** Recall scoring incorporates `accessed_at`, `created_at`, and `access_count`. Memories with `expires_at` in the past are deprioritized. The agent proactively marks memories as potentially stale when contradictory new information arrives.

---

## 5. Functional Requirements

### 5.1 Storage

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-S1 | Store memories in SQLite databases using sqlite-vec extension for vector operations | P0 |
| FR-S2 | Maintain two separate database files: user-private (`~/.engram/memory.db`) and project-local (`.engram/memory.db`) | P0 |
| FR-S3 | Each memory record stores: content, content_type, space, domain, summary (hot tier), detail (cold tier), confidence, importance, temporal metadata (`created_at`, `modified_at`, `accessed_at`, `access_count`), source session ID, project identifier, visibility | P0 |
| FR-S4 | Support content types: `fact`, `preference`, `event`, `skill`, `entity`, `relationship`, `decision` | P0 |
| FR-S5 | Store embedding vectors in a sqlite-vec virtual table (`memory_vectors`) with FLOAT[1536] dimensions | P0 |
| FR-S6 | Maintain a hierarchical graph structure via `graph_nodes` table with `id`, `label`, `level`, `parent_id`, `summary`, `child_count` | P0 |
| FR-S7 | Link memories to graph nodes via `memory_graph_nodes` junction table | P0 |
| FR-S8 | Support tags (many-to-many via `memory_tags`) and weighted keywords (via `memory_keywords`) per memory | P0 |
| FR-S9 | Support inter-memory relations via `memory_relations` with typed edges (`relates-to`, `supports`, `contradicts`, `supersedes`, `exemplifies`, `part-of`, `caused-by`, `decided-in`, `applies-to`) and strength scores | P0 |
| FR-S10 | Implement hot/cold storage: `summary` field (200-500 words) for fast context injection; `detail` field for full content loaded on-demand | P1 |
| FR-S11 | Support `superseded_by` field for memory versioning — superseded memories remain queryable but are deprioritized | P1 |
| FR-S12 | Support `expires_at` for time-bound memories (e.g., "deploy freeze ends Friday") | P2 |

### 5.2 Retrieval

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-R1 | **System-1 (fast path)**: Hybrid retrieval combining vector KNN similarity search and BM25 full-text search, fused via Reciprocal Rank Fusion (RRF) | P0 |
| FR-R2 | **System-2 (deliberate path)**: Hierarchical graph traversal starting from query-matched graph nodes, walking the semantic hierarchy to collect structurally relevant memories | P0 |
| FR-R3 | Automatic route selection: System-1 alone for specific/exact queries; System-1 + System-2 combined for broad/global queries | P0 |
| FR-R4 | Manual route override via `memory_recall(route="system1" | "system2" | "hybrid")` | P1 |
| FR-R5 | Apply domain filtering to narrow retrieval scope (e.g., `domain="professional/architecture"`) | P0 |
| FR-R6 | Apply temporal weighting: more recently accessed and created memories score higher, all else being equal | P1 |
| FR-R7 | Apply importance weighting: user-specified importance (1-10) factors into final ranking | P1 |
| FR-R8 | Deprioritize superseded memories in results (include only if explicitly requested or no better alternative exists) | P1 |
| FR-R9 | Return related memories alongside primary results when relation strength exceeds threshold | P1 |
| FR-R10 | `memory_search` provides a simplified retrieval interface (always System-1, fewer parameters) for quick lookups | P0 |

### 5.3 Capture

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-C1 | `memory_capture` accepts content, optional type/tags/domain/space/importance and returns a memory ID | P0 |
| FR-C2 | On capture, automatically generate a summary (hot tier) from content if not provided | P0 |
| FR-C3 | On capture, generate an embedding vector from the formatted string `"{content_type}: {summary}\n\n{content[:512]}"` using `text-embedding-3-small` | P0 |
| FR-C4 | On capture, extract keywords from content and store with weights in `memory_keywords` | P1 |
| FR-C5 | On capture, assign the memory to appropriate graph nodes based on domain taxonomy; create graph nodes if needed | P1 |
| FR-C6 | After capture, run a cross-reference pass: identify related existing memories, create relations, flag contradictions, detect potential supersessions | P1 |
| FR-C7 | Deduplicate on capture: if a near-identical memory exists (cosine similarity > 0.95), merge or update rather than create a duplicate | P1 |
| FR-C8 | Default space selection: if within a git project directory, default to `project`; otherwise default to `user` | P2 |

### 5.4 Tools

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-T1 | Expose `memory_capture(content, type?, tags?, domain?, space?, importance?)` as a callable tool | P0 |
| FR-T2 | Expose `memory_recall(query, route?, limit?, domain?, filters?)` as a callable tool | P0 |
| FR-T3 | Expose `memory_search(query, domain?, limit?, filters?)` as a callable tool | P0 |
| FR-T4 | Expose `memory_update(memory_id, content?, tags?, importance?, confidence?)` as a callable tool | P0 |
| FR-T5 | Expose `memory_relate(from_id, to_id, relation_type, strength?)` as a callable tool | P1 |
| FR-T6 | Expose `memory_forget(memory_id, reason?)` as a callable tool | P0 |
| FR-T7 | Expose `memory_graph_explore(query?, node_id?)` as a callable tool for browsing the hierarchical graph | P1 |
| FR-T8 | Expose `memory_stats(space?)` as a callable tool for introspection | P2 |
| FR-T9 | All tools return structured JSON responses parseable by the AI agent | P0 |
| FR-T10 | All tools validate inputs and return clear error messages on invalid parameters | P0 |

### 5.5 Hooks & Context Injection

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-H1 | On session start, automatically load and inject: top user preferences, recent project memories, active decisions | P0 |
| FR-H2 | On `prompt:submit` (Amplifier) and `UserPromptSubmit` (Claude Code), inject a compact recall reminder into the agent context | P0 |
| FR-H3 | On `response:complete` (Amplifier) and `Stop` (Claude Code), inject a capture reminder prompting the agent to evaluate what should be remembered | P0 |
| FR-H4 | Injected context must be compact — budget of ~500 tokens for session-start injection, ~100 tokens for per-turn reminders | P1 |
| FR-H5 | Hook injection must add <100ms latency to the user's perceived response time | P0 |
| FR-H6 | Hooks must be configurable (enable/disable per space, adjust injection budget) | P2 |

### 5.6 Privacy & Security

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-P1 | User-private space (`~/.engram/`) must never be readable by, or leak into, project-space operations | P0 |
| FR-P2 | Apply the "README test" for project-space writes: content must be safe to appear in a public README without harm | P0 |
| FR-P3 | `memory_capture(space="project")` must reject or flag content that fails the README test (PII, credentials, private opinions about people) | P1 |
| FR-P4 | Memory content is stored locally on disk only — no network transmission of memory data | P0 |
| FR-P5 | `memory_forget` performs soft-delete (marks as deleted, excluded from retrieval) with an option for hard-delete that removes all data including embeddings | P1 |
| FR-P6 | Memory databases are user-readable files — no encryption at rest (user's OS-level permissions apply) | P0 |
| FR-P7 | Project-space `.engram/` directory should include a `.gitignore` for embedding cache files and a README explaining the memory format | P2 |

### 5.7 Management & Maintenance

| ID | Requirement | Priority |
|----|-------------|----------|
| FR-M1 | Track `access_count` and `accessed_at` on every retrieval hit to enable usage-based ranking | P1 |
| FR-M2 | Support confidence scoring (0.0-1.0) that degrades over time for unaccessed memories | P2 |
| FR-M3 | Provide `memory_stats` reporting: total memories by space/type/domain, storage size, graph node count, most/least accessed | P2 |
| FR-M4 | Support bulk operations: export memories as JSON, import from JSON | P2 |
| FR-M5 | Database migration support: schema versioning with forward-compatible migrations | P1 |
| FR-M6 | Graph maintenance: periodically recompute `child_count`, prune orphaned graph nodes, update node summaries | P2 |

---

## 6. Non-Functional Requirements

### 6.1 Performance

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-P1 | System-1 recall latency (vector + BM25 + RRF) | < 100ms for databases up to 50,000 memories |
| NFR-P2 | System-2 recall latency (graph traversal + aggregation) | < 300ms for graphs up to 10,000 nodes |
| NFR-P3 | Combined dual-route recall latency | < 500ms total |
| NFR-P4 | Memory capture latency (excluding embedding generation) | < 50ms |
| NFR-P5 | Embedding generation latency (API call to OpenAI) | < 500ms per memory (network-dependent) |
| NFR-P6 | Hook injection overhead per turn | < 100ms |
| NFR-P7 | Session-start context load time | < 1s |

### 6.2 Storage & Scale

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-S1 | Support up to 100,000 memories per database (user or project) | Validated at this scale |
| NFR-S2 | Database file size for 10,000 memories with embeddings | < 200MB |
| NFR-S3 | Memory record size (content + summary + metadata, excluding embedding) | Avg < 4KB |
| NFR-S4 | Embedding storage per memory (1536 x float32) | ~6KB |

### 6.3 Compatibility

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-C1 | Platform support | macOS (arm64, x86_64), Linux (x86_64, arm64) |
| NFR-C2 | Python version | 3.11+ |
| NFR-C3 | SQLite version | 3.41+ (for sqlite-vec compatibility) |
| NFR-C4 | Amplifier integration | Hook API v1 (`prompt:submit`, `response:complete`) |
| NFR-C5 | Claude Code integration | Hook API (`UserPromptSubmit`, `Stop`) |

### 6.4 Reliability

| ID | Requirement | Target |
|----|-------------|--------|
| NFR-R1 | Memory capture must be atomic — no partial writes on failure | WAL mode, transactions |
| NFR-R2 | Database corruption recovery: detect and rebuild from WAL | Automatic on startup |
| NFR-R3 | Graceful degradation: if embedding API is unavailable, capture memory without embedding and backfill later | Required |
| NFR-R4 | If memory system fails entirely, agent continues normally without memory (no user-visible errors) | Required |

---

## 7. Success Metrics

| Metric | Definition | Target | Measurement |
|--------|-----------|--------|-------------|
| **Preference retention rate** | % of user-stated preferences correctly applied in subsequent sessions without re-prompting | > 90% after 5+ sessions | Manual evaluation over test scenarios |
| **Decision recall accuracy** | When asked "Why did we decide X?", % of responses that correctly cite the stored decision and rationale | > 85% | Automated test suite with known decisions |
| **Retrieval relevance (System-1)** | Precision@5 for specific/factual queries against stored memories | > 0.80 | Benchmark suite with labeled queries |
| **Retrieval coverage (System-2)** | Recall for broad queries (e.g., "all security considerations") | > 0.75 | Benchmark suite with known memory sets |
| **Capture-to-recall round-trip** | % of captured memories that are successfully retrievable via natural language query | > 95% | Automated test: capture N memories, query each |
| **Silent operation compliance** | % of sessions where zero memory-related messages appear in user-facing output | 100% | Automated output scanning |
| **Latency budget compliance** | % of memory operations completing within NFR targets | > 99% | Performance monitoring / profiling |
| **Deduplication effectiveness** | % reduction in near-duplicate memories after dedup is active | > 90% | Compare with/without dedup on same workload |

---

## 8. Out of Scope

The following are explicitly **not** part of engram-lite v0.1:

| Item | Rationale |
|------|-----------|
| **Multi-user real-time sync** | Project-space sharing happens via git commits, not live collaboration. Multi-user concurrency is not a goal. |
| **Cloud storage or backup** | All data is local. Cloud sync (iCloud, Dropbox, S3) is the user's responsibility at the filesystem level. |
| **Web UI or dashboard** | No GUI. Memory is accessed exclusively through AI agent tool calls and CLI utilities for debugging. |
| **Memory import from other AI tools** | No importers for ChatGPT history, Copilot context, or other systems. JSON import/export covers manual migration. |
| **Automatic summarization of conversation transcripts** | The agent decides what to capture. We do not automatically ingest entire conversations. |
| **Embedding model fine-tuning** | We use `text-embedding-3-small` as-is. Custom embedding models are an extension point but not a v0.1 deliverable. |
| **Access control or permissions** | File-system permissions are the security boundary. No application-level RBAC. |
| **Memory sharing across different users** | Project-space memories are shared via git. There is no user-to-user memory sharing mechanism. |
| **Proactive memory surfacing** | The AI retrieves memory in response to queries or hook triggers. It does not proactively interrupt the user with "I remembered something relevant." (Hook-injected context at session start is the closest analog.) |

---

## Appendix A: Glossary

| Term | Definition |
|------|-----------|
| **Hot tier** | The `summary` field of a memory — compact (200-500 words), suitable for context injection |
| **Cold tier** | The `detail` field — full content, loaded on-demand when the summary is insufficient |
| **System-1** | Fast, similarity-based retrieval route (vector KNN + BM25, fused via RRF) |
| **System-2** | Deliberate, structure-based retrieval route (hierarchical graph traversal) |
| **RRF** | Reciprocal Rank Fusion — method for combining ranked lists from multiple retrieval sources |
| **User space** | Private memory at `~/.engram/` — persists across all projects, never shared |
| **Project space** | Shared memory at `.engram/` — project-specific, committable to git |
| **Domain taxonomy** | Hierarchical classification of memories (personal/, professional/, projects/, people/) |
| **README test** | Privacy heuristic: "Would this content be safe in a public README?" — gate for project-space writes |
| **Hook** | Event-driven injection point in the agent platform (fires on prompt submit, response complete, etc.) |
| **Cross-reference cascade** | Post-capture process that identifies related memories and creates relation edges |

---

## Appendix B: Tool Interface Summary

```
memory_capture(content, type?, tags?, domain?, space?, importance?) -> memory_id
memory_recall(query, route?, limit?, domain?, filters?)            -> memories[]
memory_search(query, domain?, limit?, filters?)                    -> memories[]
memory_update(memory_id, content?, tags?, importance?, confidence?) -> success
memory_relate(from_id, to_id, relation_type, strength?)            -> success
memory_forget(memory_id, reason?)                                  -> success
memory_graph_explore(query?, node_id?)                             -> graph_nodes[]
memory_stats(space?)                                               -> stats{}
```
