# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""Tests for safe path handling in the offline examples."""

import importlib.util
import tempfile
import unittest
from pathlib import Path

MODULE_PATH = Path(__file__).parent.parent / "examples" / "_path.py"
SPEC = importlib.util.spec_from_file_location("example_path_helpers", MODULE_PATH)
assert SPEC is not None and SPEC.loader is not None
PATH_HELPERS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PATH_HELPERS)
normalize_user_path = PATH_HELPERS.normalize_user_path


class TestNormalizeUserPath(unittest.TestCase):
    def test_resolves_relative_components(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            base = Path(temp_dir)
            normalized = normalize_user_path(base / "nested" / ".." / "dataset.arrow")

        self.assertEqual(normalized, base / "dataset.arrow")
        self.assertTrue(normalized.is_absolute())

    def test_rejects_control_characters(self) -> None:
        with self.assertRaisesRegex(ValueError, "control characters"):
            normalize_user_path(Path("dataset\n.arrow"))


if __name__ == "__main__":
    unittest.main()
