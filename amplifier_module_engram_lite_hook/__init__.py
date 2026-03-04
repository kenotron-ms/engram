"""Shim package for Amplifier module discovery.

Amplifier's module loader expects a package named
``amplifier_module_{module_id_with_underscores}`` for each module ID.
This shim satisfies that convention by re-exporting the hook ``mount``
from the main ``amplifier_module_engram_lite.hooks.amplifier_hook`` submodule.
"""

__amplifier_module_type__ = "hook"

from amplifier_module_engram_lite.hooks.amplifier_hook import mount  # noqa: F401

__all__ = ["mount", "__amplifier_module_type__"]
