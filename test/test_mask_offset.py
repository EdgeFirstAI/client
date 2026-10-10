# SPDX-License-Identifier: Apache-2.0
# Copyright © 2026 Au-Zone Technologies. All Rights Reserved.

import struct
import unittest
import zlib


def _chunk(kind: bytes, data: bytes) -> bytes:
    crc = zlib.crc32(kind + data) & 0xFFFFFFFF
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", crc)


def _gray8_png(width: int, height: int, offs: bytes = None) -> bytes:
    """Builds an 8-bit grayscale PNG, optionally with a raw oFFs payload."""
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 0, 0, 0, 0)
    raw = b"".join(b"\x00" + b"\xff" * width for _ in range(height))
    png = b"\x89PNG\r\n\x1a\n" + _chunk(b"IHDR", ihdr)
    if offs is not None:
        png += _chunk(b"oFFs", offs)
    png += _chunk(b"IDAT", zlib.compress(raw)) + _chunk(b"IEND", b"")
    return png


class TestMaskOffset(unittest.TestCase):
    def test_no_mask(self):
        import edgefirst_client as ec

        self.assertIsNone(ec.Annotation().mask_offset)

    def test_image_sized_mask_has_no_offset(self):
        import edgefirst_client as ec

        ann = ec.Annotation()
        ann.set_mask(_gray8_png(4, 3))
        self.assertIsNone(ann.mask_offset)

    def test_pixel_offset(self):
        import edgefirst_client as ec

        ann = ec.Annotation()
        ann.set_mask(_gray8_png(4, 3, struct.pack(">iiB", 120, -7, 0)))
        self.assertEqual(ann.mask_offset, (120, -7))

    def test_non_pixel_unit_ignored(self):
        import edgefirst_client as ec

        ann = ec.Annotation()
        ann.set_mask(_gray8_png(4, 3, struct.pack(">iiB", 120, 7, 1)))
        self.assertIsNone(ann.mask_offset)


if __name__ == "__main__":
    unittest.main()
