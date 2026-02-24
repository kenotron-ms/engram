def test_search_imported_from_tool_module():
    """Verify search functions come from the tool module, not a local _search.py."""
    from amplifier_module_tool_memory_search._search import extract_keywords, search_memory

    assert callable(extract_keywords)
    assert callable(search_memory)
