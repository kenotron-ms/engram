#!/bin/bash
# engram one-command installer
# See https://github.com/kenotron-ms/engram for more info.
set -e

echo ""
echo "Installing engram persistent memory for Amplifier..."
echo ""

# 1. Allow Amplifier to write to the engram memory directory
amplifier allowed-dirs add ~/.engram/

# 2. Install the engram bundle at the application level
amplifier bundle add git+https://github.com/kenotron-ms/engram@main --app

echo ""
echo "engram installed successfully!"
echo ""
echo "---"
echo ""
echo "What happens next:"
echo "  - Your first Amplifier session will automatically have persistent memory"
echo "    via ~/.engram/MEMORY.md (hot surface) and ~/.engram/memories.db (vector DB)."
echo "  - No manual setup needed — memory starts working immediately."
echo ""
echo "How it works:"
echo "  - MEMORY.md is injected into every session for instant recall of key context"
echo "  - The vector DB stores deep semantic memories for recall-on-demand"
echo "  - Both layers update silently after each turn"
echo ""
echo "To uninstall:"
echo "  amplifier bundle remove engram"
echo "  # Optionally: rm -rf ~/.engram"
echo ""
