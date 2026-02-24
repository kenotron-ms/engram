# tool-memory-search Design

**Date:** 2026-02-24
**Status:** Approved

## Problem

Amplifier users have no registered tool for explicit memory search. When auto-retrieval (via `hooks-protocol-reminder`) misses something, agents fall back to plain `grep`, which lacks YAML frontmatter awareness and domain-scoped search. We need a proper Amplifier tool so the LLM can invoke search explicitly when needed.

## Decision

Create a new `tool-memory-search` Amplifier tool module. The tool becomes the canonical owner of the search logic (`_search.py`). The existing hook becomes a thin consumer that imports from the tool module.

## Architecture

```
modules/
├── tool-memory-search/
│   ├── pyproject.toml
│   └── amplifier_module_tool_memory_search/
│       ├── __init__.py       # MemorySearchTool class + mount()
│       └── _search.py        # MOVED from hooks-protocol-reminder
│
└── hooks-protocol-reminder/
    ├── pyproject.toml        # dep on tool-memory-search; amplifier-core removed from [project]
    └── amplifier_module_hooks_protocol_reminder/
        ├── __init__.py       # imports from amplifier_module_tool_memory_search._search
        └── _search.py        # DELETED
```

## Tool Interface

```
memory_search(query: str, memory_base: "project" | "user" | "both")
```

The tool extracts keywords from the query string and searches the appropriate canvas memory paths, returning matching entries with relevance context.

## Key Constraints

- `amplifier-core` is a **peer dep** — must NOT appear in `[project] dependencies`, only in `[dependency-groups] dev`
- Entry point key in `pyproject.toml` must exactly match `module:` value in YAML (`tool-memory-search`)
- Tool uses duck typing — no base class, just `name`, `description`, `input_schema` properties + `async execute()`
- `execute()` must never raise — always return `ToolResult(success=False, error=...)`

## Alternatives Rejected

- **Shared lib package**: Unnecessary indirection — two packages in the same repo sharing a third is complexity without benefit
- **Subprocess to script**: Fragile, adds encoding/quoting edge cases, regressive given we already have a Python module
