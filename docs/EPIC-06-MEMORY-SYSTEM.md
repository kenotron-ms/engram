# Epic 06: Memory System

**Owner:** Chris Park
**Contributors:** Ken Chau, AI Assistant

---

## 1. Summary

Enable AI to remember across projects so context compounds over time. Users stop repeating themselves—AI learns preferences, patterns, and constraints automatically from conversations, becoming smarter with each interaction.

**Core insight:** "Every new project feels like talking to someone with amnesia." This epic eliminates that frustration through ChatGPT-style auto-discovery combined with Claude-style progressive retrieval.

---

## 2. Problem

**What's Broken:**
Every new project starts from zero. The AI has no memory of:
- Your preferences ("I told you I prefer 3 bullets max in the last 5 projects")
- Your constraints ("I explicitly said HIPAA compliant - no real patient data in demos")
- Your patterns ("I always start presentations with the bottom line")
- Your context ("I work with non-technical HR managers - avoid jargon")

**The Frustration:**
- Repeating the same preferences project after project
- Inconsistent voice and tone across your content
- No intelligence compounding over time
- Wasted cognitive load re-establishing who you are and how you work

**Who's Impacted:**
- Knowledge workers with dozens of projects (most frustration)
- Teams with shared standards (repeated teaching)
- Domain experts with specialized constraints (constant re-explanation)

---

## 3. Proposed Solution

Build a memory system that learns automatically from conversations—not one that requires manual filing or explicit commands.

**Design Foundation (from industry research):**

- **ChatGPT Memory:** Auto-discovers facts from conversations; deletion-only UI (proven sufficient)
- **Claude Projects:** Tool-based progressive discovery (AI searches on-demand, 84% token reduction)
- **Mem0/MemU:** Usage tracking for ranking; memory decay for cleanup

**Three-Phase Approach:**

### Phase 1: Auto-Discovery Foundation (MVP)

AI learns automatically and applies context transparently:

**Memory System (Facts):**
- **Automatic Fact Extraction** - AI extracts discrete facts after each session (0-5 facts per session, >70% confidence threshold)
- **Progressive Memory Discovery** - AI searches and loads memories on-demand via tools, not pre-loaded (70-90% token reduction)
- **Three Memory Scopes** - Facts automatically scoped to user (everywhere), project (one project), or workspace (all projects in workspace)
- **Silent Memory Application** - AI uses memories transparently without announcing every time
- **Memory Management UI** - Simple deletion-only interface in Settings (no editing needed - delete and re-learn)
- **Conversation Commands** - Optional natural language control ("Remember this", "Forget about X", "What do you remember?")

**Chat History Reference (Full Conversations):**
- **Chat History Storage** - Store all project conversations in searchable format for later reference
- **Chat History Search Tool** - AI searches past conversations on-demand ("Remember when we discussed X?")
- **Conversation Context Loading** - Load relevant conversation snippets into context when needed

**Privacy Controls:**
- **Memory Collection Toggle** - Enable/disable automatic fact extraction (user controls data collection)
- **Chat History Reference Toggle** - Enable/disable AI's ability to reference past conversations
- **Memory Settings UI** - Settings page with toggles, "Manage" button, and privacy information

**Phase 1 Goal:** User works across 5+ projects without repeating preferences

### Phase 2: Intelligence Layer

AI gets smarter from patterns, usage, and learning:

- **Usage Tracking & Smart Ranking** - Track which facts are useful; rank search by usage + recency + confidence
- **Memory Decay** - Remove stale, unused facts automatically (90-day rule, 30-day undo window)
- **Pattern Suggestion** - AI notices repeated behavior and suggests saving as preference (3+ occurrences, max 1 suggestion/day)
- **Memory Clarification** - When AI notices differences from memories, ask for clarification humbly ("Can you clarify?" not "Conflict detected!")

**Phase 2 Goal:** Memory quality improves over time (high usage facts, few deletions)

### Phase 3: Team Collaboration

Share institutional knowledge across team members:

- **Workspace Team Memory** - Workspace-scoped facts shared across all team members; new members inherit context automatically
- **Personal vs Team Priority** - Handle conflicts between personal preferences and team workspace (workspace wins in team context)

**Phase 3 Goal:** Team knowledge survives turnover; new members productive day 1

**Core Principles:**
- Auto-discovery first (80%+ facts extracted automatically)
- Progressive loading (AI searches on-demand, not pre-loaded)
- Deletion-only management (no editing needed)
- Three scopes (user/project/workspace)
- Conversation-first (optional commands, not required)
- Settings-based UI (not prominent sidebar panel)

---

## 4. User Stories

**IMPORTANT:** Only include user stories for IMPLEMENTED features. Do NOT create user story files for future work. Epic describes future capabilities, but detailed user story files are created when ready to build.

### Implemented

**Infrastructure (Complete)**

| # | Feature | Status | PR |
|---|---------|--------|-----|
| I1 | Database Schema (`memory_facts` table) | ✅ Done | #277 |
| I2 | `@workspaces/memory` Package (types, db, llm) | ✅ Done | #277 |
| I3 | Extraction Job Queue (`extraction_jobs` table) | ✅ Done | #277 |
| I4 | Amplifier Runtime Integration | ✅ Done | #277 |
| I5 | pg-boss Background Job System (5 workers, 30-min scheduler) | ✅ Done | #323 |

**Phase 1: Foundation (12 Features)**

| # | Feature | Status | Description |
|---|---------|--------|-------------|
| **Memory System (Facts)** ||||
| F1.1 | Automatic Fact Extraction | ✅ Done | AI extracts discrete facts from conversations after each session |
| F1.2 | Progressive Memory Discovery | ✅ Done | Backend API + agent tools registered with Claude Agent SDK |
| F1.3 | Three Memory Scopes | ✅ Done | Facts automatically scoped to user, project, or workspace |
| F1.4 | Silent Memory Application | ✅ Done | `loadMemoryContext()` called at session start; cross-project sharing enabled |
| F1.5 | Memory Management UI | ✅ Done | Memory modal with tabs (User/Workspace/Project), search, project selector, edit/delete (PR #316) |
| F1.6 | Conversation Commands | ✅ Done | memory_search, memory_list, memory_create, memory_delete registered with Claude Agent SDK |
| **Chat History Reference** ||||
| F1.7 | Chat History Storage | 🔲 Pending | Store all project conversations in searchable format |
| F1.8 | Chat History Search Tool | 🔲 Pending | AI searches past conversations on-demand via `conversation_search` tool |
| F1.9 | Conversation Context Loading | 🔲 Pending | Load relevant conversation snippets into context |
| **Privacy Controls** ||||
| F1.10 | Memory Collection Toggle | ✅ Done | Enable/disable automatic fact extraction (user setting in Memory UI) |
| F1.11 | Chat History Reference Toggle | 🔲 Pending | Enable/disable AI's ability to reference past conversations |
| F1.12 | Memory Settings UI | ✅ Done | Settings page with toggle, "Manage Memories" button, privacy info (PR #316) |

**Phase 2: Intelligence (4 Features)**

| # | Feature | Status | Description |
|---|---------|--------|-------------|
| F2.1 | Usage Tracking & Smart Ranking | ✅ Done | Track which facts are useful; rank by usage + recency + confidence |
| F2.2 | Memory Decay | 🔲 Pending | Remove stale, unused facts automatically (90-day rule) |
| F2.3 | Pattern Suggestion | 🔲 Pending | AI notices repeated behavior and suggests saving as preference |
| F2.4 | Memory Clarification | 🔲 Pending | When differences noticed, ask for clarification humbly |

**Phase 3: Collaboration (2 Features)**

| # | Feature | Status | Description |
|---|---------|--------|-------------|
| F3.1 | Workspace Team Memory | 🔲 Pending | Workspace-scoped facts shared across team members |
| F3.2 | Personal vs Team Priority | 🔲 Pending | Handle conflicts between personal and team preferences |

### Implementation Summary

**Backend Complete:**
- `packages/memory/` - Core memory package with extraction, search, ranking
- `backend/src/api/routes/memory.routes.ts` - Full REST API (search, list, CRUD, stats)
- `backend/src/memory/memoryTools.ts` - Tool definitions and handlers
- `backend/src/memory/memoryMcpServer.ts` - MCP server for Claude Agent SDK with proper tool schemas
- `backend/src/jobs/pgBossService.ts` - pg-boss job system (5 workers, 30-min scheduler)
- `backend/src/jobs/handlers/extraction.ts` - User-initiated extraction (Sonnet)
- `backend/src/jobs/handlers/batchExtraction.ts` - Scheduled batch extraction (Haiku)
- `backend/amplifier_runtime/extract_facts.py` - LLM extraction via Amplifier
- `backend/amplifier_runtime/run_recipe.py` - Recipe executor with heredoc-aware variable substitution

**Agent Integration Complete (✅):**
- MCP server with tool schemas (Claude can discover and call tools)
- Tool registration: `mcp__memory-tools__search`, `mcp__memory-tools__list`, `mcp__memory-tools__create`, `mcp__memory-tools__delete`
- `loadMemoryContext()` called at session start for context injection
- `loadWorkspaceContext()` shows other projects in the workspace (cross-project sharing)
- Memory injection strategy: user facts + workspace facts + OTHER project facts (not current project - redundant with conversation history)

**Extraction Pipeline Complete (✅):**
- Recipe-based extraction via `memory-extraction.yaml` (5 steps: extract → load existing → deduplicate → check limits → store)
- Heredoc JSON handling fixed in `run_recipe.py` (shell escaping was breaking JSON structure)
- Database pool initialization for MCP server handlers

**Background Job Infrastructure Complete (✅):**
- pg-boss worker pool (5 concurrent workers, async I/O)
- Scheduled batch extraction (every 30 minutes, cron-based)
- Cost optimization (Haiku for batch, Sonnet for user-initiated)
- Multi-replica safe (pg-boss handles leader election)
- Job handlers: `extraction` (priority 10), `batch-extraction` (priority 1)

**Agent Integration Pending:**
- Tool registration with Amplifier CLI (behavior bundle)
- Tool registration with Codex

**Frontend Complete (✅):**
- Memory Settings UI in ProfileSettings (toggle, "Manage Memories" button, privacy info)
- Memory modal with tabs (User/Workspace/Project)
- Memory List component (view/edit/delete facts)
- Search and filtering
- Project selector for project-scoped memories

### Future

---

## 5. Outcomes

**Success Looks Like:**

**Phase 1:**
- User works across 5+ projects without repeating preferences
- 80%+ facts auto-discovered (not manual)
- 90%+ facts accurate (user doesn't delete immediately)
- Token usage <10% of full-context baseline
- User satisfaction: "AI remembers me"

**Phase 2:**
- Pattern suggestion acceptance rate >50%
- Memory decay removes 5-10% facts per quarter
- Search relevance improves 20-30% vs chronological
- Zero complaints "AI used wrong preference"

**Phase 3:**
- New team members inherit workspace knowledge automatically
- Knowledge survives team turnover
- Clear priority hierarchy (project > workspace > user)
- Team satisfaction: "AI knows our standards"

**We'll Measure:**
- Facts auto-discovered vs manually created (target: 80%+ auto)
- Fact deletion rate within 24 hours (target: <10%, proves accuracy)
- Token usage per conversation (target: 70-90% reduction)
- User retention improvement for memory users vs non-users

---

## 6. Dependencies

**Requires:**
- Epic 03: Agent Chat Foundation (conversations where memories are formed)

**Enables:**
- Epic 05: Agent Drives Outcomes (memory of past outcomes informs future guidance)
- Epic 07: Collaboration (shared team memory foundation)
- Smarter AI over time (learning compounds)

**Blocks:**
- Nothing - other epics can proceed independently

---

## 7. Risks & Mitigations

| Risk | Impact | Probability | Strategic Response |
|------|--------|-------------|-------------------|
| AI extracts inaccurate facts | High | Medium | 70% confidence threshold; deletion-only UI for easy correction |
| Privacy concerns with shared memory | High | Low | Personal-only in Phase 1; explicit sharing in Phase 3 |
| Memory becomes outdated | Medium | High | Memory decay (90-day auto-cleanup with 30-day undo) |
| Remembering too much (context pollution) | Medium | Medium | Progressive discovery (load relevant facts only, not everything) |
| Duplicate facts accumulate | Medium | Medium | LLM-based duplicate detection before saving |
| System feels invasive | High | Low | Settings badge (passive indicator), not notifications; full user control |
| Team/personal conflicts confuse AI | Medium | Low | Clear priority hierarchy (project > workspace > user) |
| **Over-extraction of one-time choices** | Medium | High | Extraction criteria: require pattern seen 2+ times before saving as broad preference; "3 bullets for CEO" is project-specific, not universal |
| **Memory distracts conversation** | Medium | Medium | Surfacing logic: AI evaluates relevance before applying; don't surface "Ken is a PM" when debugging code |
| **Over-personalization narrows output** | Medium | Medium | Monitor for convergence; consider "fresh perspective" setting if problem emerges in practice |

---

## 8. Infrastructure: Background Job System

**Status:** ✅ Implemented (PR #323)  
**Date:** 2026-01-30 (Design) → 2026-02-01 (Implemented)  
**Priority:** Critical - Blocks scale beyond 50 users

### Problem: Current System Won't Scale

**MVP infrastructure identified as critical deficiency:**
- **Single worker**: Processes ONE job at a time (720 jobs/hour max)
- **Serial processing**: Queue backs up with multiple concurrent users
- **No scheduler**: Jobs only created reactively (UI-triggered)
- **Not extensible**: Tightly coupled to extraction use case
- **Breaking point**: 50+ users = queue overload

### Approved Solution: Worker Pool + Job Scheduler

**Three-component production system:**

**1. Worker Pool** (5 concurrent workers)
- Async I/O concurrency (not cluster/threads - extraction is I/O-bound)
- FOR UPDATE SKIP LOCKED prevents conflicts
- Graceful shutdown (waits for in-flight jobs)
- **Result:** 5,000 jobs/hour (7x improvement)

**2. Job Scheduler** (periodic batch processing)
- In-process with PostgreSQL advisory lock leader election
- Runs batch extraction every 30 minutes
- Multi-replica safe (only one replica schedules)
- No external dependencies (no Zookeeper/etcd)

**3. Generic Job System** (extensible for future)
- Job handler registry (easy to add cleanup, analytics, notifications)
- `background_jobs` table (generic) + keep `extraction_jobs` (backward compat)
- Fast claiming indexes (<10ms query time)

### Implementation Phases

**Phase 1: Worker Pool** (Complexity: M, ~2 days)
- Replace single worker with 5 concurrent workers
- No breaking changes, drop-in replacement
- Use existing extraction_jobs table

**Phase 2: Scheduler + Batch** (Complexity: M, ~3 days)
- Add SimpleScheduler with leader election
- Batch extraction every 30 minutes
- Fast pre-filter query (<10ms) + indexes

**Phase 3: Generic Jobs** (Complexity: L, ~5 days - Future)
- Add background_jobs table (generic)
- Job handler registry (extensible)
- Support cleanup, analytics, notifications

**Total:** ~10 days with AI velocity

### Key Features

**Batch Processing Strategy:**
```
Scheduler (every 30 min):
  ↓
Fast pre-filter: Find projects with updated_at > last_extracted_at
  ↓
Create 50 individual jobs with batch_key
  ↓
Worker pool processes 5 at a time (parallel)
  ↓
50 projects ÷ 5 workers = ~10 minutes
```

**Cost Optimization:**
- Batch extraction: Claude Haiku 4.5 (fastest, cheapest for batch work)
- User-initiated: Claude Sonnet 4 (higher quality for immediate requests)
- **Savings:** ~29% reduction in extraction costs

**Database Indexes:**
```sql
-- Fast job claiming
CREATE INDEX idx_background_jobs_claiming
  ON background_jobs(priority DESC, created_at ASC)
  WHERE status = 'pending';

-- Fast pre-filter for batch
CREATE INDEX idx_projects_extraction_needed
  ON projects(updated_at DESC)
  WHERE last_extracted_at IS NULL OR updated_at > last_extracted_at;
```

### Trade-offs Accepted

**Benefits:**
- ✅ 7x throughput (720 → 5,000 jobs/hour)
- ✅ Scales to 500+ users (vs 50 limit)
- ✅ Predictable costs (scheduled batch vs UI-triggered)
- ✅ Simple (PostgreSQL only, no Redis/RabbitMQ)
- ✅ Multi-replica safe (leader election)

**Drawbacks:**
- ⚠️ 30-minute delay for batch extraction (vs real-time)
- ⚠️ Added complexity (~500 LOC)
- ⚠️ Database load (more concurrent queries)

**Decision:** Benefits far outweigh drawbacks. Infrastructure is critical for scale.

### Documentation

**Specifications created:**
- **ADR-014:** Background Job Infrastructure (decision rationale, alternatives considered)
- **Pattern:** Background Job Worker Pool (reusable pattern documentation)
- **Implementation Plan:** ai_working/ken/epic-06-memory/implementation/implementation_plan/14-background-job-infrastructure.md

**Files created (✅):**
```
backend/src/jobs/
├── pgBossService.ts (pg-boss wrapper, worker pool, scheduler)
├── handlers/extraction.ts (user-initiated extraction)
├── handlers/batchExtraction.ts (scheduled batch extraction)
├── extractionJobs.ts (job creation API)
├── extractionWorker.ts (worker entry point)
└── types.ts (job type definitions)
```

**Implementation approach:**
- Used pg-boss library (handles worker pool, scheduling, leader election internally)
- Simpler than custom implementation (no need for WorkerPool.ts, SimpleScheduler.ts, leaderElection.ts)
- pg-boss provides: partitioned queues, advisory locks, graceful shutdown, monitoring

---

## 9. Open Questions

### Resolved by Design

- [x] How does AI learn? Auto-discovery from conversations (ChatGPT pattern)
- [x] How does AI retrieve? Progressive tool-based search (Claude pattern)
- [x] How do users manage? Deletion-only UI in Settings
- [x] How are scopes determined? AI infers (project/workspace/user), user can delete and re-learn
- [x] Where is management UI? Context-aware (project settings shows project facts, workspace settings shows workspace facts, global settings shows user facts)

### Still Open

**Memory System:**
- [ ] What's the fact limit per scope before performance degrades?
- [ ] Should vector search (pgvector) be added when facts exceed 50 per scope?
- [ ] How do we handle fact migration when user leaves team?

**Team Memory (Phase 3):**
- [ ] Who can create/delete team memories? Any member or admins only?
- [ ] How do we handle company rebrand (20+ memories reference old brand)?
- [ ] Should team memories have approval workflow?

---

## 9. Future Feature: Bulk Extraction Strategy

**Status:** Design Discussion (Not Prioritized)
**Date:** 2026-01-30
**Context:** Memory UI implementation (Phase 1) triggered discussion about extraction costs and scalability

### Problem Statement

Current extraction triggers may cause cost/scalability issues:
- **3-minute debounce timer**: Triggers on every UI pause, too aggressive
- **Session-end trigger**: Triggers on minimize/close/switch
- **No deduplication**: Same project can queue multiple jobs
- **Serial processing**: One job at a time, 5s polling
- **Cost concern**: With 5-10 users, could generate 50-100 extractions/day

### Proposed Solution: 30-Minute Bulk Processing

**Core Concept:** Remove real-time triggers, run scheduled batch job every 30 minutes across all users.

**Key Design Points:**
1. **Fast Pre-Filter** (<10ms query)
   - Check for projects with new messages since last extraction
   - Skip projects with <4 new messages (need context)
   - Skip projects extracted <5 min ago (conversation still active)
   - If NO projects need extraction → zero compute, zero cost

2. **Batch Processing**
   - Process 20-50 projects in parallel (3-5 workers)
   - Incremental extraction (send only new messages + context)
   - Use cheaper model (Haiku instead of Sonnet)
   - Complete batch in 5-10 minutes

3. **Database Indexes** (critical for performance)
   ```sql
   CREATE INDEX idx_conversation_messages_project_created
     ON conversation_messages(project_id, created_at);
   CREATE INDEX idx_projects_last_extracted
     ON projects(last_extracted_at);
   ```

4. **Cost Impact** (estimated)
   - Before: ~$1.35/day (5 users, real-time)
   - After: ~$0.24/day (5 users, batch + Haiku)
   - Savings: 82% reduction

### Trade-offs

**Pros:**
- Predictable, manageable compute costs
- Scales to 1000+ users without queue overload
- Server stays lightweight between batches
- No UI-triggered job storms

**Cons:**
- 30-35 minute delay for memory updates (vs near real-time)
- Less "magical" immediate learning feel
- Requires monitoring of batch completion time

### Decision

**Defer to post-MVP.** Current system is acceptable for Phase 1 (limited users). Re-evaluate when:
- User count exceeds 50
- API costs become material concern
- Queue overload observed in production

### Implementation Notes (If Prioritized)

See full design in conversation transcript (2026-01-30). Key files to modify:
- `backend/src/jobs/extractionScheduler.ts` (new)
- `backend/src/agent/ClaudeAgent.ts` (remove debounce timer)
- `backend/src/jobs/extractionWorker.ts` (increase concurrency)

## 10. Change History

| Version | Date | Author | Changes |
|---------|------|--------|---------|
| v1.0 | 2025-12-10 | Chris Park | Initial outline |
| v2.0 | 2025-12-12 | Chris Park | Refactored to philosophy-driven template |
| v3.0 | 2025-12-16 | Chris Park | Deep analysis of Microsoft Amplifier + Semantic Workbench |
| v4.0 | 2026-01-14 | Ken Chau | Replaced technical examples with knowledge worker scenarios; expanded Open Questions |
| v5.0 | 2026-01-22 | Ken Chau | Major update from design docs: ChatGPT auto-discovery + Claude progressive discovery; three-phase approach with 12 features; context-aware memory hierarchy; success criteria by phase |
| v6.0 | 2026-01-22 | Ken Chau | Added Chat History Reference features (F1.7-F1.9) and Privacy Controls (F1.10-F1.12) based on ChatGPT Memory UI analysis; Phase 1 now has 12 features total |
| v6.1 | 2026-01-22 | Ken Chau | Added risks from PM feedback: over-extraction, distraction, over-personalization (implementation concerns, not new features) |
| v7.0 | 2026-01-26 | Ken Chau | Implementation progress: F1.1-F1.4, F1.6, F2.1 complete (extraction, search, ranking, agent tools); frontend UI pending |
| v7.1 | 2026-01-26 | Ken Chau | Status correction: F1.2, F1.4, F1.6 marked Partial - backend/tools done but agent integration pending (tools not registered with any agent type) |
| v7.2 | 2026-01-26 | Ken Chau | F1.2, F1.4, F1.6 now Done - tools registered with Claude Agent SDK; added workspace context (other projects); refined memory injection strategy (cross-project sharing, not echoing current project) |
| v7.3 | 2026-01-27 | Ken Chau | Fixed memory tools - created proper MCP server with tool schemas so Claude can actually discover and call them (was broken: just adding to allowedTools doesn't work) |
| v7.4 | 2026-01-27 | Ken Chau | Fixed extraction pipeline - heredoc JSON handling in run_recipe.py (shell escaping was breaking JSON structure); added DB pool init for MCP handlers |
| v7.5 | 2026-01-30 | Ken Chau | F1.5 and F1.10 complete - Memory UI merged to main (PR #316); modal-based UI with tabs, search, edit/delete, and memory collection toggle |
| v7.6 | 2026-02-02 | Ken Chau | I5 complete - pg-boss background job infrastructure (PR #323); 5 workers, 30-min scheduler, batch/single extraction handlers, cost optimization |

---

## Related Design Documents

**Full design documentation:** `/ai_working/ken/epic-06-memory/`

- **FEATURES-LIST.md** - Complete feature specification with user experience flows
- **MEMORY-HIERARCHY.md** - Context-aware management architecture (project/workspace/global)
- **EXTRACTION-CRITERIA.md** - What deserves to be remembered
- **ARCHITECTURE.md** - Technical implementation details
- **TECHNOLOGY-ROADMAP.md** - When to add pgvector/graph/versioning
