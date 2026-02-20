---
created: 2026-02-17T23:34:00Z
contributors: [ken]
tags: [cross-platform, MCP, architecture]
keywords: [MCP, model-context-protocol, cross-platform, interoperability]
relates-to: [2026-02-17-distribution-architecture.md]
---

# Cross-Platform Support Strategy

## Decision

Build an **MCP (Model Context Protocol) server** as the primary cross-platform distribution mechanism.

## Why MCP?

MCP is the emerging standard for AI agent interoperability:
- ✅ Works with Claude Desktop, Claude Code, GitHub Copilot, ChatGPT
- ✅ Provides actual tool operations (not just instructions)
- ✅ Open-source standard by Anthropic (Nov 2024)
- ✅ "USB-C port for AI applications"

## Multi-Platform Architecture

```
memory-system/
├── mcp-server/              # Universal (works everywhere)
├── amplifier/               # Amplifier-specific bundle
└── adapters/                # Platform instruction files
    ├── claude-code/CLAUDE.md
    ├── cursor/.cursorrules
    └── github-copilot/.github/copilot-instructions.md
```

## Platform Support Matrix

| Platform | Instructions | Operations | Status |
|----------|-------------|------------|---------|
| Claude Code | CLAUDE.md | MCP + Tool module | High priority |
| GitHub Copilot | .github/copilot-instructions.md | MCP server | High priority |
| Cursor | .cursorrules | MCP server | Medium priority |
| Claude Desktop | Manual upload | MCP server | Medium priority |
| Amplifier | Context files | Native tool module | High priority |

## Implementation Order

1. **MCP Server** - Universal compatibility layer
2. **Platform Adapters** - Instruction files for each platform
3. **Amplifier Bundle** - Native integration with full ecosystem support

## Impact

Users can use the memory system from ANY MCP-compatible AI platform, with persistent cross-session storage.

## Related

- See distribution-architecture.md for overall architecture
- See MEMORY-SYSTEM.md for system specification
