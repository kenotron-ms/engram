---
bundle:
  name: canvas-memory
  version: 1.0.0
  description: Canvas Memory - Personal Knowledge Graph System

hooks:
  - module: hook-shell
    source: git+https://github.com/microsoft/amplifier-module-hook-shell@main

includes:
  - bundle: git+https://github.com/microsoft/amplifier-foundation@main

context:
  include:
    - AGENTS.md
---

# Canvas Memory System

Personal knowledge graph with RETRIEVE → RESPOND → CAPTURE protocol enforcement via hooks.

The memory protocols are defined in `_protocols/` and enforced via shell hooks in `.amplifier/hooks/`.

See AGENTS.md for complete operational instructions.
