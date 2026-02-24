# tool-memory-search Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Register a proper Amplifier tool for explicit memory search so agents never fall back to grep.

**Architecture:** Create `modules/tool-memory-search/` implementing the Amplifier Tool protocol. Move `_search.py` from `hooks-protocol-reminder` into the new tool module (tool is the source of truth). The hook imports from the tool module. Wire the tool into `behaviors/engram.yaml`.

**Tech Stack:** Python 3.11+, hatchling, pytest, pytest-asyncio, amplifier-core (peer dep)

**Design doc:** `docs/plans/2026-02-24-tool-memory-search-design.md`

---

### Task 1: Create `tool-memory-search` module skeleton with failing test

**Files:**
- Create: `modules/tool-memory-search/pyproject.toml`
- Create: `modules/tool-memory-search/amplifier_module_tool_memory_search/__init__.py`
- Create: `modules/tool-memory-search/tests/__init__.py`
- Create: `modules/tool-memory-search/tests/test_tool.py`

**Step 1: Create the directory structure**

```bash
mkdir -p modules/tool-memory-search/amplifier_module_tool_memory_search
mkdir -p modules/tool-memory-search/tests
touch modules/tool-memory-search/tests/__init__.py
```

**Step 2: Create `modules/tool-memory-search/pyproject.toml`**

```toml
[project]
name = "amplifier-module-tool-memory-search"
version = "0.1.0"
description = "Explicit memory search tool for Canvas Memory system"
requires-python = ">=3.11"
dependencies = []

[project.entry-points."amplifier.modules"]
tool-memory-search = "amplifier_module_tool_memory_search:mount"

[tool.pyright]
reportMissingImports = false

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[tool.uv]
package = true

[tool.hatch.metadata]
allow-direct-references = true

[dependency-groups]
dev = [
    "amplifier-core @ git+https://github.com/microsoft/amplifier-core@main",
    "pytest>=8.0.0",
    "pytest-asyncio>=0.24.0",
]

[tool.pytest.ini_options]
asyncio_mode = "auto"
```

Note: `amplifier-core` is a **peer dependency** — it belongs only in `[dependency-groups] dev`, never in `[project] dependencies`. The host process provides it at runtime.

**Step 3: Create stub `modules/tool-memory-search/amplifier_module_tool_memory_search/__init__.py`**

```python
from typing import Any


class MemorySearchTool:
    @property
    def name(self) -> str:
        return "memory_search"

    @property
    def description(self) -> str:
        return "Search canvas memory store for relevant memories."

    @property
    def input_schema(self) -> dict:
        return {
            "type": "object",
            "properties": {
                "query": {"type": "string"},
            },
            "required": ["query"],
        }

    async def execute(self, input: dict[str, Any]) -> Any:
        raise NotImplementedError


async def mount(coordinator: Any, config: dict) -> MemorySearchTool:
    tool = MemorySearchTool()
    await coordinator.mount("tools", tool, name=tool.name)
    return tool
```

**Step 4: Write the failing tests in `modules/tool-memory-search/tests/test_tool.py`**

```python
import pytest
from amplifier_module_tool_memory_search import MemorySearchTool


@pytest.fixture
def tool():
    return MemorySearchTool()


def test_name(tool):
    assert tool.name == "memory_search"


def test_description_not_empty(tool):
    assert tool.description.strip() != ""


def test_input_schema_requires_query(tool):
    schema = tool.input_schema
    assert "query" in schema["properties"]
    assert "query" in schema["required"]


def test_input_schema_memory_base_enum(tool):
    schema = tool.input_schema
    assert "memory_base" in schema["properties"]
    assert schema["properties"]["memory_base"]["enum"] == ["project", "user", "both"]


@pytest.mark.asyncio
async def test_execute_missing_query_returns_failure(tool):
    result = await tool.execute({})
    assert result.success is False
    assert "query" in result.error["message"].lower()


@pytest.mark.asyncio
async def test_execute_empty_query_returns_failure(tool):
    result = await tool.execute({"query": "   "})
    assert result.success is False


@pytest.mark.asyncio
async def test_execute_returns_tool_result(tool, tmp_path):
    # Give the tool a real (empty) memory dir so search completes without error
    tool._user_memory_base = str(tmp_path)
    tool._project_memory_base = str(tmp_path)
    result = await tool.execute({"query": "test query", "memory_base": "both"})
    assert result.success is True
```

**Step 5: Run tests to verify they fail**

```bash
cd modules/tool-memory-search
uv run pytest tests/ -v
```

Expected: multiple FAILED — `NotImplementedError`, missing `memory_base` enum, etc.

**Step 6: Commit the skeleton**

```bash
git add modules/tool-memory-search/
git commit -m "feat: scaffold tool-memory-search module with failing tests"
```

---

### Task 2: Move `_search.py` and implement `MemorySearchTool`

**Files:**
- Create: `modules/tool-memory-search/amplifier_module_tool_memory_search/_search.py` (moved)
- Modify: `modules/tool-memory-search/amplifier_module_tool_memory_search/__init__.py`

**Step 1: Copy `_search.py` to the tool module**

```bash
cp modules/hooks-protocol-reminder/amplifier_module_hooks_protocol_reminder/_search.py \
   modules/tool-memory-search/amplifier_module_tool_memory_search/_search.py
```

Do not delete the original yet — that happens in Task 3 after the hook is updated.

**Step 2: Replace `modules/tool-memory-search/amplifier_module_tool_memory_search/__init__.py`**

```python
from __future__ import annotations

from pathlib import Path
from typing import Any

from amplifier_core import ToolResult

from ._search import extract_keywords, search_memory


class MemorySearchTool:
    def __init__(
        self,
        user_memory_base: str = "~/.canvas/memory",
        project_memory_base: str = ".canvas/memory",
    ) -> None:
        self._user_memory_base = user_memory_base
        self._project_memory_base = project_memory_base

    @property
    def name(self) -> str:
        return "memory_search"

    @property
    def description(self) -> str:
        return (
            "Search the canvas memory store for relevant memories.\n\n"
            "Use this when auto-retrieval didn't surface something you need, "
            "or when you need to look up a specific topic explicitly.\n\n"
            "Input: {\"query\": \"search terms\", \"memory_base\": \"project|user|both\"}\n"
            "Returns: Matching memory entries with relevance context."
        )

    @property
    def input_schema(self) -> dict:
        return {
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Keywords or phrase to search for",
                },
                "memory_base": {
                    "type": "string",
                    "enum": ["project", "user", "both"],
                    "description": "Which memory base to search",
                    "default": "both",
                },
            },
            "required": ["query"],
        }

    async def execute(self, input: dict[str, Any]) -> ToolResult:
        query = input.get("query", "").strip()
        if not query:
            return ToolResult(
                success=False,
                error={"message": "query is required and cannot be empty"},
            )

        memory_base = input.get("memory_base", "both")

        try:
            keywords = extract_keywords(query)
            results: list[dict] = []

            if memory_base in ("user", "both"):
                user_path = Path(self._user_memory_base).expanduser()
                if user_path.exists():
                    results.extend(search_memory(keywords, user_path))

            if memory_base in ("project", "both"):
                project_path = Path(self._project_memory_base).expanduser()
                if project_path.exists():
                    results.extend(search_memory(keywords, project_path))

            return ToolResult(success=True, output=results)

        except Exception as e:
            return ToolResult(
                success=False,
                error={"message": str(e), "type": type(e).__name__},
            )


async def mount(coordinator: Any, config: dict) -> MemorySearchTool:
    tool = MemorySearchTool(
        user_memory_base=config.get("user_memory_base", "~/.canvas/memory"),
        project_memory_base=config.get("project_memory_base", ".canvas/memory"),
    )
    await coordinator.mount("tools", tool, name=tool.name)
    return tool
```

**Step 3: Run tests to verify they pass**

```bash
cd modules/tool-memory-search
uv run pytest tests/ -v
```

Expected: all PASSED.

**Step 4: Commit**

```bash
git add modules/tool-memory-search/
git commit -m "feat: implement MemorySearchTool with moved _search.py"
```

---

### Task 3: Update `hooks-protocol-reminder` to depend on the tool module

**Files:**
- Modify: `modules/hooks-protocol-reminder/pyproject.toml`
- Modify: `modules/hooks-protocol-reminder/amplifier_module_hooks_protocol_reminder/__init__.py`
- Delete: `modules/hooks-protocol-reminder/amplifier_module_hooks_protocol_reminder/_search.py`

**Step 1: Write a failing test that imports search from the tool module**

Add to the hook module's test file (create `modules/hooks-protocol-reminder/tests/test_imports.py` if it doesn't exist):

```python
def test_search_imported_from_tool_module():
    # This import should come from the tool module, not a local _search.py
    from amplifier_module_tool_memory_search._search import extract_keywords, search_memory
    assert callable(extract_keywords)
    assert callable(search_memory)
```

Run: `cd modules/hooks-protocol-reminder && uv run pytest tests/ -v`
Expected: PASS (the tool module is importable once installed). If it fails with ImportError, install the tool module first (Step 2).

**Step 2: Update `modules/hooks-protocol-reminder/pyproject.toml`**

Replace the current content:

```toml
[project]
name = "amplifier-module-hooks-protocol-reminder"
version = "0.1.0"
description = "Memory protocol reminder hook for Canvas Memory system"
requires-python = ">=3.11"
dependencies = [
    "amplifier-module-tool-memory-search @ git+https://github.com/kenotron-ms/engram@main#subdirectory=modules/tool-memory-search",
]

[project.entry-points."amplifier.modules"]
hooks-protocol-reminder = "amplifier_module_hooks_protocol_reminder:mount"

[tool.pyright]
reportMissingImports = false

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[tool.uv]
package = true

[tool.hatch.metadata]
allow-direct-references = true

[dependency-groups]
dev = [
    "amplifier-core @ git+https://github.com/microsoft/amplifier-core@main",
    "pytest>=8.0.0",
    "pytest-asyncio>=0.24.0",
]
```

Two changes from the original:
1. Remove `amplifier-core>=0.8.0` from `[project] dependencies` (it's a peer dep — should not be declared here)
2. Add `amplifier-module-tool-memory-search` as a runtime dependency

**Step 3: Update the import in `modules/hooks-protocol-reminder/amplifier_module_hooks_protocol_reminder/__init__.py`**

Find line 18:
```python
from ._search import extract_keywords, search_memory
```

Replace with:
```python
from amplifier_module_tool_memory_search._search import extract_keywords, search_memory
```

**Step 4: Delete the now-redundant `_search.py`**

```bash
git rm modules/hooks-protocol-reminder/amplifier_module_hooks_protocol_reminder/_search.py
```

**Step 5: Run the hook tests to confirm nothing broke**

```bash
cd modules/hooks-protocol-reminder
uv run pytest tests/ -v
```

Expected: all PASSED.

**Step 6: Commit**

```bash
git add modules/hooks-protocol-reminder/
git commit -m "refactor: hook imports search from tool module; remove local _search.py"
```

---

### Task 4: Wire tool into `behaviors/engram.yaml` and update `AGENTS.md`

**Files:**
- Modify: `behaviors/engram.yaml`
- Modify: `AGENTS.md`

**Step 1: Add the `tools:` section to `behaviors/engram.yaml`**

Insert before the `hooks:` section:

```yaml
tools:
  - module: tool-memory-search
    source: git+https://github.com/kenotron-ms/engram@main#subdirectory=modules/tool-memory-search
    config:
      project_memory_base: ".canvas/memory"
      user_memory_base: "~/.canvas/memory"
```

The module ID `tool-memory-search` must exactly match the entry point key in `pyproject.toml`.

**Step 2: Update the manual search fallback in `AGENTS.md`**

Find the manual search section (currently around line 84-87):
```bash
grep -r "term" ~/.canvas/memory/information/{domain}/
```

Replace with:
```markdown
**Manual search:** Use the `memory_search` tool with a specific query rather than falling back to `grep`. The tool is YAML-frontmatter-aware and handles domain scoping.

If the tool is unavailable for any reason:
```bash
grep -r "term" ~/.canvas/memory/information/{domain}/
```
```

**Step 3: Commit**

```bash
git add behaviors/engram.yaml AGENTS.md
git commit -m "feat: wire tool-memory-search into engram behavior; update AGENTS.md"
```
