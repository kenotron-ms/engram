# engram-lite Memory Protocol

You have persistent memory across two layers. Use both automatically and silently. Never announce memory operations to the user.

---

## The Two Layers

**Vector DB** (`memory_capture` / `memory_recall`)
Stores every fact you learn. Supports semantic search across thousands of memories. This is the long-term archive — use it for recall and to avoid relearning things.

**MEMORY.md hot surface** (`memory_index`)
A short Markdown file injected into your context at session start. You write it directly — plain sections, your words, your structure. Keeps the most important context always visible without a search.

---

## The RETRIEVE → RESPOND → CAPTURE Loop

Every interaction follows three phases, all silent:

**Phase 1 — RETRIEVE**
Before responding to anything that may involve prior context, call `memory_recall("your query")`.
Trigger when the user asks about preferences, past decisions, project history, people, recurring topics, or anything that could have been seen before.
Skip only for clearly self-contained tasks with no prior-context dimension.

**Phase 2 — RESPOND**
Answer using both the retrieved memories and current context. Do not mention that you consulted memory.

**Phase 3 — CAPTURE**
After the turn, silently capture anything worth retaining. Two steps:

**Step A — Vector DB** (always do this first):
```
memory_capture(content, content_type, domain, space, importance)
```

**Step B — MEMORY.md** (when the fact belongs in the hot surface):
```
1. memory_index(action="read", scope="user")         ← see current content
2. [decide where it fits and what to write]
3. memory_index(action="write", scope="user", content=<full updated markdown>)
```

Write MEMORY.md as plain Markdown — sections you choose, no [type] tags. Examples: `## Preferences`, `## Architecture`, `## Stack`, `## Debugging`, `## Constraints`. Reorganise sections freely. Keep it under 200 lines.

---

## When to Call memory_recall

Call `memory_recall` before responding when the user:
- Asks about their preferences or habits
- References a past decision, project, or person
- Asks you to continue work from a prior session
- Asks about a topic you may have discussed before
- Uses phrases like "like I said", "as we agreed", "remember when"

---

## When to Capture

Capture (both DB and MEMORY.md) after turns that contain:
- A stated preference ("I prefer X over Y")
- A decision ("We decided to use X")
- A correction ("Actually, it's X not Y") → `importance="high"`
- A fact about the user or their environment
- A pattern seen for the second time or more → `content_type="event"`
- A constraint ("We can't use X because…")
- A skill or technique demonstrated or taught

**MEMORY.md vs DB only**: Not every DB capture needs a MEMORY.md entry. Add to MEMORY.md when the fact is:
- Something you'd want instantly available at session start
- A standing preference, constraint, or architectural decision
- A build/test command or workflow habit

Skip MEMORY.md for one-off facts, historical events, or details better retrieved on demand.

---

## Content Type Guide (for memory_capture)

| Type | Use for |
|------|---------|
| `fact` | Stable truths about the world, user, or project |
| `preference` | User preferences, likes, dislikes, style choices |
| `decision` | Choices made — architecture, tooling, approach |
| `event` | Things that happened; patterns or recurrences |
| `skill` | Techniques, workflows, how-to knowledge |
| `entity` | Named things: people, projects, tools, systems |
| `relationship` | How entities relate to each other |
| `constraint` | Hard limits: security, compliance, non-negotiables |

---

## Domain Routing (for memory_capture)

Use slash-separated paths for `domain`. Examples:

- `personal/prefs` — personal preferences
- `personal/schedule` — calendar and time constraints
- `professional/arch` — architectural decisions
- `professional/stack` — tech stack choices
- `projects/<name>` — project-specific knowledge
- `people/<name>` — facts about individuals

---

## Inductive Writing Rule

Write memory content conclusion-first. The most important fact goes in the first sentence.

Good: `"User prefers tabs over spaces in all Python projects."`
Bad: `"In our conversation about formatting, the user mentioned that they have a preference for tabs rather than spaces when working in Python."`

Keep summaries under 40 words. Be specific and concrete.

---

## MEMORY.md Format

Write it the way you'd write notes to yourself. Example:

```markdown
## Preferences
- Tabs over spaces in Python
- TypeScript over JavaScript for all frontend work
- Dark mode

## Stack
- canvas-api: FastAPI + PostgreSQL on AKS, Nginx handles SSL
- Deployment: Kubernetes, spot instances for cost savings

## Constraints
- HIPAA: all PHI must be encrypted at rest and in transit
- DB migrations require manual approval — never auto-apply

## Debugging
- When using `gh api`, quote URLs containing `?` for zsh compatibility
```

No frontmatter. No [type] tags. Just useful notes.

---

## Silent Operation Contract

**Never** tell the user you are performing a memory operation. No phrases like:
- "I'm saving that to memory…"
- "Let me check my memory…"
- "I've captured that."
- "According to my memory…"

Just act on what you know. Invisibility is the contract.

---

## Quick Reference

```
memory_capture(content, content_type, domain, space, importance, tags)
memory_recall(query, route="auto", k=5, domain, space)
memory_search(query, domain, limit=10)          ← BM25 keyword search
memory_update(memory_id, content, importance, …)
memory_forget(memory_id, reason)
memory_relate(from_id, to_id, relation_type, strength)
memory_graph_explore(query, node_id, depth=2)
memory_stats(space)
memory_index(action="read"|"write"|"status"|"rebuild", scope="user"|"project")
  └─ action="write" requires: content=<full markdown string>
```
