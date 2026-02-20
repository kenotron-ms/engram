#!/usr/bin/env bash
# Engram — installer
# Usage (one-liner): curl -fsSL https://raw.githubusercontent.com/kenotron-ms/engram/main/install.sh | bash
# Usage (local):     bash install.sh

set -euo pipefail

REPO="kenotron-ms/engram"
BRANCH="main"
RAW_URL="https://raw.githubusercontent.com/$REPO/$BRANCH"

CANVAS_DIR="$HOME/.canvas"
SCRIPTS_DIR="$CANVAS_DIR/scripts"
MEMORY_DIR="$CANVAS_DIR/memory"
CLAUDE_SETTINGS="$HOME/.claude/settings.json"

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

info()    { echo -e "${BLUE}→${NC} $*"; }
success() { echo -e "${GREEN}✓${NC} $*"; }
warn()    { echo -e "${YELLOW}!${NC} $*"; }

echo ""
echo "  Engram"
echo "  ──────"
echo ""

# ── Prerequisites ──────────────────────────────────────────────────────────────

if ! command -v python3 &>/dev/null; then
  echo "Error: Python 3 is required. Install it from https://python.org and re-run."
  exit 1
fi

# ── Detect local vs remote install ─────────────────────────────────────────────
# When piped through bash, BASH_SOURCE[0] is empty. When run directly, it's the script path.

LOCAL_REPO=""
if [ -n "${BASH_SOURCE[0]:-}" ]; then
  CANDIDATE="$(cd "$(dirname "${BASH_SOURCE[0]}")" 2>/dev/null && pwd)"
  if [ -f "$CANDIDATE/scripts/canvas-memory-search.py" ]; then
    LOCAL_REPO="$CANDIDATE"
  fi
fi

copy_file() {
  local src_path="$1"
  local dest="$2"
  if [ -n "$LOCAL_REPO" ]; then
    cp "$LOCAL_REPO/$src_path" "$dest"
  else
    curl -fsSL "$RAW_URL/$src_path" -o "$dest"
  fi
}

# ── Create directory structure ─────────────────────────────────────────────────

info "Creating ~/.canvas/ directory structure..."
mkdir -p "$SCRIPTS_DIR"
mkdir -p "$MEMORY_DIR/personal/preferences"
mkdir -p "$MEMORY_DIR/personal/people"
mkdir -p "$MEMORY_DIR/projects"
mkdir -p "$MEMORY_DIR/professional"
mkdir -p "$HOME/.claude"
success "Directories ready"

# ── Install scripts ────────────────────────────────────────────────────────────

info "Installing hook scripts..."
copy_file ".claude/scripts/protocol-reminder.sh"  "$SCRIPTS_DIR/protocol-reminder.sh"
copy_file ".claude/scripts/memory-search.sh"      "$SCRIPTS_DIR/memory-search.sh"
copy_file ".claude/scripts/validate-capture.sh"   "$SCRIPTS_DIR/validate-capture.sh"
copy_file "scripts/canvas-memory-search.py"       "$SCRIPTS_DIR/canvas-memory-search.py"
chmod +x "$SCRIPTS_DIR"/*.sh
success "Scripts installed to $SCRIPTS_DIR"

# ── Register marketplace + enable plugin in user settings ──────────────────────

info "Registering marketplace in ~/.claude/settings.json..."
python3 - "$CLAUDE_SETTINGS" "$REPO" << 'PYEOF'
import json, sys, os

settings_file = sys.argv[1]
repo = sys.argv[2]

if os.path.exists(settings_file):
    with open(settings_file) as f:
        settings = json.load(f)
else:
    settings = {}

# Register marketplace
settings.setdefault("extraKnownMarketplaces", {})
settings["extraKnownMarketplaces"]["canvas-memory"] = {
    "source": {"source": "github", "repo": repo}
}

# Enable plugin (auto-updates on by default for user-added marketplaces)
settings.setdefault("enabledPlugins", {})
settings["enabledPlugins"]["canvas-memory@canvas-memory"] = True

with open(settings_file, "w") as f:
    json.dump(settings, f, indent=2)
    f.write("\n")
PYEOF
success "Marketplace registered and plugin enabled"

# ── Done ───────────────────────────────────────────────────────────────────────

echo ""
echo "  ─────────────────────────────────────────────────────"
success "Canvas Memory installed"
echo ""
echo "  Memory locations:"
echo "    User (private):    ~/.canvas/memory/"
echo "    Project (shared):  .canvas/memory/  (per-project)"
echo ""
echo "  To init memory in a project:"
echo "    mkdir -p .canvas/memory"
echo ""
echo "  Restart Claude Code to activate."
echo ""
