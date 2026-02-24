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

    async def execute(self, tool_input: dict[str, Any]) -> Any:
        raise NotImplementedError


async def mount(coordinator: Any, config: dict) -> MemorySearchTool:
    tool = MemorySearchTool()
    await coordinator.mount("tools", tool, name=tool.name)
    return tool
