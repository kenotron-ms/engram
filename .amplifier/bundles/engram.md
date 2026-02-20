---
bundle:
  name: engram
  version: 1.0.0
  description: Engram - Agentic memory augmentation with capture, retrieval, and compute at rest

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
      project_memory_base: ".canvas/memory"
      user_memory_base: "~/.canvas/memory"

  - module: hooks-memory-tracker
    config:
      priority: 90
      project_memory_base: ".canvas/memory"
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

# Engram Bundle

Engram provides agentic memory augmentation - external memory storage with automatic capture, intelligent retrieval, and cognitive compute offload.

## Hooks

### Protocol Reminder Hook (`hooks-protocol-reminder`)
- **Event**: `prompt:submit`
- **Purpose**: Injects RETRIEVE → RESPOND → CAPTURE protocol reminder before each user prompt
- **Behavior**: Ephemeral context injection (not stored in history)

### Memory Tracker Hook (`hooks-memory-tracker`)
- **Event**: `execution:end`
- **Purpose**: Validates that knowledge was captured after orchestration completes
- **Behavior**: Checks git status for changes in `.canvas/memory/` paths

## Usage

Test this bundle standalone without affecting the current session:

```bash
# Run a test prompt with this bundle
amplifier run --bundle .amplifier/bundles/engram.md "Tell me about engram"

# Interactive session with this bundle
amplifier --bundle .amplifier/bundles/engram.md
```
