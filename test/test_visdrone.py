"""
VisDrone2019 conversion tests for the Python API.

Credential-free: builds a synthetic DET split in a temporary directory.
"""

import os
import struct
import tempfile
import unittest
import zlib


def _write_png(path: str, width: int, height: int) -> None:
    """Write a minimal grayscale PNG so image sizes can be read."""

    def chunk(tag: bytes, data: bytes) -> bytes:
        body = tag + data
        return (
            struct.pack(">I", len(data))
            + body
            + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)
        )

    raw = b"".join(b"\x00" + b"\x00" * width for _ in range(height))
    png = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 0, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(raw))
        + chunk(b"IEND", b"")
    )
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        f.write(png)


class TestVisDroneConversion(unittest.TestCase):
    def _make_det_split(self, root: str) -> str:
        split = os.path.join(root, "VisDrone2019-DET-val")
        _write_png(os.path.join(split, "images", "a.png"), 200, 100)
        os.makedirs(os.path.join(split, "annotations"), exist_ok=True)
        with open(os.path.join(split, "annotations", "a.txt"), "w") as f:
            f.write(
                "10,20,40,10,1,4,0,1\r\n0,0,200,100,0,0,0,0\r\n50,50,20,20,0,11,0,0\r\n"
            )
        return split

    def test_det_split_to_arrow(self):
        import edgefirst_client as ec
        import polars as pl

        with tempfile.TemporaryDirectory() as tmp:
            split = self._make_det_split(tmp)
            output = os.path.join(tmp, "out", "out.arrow")
            rows = ec.visdrone_to_arrow([split], output, stage_images=True)
            self.assertEqual(rows, 1)

            df = pl.read_ipc(output)
            self.assertEqual(df.height, 1)
            self.assertNotIn("ignore", df.columns)
            self.assertNotIn("exclude", df.columns)
            self.assertEqual(df["label"].cast(pl.String).to_list(), ["car"])
            self.assertEqual(df["label_index"].to_list(), [3])
            self.assertEqual(df["truncation"].to_list(), [0])
            self.assertEqual(df["occlusion"].to_list(), [1])
            self.assertEqual(df["group"].cast(pl.String).to_list(), ["val"])
            self.assertTrue(os.path.exists(os.path.join(tmp, "out", "out", "a.png")))

        with tempfile.TemporaryDirectory() as tmp:
            split = self._make_det_split(tmp)
            output = os.path.join(tmp, "out", "out.arrow")
            rows = ec.visdrone_to_arrow([split], output, keep_ignored=True)
            self.assertEqual(rows, 3)

            df = pl.read_ipc(output)
            car = df.filter(pl.col("label_index") == 3)
            self.assertEqual(car.height, 1)
            self.assertEqual(car["ignore"].to_list(), [None])
            self.assertEqual(car["exclude"].to_list(), [None])

            region = df.filter(pl.col("ignore"))
            self.assertEqual(region.height, 1)
            self.assertEqual(region["exclude"].to_list(), [None])
            self.assertTrue(region["label"].is_null().all())
            self.assertTrue(region["label_index"].is_null().all())

            other = df.filter(pl.col("exclude"))
            self.assertEqual(other.height, 1)
            self.assertEqual(other["ignore"].to_list(), [None])
            self.assertTrue(other["label"].is_null().all())
            self.assertTrue(other["label_index"].is_null().all())

    def test_progress_callback_and_group(self):
        import edgefirst_client as ec

        calls = []

        def on_progress(current, total, status=None):
            calls.append((current, total))

        with tempfile.TemporaryDirectory() as tmp:
            split = self._make_det_split(tmp)
            renamed = os.path.join(tmp, "unnamed")
            os.rename(split, renamed)
            output = os.path.join(tmp, "o", "o.parquet")
            with self.assertRaises(Exception):
                ec.visdrone_to_arrow([renamed], output)
            rows = ec.visdrone_to_arrow(
                [renamed], output, group="test-dev", progress=on_progress
            )
            self.assertEqual(rows, 1)
            self.assertGreater(len(calls), 0)
            self.assertEqual(calls[-1][0], calls[-1][1])


if __name__ == "__main__":
    unittest.main()
