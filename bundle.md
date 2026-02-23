---
bundle:
  name: engram
  version: 1.0.0
  description: Engram - Agentic memory augmentation with capture, retrieval, and compute at rest

includes:
  - bundle: git+https://github.com/microsoft/amplifier-foundation@main
  - bundle: engram:behaviors/engram
---

@foundation:context/shared/common-system-base.md
