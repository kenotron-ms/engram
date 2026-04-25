"""Add module package to sys.path for pytest imports."""
import sys
from pathlib import Path

# Insert parent dir of tests/ so the module package is importable
sys.path.insert(0, str(Path(__file__).parent.parent))
