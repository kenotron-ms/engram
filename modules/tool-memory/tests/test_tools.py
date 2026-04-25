"""Tests for tool-memory module."""
from __future__ import annotations

import subprocess
from unittest.mock import MagicMock, patch

import pytest

from amplifier_module_tool_memory import mount, _run_engram


@pytest.fixture
def coordinator():
    coord = MagicMock()
    coord.tools = MagicMock()
    coord.tools.register = MagicMock()
    return coord


@pytest.mark.asyncio
async def test_mount_registers_three_tools(coordinator):
    await mount(coordinator, {})
    assert coordinator.tools.register.call_count == 3
    names = [call[0][0].__name__ for call in coordinator.tools.register.call_args_list]
    assert "memory_search" in names
    assert "memory_load" in names
    assert "memory_status" in names


def test_run_engram_returns_stdout_on_success():
    with patch("subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(returncode=0, stdout="results here\n")
        result = _run_engram(["search", "test"])
    assert result == "results here\n"


def test_run_engram_returns_error_on_nonzero():
    with patch("subprocess.run") as mock_run:
        mock_run.return_value = MagicMock(returncode=1, stderr="something went wrong", stdout="")
        result = _run_engram(["status"])
    assert "engram error" in result


def test_run_engram_handles_missing_binary():
    with patch("subprocess.run", side_effect=FileNotFoundError):
        result = _run_engram(["status"])
    assert "not found" in result


def test_run_engram_handles_timeout():
    with patch("subprocess.run", side_effect=subprocess.TimeoutExpired(cmd="engram", timeout=5)):
        result = _run_engram(["search", "query"], timeout=5)
    assert "timed out" in result


@pytest.mark.asyncio
async def test_memory_search_calls_engram_search(coordinator):
    await mount(coordinator, {})
    search_fn = next(
        call[0][0] for call in coordinator.tools.register.call_args_list
        if call[0][0].__name__ == "memory_search"
    )
    with patch("amplifier_module_tool_memory._run_engram") as mock_run:
        mock_run.return_value = "search results"
        result = await search_fn("Sofia vegetarian", limit=5)
    mock_run.assert_called_once_with(["search", "Sofia vegetarian", "--limit", "5"], timeout=10)
    assert result == "search results"


@pytest.mark.asyncio
async def test_memory_load_calls_engram_load(coordinator):
    await mount(coordinator, {})
    load_fn = next(
        call[0][0] for call in coordinator.tools.register.call_args_list
        if call[0][0].__name__ == "memory_load"
    )
    with patch("amplifier_module_tool_memory._run_engram") as mock_run:
        mock_run.return_value = "<engram-context>test</engram-context>"
        result = await load_fn(format="context")
    mock_run.assert_called_once_with(["load", "--format=context"], timeout=5)


@pytest.mark.asyncio
async def test_memory_status_calls_engram_status(coordinator):
    await mount(coordinator, {})
    status_fn = next(
        call[0][0] for call in coordinator.tools.register.call_args_list
        if call[0][0].__name__ == "memory_status"
    )
    with patch("amplifier_module_tool_memory._run_engram") as mock_run:
        mock_run.return_value = "vault: ok"
        result = await status_fn()
    mock_run.assert_called_once_with(["status"], timeout=5)
