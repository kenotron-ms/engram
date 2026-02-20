# Engram Configuration

Engram supports flexible base directory configuration.

## Default Paths

| Memory Type | Default Location | Purpose |
|-------------|------------------|---------|
| **User Memory** | `~/.canvas/memory/` | Personal knowledge that endures across all projects |
| **Project Memory** | `.canvas/memory/` | Project-specific knowledge (safe to share) |

## Configuring Custom Paths

To use different base directories, configure the hooks in your bundle:

```yaml
hooks:
  - module: hooks-protocol-reminder
    config:
      project_memory_base: ".memory"          # Custom project path
      user_memory_base: "~/.my-memory"        # Custom user path
      
  - module: hooks-memory-tracker
    config:
      project_memory_base: ".memory"          # Must match protocol reminder
      user_memory_base: "~/.my-memory"        # Must match protocol reminder
```

## Important Notes

1. **Both hooks must use the same paths** - The protocol reminder tells the agent where to search/capture, and the memory tracker validates those paths.

2. **User memory paths support tilde expansion** - `~/.canvas/memory` expands to your home directory.

3. **Project memory paths are relative** - `.canvas/memory` is relative to the project root (git repo root).

## Examples

### Example 1: XDG Base Directory Compliance

```yaml
hooks:
  - module: hooks-protocol-reminder
    config:
      project_memory_base: ".local/memory"
      user_memory_base: "~/.local/share/canvas-memory"
```

### Example 2: Hidden Directory in Project

```yaml
hooks:
  - module: hooks-protocol-reminder
    config:
      project_memory_base: ".memory"
      user_memory_base: "~/.canvas/memory"
```

### Example 3: Non-Hidden Project Directory

```yaml
hooks:
  - module: hooks-protocol-reminder
    config:
      project_memory_base: "docs/memory"
      user_memory_base: "~/.canvas/memory"
```

## What Gets Configured

When you set these paths:

- **Protocol reminder hook**: Injects the paths into the RETRIEVE → RESPOND → CAPTURE loop, telling the agent where to search and capture
- **Memory tracker hook**: Validates git status changes in the configured project memory path
- **Agent behavior**: The agent follows the paths specified in the protocol reminder

## Migration Guide

If you're switching from `.canvas/memory` to a different path:

1. Update your bundle configuration with the new paths
2. Move existing memory files: `mv .canvas/memory ./new-path/`
3. Update any scripts that reference the old path
4. Test with `amplifier run --bundle your-bundle "test capture"`
