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


async def test_execute_missing_query_returns_failure(tool):
    result = await tool.execute({})
    assert result.success is False
    assert "query" in result.error["message"].lower()


async def test_execute_empty_query_returns_failure(tool):
    result = await tool.execute({"query": "   "})
    assert result.success is False


@pytest.mark.asyncio
async def test_execute_returns_tool_result(tmp_path):
    tool = MemorySearchTool(
        user_memory_base=str(tmp_path),
        project_memory_base=str(tmp_path),
    )
    result = await tool.execute({"query": "test query", "memory_base": "both"})
    assert result.success is True


@pytest.mark.asyncio
async def test_execute_invalid_memory_base_returns_failure(tool):
    result = await tool.execute({"query": "something", "memory_base": "everywhere"})
    assert result.success is False
    assert "memory_base" in result.error["message"]


@pytest.mark.asyncio
async def test_execute_returns_matching_memory_entries(tmp_path):
    # Arrange: create a memory file with matching frontmatter keywords
    info_dir = tmp_path / "information"
    info_dir.mkdir()
    mem_file = info_dir / "auth.md"
    mem_file.write_text("---\nkeywords: [authentication, jwt]\n---\nJWT auth notes.\n")
    tool = MemorySearchTool(
        user_memory_base=str(tmp_path),
        project_memory_base=str(tmp_path / "nonexistent"),
    )
    result = await tool.execute({"query": "authentication", "memory_base": "user"})
    assert result.success is True
    assert len(result.output) >= 1
