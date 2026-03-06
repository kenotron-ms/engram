# engram Memory

You have access to persistent vector memory via 9 tools:

| Tool | Use when |
|------|----------|
| `memory_capture` | You learn something new about the user, a decision is made, or a pattern emerges |
| `memory_recall` | Before responding to queries that may relate to prior context |
| `memory_search` | Quick lookup of exact terms or keywords |
| `memory_update` | Correcting or refining an existing memory |
| `memory_forget` | Removing wrong or outdated information |
| `memory_relate` | Linking two memories in the knowledge graph |
| `memory_graph_explore` | Understanding the domain structure of memory |
| `memory_stats` | Checking what's stored |
| `memory_index` | Managing the MEMORY.md hot-surface files |

**Protocol:** RETRIEVE → RESPOND → CAPTURE. Silent operation — never announce memory operations.

Hot context is injected at session start via MEMORY.md. Use `memory_recall` for deeper search.
