# engram-lite

**Persistent, vector-backed memory for AI agents.**

![Python 3.11+](https://img.shields.io/badge/python-3.11%2B-blue)
![License: MIT](https://img.shields.io/badge/license-MIT-green)
![PyPI](https://img.shields.io/badge/pypi-engram--lite-orange)

---

## Quick start

### Amplifier — add the behavior to your bundle

Add one line to your `bundle.md` and Amplifier handles installation automatically:

```yaml
includes:
  - bundle: git+https://github.com/kenotron-ms/engram-lite@main#subdirectory=behaviors/engram-lite.yaml
```

That's it. Memory is active the next time you run `amplifier run`. The behavior wires up the hook (RETRIEVE → RESPOND → CAPTURE loop), the memory tools, and the behavioral protocol — all in one include.

### Claude Code — zero install

No `pip install` required. Register the MCP server and Claude Code handles the rest:

```bash
# Option A — register directly
claude mcp add --transport stdio engram-lite -- \
  uvx --from git+https://github.com/kenotron-ms/engram-lite engram-lite-mcp

# Option B — copy .mcp.json into your project root
curl -sO https://raw.githubusercontent.com/kenotron-ms/engram-lite/main/.mcp.json
```

On first use, `uvx` downloads and caches the package. Subsequent sessions start in under a second.

### Initialize MEMORY.md (optional, one-time)

```bash
# Sets up ~/.engram/ and .engram/ with blank MEMORY.md files.
uvx --from git+https://github.com/kenotron-ms/engram-lite engram-lite init
```

---

## What is engram-lite?

engram-lite gives AI agents persistent memory that follows you across sessions, stored locally in SQLite. Instead of starting every conversation as a blank slate, the agent remembers your preferences, past decisions, project context, and working patterns — and applies them silently, without announcing that it's doing so. Everything stays on your machine in a single database file per space, backed by [sqlite-vec](https://github.com/asg017/sqlite-vec) for vector search and FTS5 for keyword search. It works as both an [Amplifier](https://github.com/microsoft/amplifier) module and a Claude Code plugin.

## How it works

engram-lite operates through a silent **RETRIEVE → RESPOND → CAPTURE** behavioral loop, injected automatically via platform hooks:

```
  User sends prompt
        │
        ▼
  ┌─────────────┐    Hook silently injects recall reminder
  │  RETRIEVE   │──▶ Agent calls memory_recall / memory_search
  └──────┬──────┘    Relevant memories loaded into context
         │
         ▼
  ┌─────────────┐    Agent responds using conversation + memories
  │  RESPOND    │──▶ Memory operations are never mentioned to user
  └──────┬──────┘
         │
         ▼
  ┌─────────────┐    Hook silently injects capture reminder
  │  CAPTURE    │──▶ Agent evaluates what's worth remembering
  └─────────────┘    New knowledge stored with embeddings + graph links
```

Retrieval uses a **dual-route architecture** adapted from the Mnemis paper:

- **System-1 (fast path)** — Vector KNN similarity + BM25 full-text search, fused via Reciprocal Rank Fusion (RRF). Handles specific queries like *"what port does the API run on?"*
- **System-2 (deliberate path)** — Hierarchical graph traversal across a semantic taxonomy. Handles broad queries like *"summarize all the security decisions we've made."*

The system auto-selects the route per query, or you can force one via the `route` parameter.

---

## Memory tools reference

All tools return structured JSON. The agent calls them via native function calling — you never need to invoke them manually.

| Tool | Description |
|------|-------------|
| `memory_capture` | Store new knowledge with embeddings, tags, domain routing, and content type classification. Deduplicates against existing memories (cosine > 0.95 triggers merge). |
| `memory_recall` | Dual-route retrieval. Supports `route`: `auto`, `vector`, `graph`, `hybrid`, `keyword`. Returns ranked memories with scores and related memories. |
| `memory_search` | Quick System-1 search with filters. Simpler interface than `memory_recall` for targeted lookups. |
| `memory_update` | Update an existing memory's content, tags, importance, or confidence score. |
| `memory_relate` | Create typed edges between memories: `relates-to`, `supports`, `contradicts`, `supersedes`, `exemplifies`, `part-of`, `caused-by`, `decided-in`, `applies-to`. |
| `memory_forget` | Soft-delete a memory (excluded from retrieval but retained). Pass `hard=true` to permanently remove all data including embeddings. |
| `memory_graph_explore` | Browse the hierarchical knowledge graph. Start from a query or a specific node ID and traverse the taxonomy. |
| `memory_stats` | Summarize the memory store: counts by space/type/domain, storage size, graph node count, most and least accessed memories. |

### Tool signatures

```
memory_capture(content, type?, tags?, domain?, space?, importance?)  → memory_id
memory_recall(query, route?, limit?, domain?, filters?)              → memories[]
memory_search(query, domain?, limit?, filters?)                      → memories[]
memory_update(memory_id, content?, tags?, importance?, confidence?)   → success
memory_relate(from_id, to_id, relation_type, strength?)              → success
memory_forget(memory_id, reason?, hard?)                             → success
memory_graph_explore(query?, node_id?)                               → graph_nodes[]
memory_stats(space?)                                                 → stats{}
```

---

## Configuration

### Environment variables

| Variable | Default | Description |
|----------|---------|-------------|
| `ENGRAM_USER_DB` | `~/.engram/engram.db` | Path to the user-private memory database |
| `ENGRAM_PROJECT_DB` | `.engram/engram.db` | Path to the project-shared memory database (relative to project root) |
| `ENGRAM_EMBEDDING_PROVIDER` | `openai` | Embedding provider: `openai`, `azure`, `ollama` |
| `ENGRAM_EMBEDDING_MODEL` | `text-embedding-3-small` | Embedding model name |
| `ENGRAM_EMBEDDING_DIMENSIONS` | `1536` | Vector dimensions (must match model output) |
| `OPENAI_API_KEY` | — | OpenAI API key (required for default provider) |
| `AZURE_OPENAI_ENDPOINT` | — | Azure OpenAI endpoint URL |
| `AZURE_OPENAI_API_KEY` | — | Azure OpenAI API key |
| `OLLAMA_BASE_URL` | `http://localhost:11434` | Ollama server URL for local embeddings |
| `ENGRAM_HOOKS_ENABLED` | `true` | Enable/disable behavioral hook injection |
| `ENGRAM_CONTEXT_BUDGET` | `500` | Max tokens for session-start context injection |

### Memory content types

Memories are classified into types that affect how they're stored, retrieved, and weighted:

| Type | Use case |
|------|----------|
| `fact` | Concrete knowledge: "The API runs on port 8080" |
| `preference` | User preferences: "I prefer composition over inheritance" |
| `decision` | Architectural or design decisions with rationale |
| `event` | Things that happened: "Deployed v2.3 to production" |
| `skill` | Patterns and techniques the user has demonstrated |
| `entity` | People, projects, services, tools |
| `relationship` | Connections between entities |

---

## Dual-space memory

engram-lite maintains two separate SQLite databases — one private to you, one shareable with your team:

```
~/.engram/                          <project-root>/.engram/
├── engram.db     ← USER SPACE      ├── engram.db     ← PROJECT SPACE
│                                    │
│  Your preferences                  │  Architecture decisions
│  Personal workflow habits           │  Project conventions
│  Cross-project knowledge            │  Team patterns
│  People & relationships             │  Why-we-chose-X rationale
│                                    │
│  NEVER leaves your machine          │  Safe to commit to git
│  NEVER leaks into project space     │  Shared via version control
```

**How space is selected:**

- Personal preferences, bio, constraints, people → always `user` space
- Project decisions, architecture, context → `project` space (if available)
- Professional knowledge → `user` space by default, unless project-specific
- You can always override with `space="user"` or `space="project"` explicitly

**Privacy gate:** Content written to project space must pass the "README test" — would this be safe in a public README? PII, credentials, and private opinions are rejected or routed to user space automatically.

---

## Dual-route retrieval

The retrieval architecture is adapted from the [Mnemis](https://arxiv.org/abs/2602.15313) dual-route model. Where Mnemis targets large-scale enterprise memory systems, engram-lite adapts the core ideas for individual developer sessions with a local SQLite backend.

### System-1: fast similarity (vector + BM25)

For specific, focused queries. Embeds the query, runs KNN against stored vectors via sqlite-vec, runs BM25 full-text search via FTS5, and fuses both ranked lists using Reciprocal Rank Fusion. Fast (< 100ms for 50K memories) and precise.

### System-2: hierarchical graph traversal

For broad, structural queries. Memories are organized into a hierarchical semantic graph (domain taxonomy). System-2 starts from query-matched graph nodes and walks the hierarchy top-down, collecting structurally related memories that vector search alone would miss. Slower (< 300ms) but comprehensive.

### Auto-routing

The query analyzer examines each query and selects the best route:

| Query pattern | Route selected | Example |
|---------------|---------------|---------|
| Specific lookup | System-1 | "What test framework do we use?" |
| Broad domain sweep | System-1 + System-2 | "All security considerations" |
| Exact term match | Keyword (BM25-only) | "HIPAA compliance" |
| Exploratory | System-2 | "What do we know about auth?" |

Results from both routes are deduplicated and re-ranked, with temporal recency, importance, and access frequency factored into the final score.

---

## Privacy

engram-lite is designed to keep your data local.

**What stays on your machine:**
- All memory content (both user and project databases)
- The knowledge graph structure
- Tags, relations, metadata
- BM25 full-text index

**What goes over the network:**
- Text sent to the embedding API for vector generation (OpenAI `text-embedding-3-small` by default)
- Only the formatted embedding input is sent: `"{content_type}: {summary}\n\n{content[:512]}"`
- No memory IDs, metadata, tags, or graph structure are transmitted

**To keep everything local**, use Ollama for embeddings:

```bash
# Run a local embedding model
ollama pull nomic-embed-text

# Configure engram-lite to use it
export ENGRAM_EMBEDDING_PROVIDER=ollama
export ENGRAM_EMBEDDING_MODEL=nomic-embed-text
export ENGRAM_EMBEDDING_DIMENSIONS=768
```

With Ollama, zero data leaves your machine. Retrieval quality is slightly lower than OpenAI's models but fully functional.

---

## Project structure

```
engram-lite/
├── behaviors/
│   └── engram-lite.yaml             # Behavior bundle (hooks + tools + context)
├── context/
│   ├── memory-instructions.md       # Behavioral protocol injected at session start
│   └── memory-awareness.md          # Tool awareness context
├── bundle.md                        # Standalone root bundle (Amplifier)
├── amplifier_module_engram_lite/
│   ├── core/                        # Shared core library
│   │   ├── storage.py               #   SQLite + sqlite-vec database layer
│   │   ├── retrieval.py             #   Dual-route retrieval engine
│   │   ├── capture.py               #   Capture pipeline (embed, classify, dedup)
│   │   ├── graph.py                 #   Hierarchical knowledge graph
│   │   ├── embeddings.py            #   Embedding provider abstraction
│   │   └── models.py                #   Data models and schemas
│   ├── amplifier_hook/              # Amplifier hook module
│   │   ├── __init__.py              #   mount() + hook handlers
│   │   └── config.py                #   Hook configuration
│   ├── amplifier_tool/              # Amplifier tool module
│   │   ├── __init__.py              #   mount() + tool registration
│   │   └── schemas.py               #   JSON schemas for tool params
│   ├── mcp/                         # Claude Code MCP server
│   │   └── server.py                #   MCP tool handlers (stdio transport)
│   └── cli.py                       # CLI entry point
├── claude-code/                     # Claude Code plugin artifacts
│   ├── .claude-plugin/
│   │   └── plugin.json              #   Plugin manifest
│   └── .claude/
│       └── commands/                #   Slash commands
├── tests/
├── docs/
│   ├── ARCHITECTURE.md
│   ├── PRD.md
│   └── SPEC-*.md                    # Detailed specifications
└── pyproject.toml
```

---

## Development

### Setup

```bash
git clone https://github.com/kenotron-ms/engram-lite.git
cd engram-lite
uv venv && uv pip install -e ".[dev]"
```

### Run tests

```bash
pytest
pytest --cov=amplifier_module_engram_lite
```

### Lint and type-check

```bash
ruff check src/
ruff format src/
pyright src/
```

### Architecture docs

The `docs/` directory contains detailed specifications for every subsystem:

- `ARCHITECTURE.md` — System overview and component design
- `PRD.md` — Product requirements and user stories
- `SPEC-STORAGE.md` — SQLite schema, spaces, migrations
- `SPEC-RETRIEVAL.md` — Dual-route retrieval engine
- `SPEC-TOOLS.md` — All 8 tool APIs with schemas
- `SPEC-HOOKS.md` — Platform hook integration
- `SPEC-EMBEDDINGS.md` — Embedding providers and configuration
- `SPEC-AMPLIFIER-BUNDLE.md` — Amplifier bundle structure
- `SPEC-CLAUDE-CODE-PLUGIN.md` — Claude Code plugin structure
- `SPEC-PROTOCOLS.md` — Behavioral protocols
- `SPEC-TAGGING.md` — Tag and keyword systems

---

## License

MIT. See [LICENSE](LICENSE) for details.

---

## Citation

If you use engram-lite in your work, please cite the Mnemis paper that inspired the dual-route retrieval architecture:

```bibtex
@article{tang2026mnemis,
  title={Mnemis: Dual-Route Retrieval on Hierarchical Graphs for Long-Term LLM Memory},
  author={Tang, Zihao and Yu, Xin and Xiao, Ziyu and Wen, Zengxuan and Li, Zelin and Zhou, Jiaxi and Wang, Hualei and Wang, Haohua and Huang, Haizhen and Deng, Weiwei and Sun, Feng and Zhang, Qi},
  journal={arXiv preprint arXiv:2602.15313},
  year={2026}
}
```

**Paper:** [arXiv:2602.15313](https://arxiv.org/abs/2602.15313) |
**DOI:** [10.48550/arXiv.2602.15313](https://doi.org/10.48550/arXiv.2602.15313)

---

## Acknowledgements

**Dual-route retrieval architecture** adapted from
[Mnemis: Dual-Route Retrieval on Hierarchical Graphs for Long-Term LLM Memory](https://arxiv.org/abs/2602.15313)
by Zihao Tang, Xin Yu, Ziyu Xiao, Zengxuan Wen, Zelin Li, Jiaxi Zhou, Hualei Wang,
Haohua Wang, Haizhen Huang, Weiwei Deng, Feng Sun, and Qi Zhang
(arXiv:2602.15313, 2026).

**Behavioral protocol patterns** (hook injection, silent RETRIEVE-RESPOND-CAPTURE loop,
dual-space memory) inspired by [Engram](https://github.com/kenotron-ms/engram) —
a file-based memory system for AI agents.

engram-lite is a **clean-room implementation**. It does not copy code from Engram or Mnemis —
only borrows design ideas.
