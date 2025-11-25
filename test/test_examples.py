# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""
Tests for example scripts to ensure they work correctly and are covered.

These integration tests verify:
- download.py example works with Deer dataset
- coco.py example works (future)
"""

import os
import shutil
from pathlib import Path
from unittest import TestCase

from examples.download import download_dataset_yolo
from test import get_client


class ExamplesTest(TestCase):
    """Test suite for example scripts."""

    def test_download_example(self):
        """Test download.py example with Deer dataset."""
        # Skip if credentials not available
        if not os.environ.get("STUDIO_USERNAME") or not os.environ.get(
            "STUDIO_PASSWORD"
        ):
            self.skipTest("Studio credentials not available")

        client = get_client()

        # Find the Deer dataset in the Unit Testing project
        projects = client.projects("Unit Testing")
        assert len(projects) > 0
        project = projects[0]

        datasets = client.datasets(project.id, "Deer")
        assert len(datasets) > 0
        dataset = datasets[0]

        # Create temporary output directory
        output_dir = Path("target/test_download_example")
        if output_dir.exists():
            shutil.rmtree(output_dir)
        output_dir.mkdir(parents=True, exist_ok=True)

        try:
            # Call the download function directly (covered by slipcover)
            download_dataset_yolo(str(dataset.id), str(output_dir), "val")

            # Verify output directory structure
            val_dir = output_dir / "val"
            self.assertTrue(val_dir.exists(), "val directory should exist")

            # Verify annotation files were created (YOLO format in nested directories)
            # Annotations should be in the same directory as their corresponding images
            txt_files = list(val_dir.rglob("*.txt"))
            self.assertGreater(
                len(txt_files), 0, "Should have created annotation files"
            )

            # Verify image files were downloaded (in nested sequence directories)
            jpg_files = list(val_dir.rglob("*.jpg"))
            png_files = list(val_dir.rglob("*.png"))
            image_files = jpg_files + png_files

            self.assertGreater(
                len(image_files),
                0,
                f"Should have downloaded image files. Found {len(txt_files)} txt files but no images",
            )

            # Verify annotations are co-located with images
            self.assertEqual(
                len(txt_files),
                len(image_files),
                f"Should have same number of annotations ({len(txt_files)}) and images ({len(image_files)})",
            )

            # Verify annotations are properly formatted (YOLO format)
            if txt_files:
                with open(txt_files[0], "r") as f:
                    lines = f.readlines()
                    if lines:
                        # Each line should have 5 values: class cx cy width height
                        parts = lines[0].strip().split()
                        self.assertEqual(
                            len(parts),
                            5,
                            "Annotation line should have 5 values (class cx cy w h)",
                        )

        finally:
            # Cleanup
            if output_dir.exists():
                shutil.rmtree(output_dir)
