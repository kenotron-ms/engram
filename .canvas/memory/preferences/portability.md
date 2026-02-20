---
domain: preferences
keywords: [portability, repo-specific, architecture, configuration, self-contained, dependencies]
last_updated: 2025-01-30
---

# Portability Preference

Ken prefers systems that are **portable across repos**.

## Implications

This preference influences architectural decisions:

- **Favor self-contained configurations** - Keep setup within the repository
- **Avoid global dependencies** - Minimize reliance on user-global state
- **Repo-specific over global** - Configuration should travel with the code
- **Reduce external coupling** - Systems should work independently per project

## Examples

- `.amplifier/` bundles living in-repo rather than requiring global installation
- Memory systems storing data within `.canvas/memory/` 
- Configuration files committed to version control
