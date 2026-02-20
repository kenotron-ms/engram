---
created: 2026-02-17T23:30:00Z
contributors: [ken]
tags: [architecture, distribution, amplifier-ecosystem]
keywords: [tool-module, bundle-pattern, behavior, distribution-strategy]
relates-to: [context.md]
---

# Distribution Architecture Decision

## Decision

Distribute the memory system as a **Tool Module + Bundle** in the Amplifier ecosystem.

## Architecture

```
amplifier-bundle-memory/
├── modules/tool-memory/           # Core operations
├── behaviors/memory.yaml          # Reusable behavior
├── agents/                        # memory-agent, search-agent
├── context/instructions.md        # MEMORY-SYSTEM.md content
└── bundle.md                      # Thin root bundle
```

## Components

| Component | Purpose |
|-----------|---------|
| **Tool Module** | Provides `memory_capture()`, `memory_retrieve()` operations |
| **Bundle** | Packages tool + agents + instructions |
| **Behavior** | Enables others to include memory in their bundles |

## Rationale

- **Tool provides mechanism** - Core operations AI can invoke
- **Bundle provides policy** - When and how to use those operations
- **Behavior enables composition** - Others can add memory to their bundles
- **Aligns with Amplifier philosophy** - Mechanism not policy, composable, text-first

## Context

Analyzed with `amplifier:amplifier-expert` on 2026-02-17. Based on:
- Amplifier ecosystem patterns
- Canonical example: amplifier-bundle-recipes
- Existing `tool-memory` module in ecosystem

## Implementation Priority

1. Tool Module - Core Python implementation
2. Behavior - Package with agents and context
3. Root Bundle - Thin bundle pattern
4. Optional Hook - Auto-capture enhancement

## Related

- See MEMORY-SYSTEM.md for system specification
- See AGENTS.md for agent instructions
