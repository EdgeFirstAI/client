# SPDX-License-Identifier: Apache-2.0
# Copyright © 2026 Au-Zone Technologies. All Rights Reserved.

"""
ADE20K conversion with ade20k_to_arrow().

A small ADEChallengeData2016 tree is converted and each stored label map is
decoded and compared pixel for pixel with its source PNG.
"""

import io
import json
import os
import tempfile
import unittest

import numpy as np
import polars as pl
from PIL import Image

OBJECT_INFO = (
    "Idx\tRatio\tTrain\tVal\tName\n"
    "1\t0.1\t1\t1\twall\n"
    "2\t0.1\t1\t1\tbuilding, edifice\n"
    "3\t0.1\t1\t1\tsky\n"
)


def _make_ade20k(root: str) -> tuple:
    """Write one validation and one training image with label maps;
    return the dataset root and the label arrays by stem."""
    ade = os.path.join(root, "ADEChallengeData2016")
    os.makedirs(ade)
    with open(os.path.join(ade, "objectInfo150.txt"), "w") as f:
        f.write(OBJECT_INFO)
    rng = np.random.default_rng(150)
    labels = {}
    for split, stem in [
        ("validation", "ADE_val_00000001"),
        ("training", "ADE_train_00000001"),
    ]:
        for kind in ("images", "annotations"):
            os.makedirs(os.path.join(ade, kind, split), exist_ok=True)
        Image.new("RGB", (6, 4)).save(os.path.join(ade, "images", split, f"{stem}.jpg"))
        label = rng.integers(0, 4, size=(4, 6), dtype=np.uint8)
        Image.fromarray(label).save(
            os.path.join(ade, "annotations", split, f"{stem}.png")
        )
        labels[stem] = label
    return ade, labels


class TestAde20k(unittest.TestCase):
    """ade20k_to_arrow() stores ADE20K label maps losslessly."""

    def test_label_maps_match_source_pixels(self):
        import edgefirst_client as ec

        with tempfile.TemporaryDirectory() as tmpdir:
            ade, labels = _make_ade20k(tmpdir)
            output = os.path.join(tmpdir, "out", "out.arrow")
            self.assertEqual(ec.ade20k_to_arrow(ade, output), 2)
            df = pl.read_ipc(output, memory_map=False)

        self.assertEqual(sorted(df["group"].cast(pl.String)), ["train", "val"])
        for row in df.iter_rows(named=True):
            png = Image.open(io.BytesIO(row["mask"]))
            self.assertEqual(png.mode, "L", "label maps are 8-bit grayscale")
            np.testing.assert_array_equal(
                np.asarray(png), labels[row["name"]], err_msg=row["name"]
            )

    def test_group_and_metadata(self):
        import edgefirst_client as ec

        with tempfile.TemporaryDirectory() as tmpdir:
            ade, _ = _make_ade20k(tmpdir)
            output = os.path.join(tmpdir, "val", "val.parquet")
            self.assertEqual(ec.ade20k_to_arrow(ade, output, group="val"), 1)
            meta = pl.read_parquet_metadata(output)

        self.assertEqual(json.loads(meta["labels"])[:2], ["unlabeled", "wall"])
        self.assertEqual(meta["ignore_index"], "0")
        self.assertEqual(meta["mask_interpretation"], "class_index")


if __name__ == "__main__":
    unittest.main()
