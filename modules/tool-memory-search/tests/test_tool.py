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
