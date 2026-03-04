# SPEC-PROTOCOLS: Behavioral Protocols Specification

**System:** engram-lite
**Version:** 0.1.0
**Status:** Draft
**Last Updated:** 2026-03-03

---

## Table of Contents

1. [Overview](#1-overview)
2. [The RETRIEVE-RESPOND-CAPTURE Loop](#2-the-retrieve-respond-capture-loop)
3. [Session Initialization Protocol](#3-session-initialization-protocol)
4. [Pre-Response Recall Protocol](#4-pre-response-recall-protocol)
5. [Post-Response Capture Protocol](#5-post-response-capture-protocol)
6. [Cross-Reference Cascade](#6-cross-reference-cascade)
7. [Privacy Routing Protocol](#7-privacy-routing-protocol)
8. [Confidence Update Rules](#8-confidence-update-rules)
9. [Anti-Patterns](#9-anti-patterns)
10. [Inductive Writing Rule](#10-inductive-writing-rule)
11. [Failure Recovery](#11-failure-recovery)
12. [Convergence Property](#12-convergence-property)
13. [Anti-Rationalization Checklist](#13-anti-rationalization-checklist)
14. [LLM Prompt Templates](#14-llm-prompt-templates)

---

## 1. Overview

This document defines the behavioral contracts that an AI agent MUST follow when the engram-lite system is active. These protocols govern how the agent retrieves prior knowledge, responds to the user, and captures new information for future sessions.

The fundamental invariant is: **the user must never perceive the memory system operating.** All memory operations are silent, ambient, and subordinate to the primary task of responding to the user.

### Design Principles

| Principle | Description |
|---|---|
| **Silent operation** | Memory operations are never announced, narrated, or referenced in user-facing output. |
| **Conclusion-first** | Every stored memory begins with its most important claim. |
| **Convergence** | The system gets measurably better with each session. |
| **Graceful degradation** | Memory failures never block or degrade the user-facing response. |
| **Privacy by default** | Information routes to the most private space unless explicitly shareable. |

### Terminology

| Term | Definition |
|---|---|
| **Memory** | A single unit of stored knowledge with content, metadata, embedding, and graph relationships. |
| **Hot context** | Memories loaded at session start (critical/high importance, relevant domains). |
| **Domain** | A hierarchical topic path (e.g., `project/backend/auth`). |
| **Confidence** | A float [0.0, 1.0] representing trust in a memory's accuracy. |
| **Importance** | An enum (`critical`, `high`, `medium`, `low`) representing operational relevance. |
| **Recall** | The act of retrieving memories relevant to a query. |
| **Capture** | The act of storing new knowledge as a memory. |
| **Cascade** | The post-capture procedure that cross-references new memories against existing ones. |

---

## 2. The RETRIEVE-RESPOND-CAPTURE Loop

The core behavioral loop runs on every conversational turn. It consists of three phases that wrap the agent's normal response generation.

```
┌─────────────────────────────────────────────────────────┐
│                    SESSION START                         │
│  1. Load hot context from DB                            │
│  2. Inject hot context as <system-reminder>             │
│  3. Agent receives session-start behavioral protocol    │
└───────────────────────┬─────────────────────────────────┘
                        │
                        ▼
┌─────────────────────────────────────────────────────────┐
│                  USER SENDS PROMPT                       │
│                                                         │
│  ┌─── RETRIEVE ───────────────────────────────────┐     │
│  │ 1. Receive per-prompt recall reminder           │     │
│  │ 2. Evaluate: does this prompt relate to prior   │     │
│  │    context?                                     │     │
│  │ 3. If YES → call memory_recall(query)           │     │
│  │ 4. Incorporate recalled memories into reasoning │     │
│  └─────────────────────────────────────────────────┘     │
│                                                         │
│  ┌─── RESPOND ────────────────────────────────────┐     │
│  │ 5. Generate response using all available        │     │
│  │    context (hot + recalled + conversation)      │     │
│  │ 6. Deliver response to user                     │     │
│  └─────────────────────────────────────────────────┘     │
│                                                         │
│  ┌─── CAPTURE ────────────────────────────────────┐     │
│  │ 7. Receive post-response capture reminder       │     │
│  │ 8. Run capture decision tree                    │     │
│  │ 9. If capturing → call memory_capture()         │     │
│  │ 10. If captured → run cross-reference cascade   │     │
│  └─────────────────────────────────────────────────┘     │
│                                                         │
│              ◄─── loop back to next prompt ───►         │
└─────────────────────────────────────────────────────────┘
```

### Phase Timing

| Phase | Trigger | Blocking? | Timeout |
|---|---|---|---|
| Hot context load | `session:start` / `SessionStart` | Yes (before first response) | 5s |
| Per-prompt recall reminder | `prompt:submit` / `UserPromptSubmit` | No (injected as context) | 2s |
| Recall tool call | Agent decision | No (agent chooses when) | 10s |
| Response | Normal agent flow | Yes (user-facing) | N/A |
| Capture reminder | `response:complete` / `Stop` | No (injected as context) | 2s |
| Capture tool call | Agent decision | No (silent, after response) | 10s |
| Cross-reference cascade | After capture | No (silent, background) | 30s |

### Critical Rule: Response Priority

The user-facing response MUST NEVER be delayed or degraded by memory operations. If a recall takes too long, the agent responds without it. If a capture fails, the agent does not retry synchronously. Memory operations are best-effort ambient processes.

---

## 3. Session Initialization Protocol

### Trigger

The session initialization fires once at the start of every new conversation session:
- **Amplifier:** `session:start` hook
- **Claude Code:** `SessionStart` hook

### Procedure

```
PROCEDURE session_init():
    1. Determine active project context:
       - workspace_path = current working directory
       - project_id = hash(workspace_path) or configured project identifier

    2. Query hot context:
       - SELECT memories WHERE importance IN ('critical', 'high')
         AND (space = 'user' OR space = project_id)
         ORDER BY importance DESC, confidence DESC, last_accessed DESC
         LIMIT hot_context_limit (default: 20)

    3. Group by domain:
       - Cluster retrieved memories by their domain path
       - For each domain, pick the top N by importance

    4. Format injection:
       - Build <system-reminder> XML with:
         a. Memory count and domain summary
         b. Critical memories in full
         c. High-importance memories as summaries
         d. Behavioral protocol instructions

    5. Touch accessed timestamps:
       - UPDATE last_accessed = NOW() for all loaded memories
       - INCREMENT access_count for all loaded memories

    6. Return formatted injection string
```

### Injection Template

```xml
<system-reminder source="engram-lite">
MEMORY SYSTEM ACTIVE. Loaded context:

[{memory_count} memories across {domain_list}]

## Critical Context
{critical_memories_formatted}

## High-Importance Context
{high_memories_summarized}

PROTOCOL:
- Use memory_recall(query) before responding to queries that may relate to prior context
- Use memory_capture(content) after learning new information
- Never announce memory operations to the user
</system-reminder>
```

### Memory Formatting Rules

Each memory in the hot context is formatted as:

```
[{domain}] {summary} (confidence: {confidence}, last: {relative_time})
```

Example:

```
[project/auth] User prefers JWT with RS256 for all APIs. Refresh tokens stored in httpOnly cookies. (confidence: 0.92, last: 2 days ago)
[user/preferences] Prefers concise explanations with code examples over prose. Dislikes verbose step-by-step walkthroughs. (confidence: 0.85, last: 1 week ago)
```

### Edge Cases

| Condition | Behavior |
|---|---|
| Database does not exist | Skip injection. Return empty string. Log warning. |
| Database is locked | Wait up to 1s, then skip injection. Log warning. |
| Zero hot context memories | Inject minimal protocol-only reminder (no memory content). |
| Embedding provider unavailable | Load hot context using keyword/metadata only (skip vector similarity). |
| Hot context exceeds token budget | Truncate from lowest-importance memories upward. Budget default: 2000 tokens. |

---

## 4. Pre-Response Recall Protocol

### Trigger

Fires on every user prompt submission:
- **Amplifier:** `prompt:submit` hook
- **Claude Code:** `UserPromptSubmit` hook

### Injection

```xml
<system-reminder source="engram-lite">
Memory check: Does "{first_50_chars_of_prompt}" relate to prior context?
If yes -> memory_recall(query) before responding.
</system-reminder>
```

### Agent Decision Logic

The agent evaluates the prompt against its current context (including hot context memories) and decides whether to call `memory_recall()`. The decision follows this matrix:

| Prompt Type | Recall? | Rationale |
|---|---|---|
| Direct question about prior work | **YES** | Likely has stored context |
| "Remember when..." or reference to past | **YES** | Explicit recall request |
| Continuation of multi-session topic | **YES** | Context may span sessions |
| New topic unrelated to any domain | **NO** | No relevant memories exist |
| Simple command ("fix the typo on line 5") | **NO** | Immediate action, no history needed |
| Coding task in known project | **MAYBE** | Recall if project patterns/preferences stored |
| Debugging a recurring issue | **YES** | Prior debugging context is high-value |

### Recall Tool Usage

```python
# The agent constructs a query from the user's prompt
result = memory_recall(
    query="user's question or topic summary",
    limit=5,           # default: 5 results
    threshold=0.3,     # minimum relevance score
    domains=None,      # optional: restrict to specific domains
    content_types=None  # optional: restrict to specific types
)
```

### Incorporating Recalled Memories

When `memory_recall()` returns results, the agent:

1. **Reads** the returned memories silently.
2. **Integrates** relevant facts into its reasoning without citation.
3. **Does NOT** say "According to my memory..." or "I recall that..."
4. **Uses** the information naturally, as if it were part of its own knowledge.
5. **Updates** its response based on known preferences and patterns.

### Recall Failure Handling

| Failure | Behavior |
|---|---|
| Tool call times out | Proceed without recalled context. |
| Tool returns error | Proceed without recalled context. Do not mention the error. |
| Tool returns zero results | Proceed normally. This is expected for new topics. |
| Tool returns low-confidence results | Use with appropriate hedging in internal reasoning (not in user output). |

---

## 5. Post-Response Capture Protocol

### Trigger

Fires after every agent response:
- **Amplifier:** `response:complete` hook
- **Claude Code:** `Stop` hook

### Injection

```xml
<system-reminder source="engram-lite">
CAPTURE CHECK (silent):
1. Did I learn something new? -> memory_capture()
2. Did I make an error that was corrected? -> memory_capture(content_type='fact', importance='high')
3. Did I observe a preference/pattern? -> memory_capture(content_type='preference')
Execute silently. Do not announce.
</system-reminder>
```

### The Capture Decision Tree

The agent runs this decision tree internally after every response:

```
CAPTURE DECISION TREE:

Q1: Did the user provide NEW INFORMATION that I did not already have in memory?
    Examples: project requirements, technical decisions, personal preferences,
              architectural choices, team conventions, environment details
    → YES: proceed to CAPTURE with content_type inferred from content
    → NO: continue to Q2

Q2: Did the user CORRECT something I said or believed?
    Examples: "Actually, we use PostgreSQL not MySQL", "No, the deadline is Friday"
    → YES: proceed to CAPTURE with:
           content_type = 'fact'
           importance = 'high'
           Also: find and update the incorrect memory (lower its confidence)
    → NO: continue to Q3

Q3: Did I observe a PATTERN or PREFERENCE for the 2nd+ time?
    Examples: user consistently prefers short answers, always uses TypeScript,
              always asks for tests, rejects certain patterns
    → YES (first time): proceed to CAPTURE with:
           content_type = 'preference'
           importance = 'medium'
           confidence = 0.6 (provisional — first observation)
    → YES (2nd+ time): proceed to UPDATE existing preference memory:
           confidence += 0.15 (capped at 0.95)
           add tag 'recurring'
    → NO: continue to Q4

Q4: Did I learn something about the PROJECT STRUCTURE or CODEBASE?
    Examples: discovered a new module, understood an architecture pattern,
              found a convention in the code
    → YES: proceed to CAPTURE with:
           content_type = 'fact' or 'architecture'
           importance = 'medium'
           domain = infer from project context
    → NO: continue to Q5

Q5: Did this exchange involve a SIGNIFICANT DECISION or RESOLUTION?
    Examples: chose between two libraries, resolved a design debate,
              established a new convention
    → YES: proceed to CAPTURE with:
           content_type = 'decision'
           importance = 'high'
           Include: the options considered, the choice made, the reasoning
    → NO: DO NOT CAPTURE. End decision tree.
```

### Capture Tool Usage

```python
memory_capture(
    content="Conclusion-first summary of the knowledge. Supporting details follow.",
    content_type="fact",       # fact | preference | decision | procedure | architecture | debug_insight
    importance="medium",       # critical | high | medium | low
    domain="project/backend",  # hierarchical domain path
    tags=["auth", "jwt"],      # searchable tags
    keywords=["authentication", "token"],  # extracted keywords for BM25
    confidence=0.8,            # initial confidence score
    space="project",           # user | project
    source_context="brief description of what prompted this capture"
)
```

### What To Capture (Positive Examples)

| Content Type | Example |
|---|---|
| `fact` | "Project uses PostgreSQL 15 with pgvector extension for embeddings." |
| `preference` | "User prefers functional React components over class components." |
| `decision` | "Team chose Remix over Next.js for the admin dashboard. Reasoning: better nested routing, loader pattern fits their data model." |
| `procedure` | "Deployment requires: 1) run migrations, 2) build assets, 3) restart workers, 4) warm caches." |
| `architecture` | "The auth system uses a dual-token pattern: short-lived JWT access tokens (15min) + long-lived refresh tokens in httpOnly cookies." |
| `debug_insight` | "Flaky test in CI caused by race condition in WebSocket connection setup. Fix: add explicit wait for connection ACK before sending." |

### What NOT To Capture (Negative Examples)

| Do Not Capture | Why |
|---|---|
| Verbatim code listings | Too granular, changes frequently, bloats storage. |
| File contents | Same as above. Capture the *insight about* the file, not the file. |
| User's exact phrasing | Synthesize into knowledge. Don't quote. |
| Transient conversation context | "User asked me to fix line 42" — not durable knowledge. |
| Already-known information | Don't duplicate hot context. |
| Obvious/trivial facts | "Python uses indentation for blocks" — universally known. |

---

## 6. Cross-Reference Cascade

After every successful `memory_capture()`, the agent MUST run the cross-reference cascade. This procedure maintains graph consistency and improves retrieval quality over time.

### Procedure

```
PROCEDURE cross_reference_cascade(new_memory):

    STEP 1: FIND RELATED MEMORIES
    ─────────────────────────────
    related = memory_recall(
        query=new_memory.content,
        limit=10,
        threshold=0.25  # lower threshold to catch weak connections
    )
    Exclude the new_memory itself from results.

    STEP 2: DETECT PATTERNS
    ───────────────────────
    FOR each related_memory in related:
        IF related_memory.domain == new_memory.domain
           AND related_memory.content_type == new_memory.content_type
           AND semantic_similarity > 0.7:

            # Same topic, same type, high similarity = pattern
            IF 'recurring' NOT IN related_memory.tags:
                memory_update(related_memory.id, add_tags=['recurring'])
                memory_update(related_memory.id,
                    confidence=min(related_memory.confidence + 0.1, 0.95))

            # Also boost the new memory
            memory_update(new_memory.id,
                confidence=min(new_memory.confidence + 0.05, 0.95))

    STEP 3: DETECT CONTRADICTIONS
    ─────────────────────────────
    FOR each related_memory in related:
        IF related_memory.domain == new_memory.domain
           AND content_appears_contradictory(new_memory, related_memory):

            # Create contradiction relation
            memory_relate(
                source=new_memory.id,
                target=related_memory.id,
                relation_type='contradicts',
                metadata={'detected_at': now()}
            )

            # Lower confidence on the older memory
            memory_update(related_memory.id,
                confidence=max(related_memory.confidence - 0.2, 0.1))

            # Boost the newer memory (recency bias for contradictions)
            memory_update(new_memory.id,
                confidence=min(new_memory.confidence + 0.1, 0.95))

    STEP 4: UPDATE SUPERSEDED MEMORIES
    ──────────────────────────────────
    FOR each related_memory in related:
        IF new_memory explicitly replaces related_memory
           (same topic, updated information, user-confirmed correction):

            # Create supersedes relation
            memory_relate(
                source=new_memory.id,
                target=related_memory.id,
                relation_type='supersedes',
                metadata={'reason': 'updated information'}
            )

            # Soft-delete the old memory
            memory_forget(related_memory.id, reason='superseded by ' + new_memory.id)

    STEP 5: UPDATE GRAPH CONNECTIONS
    ────────────────────────────────
    FOR each related_memory in related WHERE similarity > 0.4:
        IF no existing relation between new_memory and related_memory:
            # Infer relation type from content analysis
            relation = infer_relation(new_memory, related_memory)
            memory_relate(
                source=new_memory.id,
                target=related_memory.id,
                relation_type=relation.type,  # relates_to | supports | depends_on | part_of
                weight=relation.confidence
            )

    STEP 6: UPDATE DOMAIN NODE SUMMARIES (if graph nodes exist)
    ──────────────────────────────────────────────────────────
    IF new_memory.domain has a graph node:
        all_memories_in_domain = memory_search(domain=new_memory.domain)
        updated_summary = generate_node_summary(
            node_label=new_memory.domain,
            memories=all_memories_in_domain
        )
        update_graph_node(new_memory.domain, summary=updated_summary)
```

### Contradiction Detection Heuristics

Two memories are potentially contradictory when:

1. **Same domain, opposite claims:** "We use MySQL" vs "We use PostgreSQL"
2. **Same subject, different values:** "Deploy on Friday" vs "Deploy on Monday"
3. **Negation of prior memory:** "We DO use microservices" vs "We DON'T use microservices"
4. **Superseding decisions:** "We chose React" vs "We switched to Svelte"

The agent uses semantic reasoning — not string matching — to detect contradictions. When in doubt, create a `relates_to` relation instead of `contradicts`.

### Relation Types

| Type | Meaning | Example |
|---|---|---|
| `relates_to` | General topical connection | Auth memory ↔ Security memory |
| `supports` | One memory provides evidence for another | Test result supports architecture decision |
| `contradicts` | Memories make conflicting claims | Old DB choice vs new DB choice |
| `supersedes` | New memory replaces old one | Updated deployment procedure |
| `depends_on` | One memory requires another for context | Feature spec depends on architecture decision |
| `part_of` | Hierarchical containment | Endpoint spec is part of API design |

### Cascade Timing

The cross-reference cascade runs **after** the user-facing response has been delivered. It is not blocking. If the cascade fails partway through, the new memory is still persisted — the cascade is best-effort graph maintenance.

---

## 7. Privacy Routing Protocol

Every memory must be assigned to a **space** that determines its visibility scope. This decision is made at capture time.

### Spaces

| Space | Scope | Persistence | Examples |
|---|---|---|---|
| `user` | Follows the user across all projects | Permanent until explicitly deleted | Personal preferences, general knowledge, communication style |
| `project` | Scoped to a specific workspace/project | Permanent within project | Architecture decisions, codebase conventions, team norms |

### The README Test

The primary heuristic for privacy routing is the **README test**:

> Would this information appear in a public README without causing harm?

```
PROCEDURE privacy_route(memory_content, context):

    STEP 1: Apply the README test
    ─────────────────────────────
    Q: If this information appeared in the project's public README,
       would it cause any harm?

    → YES (would cause harm): route to 'user' space
      Examples:
        - Personal opinions about colleagues
        - Salary/compensation information
        - Authentication secrets, API keys
        - Personal health or life details
        - Emotional state or personal struggles

    → NO (harmless in README): continue to Step 2

    STEP 2: Is this project-specific or personal?
    ──────────────────────────────────────────────
    Q: Does this information ONLY make sense in the context
       of this specific project?

    → YES: route to 'project' space
      Examples:
        - "The project uses Next.js 14 with app router"
        - "Deploy target is AWS us-east-1"
        - "Database naming convention: snake_case plural"

    → NO: continue to Step 3

    STEP 3: Is this a personal preference or general knowledge?
    ──────────────────────────────────────────────────────────
    Q: Would this be useful to remember across different projects?

    → YES: route to 'user' space
      Examples:
        - "User prefers tabs over spaces"
        - "User's timezone is PST"
        - "User likes concise code comments"

    → NO: route to 'project' space (default for ambiguous cases
          within a project context)
```

### Override Rules

1. If the user explicitly says "remember this for all projects" → `user` space.
2. If the user explicitly says "this is just for this project" → `project` space.
3. If the memory contains any PII → `user` space (most restrictive scope).
4. Content type `preference` defaults to `user` space.
5. Content types `architecture`, `decision`, `procedure` default to `project` space.
6. Content type `fact` → apply the README test.
7. Content type `debug_insight` → `project` space unless it's a general technique.

---

## 8. Confidence Update Rules

Confidence is a float in `[0.0, 1.0]` representing the system's trust that a memory is accurate and current.

### Initial Confidence Assignment

| Scenario | Initial Confidence |
|---|---|
| User explicitly states a fact | 0.85 |
| Agent infers from context | 0.60 |
| Agent observes a pattern (first time) | 0.55 |
| User corrects the agent | 0.90 (new memory), decrease old |
| Agent discovers from code analysis | 0.75 |
| Decision/agreement between user and agent | 0.85 |
| Tentative/uncertain information | 0.40 |

### Confidence Increase Events

| Event | Adjustment | Cap |
|---|---|---|
| Pattern confirmed (2nd occurrence) | +0.15 | 0.95 |
| Pattern confirmed (3rd+ occurrence) | +0.10 | 0.98 |
| User explicitly confirms memory | +0.15 | 0.99 |
| Memory successfully used in recall and agent response was accepted | +0.05 | 0.95 |
| Cross-reference finds supporting evidence | +0.05 | 0.95 |

### Confidence Decrease Events

| Event | Adjustment | Floor |
|---|---|---|
| Contradiction detected (newer memory disagrees) | -0.20 | 0.10 |
| User says memory is wrong | Set to 0.10 | — |
| Memory unused for 90+ days | -0.05 per 30 days | 0.20 |
| Agent finds conflicting evidence in code | -0.15 | 0.10 |

### Confidence Thresholds

| Threshold | Behavior |
|---|---|
| >= 0.80 | High confidence. Use without hedging. |
| 0.50 – 0.79 | Moderate confidence. Use but weight against other evidence. |
| 0.20 – 0.49 | Low confidence. Only use if no better information available. |
| < 0.20 | Effectively untrusted. Exclude from recall results. Candidate for garbage collection. |

### Decay Function

Memories that are never accessed decay slowly:

```
confidence_decay(memory):
    days_since_access = (now - memory.last_accessed).days
    if days_since_access > 90:
        decay_periods = (days_since_access - 90) / 30
        decayed = memory.confidence - (0.05 * decay_periods)
        return max(decayed, 0.20)
    return memory.confidence
```

---

## 9. Anti-Patterns

The following behaviors are **strictly prohibited** when the memory system is active.

### 9.1 Never Announce Memory Operations

**FORBIDDEN:**
```
"Let me check my memory for that..."
"I've saved that to memory."
"I remember from our last session that..."
"According to my stored memories..."
"I'll make a note of that preference."
```

**CORRECT:**
```
(silently recall, then respond naturally)
"The project uses PostgreSQL with pgvector — you'll want to..."
(silently capture, user sees nothing)
```

The user should perceive the agent as simply *knowing things*, not as operating a database.

### 9.2 Never Capture Session Artifacts

**FORBIDDEN captures:**
```
memory_capture(content="The user showed me this code:\n```python\ndef auth():\n    ...\n```")
memory_capture(content="File src/auth.py contains 200 lines of authentication logic")
```

**CORRECT captures:**
```
memory_capture(content="Authentication module (src/auth.py) uses a decorator-based pattern with JWT validation middleware. Tokens are verified via RS256 public key from env var AUTH_PUBLIC_KEY.")
```

Capture the **insight**, not the **artifact**.

### 9.3 Never Capture Verbatim User Quotes

**FORBIDDEN:**
```
memory_capture(content='User said: "I hate when you write long explanations, just give me the code"')
```

**CORRECT:**
```
memory_capture(content="User strongly prefers code-first responses over explanatory prose. Dislikes lengthy walkthroughs.", content_type="preference")
```

Synthesize into knowledge. Preserve the meaning, not the words.

### 9.4 Never Store Unnecessary PII

**FORBIDDEN:**
```
memory_capture(content="User's name is John Smith, email john@example.com, lives in Seattle")
```

**CORRECT:**
```
memory_capture(content="User's timezone is US/Pacific.", content_type="preference", space="user")
```

Only store PII that is operationally necessary for providing better assistance. Apply data minimization.

### 9.5 Never Block Response for Memory Operations

**FORBIDDEN behavioral pattern:**
```
1. User asks question
2. Agent calls memory_recall() and WAITS
3. memory_recall() times out (10s)
4. Agent then starts generating response (user waited 10s for nothing)
```

**CORRECT behavioral pattern:**
```
1. User asks question
2. Agent calls memory_recall() with timeout
3. If result arrives quickly → incorporate
4. If timeout → respond without memory context (gracefully degrade)
5. Never make the user aware of the delay
```

### 9.6 Never Capture Duplicates

Before capturing, the agent should consider whether the information is already in hot context or was recently recalled. If the knowledge is already stored with equivalent or higher confidence, do not create a duplicate.

### 9.7 Never Perform Speculative Mass Captures

**FORBIDDEN:**
```
(User shares a large document)
→ Agent captures 15 separate memories from it
```

**CORRECT:**
```
(User shares a large document)
→ Agent captures 1-3 high-value insights synthesized from the document
→ Additional captures only if user discusses specific parts in depth
```

---

## 10. Inductive Writing Rule

Every memory's `content` field MUST follow the **inductive writing rule**: the conclusion or most important claim comes first, followed by supporting details.

### Structure

```
[CONCLUSION/CLAIM]. [Supporting detail 1]. [Supporting detail 2]. [Context if needed].
```

### Examples

**CORRECT (inductive):**
```
"Project uses a monorepo with Turborepo for build orchestration. Three packages: web (Next.js), api (Fastify), shared (TypeScript types). CI runs on GitHub Actions with package-level caching."
```

**INCORRECT (narrative):**
```
"The user mentioned they have a monorepo. They said it has three packages. The first is a web app built with Next.js. There's also an API using Fastify. And a shared types package. They use Turborepo and GitHub Actions."
```

### Rationale

Inductive writing ensures:
1. **Scan-ability:** The first sentence tells you what the memory is about.
2. **Truncation safety:** If the memory is truncated for token budget, the most important information survives.
3. **Embedding quality:** The opening claim dominates the vector embedding, improving retrieval relevance.

---

## 11. Failure Recovery

### Failure Modes and Recovery Actions

| Component | Failure | Detection | Recovery |
|---|---|---|---|
| SQLite DB | File missing | `FileNotFoundError` on first access | Auto-initialize empty DB with schema. Log warning. |
| SQLite DB | File locked | `sqlite3.OperationalError: database is locked` | Retry with exponential backoff (3 attempts, 100ms/200ms/500ms). If still locked, skip operation. |
| SQLite DB | Corrupted | Integrity check fails | Copy corrupt DB to `*.corrupt.bak`. Initialize fresh DB. Log error. |
| Embedding provider | API unreachable | Timeout or connection error | Fall back to keyword-only search (BM25). Cache content for later embedding. |
| Embedding provider | Rate limited | 429 response | Exponential backoff. Queue captures for batch processing. |
| Embedding provider | Invalid API key | 401/403 response | Disable vector search. Use keyword-only mode. Log error prominently. |
| Vector store | Dimension mismatch | Insert/query error | Rebuild vector index (re-embed all memories). This is expensive — prompt user. |
| MCP server | Not running | Connection refused | Shell hooks still work (inject reminders). Tools unavailable — degrade to reminders only. |
| Hook injection | Timeout | Hook exceeds timeout | Return empty string. Agent proceeds without memory context for this turn. |

### Graceful Degradation Levels

```
Level 0: FULL OPERATION
  All components working. Vector + keyword search. Full cascade.

Level 1: DEGRADED EMBEDDINGS
  Embedding provider down. Keyword search only. Captures queued for later embedding.

Level 2: READ-ONLY
  DB locked or write failures. Can recall but cannot capture. Queue captures in memory.

Level 3: REMINDERS ONLY
  DB inaccessible. Hooks still inject behavioral reminders. No recall/capture possible.

Level 4: FULLY OFFLINE
  All components failed. Agent operates without memory. No errors shown to user.
```

### Capture Cache

When captures fail, they are written to a local cache file:

```
~/.engram-lite/capture-cache.jsonl
```

Each line is a JSON object:

```json
{"timestamp": "2026-03-03T12:00:00Z", "content": "...", "content_type": "fact", "importance": "medium", "domain": "project/auth", "tags": ["jwt"], "space": "project", "retry_count": 0}
```

On next successful DB connection, the cache is drained:

```
PROCEDURE drain_capture_cache():
    FOR each entry in capture-cache.jsonl:
        try:
            memory_capture(**entry)
            remove entry from cache
        except:
            entry.retry_count += 1
            IF entry.retry_count >= 5:
                move to dead-letter file
                remove from cache
```

---

## 12. Convergence Property

The engram-lite system is designed to **converge** — to measurably improve the quality of agent assistance over time. This section defines what convergence means and how to measure it.

### Convergence Dimensions

| Dimension | Metric | Target |
|---|---|---|
| **Recall precision** | % of recalled memories that were relevant to the query | > 80% after 50 sessions |
| **Capture rate** | Memories captured per session | Stabilize at 2-5 per session (not growing unboundedly) |
| **Contradiction resolution** | Contradicted memories resolved (superseded or deleted) | < 5% of total memories in contradiction state |
| **Domain coverage** | % of user's active domains with >= 3 memories | > 70% after 20 sessions |
| **Confidence distribution** | Mean confidence across all active memories | > 0.70 after 30 sessions |
| **Hot context hit rate** | % of sessions where hot context contained relevant info | > 60% after 10 sessions |

### Why It Converges

1. **Confidence calibration:** Correct memories gain confidence through confirmation. Incorrect memories lose confidence through contradiction detection. Over time, high-confidence memories are accurate.

2. **Deduplication pressure:** The cascade detects duplicates and superseded information, keeping the memory store lean.

3. **Domain coverage growth:** Each session naturally explores the user's problem space, adding coverage to underrepresented domains.

4. **Pattern detection:** Recurring patterns get flagged and boosted, making them reliable hot context for future sessions.

5. **Decay and garbage collection:** Unused memories slowly lose confidence. Memories below the trust threshold (0.20) are candidates for cleanup. This prevents unbounded growth.

### Convergence Anti-Patterns

The system can **fail to converge** if:

- The agent captures too aggressively (noise overwhelms signal)
- The agent never captures (no learning occurs)
- Contradictions are never resolved (conflicting memories degrade recall)
- The embedding model changes without re-indexing (vector space inconsistency)
- The user works on too many unrelated projects with a shared `user` space

---

## 13. Anti-Rationalization Checklist

Before the agent decides **not** to capture something, it MUST run this internal checklist. This prevents the agent from rationalizing away captures that should be made.

```
ANTI-RATIONALIZATION CHECKLIST (run before deciding NOT to capture):

□ "I already know this" → CHECK: Is it actually in memory? Or do I just
  know it from training data? Training data knowledge is NOT memory.
  If the user told me this, it should be captured.

□ "This is too trivial" → CHECK: Is it trivial GENERALLY, or trivial
  FOR THIS USER? "User prefers tabs" is trivial generally but valuable
  for personalization.

□ "I'll remember it from conversation context" → CHECK: Will I remember
  it NEXT SESSION? Conversation context does not persist. If it matters
  tomorrow, capture it today.

□ "This is temporary" → CHECK: Is it actually temporary? "We're using
  a workaround for the auth bug" might seem temporary but could last
  months. Capture with importance='low'.

□ "The user didn't explicitly ask me to remember" → CHECK: The protocol
  says capture proactively. Users expect the agent to learn without
  being told. Explicit requests are not required.

□ "I just captured something similar" → CHECK: Is it actually a
  duplicate, or new information that UPDATES the prior capture?
  If updating → capture and let the cascade handle deduplication.

□ "This is just conversation filler" → CHECK: Reread the exchange.
  Is there a genuine fact, preference, or decision buried in the filler?
  Extract the signal.
```

If ANY checkbox makes you reconsider, proceed with the capture.

---

## 14. LLM Prompt Templates

These templates are used internally by the engram-lite system when it needs the LLM to make structured decisions. They are NOT shown to the user.

### 14.1 Capture Decision Prompt

Used after each response to determine if a capture should be made.

```
SYSTEM:
You are a memory capture analyst. Given a conversation exchange, determine
whether any new knowledge should be captured for long-term memory.

Rules:
- Only capture DURABLE knowledge (useful beyond this session)
- Synthesize, don't quote
- Write conclusion-first (inductive style)
- One capture per distinct piece of knowledge
- Do NOT capture: code listings, file contents, transient instructions

INPUT:
<session_context>
Active project: {project_id}
Active domains: {domain_list}
Hot context summary: {hot_context_summary}
</session_context>

<exchange>
User: {user_message}
Assistant: {assistant_response}
</exchange>

OUTPUT FORMAT (JSON):
{
  "should_capture": true/false,
  "reasoning": "brief explanation of decision",
  "captures": [
    {
      "content": "Conclusion-first synthesized knowledge.",
      "content_type": "fact|preference|decision|procedure|architecture|debug_insight",
      "importance": "critical|high|medium|low",
      "domain": "hierarchical/domain/path",
      "tags": ["tag1", "tag2"],
      "keywords": ["keyword1", "keyword2"],
      "confidence": 0.0-1.0,
      "space": "user|project"
    }
  ]
}

If should_capture is false, captures array must be empty.
Respond ONLY with valid JSON. No commentary.
```

### 14.2 Domain Inference Prompt

Used to determine the correct hierarchical domain path for a new memory.

```
SYSTEM:
You are a domain classifier. Given content to be stored as a memory,
determine the most appropriate hierarchical domain path.

Domain paths use "/" separators and go from general to specific:
  project/backend/auth
  project/frontend/components
  project/infra/deploy
  user/preferences/code-style
  user/preferences/communication
  user/knowledge/python
  meta/workflow
  meta/tools

Rules:
- Maximum 4 levels deep
- Use lowercase, hyphenated segments
- Prefer existing domains when possible
- Create new domains only when existing ones don't fit

INPUT:
<content>{memory_content}</content>
<content_type>{content_type}</content_type>
<existing_domains>{list_of_existing_domains}</existing_domains>
<project_context>{project_name_and_description}</project_context>

OUTPUT FORMAT (JSON):
{
  "domain": "the/domain/path",
  "reasoning": "Why this domain was chosen",
  "is_new_domain": true/false,
  "alternative_domains": ["other/possible/path"]
}

Respond ONLY with valid JSON. No commentary.
```

### 14.3 Keyword Extraction Prompt

Used to extract searchable tags and keywords for BM25 indexing.

```
SYSTEM:
You are a keyword extractor. Given memory content and its type,
extract relevant tags and keywords for search indexing.

Tags: short categorical labels (1-2 words, lowercase, hyphenated)
Keywords: longer search terms and phrases that someone might search for

Rules:
- Tags: 2-6 tags per memory
- Keywords: 3-10 keywords per memory
- Include both specific terms and broader category terms
- Include acronyms and their expansions
- Do not include stop words or generic terms ("the", "system", "code")

INPUT:
<content>{memory_content}</content>
<content_type>{content_type}</content_type>

OUTPUT FORMAT (JSON):
{
  "tags": ["tag1", "tag2", "tag3"],
  "keywords": ["search phrase 1", "search phrase 2", "specific term"]
}

Respond ONLY with valid JSON. No commentary.
```

### 14.4 Cross-Reference Prompt

Used during the cascade to analyze relationships between a new memory and existing memories.

```
SYSTEM:
You are a memory analyst. Given a new memory and a list of existing
related memories, determine the relationships between them.

Relation types:
- relates_to: General topical connection
- supports: New memory provides evidence for existing, or vice versa
- contradicts: Memories make conflicting claims about the same topic
- supersedes: New memory replaces/updates existing memory
- depends_on: One memory requires another for full context
- part_of: Hierarchical containment

Rules:
- Only flag contradictions when claims are genuinely incompatible
- "supersedes" means the new memory makes the old one obsolete
- A memory can relate to multiple existing memories
- Include confidence for each detected relation (0.0-1.0)
- Flag patterns: if same topic appears 2+ times, note it

INPUT:
<new_memory>
  ID: {new_memory_id}
  Content: {new_memory_content}
  Domain: {new_memory_domain}
  Type: {new_memory_content_type}
</new_memory>

<existing_memories>
{for each existing memory:}
  - ID: {id}
    Content: {content}
    Domain: {domain}
    Type: {content_type}
    Confidence: {confidence}
    Created: {created_at}
{end for}
</existing_memories>

OUTPUT FORMAT (JSON):
{
  "relations": [
    {
      "source_id": "new_memory_id",
      "target_id": "existing_memory_id",
      "relation_type": "relates_to|supports|contradicts|supersedes|depends_on|part_of",
      "confidence": 0.0-1.0,
      "reasoning": "brief explanation"
    }
  ],
  "contradictions": [
    {
      "new_claim": "what the new memory says",
      "old_claim": "what the existing memory says",
      "old_memory_id": "existing_memory_id",
      "severity": "direct|partial|nuanced"
    }
  ],
  "patterns": [
    {
      "description": "what recurring pattern was detected",
      "memory_ids": ["id1", "id2"],
      "occurrence_count": 2
    }
  ],
  "superseded": [
    {
      "old_memory_id": "existing_memory_id",
      "reason": "why the new memory makes this one obsolete"
    }
  ]
}

Respond ONLY with valid JSON. No commentary.
```

### 14.5 Graph Node Summary Update Prompt

Used when a domain node's summary needs regeneration after new memories are added.

```
SYSTEM:
You are a summarizer. Given a domain label and all memories belonging
to that domain, produce a single concise paragraph that summarizes
the collective knowledge in that domain.

Rules:
- One paragraph, 2-5 sentences
- Conclusion-first (inductive style): start with the most important fact
- Weight by confidence: high-confidence memories dominate the summary
- Weight by importance: critical/high before medium/low
- If contradictions exist, note the most recent/confident claim
- Do not list memories — synthesize into coherent prose
- Include key numbers, names, and specifics — don't be vague

INPUT:
<domain>{node_label}</domain>

<memories>
{for each memory in domain:}
  - Content: {content}
    Type: {content_type}
    Confidence: {confidence}
    Importance: {importance}
    Created: {created_at}
    Tags: {tags}
{end for}
</memories>

OUTPUT FORMAT (JSON):
{
  "summary": "The synthesized paragraph summarizing all knowledge in this domain.",
  "key_facts_count": 5,
  "avg_confidence": 0.82,
  "has_contradictions": false
}

Respond ONLY with valid JSON. No commentary.
```

---

## Appendix A: Quick Reference Card

```
┌──────────────────────────────────────────────────────────┐
│              CANVAS-MEMORY PROTOCOL CARD                  │
├──────────────────────────────────────────────────────────┤
│                                                          │
│  SESSION START:                                          │
│    → Hot context loaded automatically                    │
│    → Read it. Use it. Don't mention it.                  │
│                                                          │
│  ON EACH PROMPT:                                         │
│    → Does this relate to prior context?                  │
│    → If yes: memory_recall(query)                        │
│    → Use recalled info naturally                         │
│                                                          │
│  AFTER EACH RESPONSE:                                    │
│    → New info? → capture                                 │
│    → Correction? → capture (high importance)             │
│    → Pattern? → capture or update                        │
│    → After capture → run cascade                         │
│                                                          │
│  ALWAYS:                                                 │
│    → Silent operations                                   │
│    → Conclusion-first writing                            │
│    → Synthesize, don't quote                             │
│    → Graceful degradation on failure                     │
│    → Privacy by default                                  │
│                                                          │
│  NEVER:                                                  │
│    → Announce memory operations                          │
│    → Capture code listings or file contents              │
│    → Store unnecessary PII                               │
│    → Block response for memory ops                       │
│    → Skip anti-rationalization checklist                 │
│                                                          │
└──────────────────────────────────────────────────────────┘
```

---

## Appendix B: Content Type Reference

| Type | When to Use | Typical Importance | Typical Confidence | Default Space |
|---|---|---|---|---|
| `fact` | Concrete, verifiable information | medium | 0.75–0.85 | project |
| `preference` | User's style, taste, or behavioral preference | medium | 0.55–0.85 | user |
| `decision` | A choice made between alternatives | high | 0.85 | project |
| `procedure` | A multi-step process or workflow | medium–high | 0.80 | project |
| `architecture` | System design, structure, or pattern | high | 0.75 | project |
| `debug_insight` | Lessons from debugging or troubleshooting | medium | 0.70 | project |

---

## Appendix C: Importance Level Reference

| Level | When to Assign | Hot Context? | Decay Rate |
|---|---|---|---|
| `critical` | Information the agent must ALWAYS have available. Errors or user safety. | Always loaded | None (immune to decay) |
| `high` | Important context that frequently affects responses. | Loaded if capacity allows | Slow (0.02/30d after 120d) |
| `medium` | Useful context that occasionally affects responses. | Not auto-loaded | Normal (0.05/30d after 90d) |
| `low` | Nice-to-have context. Temporary workarounds, minor details. | Never auto-loaded | Fast (0.05/30d after 60d) |
