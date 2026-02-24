def test_search_imported_from_tool_module():
    """Verify search functions originate from tool-memory-search, not a local _search.py."""
    from amplifier_module_tool_memory_search._search import (
        extract_keywords,
        search_memory,
    )

    assert extract_keywords.__module__ == "amplifier_module_tool_memory_search._search"
    assert search_memory.__module__ == "amplifier_module_tool_memory_search._search"
    assert callable(extract_keywords)
    assert callable(search_memory)
