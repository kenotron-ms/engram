---
bundle:
  name: engram-lite
  version: 0.1.0
  description: Lightweight persistent vector memory for Amplifier agents

includes:
  - bundle: git+https://github.com/microsoft/amplifier-foundation@main
  - bundle: engram-lite:behaviors/engram-lite
---

@engram-lite:context/memory-instructions.md

---

@foundation:context/shared/common-system-base.md
