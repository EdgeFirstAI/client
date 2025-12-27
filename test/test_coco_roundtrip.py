"""
COCO format round-trip tests for Python API.

Tests the coco_to_arrow() and arrow_to_coco() functions for format conversion.
"""

import json
import os
import tempfile
import unittest


class TestCocoRoundtrip(unittest.TestCase):
    """Test COCO <-> Arrow round-trip conversion via Python API."""

    @classmethod
    def setUpClass(cls):
        """Check if COCO dataset is available."""
        cls.coco_path = os.path.expanduser(
            "~/Datasets/COCO/annotations/instances_val2017.json"
        )
        cls.coco_available = os.path.exists(cls.coco_path)

    def test_coco_to_arrow_basic(self):
        """Test basic COCO JSON -> Arrow conversion."""
        import edgefirst_client as ec

        if not self.coco_available:
            self.skipTest("COCO dataset not available")

        with tempfile.TemporaryDirectory() as tmpdir:
            arrow_path = os.path.join(tmpdir, "test.arrow")
            count = ec.coco_to_arrow(self.coco_path, arrow_path, group="val")

            self.assertGreater(count, 0, "Should convert at least some annotations")
            self.assertTrue(os.path.exists(arrow_path), "Arrow file should be created")

    def test_coco_to_arrow_with_masks(self):
        """Test COCO JSON -> Arrow conversion with segmentation masks."""
        import edgefirst_client as ec

        if not self.coco_available:
            self.skipTest("COCO dataset not available")

        with tempfile.TemporaryDirectory() as tmpdir:
            arrow_path = os.path.join(tmpdir, "test_masks.arrow")
            count = ec.coco_to_arrow(
                self.coco_path,
                arrow_path,
                include_masks=True,
                group="val",
            )

            self.assertGreater(count, 0, "Should convert annotations with masks")
            self.assertTrue(os.path.exists(arrow_path), "Arrow file should be created")

    def test_arrow_to_coco_basic(self):
        """Test Arrow -> COCO JSON conversion."""
        import edgefirst_client as ec

        if not self.coco_available:
            self.skipTest("COCO dataset not available")

        with tempfile.TemporaryDirectory() as tmpdir:
            arrow_path = os.path.join(tmpdir, "intermediate.arrow")
            coco_path = os.path.join(tmpdir, "output.json")

            # COCO -> Arrow
            count1 = ec.coco_to_arrow(self.coco_path, arrow_path, group="val")
            self.assertGreater(count1, 0)

            # Arrow -> COCO
            count2 = ec.arrow_to_coco(arrow_path, coco_path)
            self.assertGreater(count2, 0, "Should convert back to COCO format")
            self.assertTrue(os.path.exists(coco_path), "COCO JSON should be created")

    def test_coco_arrow_roundtrip(self):
        """Test full COCO JSON -> Arrow -> COCO JSON round-trip."""
        import edgefirst_client as ec

        if not self.coco_available:
            self.skipTest("COCO dataset not available")

        with tempfile.TemporaryDirectory() as tmpdir:
            arrow_path = os.path.join(tmpdir, "test.arrow")
            restored_path = os.path.join(tmpdir, "restored.json")

            # COCO -> Arrow
            count = ec.coco_to_arrow(
                self.coco_path,
                arrow_path,
                include_masks=True,
                group="val",
            )

            # Arrow -> COCO
            ec.arrow_to_coco(arrow_path, restored_path, include_masks=True)

            # Verify restored file
            with open(restored_path) as f:
                restored = json.load(f)

            self.assertEqual(
                len(restored["annotations"]),
                count,
                "Annotation count should match",
            )
            self.assertEqual(
                len(restored["categories"]),
                80,
                "Should have 80 COCO categories",
            )

    def test_coco_with_progress(self):
        """Test conversion with progress callback."""
        import edgefirst_client as ec

        if not self.coco_available:
            self.skipTest("COCO dataset not available")

        progress_updates = []

        def on_progress(current, total):
            progress_updates.append((current, total))

        with tempfile.TemporaryDirectory() as tmpdir:
            arrow_path = os.path.join(tmpdir, "test.arrow")
            ec.coco_to_arrow(self.coco_path, arrow_path, progress=on_progress)

            self.assertGreater(
                len(progress_updates),
                0,
                "Progress callback should be called",
            )
            # Note: Progress updates may arrive out of order due to parallel processing
            # Just verify we get sensible values
            for current, total in progress_updates:
                self.assertGreaterEqual(current, 0, "Current should be non-negative")
                self.assertGreater(total, 0, "Total should be positive")
                self.assertLessEqual(current, total, "Current should not exceed total")

    def test_coco_roundtrip_with_synthetic_data(self):
        """Test round-trip with synthetic COCO data (no external dependencies)."""
        import edgefirst_client as ec

        synthetic_coco = {
            "info": {
                "description": "Synthetic test dataset",
                "version": "1.0",
            },
            "licenses": [],
            "images": [
                {
                    "id": 1,
                    "width": 640,
                    "height": 480,
                    "file_name": "test1.jpg",
                },
                {
                    "id": 2,
                    "width": 800,
                    "height": 600,
                    "file_name": "test2.jpg",
                },
            ],
            "annotations": [
                {
                    "id": 1,
                    "image_id": 1,
                    "category_id": 1,
                    "bbox": [10, 20, 100, 80],
                    "area": 8000,
                    "iscrowd": 0,
                    "segmentation": [[10, 20, 110, 20, 110, 100, 10, 100]],
                },
                {
                    "id": 2,
                    "image_id": 1,
                    "category_id": 2,
                    "bbox": [200, 150, 50, 60],
                    "area": 3000,
                    "iscrowd": 0,
                },
                {
                    "id": 3,
                    "image_id": 2,
                    "category_id": 1,
                    "bbox": [50, 50, 200, 150],
                    "area": 30000,
                    "iscrowd": 0,
                    "segmentation": [[50, 50, 250, 50, 250, 200, 50, 200]],
                },
            ],
            "categories": [
                {"id": 1, "name": "person", "supercategory": "human"},
                {"id": 2, "name": "car", "supercategory": "vehicle"},
            ],
        }

        with tempfile.TemporaryDirectory() as tmpdir:
            input_path = os.path.join(tmpdir, "input.json")
            arrow_path = os.path.join(tmpdir, "converted.arrow")
            output_path = os.path.join(tmpdir, "output.json")

            # Write synthetic COCO
            with open(input_path, "w") as f:
                json.dump(synthetic_coco, f)

            # COCO -> Arrow
            count1 = ec.coco_to_arrow(
                input_path,
                arrow_path,
                include_masks=True,
                group="test",
            )
            self.assertEqual(count1, 3, "Should convert 3 annotations")

            # Arrow -> COCO
            count2 = ec.arrow_to_coco(arrow_path, output_path, include_masks=True)
            self.assertEqual(count2, 3, "Should restore 3 annotations")

            # Verify content
            with open(output_path) as f:
                restored = json.load(f)

            self.assertEqual(len(restored["images"]), 2)
            self.assertEqual(len(restored["annotations"]), 3)
            self.assertEqual(len(restored["categories"]), 2)

            # Verify category names are preserved
            category_names = {c["name"] for c in restored["categories"]}
            self.assertIn("person", category_names)
            self.assertIn("car", category_names)

    def test_coco_group_filtering(self):
        """Test Arrow -> COCO with group filtering."""
        import edgefirst_client as ec

        if not self.coco_available:
            self.skipTest("COCO dataset not available")

        with tempfile.TemporaryDirectory() as tmpdir:
            arrow_path = os.path.join(tmpdir, "test.arrow")
            output_path = os.path.join(tmpdir, "filtered.json")

            # COCO -> Arrow with group
            count1 = ec.coco_to_arrow(self.coco_path, arrow_path, group="val")
            self.assertGreater(count1, 0)

            # Arrow -> COCO with group filter
            count2 = ec.arrow_to_coco(
                arrow_path,
                output_path,
                groups=["val"],
            )
            self.assertEqual(
                count1,
                count2,
                "Filtered count should match original",
            )


class TestCocoConversionEdgeCases(unittest.TestCase):
    """Test edge cases and error handling in COCO conversion."""

    def test_coco_to_arrow_missing_file(self):
        """Test error handling for missing input file."""
        import edgefirst_client as ec

        with tempfile.TemporaryDirectory() as tmpdir:
            arrow_path = os.path.join(tmpdir, "output.arrow")

            with self.assertRaises(Exception):
                ec.coco_to_arrow("/nonexistent/path.json", arrow_path)

    def test_arrow_to_coco_missing_file(self):
        """Test error handling for missing Arrow file."""
        import edgefirst_client as ec

        with tempfile.TemporaryDirectory() as tmpdir:
            output_path = os.path.join(tmpdir, "output.json")

            with self.assertRaises(Exception):
                ec.arrow_to_coco("/nonexistent/path.arrow", output_path)


if __name__ == "__main__":
    unittest.main()
