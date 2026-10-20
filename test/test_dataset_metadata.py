# SPDX-License-Identifier: Apache-2.0
# Copyright © 2026 Au-Zone Technologies. All Rights Reserved.

"""
Typed dataset metadata read by read_dataset_metadata().

Files are written with pyarrow-compatible Parquet metadata through Polars,
so no EdgeFirst Studio account or network connection is needed.
"""

import json
import os
import tempfile
import unittest

import polars as pl


def _write(path: str, metadata: dict) -> None:
    pl.DataFrame({"name": ["a"]}).write_parquet(path, metadata=metadata)


class TestDatasetMetadata(unittest.TestCase):
    """read_dataset_metadata() parses and validates file metadata."""

    def test_label_map_metadata(self):
        import edgefirst_client as ec

        with tempfile.TemporaryDirectory() as tmpdir:
            path = os.path.join(tmpdir, "ds.parquet")
            _write(
                path,
                {
                    "schema_version": "2026.10",
                    "labels": json.dumps(["unlabeled", "wall"]),
                    "ignore_index": "0",
                    "mask_interpretation": "class_index",
                    "category_metadata": json.dumps({"wall": {"id": 1}}),
                    "source": "test",
                },
            )
            meta = ec.read_dataset_metadata(path)

        self.assertEqual(meta.schema_version, "2026.10")
        self.assertEqual(meta.labels, ["unlabeled", "wall"])
        self.assertEqual(meta.ignore_index, 0)
        self.assertEqual(meta.mask_interpretation, "class_index")
        self.assertEqual(meta.category_metadata, {"wall": {"id": 1}})
        self.assertEqual(meta.extra.get("source"), "test")
        self.assertIn("class_index", repr(meta))

    def test_absent_keys_take_defaults(self):
        import edgefirst_client as ec

        with tempfile.TemporaryDirectory() as tmpdir:
            path = os.path.join(tmpdir, "plain.parquet")
            _write(path, {})
            meta = ec.read_dataset_metadata(path)

        self.assertIsNone(meta.schema_version)
        self.assertIsNone(meta.labels)
        self.assertIsNone(meta.ignore_index)
        self.assertEqual(meta.mask_interpretation, "binary")
        self.assertIsNone(meta.category_metadata)

    def test_invalid_ignore_index_raises_value_error(self):
        import edgefirst_client as ec

        with tempfile.TemporaryDirectory() as tmpdir:
            path = os.path.join(tmpdir, "bad.parquet")
            _write(path, {"labels": json.dumps(["a", "b"]), "ignore_index": "2"})
            with self.assertRaises(ValueError) as caught:
                ec.read_dataset_metadata(path)
        message = str(caught.exception)
        self.assertIn("ignore_index 2 is outside labels", message)
        self.assertNotIn("Invalid parameters", message)

    def test_unreadable_file_raises(self):
        import edgefirst_client as ec

        with tempfile.TemporaryDirectory() as tmpdir:
            path = os.path.join(tmpdir, "garbage.arrow")
            with open(path, "wb") as f:
                f.write(b"not an arrow file")
            with self.assertRaisesRegex(Exception, "garbage.arrow"):
                ec.read_dataset_metadata(path)

    def test_ade20k_output(self):
        import edgefirst_client as ec

        from test.test_ade20k import _make_ade20k

        with tempfile.TemporaryDirectory() as tmpdir:
            ade, _ = _make_ade20k(tmpdir)
            output = os.path.join(tmpdir, "out", "out.arrow")
            ec.ade20k_to_arrow(ade, output)
            meta = ec.read_dataset_metadata(output)

        self.assertEqual(meta.ignore_index, 0)
        self.assertEqual(meta.labels[0], "unlabeled")
        self.assertEqual(meta.mask_interpretation, "class_index")


if __name__ == "__main__":
    unittest.main()
