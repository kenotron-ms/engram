---
bundle:
  name: engram-custom
  version: 1.0.0
  description: Engram with CUSTOM memory paths (for testing)

session:
  orchestrator:
    module: loop-streaming
    source: git+https://github.com/microsoft/amplifier-module-loop-streaming@main
  context:
    module: context-simple
    source: git+https://github.com/microsoft/amplifier-module-context-simple@main

hooks:
  - module: hooks-protocol-reminder
    config:
      priority: 5
      inject_role: system
      project_memory_base: ".memory/project"
      user_memory_base: "~/.canvas/memory"

  - module: hooks-memory-tracker
    config:
      priority: 90
      project_memory_base: ".memory/project"
      user_memory_base: "~/.canvas/memory"

tools:
  - module: tool-bash
    source: git+https://github.com/microsoft/amplifier-module-tool-bash@main
  - module: tool-filesystem
    source: git+https://github.com/microsoft/amplifier-module-tool-filesystem@main

includes:
  - path: ../../AGENTS.md
  - path: ../../MEMORY-SYSTEM.md
  - path: ../../_protocols/inline-capture.md
  - path: ../../_protocols/knowledge-extraction.md
  - path: ../../_protocols/dual-write-decision.md
  - path: ../../_protocols/scope-routing.md
  - path: ../../_protocols/cross-reference-cascade.md
  - path: ../../_protocols/source-intake.md
---

# Engram Bundle - Custom Paths

This is a TEST bundle that uses custom memory paths instead of the defaults.

**Paths configured:**
- Project memory: `.memory/project/` (instead of `.canvas/memory/`)
- User memory: `~/.canvas/memory/` (unchanged)

Use this to verify Engram works with arbitrary base directories.
