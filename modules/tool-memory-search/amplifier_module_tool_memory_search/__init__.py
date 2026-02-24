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
            'Input: {"query": "search terms", "memory_base": "project|user|both"}\n'
            "Returns: Matching memory entries with relevance context."
        )

    @property
    def input_schema(self) -> dict[str, Any]:
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

    async def execute(self, tool_input: dict[str, Any]) -> ToolResult:
        query = tool_input.get("query", "").strip()
        if not query:
            return ToolResult(
                success=False,
                error={"message": "query is required and cannot be empty"},
            )

        memory_base = tool_input.get("memory_base", "both")

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
