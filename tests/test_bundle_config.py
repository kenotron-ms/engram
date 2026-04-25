"""Tests for bundle config update — task-8.

Acceptance criteria:
1. behaviors/engram.yaml references hook-memory-context, hook-memory-observe, tool-memory
2. behaviors/engram.yaml version is 0.3.0
3. bundle.md version is 0.3.0
4. YAML is valid (yaml.safe_load succeeds)
5. No engram-hook or engram-tool references in behaviors/engram.yaml
6. Old module directories amplifier_module_engram_hook/ and amplifier_module_engram_tool/ still exist
7. Context section preserved with both memory-awareness.md and memory-instructions.md
"""

import re
from pathlib import Path

import pytest
import yaml

# Repo root is two levels up from the tests/ directory
REPO_ROOT = Path(__file__).parent.parent
ENGRAM_YAML = REPO_ROOT / "behaviors" / "engram.yaml"
BUNDLE_MD = REPO_ROOT / "bundle.md"


# ──────────────────────────────────────────────
# Fixtures
# ──────────────────────────────────────────────


@pytest.fixture(scope="module")
def engram_yaml_text() -> str:
    return ENGRAM_YAML.read_text()


@pytest.fixture(scope="module")
def engram_yaml_data(engram_yaml_text: str) -> dict:
    return yaml.safe_load(engram_yaml_text)


@pytest.fixture(scope="module")
def bundle_md_text() -> str:
    return BUNDLE_MD.read_text()


# ──────────────────────────────────────────────
# Criterion 4 — YAML validity (runs first as it's foundational)
# ──────────────────────────────────────────────


def test_yaml_is_valid(engram_yaml_data: dict) -> None:
    """YAML must parse without errors."""
    assert isinstance(engram_yaml_data, dict), "yaml.safe_load must return a dict"


# ──────────────────────────────────────────────
# Criterion 2 — engram.yaml version is 0.3.0
# ──────────────────────────────────────────────


def test_engram_yaml_version(engram_yaml_data: dict) -> None:
    """Bundle version in behaviors/engram.yaml must be 0.3.0."""
    assert engram_yaml_data["bundle"]["version"] == "0.3.0"


# ──────────────────────────────────────────────
# Criterion 3 — bundle.md version is 0.3.0
# ──────────────────────────────────────────────


def test_bundle_md_version(bundle_md_text: str) -> None:
    """bundle.md front-matter version must be 0.3.0."""
    assert "version: 0.3.0" in bundle_md_text


# ──────────────────────────────────────────────
# Criterion 1 — new module references present
# ──────────────────────────────────────────────


def test_hook_memory_context_referenced(engram_yaml_data: dict) -> None:
    """hooks section must contain hook-memory-context."""
    hook_names = [h.get("module") for h in engram_yaml_data.get("hooks", [])]
    assert "hook-memory-context" in hook_names, f"hook-memory-context not found; hooks: {hook_names}"


def test_hook_memory_observe_referenced(engram_yaml_data: dict) -> None:
    """hooks section must contain hook-memory-observe."""
    hook_names = [h.get("module") for h in engram_yaml_data.get("hooks", [])]
    assert "hook-memory-observe" in hook_names, f"hook-memory-observe not found; hooks: {hook_names}"


def test_tool_memory_referenced(engram_yaml_data: dict) -> None:
    """tools section must contain tool-memory."""
    tool_names = [t.get("module") for t in engram_yaml_data.get("tools", [])]
    assert "tool-memory" in tool_names, f"tool-memory not found; tools: {tool_names}"


# ──────────────────────────────────────────────
# Criterion 1 (cont.) — new module sources correct
# ──────────────────────────────────────────────

BASE = "git+https://github.com/kenotron-ms/engram@main"


def test_hook_memory_context_source(engram_yaml_data: dict) -> None:
    """hook-memory-context source must point to correct subdirectory."""
    hooks = {h["module"]: h for h in engram_yaml_data.get("hooks", [])}
    hook = hooks.get("hook-memory-context", {})
    expected = f"{BASE}#subdirectory=modules/hook-memory-context"
    assert hook.get("source") == expected, f"source mismatch: {hook.get('source')!r}"


def test_hook_memory_observe_source(engram_yaml_data: dict) -> None:
    """hook-memory-observe source must point to correct subdirectory."""
    hooks = {h["module"]: h for h in engram_yaml_data.get("hooks", [])}
    hook = hooks.get("hook-memory-observe", {})
    expected = f"{BASE}#subdirectory=modules/hook-memory-observe"
    assert hook.get("source") == expected, f"source mismatch: {hook.get('source')!r}"


def test_tool_memory_source(engram_yaml_data: dict) -> None:
    """tool-memory source must point to correct subdirectory."""
    tools = {t["module"]: t for t in engram_yaml_data.get("tools", [])}
    tool = tools.get("tool-memory", {})
    expected = f"{BASE}#subdirectory=modules/tool-memory"
    assert tool.get("source") == expected, f"source mismatch: {tool.get('source')!r}"


# ──────────────────────────────────────────────
# Criterion 1 (cont.) — config priorities correct
# ──────────────────────────────────────────────


def test_hook_memory_context_priority(engram_yaml_data: dict) -> None:
    """hook-memory-context must have priority 5."""
    hooks = {h["module"]: h for h in engram_yaml_data.get("hooks", [])}
    hook = hooks.get("hook-memory-context", {})
    assert hook.get("config", {}).get("priority") == 5


def test_hook_memory_observe_priority(engram_yaml_data: dict) -> None:
    """hook-memory-observe must have priority 90."""
    hooks = {h["module"]: h for h in engram_yaml_data.get("hooks", [])}
    hook = hooks.get("hook-memory-observe", {})
    assert hook.get("config", {}).get("priority") == 90


# ──────────────────────────────────────────────
# Criterion 5 — old modules NOT referenced
# ──────────────────────────────────────────────


def test_no_old_engram_hook(engram_yaml_text: str) -> None:
    """behaviors/engram.yaml must NOT reference the legacy 'engram-hook' module."""
    assert not re.search(r"\bmodule:\s*engram-hook\b", engram_yaml_text), (
        "Found legacy 'engram-hook' module reference in engram.yaml"
    )


def test_no_old_engram_tool(engram_yaml_text: str) -> None:
    """behaviors/engram.yaml must NOT reference the legacy 'engram-tool' module."""
    assert not re.search(r"\bmodule:\s*engram-tool\b", engram_yaml_text), (
        "Found legacy 'engram-tool' module reference in engram.yaml"
    )


# ──────────────────────────────────────────────
# Criterion 6 — old module directories still exist
# ──────────────────────────────────────────────


def test_old_hook_dir_exists() -> None:
    """amplifier_module_engram_hook/ directory must still exist."""
    assert (REPO_ROOT / "amplifier_module_engram_hook").is_dir(), (
        "amplifier_module_engram_hook/ directory was removed"
    )


def test_old_tool_dir_exists() -> None:
    """amplifier_module_engram_tool/ directory must still exist."""
    assert (REPO_ROOT / "amplifier_module_engram_tool").is_dir(), (
        "amplifier_module_engram_tool/ directory was removed"
    )


# ──────────────────────────────────────────────
# Criterion 7 — context section preserved
# ──────────────────────────────────────────────


def test_context_memory_awareness_included(engram_yaml_data: dict) -> None:
    """Context section must include engram:context/memory-awareness.md."""
    includes = engram_yaml_data.get("context", {}).get("include", [])
    assert "engram:context/memory-awareness.md" in includes, (
        f"memory-awareness.md not found in context includes: {includes}"
    )


def test_context_memory_instructions_included(engram_yaml_data: dict) -> None:
    """Context section must include engram:context/memory-instructions.md."""
    includes = engram_yaml_data.get("context", {}).get("include", [])
    assert "engram:context/memory-instructions.md" in includes, (
        f"memory-instructions.md not found in context includes: {includes}"
    )
