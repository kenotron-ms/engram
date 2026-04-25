# Engram — Phase 5: Amplifier Integration Bundle

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** Replace the existing Python Amplifier hooks with three new modules that delegate all intelligence to the `engram` CLI: `hook-memory-context` (session start awareness injection), `hook-memory-observe` (session end transcript processing), `tool-memory` (on-demand recall tools).

**Architecture:** Three Python modules in a new `modules/` directory, each with its own `pyproject.toml`. Every module uses the `mount(coordinator, config)` pattern required by the Amplifier module protocol. All intelligence lives in the Rust `engram` binary — the Python layer is nothing but `subprocess.run()` wrappers. `behaviors/engram.yaml` is updated to load the three new modules; the old `engram-hook` and `engram-tool` entries are removed (their packages remain in the repo but are no longer loaded by default).

**Tech Stack:** Python 3.11+, hatchling (build), amplifier-core (peer dep — not declared in pyproject), subprocess (stdlib), pytest + pytest-asyncio + unittest.mock (tests)

---

## Codebase Orientation

Before starting, understand these existing patterns:

**Amplifier module protocol** (`creating-amplifier-modules` skill):
- Every module exposes `async mount(coordinator, config)` — this is the Iron Law.
- For **tools**: call `await coordinator.mount("tools", tool, name=tool.name)` inside `mount()`. Return a metadata dict.
- For **hooks**: call `coordinator.hooks.register(event, handler, priority=..., name=...)` inside `mount()`. Return a metadata dict.
- `amplifier-core` is a **peer dependency** — do NOT list it under `[project.dependencies]` in `pyproject.toml`. Import it inside functions with `try/except ImportError` so tests work without it.
- Entry point wires the module ID to `mount`: `hook-memory-context = "amplifier_module_hook_memory_context:mount"`.

**Existing hook module** (`amplifier_module_engram_hook/__init__.py`):
- Shim re-exporting `mount` from `amplifier_module_engram.hooks.amplifier_hook`.
- That file uses `coordinator.hooks.register(event, handler, priority=..., name=...)`.
- `_hook_result()` wraps the try/except pattern for `HookResult`.

**Existing tool module** (`amplifier_module_engram_tool/__init__.py`):
- Shim re-exporting `mount` from `amplifier_module_engram.tools.amplifier_tool`.
- That file calls `coordinator.mount_points["tools"][tool.name] = tool` (older API).
- The new modules use `await coordinator.mount("tools", tool, name=tool.name)` (current API per skill).

**`behaviors/engram.yaml`** (the file to update in Task 8):
- Currently references `module: engram-hook` and `module: engram-tool`.
- After Task 8 it references the three new module IDs.

**`bundle.md`** (the bundle root):
- Currently at version `0.2.0`.
- After Task 8: bump to `0.3.0`.

**Tests** live in `tests/` (root level for existing tests). Each new module gets its own `tests/` under its module directory. Run with `pytest tests/ -v` from inside the module directory after `pip install -e .`.

---

## File Structure Being Created

```
modules/
├── hook-memory-context/
│   ├── pyproject.toml
│   ├── tests/
│   │   ├── __init__.py
│   │   └── test_hook_memory_context.py
│   └── amplifier_module_hook_memory_context/
│       └── __init__.py
├── hook-memory-observe/
│   ├── pyproject.toml
│   ├── tests/
│   │   ├── __init__.py
│   │   └── test_hook_memory_observe.py
│   └── amplifier_module_hook_memory_observe/
│       └── __init__.py
└── tool-memory/
    ├── pyproject.toml
    ├── tests/
    │   ├── __init__.py
    │   └── test_tool_memory.py
    └── amplifier_module_tool_memory/
        └── __init__.py
```

Also modified:
- `behaviors/engram.yaml` — swap hook/tool modules (Task 8)
- `bundle.md` — version bump to 0.3.0 (Task 8)

---

## Task 1: Create hook-memory-context module structure

**Files:**
- Create: `modules/hook-memory-context/pyproject.toml`
- Create: `modules/hook-memory-context/amplifier_module_hook_memory_context/__init__.py`

**Step 1: Create directory structure**

```bash
mkdir -p modules/hook-memory-context/amplifier_module_hook_memory_context
mkdir -p modules/hook-memory-context/tests
touch modules/hook-memory-context/tests/__init__.py
```

**Step 2: Write `modules/hook-memory-context/pyproject.toml`**

```toml
[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[project]
name = "amplifier-module-hook-memory-context"
version = "0.1.0"
description = "Engram — inject memory context before each prompt via engram CLI"
readme = "README.md"
license = { text = "MIT" }
requires-python = ">=3.11"
dependencies = []   # amplifier-core is a peer dep — do NOT declare it here

[project.optional-dependencies]
dev = ["pytest>=8.0", "pytest-asyncio>=0.23"]

[project.entry-points."amplifier.modules"]
hook-memory-context = "amplifier_module_hook_memory_context:mount"

[tool.hatch.build.targets.wheel]
packages = ["amplifier_module_hook_memory_context"]

[tool.pytest.ini_options]
testpaths = ["tests"]
asyncio_mode = "auto"
asyncio_default_fixture_loop_scope = "function"
```

**Step 3: Write protocol-compliant stub `modules/hook-memory-context/amplifier_module_hook_memory_context/__init__.py`**

This stub satisfies the Amplifier module protocol immediately so the module is safe to load. Task 2 replaces the no-op handler with the real `engram` CLI call.

```python
"""Amplifier hook module — injects engram memory context before each prompt."""

from __future__ import annotations

from typing import Any

__amplifier_module_type__ = "hook"


async def mount(coordinator: Any, config: dict[str, Any] | None = None) -> dict[str, Any]:
    """Mount the memory-context hook onto the Amplifier coordinator (stub)."""
    config = config or {}
    priority = config.get("priority", 5)

    async def handle(event: str, data: dict[str, Any]) -> Any:
        return _hook_result(action="noop")

    coordinator.hooks.register(
        "provider:request",
        handle,
        priority=priority,
        name="hook-memory-context",
    )
    return {
        "name": "hook-memory-context",
        "version": "0.1.0",
        "provides": [],
    }


def _hook_result(
    action: str,
    context_injection: str | None = None,
    ephemeral: bool = True,
    suppress_output: bool = True,
) -> Any:
    """Build a HookResult, falling back to a plain dict when amplifier-core is absent."""
    try:
        from amplifier_core import HookResult  # type: ignore[import-not-found]

        return HookResult(
            action=action,
            context_injection=context_injection,
            context_injection_role="system",
            ephemeral=ephemeral,
            suppress_output=suppress_output,
        )
    except ImportError:
        return {
            "action": action,
            "context_injection": context_injection,
            "context_injection_role": "system",
            "ephemeral": ephemeral,
            "suppress_output": suppress_output,
        }
```

**Step 4: Install and verify importable**

```bash
cd modules/hook-memory-context
pip install -e .
python -c "from amplifier_module_hook_memory_context import mount; print('OK')"
```

Expected: `OK`

**Step 5: Commit**

```bash
git add modules/hook-memory-context/
git commit -m "feat: scaffold hook-memory-context module with protocol-compliant stub"
```

---

## Task 2: Implement hook-memory-context with TDD

**Files:**
- Create: `modules/hook-memory-context/tests/test_hook_memory_context.py`
- Modify: `modules/hook-memory-context/amplifier_module_hook_memory_context/__init__.py`

**Step 1: Write the failing test**

Create `modules/hook-memory-context/tests/test_hook_memory_context.py`:

```python
"""Tests for hook-memory-context — mocks subprocess, no engram binary needed."""

import subprocess
from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from amplifier_module_hook_memory_context import mount


def _make_coordinator():
    coordinator = MagicMock()
    coordinator.hooks = MagicMock()
    coordinator.hooks.register = MagicMock()
    return coordinator


# ---------------------------------------------------------------------------
# mount() contract
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_mount_registers_hook_on_provider_request():
    """mount() must register a handler for 'provider:request'."""
    coordinator = _make_coordinator()

    result = await mount(coordinator)

    coordinator.hooks.register.assert_called_once()
    event_name = coordinator.hooks.register.call_args[0][0]
    assert event_name == "provider:request"


@pytest.mark.asyncio
async def test_mount_returns_metadata_dict():
    """mount() returns a non-None dict with 'name' key."""
    coordinator = _make_coordinator()

    result = await mount(coordinator)

    assert result is not None
    assert result["name"] == "hook-memory-context"


@pytest.mark.asyncio
async def test_mount_respects_priority_config():
    """mount() passes config priority to hooks.register."""
    coordinator = _make_coordinator()

    await mount(coordinator, config={"priority": 10})

    call_kwargs = coordinator.hooks.register.call_args[1]
    assert call_kwargs["priority"] == 10


# ---------------------------------------------------------------------------
# handler behaviour
# ---------------------------------------------------------------------------


async def _get_handler(config=None):
    """Helper: mount and return the registered handler callable."""
    coordinator = _make_coordinator()
    await mount(coordinator, config=config)
    return coordinator.hooks.register.call_args[0][1]


@pytest.mark.asyncio
async def test_handler_calls_engram_load_with_correct_args():
    """Handler calls `engram load --format=context` via subprocess.run."""
    handler = await _get_handler()

    with patch("subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(returncode=0, stdout="context from engram\n")
        await handler("provider:request", {})

    mock_run.assert_called_once_with(
        ["engram", "load", "--format=context"],
        capture_output=True,
        text=True,
        timeout=5,
    )


@pytest.mark.asyncio
async def test_handler_injects_context_on_success():
    """Handler returns inject_context action when engram outputs text."""
    handler = await _get_handler()

    with patch("subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(returncode=0, stdout="  memory context here  ")
        result = await handler("provider:request", {})

    assert result["action"] == "inject_context"
    assert result["context_injection"] == "memory context here"
    assert result["ephemeral"] is True
    assert result["suppress_output"] is True


@pytest.mark.asyncio
async def test_handler_noop_on_empty_stdout():
    """Handler returns noop when engram returns whitespace-only stdout."""
    handler = await _get_handler()

    with patch("subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(returncode=0, stdout="   \n")
        result = await handler("provider:request", {})

    assert result["action"] == "noop"


@pytest.mark.asyncio
async def test_handler_noop_on_nonzero_returncode():
    """Handler returns noop when engram exits with non-zero code."""
    handler = await _get_handler()

    with patch("subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(returncode=1, stdout="error output")
        result = await handler("provider:request", {})

    assert result["action"] == "noop"


@pytest.mark.asyncio
async def test_handler_noop_on_binary_missing():
    """Handler returns noop silently when engram binary is not found."""
    handler = await _get_handler()

    with patch("subprocess.run", side_effect=FileNotFoundError):
        result = await handler("provider:request", {})

    assert result["action"] == "noop"


@pytest.mark.asyncio
async def test_handler_noop_on_timeout():
    """Handler returns noop silently on subprocess timeout."""
    handler = await _get_handler()

    with patch("subprocess.run", side_effect=subprocess.TimeoutExpired(["engram"], 5)):
        result = await handler("provider:request", {})

    assert result["action"] == "noop"
```

**Step 2: Run test to verify it fails**

```bash
cd modules/hook-memory-context
pytest tests/test_hook_memory_context.py -v
```

Expected: `FAILED` — `test_handler_calls_engram_load_with_correct_args` and `test_handler_injects_context_on_success` fail because the stub always returns `noop`.

**Step 3: Implement `modules/hook-memory-context/amplifier_module_hook_memory_context/__init__.py`**

Replace the entire file:

```python
"""Amplifier hook module — injects engram memory context before each prompt."""

from __future__ import annotations

import subprocess
from typing import Any

__amplifier_module_type__ = "hook"


async def mount(coordinator: Any, config: dict[str, Any] | None = None) -> dict[str, Any]:
    """Mount the memory-context hook onto the Amplifier coordinator.

    Fires on ``provider:request`` (before each LLM call). Calls
    ``engram load --format=context`` and injects the output as a system
    reminder. Fails silently — if engram is missing or slow, returns noop.

    Args:
        coordinator: Amplifier coordinator with a ``hooks`` registry.
        config: Optional config dict. Supported keys:
            - ``priority`` (int, default 5): Hook priority ordering.
    """
    config = config or {}
    priority = config.get("priority", 5)

    async def handle(event: str, data: dict[str, Any]) -> Any:
        try:
            result = subprocess.run(
                ["engram", "load", "--format=context"],
                capture_output=True,
                text=True,
                timeout=5,
            )
            if result.returncode == 0 and result.stdout.strip():
                return _hook_result(
                    action="inject_context",
                    context_injection=result.stdout.strip(),
                    ephemeral=True,
                    suppress_output=True,
                )
        except (subprocess.TimeoutExpired, FileNotFoundError):
            pass
        return _hook_result(action="noop")

    coordinator.hooks.register(
        "provider:request",
        handle,
        priority=priority,
        name="hook-memory-context",
    )
    return {
        "name": "hook-memory-context",
        "version": "0.1.0",
        "provides": [],
    }


def _hook_result(
    action: str,
    context_injection: str | None = None,
    ephemeral: bool = True,
    suppress_output: bool = True,
) -> Any:
    """Build a HookResult, falling back to a plain dict when amplifier-core is absent."""
    try:
        from amplifier_core import HookResult  # type: ignore[import-not-found]

        return HookResult(
            action=action,
            context_injection=context_injection,
            context_injection_role="system",
            ephemeral=ephemeral,
            suppress_output=suppress_output,
        )
    except ImportError:
        return {
            "action": action,
            "context_injection": context_injection,
            "context_injection_role": "system",
            "ephemeral": ephemeral,
            "suppress_output": suppress_output,
        }
```

**Step 4: Run test to verify it passes**

```bash
cd modules/hook-memory-context
pytest tests/test_hook_memory_context.py -v
```

Expected: all 7 tests `PASSED`.

**Step 5: Commit**

```bash
git add modules/hook-memory-context/
git commit -m "feat: implement hook-memory-context — calls engram load --format=context"
```

---

## Task 3: Create hook-memory-observe module structure

**Files:**
- Create: `modules/hook-memory-observe/pyproject.toml`
- Create: `modules/hook-memory-observe/amplifier_module_hook_memory_observe/__init__.py`

**Step 1: Create directory structure**

```bash
mkdir -p modules/hook-memory-observe/amplifier_module_hook_memory_observe
mkdir -p modules/hook-memory-observe/tests
touch modules/hook-memory-observe/tests/__init__.py
```

**Step 2: Write `modules/hook-memory-observe/pyproject.toml`**

```toml
[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[project]
name = "amplifier-module-hook-memory-observe"
version = "0.1.0"
description = "Engram — fire-and-forget session transcript observation via engram CLI"
readme = "README.md"
license = { text = "MIT" }
requires-python = ">=3.11"
dependencies = []   # amplifier-core is a peer dep — do NOT declare it here

[project.optional-dependencies]
dev = ["pytest>=8.0", "pytest-asyncio>=0.23"]

[project.entry-points."amplifier.modules"]
hook-memory-observe = "amplifier_module_hook_memory_observe:mount"

[tool.hatch.build.targets.wheel]
packages = ["amplifier_module_hook_memory_observe"]

[tool.pytest.ini_options]
testpaths = ["tests"]
asyncio_mode = "auto"
asyncio_default_fixture_loop_scope = "function"
```

**Step 3: Write protocol-compliant stub `modules/hook-memory-observe/amplifier_module_hook_memory_observe/__init__.py`**

```python
"""Amplifier hook module — runs engram observe at session end (fire-and-forget)."""

from __future__ import annotations

from typing import Any

__amplifier_module_type__ = "hook"


async def mount(coordinator: Any, config: dict[str, Any] | None = None) -> dict[str, Any]:
    """Mount the memory-observe hook onto the Amplifier coordinator (stub)."""
    config = config or {}
    priority = config.get("priority", 90)

    async def handle(event: str, data: dict[str, Any]) -> Any:
        return _hook_result(action="noop")

    coordinator.hooks.register(
        "execution:end",
        handle,
        priority=priority,
        name="hook-memory-observe",
    )
    return {
        "name": "hook-memory-observe",
        "version": "0.1.0",
        "provides": [],
    }


def _hook_result(action: str) -> Any:
    """Build a HookResult, falling back to a plain dict when amplifier-core is absent."""
    try:
        from amplifier_core import HookResult  # type: ignore[import-not-found]

        return HookResult(action=action)
    except ImportError:
        return {"action": action}
```

**Step 4: Install and verify importable**

```bash
cd modules/hook-memory-observe
pip install -e .
python -c "from amplifier_module_hook_memory_observe import mount; print('OK')"
```

Expected: `OK`

**Step 5: Commit**

```bash
git add modules/hook-memory-observe/
git commit -m "feat: scaffold hook-memory-observe module with protocol-compliant stub"
```

---

## Task 4: Implement hook-memory-observe with TDD

**Files:**
- Create: `modules/hook-memory-observe/tests/test_hook_memory_observe.py`
- Modify: `modules/hook-memory-observe/amplifier_module_hook_memory_observe/__init__.py`

**Step 1: Write the failing test**

Create `modules/hook-memory-observe/tests/test_hook_memory_observe.py`:

```python
"""Tests for hook-memory-observe — mocks subprocess.Popen, no engram binary needed."""

import os
import subprocess
from unittest.mock import MagicMock, patch

import pytest

from amplifier_module_hook_memory_observe import mount


def _make_coordinator():
    coordinator = MagicMock()
    coordinator.hooks = MagicMock()
    coordinator.hooks.register = MagicMock()
    return coordinator


async def _get_handler(config=None):
    """Helper: mount and return the registered handler callable."""
    coordinator = _make_coordinator()
    await mount(coordinator, config=config)
    return coordinator.hooks.register.call_args[0][1]


# ---------------------------------------------------------------------------
# mount() contract
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_mount_registers_hook_on_execution_end():
    """mount() must register a handler for 'execution:end'."""
    coordinator = _make_coordinator()

    result = await mount(coordinator)

    coordinator.hooks.register.assert_called_once()
    event_name = coordinator.hooks.register.call_args[0][0]
    assert event_name == "execution:end"


@pytest.mark.asyncio
async def test_mount_returns_metadata_dict():
    """mount() returns a non-None dict with 'name' key."""
    coordinator = _make_coordinator()

    result = await mount(coordinator)

    assert result is not None
    assert result["name"] == "hook-memory-observe"


@pytest.mark.asyncio
async def test_mount_uses_priority_90_by_default():
    """mount() defaults to priority=90 (low priority — runs after other hooks)."""
    coordinator = _make_coordinator()

    await mount(coordinator)

    call_kwargs = coordinator.hooks.register.call_args[1]
    assert call_kwargs["priority"] == 90


@pytest.mark.asyncio
async def test_mount_respects_priority_config():
    """mount() passes config priority to hooks.register."""
    coordinator = _make_coordinator()

    await mount(coordinator, config={"priority": 50})

    call_kwargs = coordinator.hooks.register.call_args[1]
    assert call_kwargs["priority"] == 50


# ---------------------------------------------------------------------------
# handler behaviour
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_handler_noop_when_transcript_path_absent():
    """Handler returns noop when 'transcript_path' is not in event data."""
    handler = await _get_handler()

    result = await handler("execution:end", {})

    assert result["action"] == "noop"


@pytest.mark.asyncio
async def test_handler_noop_when_data_is_none():
    """Handler returns noop when data is None (defensive guard)."""
    handler = await _get_handler()

    result = await handler("execution:end", None)

    assert result["action"] == "noop"


@pytest.mark.asyncio
async def test_handler_launches_engram_observe_with_transcript_path():
    """Handler calls `engram observe <path> --api-key <key>` as Popen when transcript_path present."""
    handler = await _get_handler()

    with patch("subprocess.Popen") as mock_popen, patch.dict(
        os.environ, {"ANTHROPIC_API_KEY": "sk-test-key"}
    ):
        result = await handler(
            "execution:end", {"transcript_path": "/tmp/abc/transcript.jsonl"}
        )

    mock_popen.assert_called_once_with(
        ["engram", "observe", "/tmp/abc/transcript.jsonl", "--api-key", "sk-test-key"],
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    assert result["action"] == "noop"


@pytest.mark.asyncio
async def test_handler_skips_observe_when_api_key_absent():
    """Handler does not launch engram observe when ANTHROPIC_API_KEY is unset."""
    handler = await _get_handler()

    env_without_key = {k: v for k, v in os.environ.items() if k != "ANTHROPIC_API_KEY"}
    with patch("subprocess.Popen") as mock_popen, patch.dict(
        os.environ, env_without_key, clear=True
    ):
        result = await handler(
            "execution:end", {"transcript_path": "/tmp/abc/transcript.jsonl"}
        )

    mock_popen.assert_not_called()
    assert result["action"] == "noop"


@pytest.mark.asyncio
async def test_handler_fire_and_forget_does_not_wait():
    """Handler uses Popen (not subprocess.run) — fire-and-forget, non-blocking."""
    handler = await _get_handler()

    with patch("subprocess.Popen") as mock_popen, patch(
        "subprocess.run"
    ) as mock_run, patch.dict(os.environ, {"ANTHROPIC_API_KEY": "key"}):
        await handler("execution:end", {"transcript_path": "/tmp/t.jsonl"})

    mock_popen.assert_called_once()
    mock_run.assert_not_called()
```

**Step 2: Run test to verify it fails**

```bash
cd modules/hook-memory-observe
pytest tests/test_hook_memory_observe.py -v
```

Expected: `FAILED` — `test_handler_launches_engram_observe_with_transcript_path` and related tests fail because the stub always returns `noop` without calling `Popen`.

**Step 3: Implement `modules/hook-memory-observe/amplifier_module_hook_memory_observe/__init__.py`**

Replace the entire file:

```python
"""Amplifier hook module — runs engram observe at session end (fire-and-forget)."""

from __future__ import annotations

import os
import subprocess
from typing import Any

__amplifier_module_type__ = "hook"


async def mount(coordinator: Any, config: dict[str, Any] | None = None) -> dict[str, Any]:
    """Mount the memory-observe hook onto the Amplifier coordinator.

    Fires on ``execution:end``. Extracts ``transcript_path`` from the event
    data and launches ``engram observe <path> --api-key <key>`` as a
    background process (fire-and-forget via Popen). If the path is absent or
    ANTHROPIC_API_KEY is unset, returns noop without launching anything.

    Args:
        coordinator: Amplifier coordinator with a ``hooks`` registry.
        config: Optional config dict. Supported keys:
            - ``priority`` (int, default 90): Hook priority (high number = low priority,
              runs after other end-of-session hooks).
    """
    config = config or {}
    priority = config.get("priority", 90)

    async def handle(event: str, data: dict[str, Any] | None) -> Any:
        transcript_path = (data or {}).get("transcript_path")
        if not transcript_path:
            return _hook_result(action="noop")

        api_key = os.environ.get("ANTHROPIC_API_KEY", "")
        if api_key:
            subprocess.Popen(
                ["engram", "observe", transcript_path, "--api-key", api_key],
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
            )
        return _hook_result(action="noop")

    coordinator.hooks.register(
        "execution:end",
        handle,
        priority=priority,
        name="hook-memory-observe",
    )
    return {
        "name": "hook-memory-observe",
        "version": "0.1.0",
        "provides": [],
    }


def _hook_result(action: str) -> Any:
    """Build a HookResult, falling back to a plain dict when amplifier-core is absent."""
    try:
        from amplifier_core import HookResult  # type: ignore[import-not-found]

        return HookResult(action=action)
    except ImportError:
        return {"action": action}
```

**Step 4: Run test to verify it passes**

```bash
cd modules/hook-memory-observe
pytest tests/test_hook_memory_observe.py -v
```

Expected: all 8 tests `PASSED`.

**Step 5: Commit**

```bash
git add modules/hook-memory-observe/
git commit -m "feat: implement hook-memory-observe — calls engram observe <path> at session end"
```

---

## Task 5: Create tool-memory module structure

**Files:**
- Create: `modules/tool-memory/pyproject.toml`
- Create: `modules/tool-memory/amplifier_module_tool_memory/__init__.py`

**Step 1: Create directory structure**

```bash
mkdir -p modules/tool-memory/amplifier_module_tool_memory
mkdir -p modules/tool-memory/tests
touch modules/tool-memory/tests/__init__.py
```

**Step 2: Write `modules/tool-memory/pyproject.toml`**

```toml
[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[project]
name = "amplifier-module-tool-memory"
version = "0.1.0"
description = "Engram — memory_search, memory_load, memory_status tools via engram CLI"
readme = "README.md"
license = { text = "MIT" }
requires-python = ">=3.11"
dependencies = []   # amplifier-core is a peer dep — do NOT declare it here

[project.optional-dependencies]
dev = ["pytest>=8.0", "pytest-asyncio>=0.23"]

[project.entry-points."amplifier.modules"]
tool-memory = "amplifier_module_tool_memory:mount"

[tool.hatch.build.targets.wheel]
packages = ["amplifier_module_tool_memory"]

[tool.pytest.ini_options]
testpaths = ["tests"]
asyncio_mode = "auto"
asyncio_default_fixture_loop_scope = "function"
```

**Step 3: Write protocol-compliant stub `modules/tool-memory/amplifier_module_tool_memory/__init__.py`**

The stub registers all three tool names with placeholder `execute()` methods. This satisfies the Amplifier protocol immediately. Tasks 6 and 7 replace the placeholders with real subprocess calls.

```python
"""Amplifier tool module — memory_search, memory_load, memory_status via engram CLI."""

from __future__ import annotations

from typing import Any

__amplifier_module_type__ = "tool"


class MemorySearchTool:
    name = "memory_search"
    description = (
        "Search your personal memory vault semantically. Returns relevant facts and vault content."
    )
    input_schema: dict[str, Any] = {
        "type": "object",
        "properties": {
            "query": {"type": "string", "description": "Semantic search query"},
            "limit": {"type": "integer", "default": 10},
        },
        "required": ["query"],
    }

    async def execute(self, input_data: dict[str, Any]) -> Any:
        return _tool_result(success=False, output="Not yet implemented (Task 6 pending).")


class MemoryLoadTool:
    name = "memory_load"
    description = "Load context from your personal memory vault."
    input_schema: dict[str, Any] = {
        "type": "object",
        "properties": {
            "format": {
                "type": "string",
                "description": "Output format (default: context)",
                "default": "context",
            },
        },
    }

    async def execute(self, input_data: dict[str, Any]) -> Any:
        return _tool_result(success=False, output="Not yet implemented (Task 7 pending).")


class MemoryStatusTool:
    name = "memory_status"
    description = (
        "Get the status of your personal memory vault, search index, and sync backend."
    )
    input_schema: dict[str, Any] = {"type": "object", "properties": {}}

    async def execute(self, input_data: dict[str, Any]) -> Any:
        return _tool_result(success=False, output="Not yet implemented (Task 7 pending).")


async def mount(coordinator: Any, config: dict[str, Any] | None = None) -> dict[str, Any]:
    """Mount memory_search, memory_load, memory_status onto the Amplifier coordinator."""
    tools = [MemorySearchTool(), MemoryLoadTool(), MemoryStatusTool()]
    for tool in tools:
        await coordinator.mount("tools", tool, name=tool.name)
    return {
        "name": "tool-memory",
        "version": "0.1.0",
        "provides": ["memory_search", "memory_load", "memory_status"],
    }


def _tool_result(success: bool, output: Any) -> Any:
    """Build a ToolResult, falling back to a plain dict when amplifier-core is absent."""
    try:
        from amplifier_core import ToolResult  # type: ignore[import-not-found]

        return ToolResult(success=success, output=output)
    except ImportError:
        return {"success": success, "output": output}
```

**Step 4: Install and verify importable**

```bash
cd modules/tool-memory
pip install -e .
python -c "from amplifier_module_tool_memory import mount; print('OK')"
```

Expected: `OK`

**Step 5: Commit**

```bash
git add modules/tool-memory/
git commit -m "feat: scaffold tool-memory module with protocol-compliant stub for all three tools"
```

---

## Task 6: Implement tool-memory — memory_search with TDD

**Files:**
- Create: `modules/tool-memory/tests/test_tool_memory.py`
- Modify: `modules/tool-memory/amplifier_module_tool_memory/__init__.py` (`MemorySearchTool.execute` only)

**Step 1: Write the failing test**

Create `modules/tool-memory/tests/test_tool_memory.py`:

```python
"""Tests for tool-memory — mocks subprocess.run, no engram binary needed."""

from unittest.mock import AsyncMock, MagicMock, patch

import pytest

from amplifier_module_tool_memory import MemoryLoadTool, MemorySearchTool, MemoryStatusTool, mount


def _make_coordinator():
    coordinator = MagicMock()
    coordinator.mount = AsyncMock()
    return coordinator


# ---------------------------------------------------------------------------
# mount() contract
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_mount_registers_all_three_tools():
    """mount() must register memory_search, memory_load, and memory_status."""
    coordinator = _make_coordinator()

    result = await mount(coordinator)

    assert coordinator.mount.call_count == 3
    registered_names = {call[1]["name"] for call in coordinator.mount.call_args_list}
    assert registered_names == {"memory_search", "memory_load", "memory_status"}


@pytest.mark.asyncio
async def test_mount_returns_metadata_with_provides():
    """mount() returns a non-None dict listing all provided tool names."""
    coordinator = _make_coordinator()

    result = await mount(coordinator)

    assert result is not None
    assert result["name"] == "tool-memory"
    assert set(result["provides"]) == {"memory_search", "memory_load", "memory_status"}


# ---------------------------------------------------------------------------
# memory_search
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_memory_search_calls_engram_search():
    """MemorySearchTool.execute calls `engram search <query> --limit <n> --format=json`."""
    tool = MemorySearchTool()

    with patch("subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(returncode=0, stdout='[{"id":"1"}]', stderr="")
        result = await tool.execute({"query": "typescript preferences", "limit": 5})

    mock_run.assert_called_once_with(
        ["engram", "search", "typescript preferences", "--limit", "5", "--format=json"],
        capture_output=True,
        text=True,
        timeout=10,
    )
    assert result["success"] is True
    assert result["output"] == '[{"id":"1"}]'


@pytest.mark.asyncio
async def test_memory_search_defaults_to_limit_10():
    """MemorySearchTool uses limit=10 when 'limit' is not provided."""
    tool = MemorySearchTool()

    with patch("subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(returncode=0, stdout="[]", stderr="")
        await tool.execute({"query": "test"})

    cmd = mock_run.call_args[0][0]
    limit_idx = cmd.index("--limit")
    assert cmd[limit_idx + 1] == "10"


@pytest.mark.asyncio
async def test_memory_search_returns_error_on_nonzero_exit():
    """MemorySearchTool returns success=False and stderr when engram exits non-zero."""
    tool = MemorySearchTool()

    with patch("subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(returncode=1, stdout="", stderr="vault not found")
        result = await tool.execute({"query": "test"})

    assert result["success"] is False
    assert result["output"] == "vault not found"
```

**Step 2: Run test to verify it fails**

```bash
cd modules/tool-memory
pytest tests/test_tool_memory.py::test_memory_search_calls_engram_search \
       tests/test_tool_memory.py::test_memory_search_defaults_to_limit_10 \
       tests/test_tool_memory.py::test_memory_search_returns_error_on_nonzero_exit -v
```

Expected: `FAILED` — `MemorySearchTool.execute` returns the placeholder string, not subprocess output.

**Step 3: Implement `MemorySearchTool.execute` in `modules/tool-memory/amplifier_module_tool_memory/__init__.py`**

Replace only the `MemorySearchTool.execute` method (keep the rest of the file unchanged):

```python
    async def execute(self, input_data: dict[str, Any]) -> Any:
        result = subprocess.run(
            [
                "engram",
                "search",
                input_data["query"],
                "--limit",
                str(input_data.get("limit", 10)),
                "--format=json",
            ],
            capture_output=True,
            text=True,
            timeout=10,
        )
        if result.returncode == 0:
            return _tool_result(success=True, output=result.stdout)
        return _tool_result(success=False, output=result.stderr)
```

Also add `import subprocess` at the top of the file (after `from __future__ import annotations`).

**Step 4: Run test to verify it passes**

```bash
cd modules/tool-memory
pytest tests/test_tool_memory.py::test_memory_search_calls_engram_search \
       tests/test_tool_memory.py::test_memory_search_defaults_to_limit_10 \
       tests/test_tool_memory.py::test_memory_search_returns_error_on_nonzero_exit \
       tests/test_tool_memory.py::test_mount_registers_all_three_tools \
       tests/test_tool_memory.py::test_mount_returns_metadata_with_provides -v
```

Expected: all 5 tests `PASSED`.

**Step 5: Commit**

```bash
git add modules/tool-memory/
git commit -m "feat: implement memory_search tool — calls engram search with subprocess"
```

---

## Task 7: Implement tool-memory — memory_load and memory_status with TDD

**Files:**
- Modify: `modules/tool-memory/tests/test_tool_memory.py` (add tests)
- Modify: `modules/tool-memory/amplifier_module_tool_memory/__init__.py` (implement two methods)

**Step 1: Write the failing tests**

Append to `modules/tool-memory/tests/test_tool_memory.py`:

```python
# ---------------------------------------------------------------------------
# memory_load
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_memory_load_calls_engram_load_format_context():
    """MemoryLoadTool.execute calls `engram load --format=context` by default."""
    tool = MemoryLoadTool()

    with patch("subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(returncode=0, stdout="context output", stderr="")
        result = await tool.execute({})

    mock_run.assert_called_once_with(
        ["engram", "load", "--format=context"],
        capture_output=True,
        text=True,
        timeout=5,
    )
    assert result["success"] is True
    assert result["output"] == "context output"


@pytest.mark.asyncio
async def test_memory_load_passes_custom_format():
    """MemoryLoadTool passes the 'format' arg to engram load."""
    tool = MemoryLoadTool()

    with patch("subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(returncode=0, stdout="{}", stderr="")
        await tool.execute({"format": "json"})

    cmd = mock_run.call_args[0][0]
    assert "--format=json" in cmd


@pytest.mark.asyncio
async def test_memory_load_returns_stderr_on_failure():
    """MemoryLoadTool returns success=False and stderr when engram exits non-zero."""
    tool = MemoryLoadTool()

    with patch("subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(returncode=1, stdout="", stderr="load failed")
        result = await tool.execute({})

    assert result["success"] is False
    assert result["output"] == "load failed"


# ---------------------------------------------------------------------------
# memory_status
# ---------------------------------------------------------------------------


@pytest.mark.asyncio
async def test_memory_status_calls_engram_status():
    """MemoryStatusTool.execute calls `engram status`."""
    tool = MemoryStatusTool()

    with patch("subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(
            returncode=0, stdout="vault: ok\nindex: fresh", stderr=""
        )
        result = await tool.execute({})

    mock_run.assert_called_once_with(
        ["engram", "status"],
        capture_output=True,
        text=True,
        timeout=5,
    )
    assert result["success"] is True
    assert result["output"] == "vault: ok\nindex: fresh"


@pytest.mark.asyncio
async def test_memory_status_returns_stderr_on_failure():
    """MemoryStatusTool returns success=False and stderr when engram exits non-zero."""
    tool = MemoryStatusTool()

    with patch("subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(returncode=1, stdout="", stderr="status error")
        result = await tool.execute({})

    assert result["success"] is False
    assert result["output"] == "status error"


@pytest.mark.asyncio
async def test_memory_status_accepts_empty_input():
    """MemoryStatusTool.execute accepts an empty input_data dict without error."""
    tool = MemoryStatusTool()

    with patch("subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(returncode=0, stdout="ok", stderr="")
        result = await tool.execute({})  # no required params

    assert result["success"] is True
```

**Step 2: Run tests to verify they fail**

```bash
cd modules/tool-memory
pytest tests/test_tool_memory.py::test_memory_load_calls_engram_load_format_context \
       tests/test_tool_memory.py::test_memory_status_calls_engram_status -v
```

Expected: `FAILED` — `MemoryLoadTool.execute` and `MemoryStatusTool.execute` still return the placeholder strings.

**Step 3: Implement `MemoryLoadTool.execute` and `MemoryStatusTool.execute`**

Replace those two methods in `modules/tool-memory/amplifier_module_tool_memory/__init__.py`:

```python
# MemoryLoadTool.execute:
    async def execute(self, input_data: dict[str, Any]) -> Any:
        fmt = input_data.get("format", "context")
        result = subprocess.run(
            ["engram", "load", f"--format={fmt}"],
            capture_output=True,
            text=True,
            timeout=5,
        )
        if result.returncode == 0:
            return _tool_result(success=True, output=result.stdout)
        return _tool_result(success=False, output=result.stderr)


# MemoryStatusTool.execute:
    async def execute(self, input_data: dict[str, Any]) -> Any:
        result = subprocess.run(
            ["engram", "status"],
            capture_output=True,
            text=True,
            timeout=5,
        )
        if result.returncode == 0:
            return _tool_result(success=True, output=result.stdout)
        return _tool_result(success=False, output=result.stderr)
```

**Step 4: Run the full test suite to verify all tests pass**

```bash
cd modules/tool-memory
pytest tests/test_tool_memory.py -v
```

Expected: all 11 tests `PASSED`.

**Step 5: Run all three module test suites from the repo root to confirm nothing is broken**

```bash
cd modules/hook-memory-context && pytest tests/ -q && cd ../..
cd modules/hook-memory-observe && pytest tests/ -q && cd ../..
cd modules/tool-memory        && pytest tests/ -q && cd ../..
```

Expected: `7 passed`, `8 passed`, `11 passed` respectively.

**Step 6: Commit**

```bash
git add modules/tool-memory/
git commit -m "feat: implement memory_load and memory_status tools — all three tool-memory tools complete"
```

---

## Task 8: Update behaviors/engram.yaml and bundle.md

**Files:**
- Modify: `behaviors/engram.yaml`
- Modify: `bundle.md`

**Step 1: Read the current content of `behaviors/engram.yaml`**

Current content (verified when writing this plan):

```yaml
bundle:
  name: engram-behavior
  version: 0.2.0
  description: Persistent vector memory — hooks + tools for Amplifier agents

hooks:
  - module: engram-hook
    source: git+https://github.com/kenotron-ms/engram@main
    config:
      priority: 5
      inject_role: system

tools:
  - module: engram-tool
    source: git+https://github.com/kenotron-ms/engram@main
    config:
      max_results: 20

context:
  include:
    - engram:context/memory-awareness.md
    - engram:context/memory-instructions.md
```

**Step 2: Write new `behaviors/engram.yaml`**

Replace the entire file with:

```yaml
bundle:
  name: engram-behavior
  version: 0.3.0
  description: Persistent vector memory — hooks + tools for Amplifier agents

# Phase 5: All intelligence delegated to the engram CLI binary.
# hook-memory-context injects recall context before each prompt.
# hook-memory-observe processes session transcripts at end-of-session.
# tool-memory exposes memory_search, memory_load, memory_status on demand.
#
# Old modules (engram-hook, engram-tool) remain in the repo but are no longer
# loaded by default. They still work standalone for backward compatibility.

hooks:
  - module: hook-memory-context
    source: git+https://github.com/kenotron-ms/engram@main#subdirectory=modules/hook-memory-context
    config:
      priority: 5

  - module: hook-memory-observe
    source: git+https://github.com/kenotron-ms/engram@main#subdirectory=modules/hook-memory-observe
    config:
      priority: 90

tools:
  - module: tool-memory
    source: git+https://github.com/kenotron-ms/engram@main#subdirectory=modules/tool-memory

context:
  include:
    - engram:context/memory-awareness.md
    - engram:context/memory-instructions.md
```

**Step 3: Read the current content of `bundle.md`**

Current content (verified when writing this plan):

```
---
bundle:
  name: engram
  version: 0.2.0
  description: Persistent vector memory for Amplifier agents

includes:
  - bundle: git+https://github.com/microsoft/amplifier-foundation@main
  - bundle: engram:behaviors/engram
---

@engram:context/memory-instructions.md

---

@foundation:context/shared/common-system-base.md
```

**Step 4: Update `bundle.md` version to 0.3.0**

Change only the version line. The new `bundle.md`:

```
---
bundle:
  name: engram
  version: 0.3.0
  description: Persistent vector memory for Amplifier agents

includes:
  - bundle: git+https://github.com/microsoft/amplifier-foundation@main
  - bundle: engram:behaviors/engram
---

@engram:context/memory-instructions.md

---

@foundation:context/shared/common-system-base.md
```

**Step 5: Verify the YAML is valid**

```bash
python -c "import yaml; yaml.safe_load(open('behaviors/engram.yaml')); print('YAML valid')"
```

Expected: `YAML valid`

**Step 6: Confirm old module packages are untouched**

```bash
ls amplifier_module_engram_hook/
ls amplifier_module_engram_tool/
```

Expected: both directories still present. They are no longer referenced in `behaviors/engram.yaml` but remain in the repo. Verify neither is listed in the new `behaviors/engram.yaml`:

```bash
grep -E "engram-hook|engram-tool" behaviors/engram.yaml && echo "FOUND (bad)" || echo "Not present (good)"
```

Expected: `Not present (good)`

**Step 7: Commit**

```bash
git add behaviors/engram.yaml bundle.md
git commit -m "feat(phase5): swap Amplifier bundle to hook-memory-context, hook-memory-observe, tool-memory

Old engram-hook and engram-tool removed from default bundle config.
New modules delegate all intelligence to the engram CLI binary.
Bundle and behavior versions bumped to 0.3.0."
```

---

## Final Verification

Run all three module test suites one last time from the repo root:

```bash
for mod in hook-memory-context hook-memory-observe tool-memory; do
  echo "=== modules/$mod ==="
  (cd modules/$mod && pytest tests/ -q)
done
```

Expected output:
```
=== modules/hook-memory-context ===
7 passed in ...s
=== modules/hook-memory-observe ===
8 passed in ...s
=== modules/tool-memory ===
11 passed in ...s
```

Verify the bundle structure at a glance:

```bash
ls modules/
# hook-memory-context  hook-memory-observe  tool-memory

grep "module:" behaviors/engram.yaml
#   - module: hook-memory-context
#   - module: hook-memory-observe
#   - module: tool-memory
```

Phase 5 is complete. The Amplifier integration now delegates entirely to the `engram` CLI binary. Three Python modules, 26 tests, zero LLM calls in Python.
