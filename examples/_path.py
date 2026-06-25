# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""Bootstrap ``sys.path`` for tutorial scripts and notebooks."""

from __future__ import annotations

import sys
from pathlib import Path


def repo_root() -> Path:
    """
    Return the EdgeFirst Client repository root.

    Scripts resolve via ``__file__``; notebooks fall back to ``Path.cwd()``
    (expected: repo root or ``examples/`` when started per ``examples/README.md``).
    """
    try:
        return Path(__file__).resolve().parent.parent
    except NameError:
        cwd = Path.cwd()
        if (cwd / "Cargo.toml").is_file():
            return cwd
        if cwd.name == "examples" and (cwd.parent / "Cargo.toml").is_file():
            return cwd.parent
        raise RuntimeError(
            "Could not locate the client repository root. "
            "Start Jupyter from the repo root and open notebooks under examples/, "
            "or run: jupyter lab examples/01_authentication.ipynb"
        )


def ensure_repo_on_path() -> None:
    """Insert the repository root on ``sys.path`` for ``from examples import ...``."""
    root = str(repo_root())
    if root not in sys.path:
        sys.path.insert(0, root)
