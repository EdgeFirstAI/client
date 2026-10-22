# SPDX-License-Identifier: Apache-2.0
# Copyright © 2026 Au-Zone Technologies. All Rights Reserved.

"""
COCO RLE conversion checked against pycocotools.

Masks are encoded with pycocotools itself, converted with coco_to_arrow(),
and the stored 1-bit masks are compared pixel for pixel with the originals.
"""

import io
import json
import os
import tempfile
import unittest

import numpy as np
import polars as pl
from PIL import Image
from pycocotools import mask as mask_utils


def _masks() -> list:
    """Masks covering run lengths that need one or several characters, and
    run deltas of both signs in the compressed encoding."""
    rng = np.random.default_rng(20261016)
    masks = [
        np.zeros((1, 1), np.uint8),
        np.ones((1, 1), np.uint8),
        np.zeros((7, 5), np.uint8),
        np.ones((7, 5), np.uint8),
        (rng.random((64, 48)) < 0.5).astype(np.uint8),
        (rng.random((37, 1)) < 0.3).astype(np.uint8),
        np.indices((33, 29)).sum(axis=0).astype(np.uint8) % 2,
    ]
    single = np.zeros((480, 640), np.uint8)
    single[-1, -1] = 1
    masks.append(single)
    blocks = np.zeros((480, 640), np.uint8)
    for _ in range(12):
        y, x = rng.integers(0, 440), rng.integers(0, 600)
        h, w = rng.integers(1, 40), rng.integers(1, 40)
        blocks[y : y + h, x : x + w] = 1
    masks.append(blocks)
    return masks


def _compressed_rle(mask: np.ndarray) -> dict:
    rle = mask_utils.encode(np.asfortranarray(mask))
    return {
        "size": [int(s) for s in rle["size"]],
        "counts": rle["counts"].decode("ascii"),
    }


class TestCocoRlePycocotools(unittest.TestCase):
    """coco_to_arrow() decodes compressed RLE exactly as pycocotools does."""

    @classmethod
    def setUpClass(cls):
        cls.masks = _masks()
        images, annotations = [], []
        for i, mask in enumerate(cls.masks, start=1):
            height, width = mask.shape
            rle = _compressed_rle(mask)
            images.append(
                {"id": i, "width": width, "height": height, "file_name": f"img{i}.jpg"}
            )
            annotations.append(
                {
                    "id": i,
                    "image_id": i,
                    "category_id": 1,
                    "iscrowd": 0,
                    "area": float(
                        mask_utils.area({**rle, "counts": rle["counts"].encode()})
                    ),
                    "bbox": [0.0, 0.0, float(width), float(height)],
                    "segmentation": rle,
                }
            )
        cls.coco = {
            "images": images,
            "annotations": annotations,
            "categories": [{"id": 1, "name": "region"}],
        }

    def _convert(self, **options) -> pl.DataFrame:
        import edgefirst_client as ec

        with tempfile.TemporaryDirectory() as tmpdir:
            coco_path = os.path.join(tmpdir, "rle.json")
            with open(coco_path, "w") as f:
                json.dump(self.coco, f)
            arrow_path = os.path.join(tmpdir, "rle.arrow")
            ec.coco_to_arrow(coco_path, arrow_path, **options)
            with open(arrow_path, "rb") as f:
                return pl.read_ipc(f.read())

    def test_masks_match_pycocotools(self):
        df = self._convert()
        self.assertEqual(len(df), len(self.masks))
        for row in df.iter_rows(named=True):
            original = self.masks[int(row["object_id"]) - 1]
            self.assertIsNotNone(
                row["mask"], f"annotation {row['object_id']} has no mask"
            )
            png = Image.open(io.BytesIO(row["mask"]))
            self.assertEqual(png.mode, "1", "binary masks are stored as 1-bit PNGs")
            np.testing.assert_array_equal(
                np.asarray(png, dtype=np.uint8),
                original,
                err_msg=f"annotation {row['object_id']} differs from pycocotools",
            )

    def test_rle_to_polygon_stores_polygons(self):
        df = self._convert(rle_to_polygon=True)
        nonempty = {i for i, m in enumerate(self.masks, start=1) if m.any()}
        for row in df.iter_rows(named=True):
            self.assertIsNone(row.get("mask"))
            if int(row["object_id"]) in nonempty:
                self.assertIsNotNone(row["polygon"], f"annotation {row['object_id']}")


    def test_progress_callback_exception_propagates(self):
        def callback(current, total):
            raise OSError("callback failed")

        with self.assertRaisesRegex(OSError, "callback failed"):
            self._convert(progress=callback)

if __name__ == "__main__":
    unittest.main()
