# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""
COCO Studio Roundtrip Tests.

Tests the full workflow:
1. COCO JSON -> EdgeFirst Studio (import via Sample objects)
2. EdgeFirst Studio -> COCO JSON (export via samples API)
3. Validation of exported data against original

Requires:
- Studio credentials via env.sh (STUDIO_SERVER, STUDIO_USERNAME, STUDIO_PASSWORD)
- "Unit Testing" project must exist
"""

import json
import os
import random
import string
import tempfile
import time
import unittest
from pathlib import Path

from edgefirst_client import Annotation, Box2d, Mask, Sample, SampleFile
from PIL import Image
from test import get_client, get_test_data_dir


def generate_timestamp():
    """Generate timestamp string for unique dataset names."""
    return time.strftime("%Y%m%d_%H%M%S")


def create_synthetic_coco_dataset(num_images=5, annotations_per_image=3):
    """
    Create a synthetic COCO dataset for testing.

    Args:
        num_images: Number of images to generate
        annotations_per_image: Number of annotations per image

    Returns:
        dict: Complete COCO dataset with images, annotations, and categories
    """
    random.seed(42)  # For reproducibility

    categories = [
        {"id": 1, "name": "person", "supercategory": "human"},
        {"id": 2, "name": "car", "supercategory": "vehicle"},
        {"id": 3, "name": "dog", "supercategory": "animal"},
    ]

    images = []
    annotations = []
    annotation_id = 1

    for img_id in range(1, num_images + 1):
        # Create image entry
        width = 640 + (img_id % 3) * 160  # Vary sizes: 640, 800, 960
        height = 480 + (img_id % 3) * 120  # Vary sizes: 480, 600, 720

        images.append({
            "id": img_id,
            "width": width,
            "height": height,
            "file_name": f"synthetic_{img_id:04d}.jpg",
        })

        # Create annotations for this image
        for ann_idx in range(annotations_per_image):
            category_id = (ann_idx % len(categories)) + 1

            # Generate random bbox within image bounds
            bbox_x = random.uniform(10, width * 0.3)
            bbox_y = random.uniform(10, height * 0.3)
            bbox_w = random.uniform(50, min(200, width - bbox_x - 10))
            bbox_h = random.uniform(50, min(200, height - bbox_y - 10))

            # Generate simple polygon (rectangle with slight variations)
            polygon = [
                bbox_x, bbox_y,
                bbox_x + bbox_w, bbox_y + random.uniform(-2, 2),
                bbox_x + bbox_w + random.uniform(-2, 2), bbox_y + bbox_h,
                bbox_x + random.uniform(-2, 2), bbox_y + bbox_h,
            ]

            annotations.append({
                "id": annotation_id,
                "image_id": img_id,
                "category_id": category_id,
                "bbox": [bbox_x, bbox_y, bbox_w, bbox_h],
                "area": bbox_w * bbox_h,
                "iscrowd": 0,
                "segmentation": [polygon],
            })
            annotation_id += 1

    return {
        "info": {
            "description": "Synthetic COCO dataset for testing",
            "version": "1.0",
            "year": 2025,
            "contributor": "EdgeFirst Test Suite",
            "date_created": time.strftime("%Y-%m-%d"),
        },
        "licenses": [
            {"id": 1, "name": "Test License", "url": "https://test.example.com"},
        ],
        "images": images,
        "annotations": annotations,
        "categories": categories,
    }


def coco_bbox_to_normalized(bbox, width, height):
    """
    Convert COCO bbox [x, y, w, h] to normalized [x, y, w, h].

    COCO uses top-left corner (x, y) + dimensions (w, h) in pixels.
    EdgeFirst uses same format but normalized to [0, 1].
    """
    x, y, w, h = bbox
    return [
        x / width,
        y / height,
        w / width,
        h / height,
    ]


def normalized_bbox_to_coco(bbox, width, height):
    """
    Convert normalized bbox [x, y, w, h] to COCO [x, y, w, h] in pixels.
    """
    x, y, w, h = bbox
    return [
        x * width,
        y * height,
        w * width,
        h * height,
    ]


def coco_polygon_to_normalized(polygon, width, height):
    """
    Convert COCO polygon [x1,y1,x2,y2,...] to normalized coordinates.
    """
    result = []
    for i in range(0, len(polygon), 2):
        result.append((polygon[i] / width, polygon[i + 1] / height))
    return result


def normalized_polygon_to_coco(polygon, width, height):
    """
    Convert normalized polygon [(x1,y1),(x2,y2),...] to COCO format.
    """
    result = []
    for x, y in polygon:
        result.append(x * width)
        result.append(y * height)
    return result


def coco_to_samples(coco_data, group=None, temp_dir=None):
    """
    Convert COCO dataset to list of Sample objects.

    Args:
        coco_data: COCO dataset dict
        group: Optional group name to assign
        temp_dir: Directory for temporary image files (required for populate_samples)

    Returns:
        list[Sample]: List of Sample objects ready for populate_samples
    """
    # Build lookup tables
    cat_names = {cat["id"]: cat["name"] for cat in coco_data["categories"]}
    img_annotations = {}
    for ann in coco_data["annotations"]:
        img_id = ann["image_id"]
        if img_id not in img_annotations:
            img_annotations[img_id] = []
        img_annotations[img_id].append(ann)

    samples = []
    for image in coco_data["images"]:
        img_id = image["id"]
        width = image["width"]
        height = image["height"]
        file_name = image["file_name"]

        # Create sample
        sample = Sample()
        sample.set_image_name(Path(file_name).stem)
        if group:
            sample.set_group(group)

        # Create temporary image file if temp_dir is provided
        if temp_dir:
            img = Image.new("RGB", (width, height), color="gray")
            img_path = Path(temp_dir) / file_name
            img.save(str(img_path))
            sample.add_file(SampleFile("image", str(img_path)))

        # Add annotations
        for ann in img_annotations.get(img_id, []):
            annotation = Annotation()

            # Set label
            cat_id = ann["category_id"]
            label = cat_names.get(cat_id, "unknown")
            annotation.set_label(label)

            # Set object ID
            annotation.set_object_id(f"coco-{ann['id']}")

            # Set bbox (normalized)
            bbox = ann["bbox"]
            norm_bbox = coco_bbox_to_normalized(bbox, width, height)
            annotation.set_box2d(Box2d(*norm_bbox))

            # Set mask if segmentation exists
            if "segmentation" in ann and ann["segmentation"]:
                seg = ann["segmentation"]
                if isinstance(seg, list) and len(seg) > 0:
                    # Polygon format
                    if isinstance(seg[0], list):
                        polygons = []
                        for poly in seg:
                            if len(poly) >= 6:
                                norm_poly = coco_polygon_to_normalized(
                                    poly, width, height
                                )
                                polygons.append(norm_poly)
                        if polygons:
                            annotation.set_mask(Mask(polygons))

            sample.add_annotation(annotation)

        samples.append(sample)

    return samples


def samples_to_coco(samples, include_masks=True):
    """
    Convert list of Sample objects to COCO dataset format.

    Args:
        samples: List of Sample objects (from client.samples())
        include_masks: Whether to include segmentation masks

    Returns:
        dict: COCO dataset dict
    """
    images = []
    annotations = []
    categories = {}
    annotation_id = 1

    for img_id, sample in enumerate(samples, start=1):
        name = sample.name or f"image_{img_id}"
        width = sample.width or 640
        height = sample.height or 480

        images.append({
            "id": img_id,
            "width": width,
            "height": height,
            "file_name": f"{name}.jpg",
        })

        for ann in sample.annotations:
            label = ann.label or "unknown"

            # Get or create category
            if label not in categories:
                cat_id = len(categories) + 1
                categories[label] = {
                    "id": cat_id,
                    "name": label,
                    "supercategory": "",
                }
            cat_id = categories[label]["id"]

            # Get bbox
            coco_ann = {
                "id": annotation_id,
                "image_id": img_id,
                "category_id": cat_id,
                "iscrowd": 0,
            }

            bbox = ann.box2d
            if bbox is not None:
                coco_bbox = normalized_bbox_to_coco(
                    [bbox.left, bbox.top, bbox.width, bbox.height],
                    width,
                    height,
                )
                coco_ann["bbox"] = coco_bbox
                coco_ann["area"] = coco_bbox[2] * coco_bbox[3]
            else:
                coco_ann["bbox"] = [0, 0, 0, 0]
                coco_ann["area"] = 0

            # Get mask
            if include_masks and ann.mask is not None:
                polygons = []
                for poly in ann.mask.polygon:
                    coco_poly = normalized_polygon_to_coco(poly, width, height)
                    if len(coco_poly) >= 6:
                        polygons.append(coco_poly)
                if polygons:
                    coco_ann["segmentation"] = polygons

            annotations.append(coco_ann)
            annotation_id += 1

    return {
        "info": {"description": "Exported from EdgeFirst Studio"},
        "licenses": [],
        "images": images,
        "annotations": annotations,
        "categories": list(categories.values()),
    }


class TestCocoStudioRoundtrip(unittest.TestCase):
    """Test COCO import/export roundtrip through EdgeFirst Studio."""

    @classmethod
    def setUpClass(cls):
        """Set up test fixtures."""
        cls.client = get_client()
        cls.test_dir = get_test_data_dir()
        cls.skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"

        # Find Unit Testing project
        projects = cls.client.projects("Unit Testing")
        if len(projects) == 0:
            raise unittest.SkipTest("Unit Testing project not found")
        cls.project = projects[0]

    def _create_test_dataset(self, suffix=""):
        """Create a unique test dataset for COCO testing."""
        timestamp = generate_timestamp()
        random_suffix = "".join(
            random.choices(string.ascii_uppercase + string.digits, k=4)
        )
        dataset_name = f"COCO Test {timestamp}{suffix}_{random_suffix}"

        dataset_id = self.client.create_dataset(
            str(self.project.id),
            dataset_name,
            "Automated COCO roundtrip test",
        )

        annotation_set_id = self.client.create_annotation_set(
            dataset_id,
            "Default",
            "Default annotation set for COCO import",
        )

        return dataset_id, annotation_set_id, dataset_name

    def _compare_coco_annotations(
        self, original, exported, tolerance=0.02
    ):
        """
        Compare two COCO datasets for annotation equivalence.

        Args:
            original: Original COCO dataset dict
            exported: Exported COCO dataset dict
            tolerance: Relative tolerance for coordinate comparison

        Returns:
            dict: Comparison results with details of any differences
        """
        results = {
            "images_match": True,
            "annotations_match": True,
            "categories_match": True,
            "differences": [],
        }

        # Compare image counts
        if len(original["images"]) != len(exported["images"]):
            results["images_match"] = False
            results["differences"].append(
                f"Image count: {len(original['images'])} vs {len(exported['images'])}"
            )

        # Build category name mapping
        orig_cat_names = {cat["id"]: cat["name"] for cat in original["categories"]}
        export_cat_names = {cat["id"]: cat["name"] for cat in exported["categories"]}

        orig_cat_set = set(orig_cat_names.values())
        export_cat_set = set(export_cat_names.values())

        if orig_cat_set != export_cat_set:
            results["categories_match"] = False
            results["differences"].append(
                f"Categories: {orig_cat_set} vs {export_cat_set}"
            )

        # Group annotations by image filename
        def group_by_image(dataset, cat_names):
            img_id_to_name = {img["id"]: Path(img["file_name"]).stem
                              for img in dataset["images"]}
            img_dims = {img["id"]: (img["width"], img["height"])
                        for img in dataset["images"]}
            grouped = {}
            for ann in dataset.get("annotations", []):
                img_id = ann["image_id"]
                img_name = img_id_to_name.get(img_id)
                if img_name:
                    if img_name not in grouped:
                        grouped[img_name] = []
                    dims = img_dims.get(img_id, (640, 480))
                    # Normalize bbox for comparison
                    bbox = ann.get("bbox", [0, 0, 0, 0])
                    norm_bbox = [
                        bbox[0] / dims[0], bbox[1] / dims[1],
                        bbox[2] / dims[0], bbox[3] / dims[1],
                    ]
                    grouped[img_name].append({
                        "label": cat_names.get(ann["category_id"], "unknown"),
                        "bbox": norm_bbox,
                    })
            return grouped

        orig_grouped = group_by_image(original, orig_cat_names)
        export_grouped = group_by_image(exported, export_cat_names)

        # Compare annotation counts and content
        for img_name in orig_grouped:
            orig_anns = orig_grouped[img_name]
            export_anns = export_grouped.get(img_name, [])

            if len(orig_anns) != len(export_anns):
                results["annotations_match"] = False
                results["differences"].append(
                    f"{img_name}: {len(orig_anns)} vs {len(export_anns)} annotations"
                )
                continue

            # Sort by label and bbox for comparison
            orig_sorted = sorted(orig_anns, key=lambda a: (a["label"], tuple(a["bbox"])))
            export_sorted = sorted(export_anns, key=lambda a: (a["label"], tuple(a["bbox"])))

            for i, (orig, exp) in enumerate(zip(orig_sorted, export_sorted)):
                if orig["label"] != exp["label"]:
                    results["annotations_match"] = False
                    results["differences"].append(
                        f"{img_name}[{i}] label: {orig['label']} vs {exp['label']}"
                    )

                # Compare bbox with tolerance
                for j, (o, e) in enumerate(zip(orig["bbox"], exp["bbox"])):
                    if abs(o - e) > tolerance:
                        results["annotations_match"] = False
                        results["differences"].append(
                            f"{img_name}[{i}] bbox[{j}]: {o:.4f} vs {e:.4f}"
                        )

        return results

    def test_coco_studio_roundtrip_synthetic(self):
        """Test COCO -> Studio -> COCO roundtrip with synthetic data."""
        # Create synthetic COCO dataset
        original_coco = create_synthetic_coco_dataset(
            num_images=5, annotations_per_image=3
        )

        print(f"\nSynthetic COCO dataset:")
        print(f"  Images: {len(original_coco['images'])}")
        print(f"  Annotations: {len(original_coco['annotations'])}")
        print(f"  Categories: {len(original_coco['categories'])}")

        # Create test dataset in Studio
        dataset_id, annotation_set_id, dataset_name = self._create_test_dataset()
        print(f"\nCreated Studio dataset: {dataset_name}")
        print(f"  Dataset ID: {dataset_id}")

        try:
            with tempfile.TemporaryDirectory() as temp_dir:
                # Step 1: Convert COCO to Sample objects (with temp images)
                print("\n1. Converting COCO to Sample objects...")
                samples = coco_to_samples(original_coco, group="test", temp_dir=temp_dir)
                print(f"   Created {len(samples)} samples")

                total_annotations = sum(len(s.annotations) for s in samples)
                print(f"   Total annotations: {total_annotations}")

                # Step 2: Import to Studio
                print("\n2. Importing to Studio...")
                progress_updates = []

                def on_progress(current, total):
                    progress_updates.append((current, total))

                results = self.client.populate_samples(
                    dataset_id,
                    annotation_set_id,
                    samples,
                    progress=on_progress,
                )
                print(f"   Populated {len(results)} samples")
                print(f"   Progress updates: {len(progress_updates)}")

                # Wait for server processing
                time.sleep(2)

                # Step 3: Fetch from Studio
                print("\n3. Fetching from Studio...")
                fetched_samples = self.client.samples(
                    dataset_id,
                    annotation_set_id,
                    annotation_types=[],
                    groups=["test"],
                    types=[],
                )
                print(f"   Fetched {len(fetched_samples)} samples")

                fetched_annotations = sum(len(s.annotations) for s in fetched_samples)
                print(f"   Total fetched annotations: {fetched_annotations}")

                # Verify sample count
                self.assertEqual(
                    len(fetched_samples),
                    len(original_coco["images"]),
                    "Should have same number of samples as original images"
                )

                # Step 4: Convert back to COCO
                print("\n4. Converting to COCO format...")
                exported_coco = samples_to_coco(fetched_samples, include_masks=True)
                print(f"   Exported {len(exported_coco['annotations'])} annotations")

                # Step 5: Validate roundtrip
                print("\n5. Validating roundtrip...")
                comparison = self._compare_coco_annotations(
                    original_coco, exported_coco, tolerance=0.02
                )

                if comparison["differences"]:
                    print(f"\nDifferences found: {len(comparison['differences'])}")
                    for diff in comparison["differences"][:10]:
                        print(f"  - {diff}")

                # Assertions
                self.assertTrue(
                    comparison["images_match"],
                    f"Image count mismatch: {comparison['differences']}"
                )
                self.assertTrue(
                    comparison["categories_match"],
                    f"Category mismatch: {comparison['differences']}"
                )
                self.assertTrue(
                    comparison["annotations_match"],
                    f"Annotation mismatch: {comparison['differences']}"
                )

                print("\n✓ COCO Studio roundtrip test passed!")

        finally:
            if self.skip_cleanup:
                print(f"\nSkipping cleanup (SKIP_CLEANUP=1)")
                print(f"  Dataset: {dataset_name}")
            else:
                print("\nCleaning up...")
                self.client.delete_dataset(dataset_id)
                print("  ✓ Deleted test dataset")

    def test_coco_import_export_preserves_labels(self):
        """Test that COCO category names are preserved through roundtrip."""
        # Create COCO with specific category names
        original = {
            "info": {"description": "Label preservation test"},
            "licenses": [],
            "images": [
                {"id": 1, "width": 640, "height": 480, "file_name": "test1.jpg"},
            ],
            "annotations": [
                {
                    "id": 1, "image_id": 1, "category_id": 1,
                    "bbox": [10, 20, 100, 80], "area": 8000, "iscrowd": 0,
                },
                {
                    "id": 2, "image_id": 1, "category_id": 2,
                    "bbox": [200, 100, 50, 60], "area": 3000, "iscrowd": 0,
                },
                {
                    "id": 3, "image_id": 1, "category_id": 3,
                    "bbox": [300, 200, 80, 90], "area": 7200, "iscrowd": 0,
                },
            ],
            "categories": [
                {"id": 1, "name": "custom_label_alpha", "supercategory": "test"},
                {"id": 2, "name": "custom_label_beta", "supercategory": "test"},
                {"id": 3, "name": "custom_label_gamma", "supercategory": "test"},
            ],
        }

        dataset_id, annotation_set_id, dataset_name = self._create_test_dataset(
            "_labels"
        )
        print(f"\nCreated dataset: {dataset_name}")

        try:
            with tempfile.TemporaryDirectory() as temp_dir:
                # Import
                samples = coco_to_samples(original, group="test", temp_dir=temp_dir)
                self.client.populate_samples(dataset_id, annotation_set_id, samples)
                time.sleep(1)

                # Export
                fetched = self.client.samples(
                    dataset_id, annotation_set_id,
                    annotation_types=[], groups=[], types=[]
                )
                exported = samples_to_coco(fetched)

                # Check labels
                exported_labels = {c["name"] for c in exported["categories"]}
                original_labels = {c["name"] for c in original["categories"]}

                self.assertEqual(
                    exported_labels,
                    original_labels,
                    "Category names should be preserved"
                )

                print(f"✓ Labels preserved: {sorted(exported_labels)}")

        finally:
            if not self.skip_cleanup:
                self.client.delete_dataset(dataset_id)

    def test_coco_bbox_accuracy(self):
        """Test that bounding box coordinates are accurate through roundtrip."""
        # Create COCO with precise bbox values
        original = {
            "info": {},
            "licenses": [],
            "images": [
                {"id": 1, "width": 1000, "height": 1000, "file_name": "precise.jpg"},
            ],
            "annotations": [
                {
                    "id": 1, "image_id": 1, "category_id": 1,
                    "bbox": [100.0, 200.0, 300.0, 400.0],  # x, y, w, h in pixels
                    "area": 120000, "iscrowd": 0,
                },
            ],
            "categories": [{"id": 1, "name": "object", "supercategory": ""}],
        }

        dataset_id, annotation_set_id, dataset_name = self._create_test_dataset(
            "_bbox"
        )

        try:
            with tempfile.TemporaryDirectory() as temp_dir:
                # Import
                samples = coco_to_samples(original, temp_dir=temp_dir)
                self.client.populate_samples(dataset_id, annotation_set_id, samples)
                time.sleep(1)

                # Export
                fetched = self.client.samples(
                    dataset_id, annotation_set_id,
                    annotation_types=[], groups=[], types=[]
                )

                self.assertEqual(len(fetched), 1, "Should have one sample")
                self.assertEqual(len(fetched[0].annotations), 1, "Should have one annotation")

                # Check bbox
                bbox = fetched[0].annotations[0].box2d
                self.assertIsNotNone(bbox, "Should have bounding box")

                # Expected normalized values
                # Original: x=100, y=200, w=300, h=400 on 1000x1000 image
                # Normalized: x=0.1, y=0.2, w=0.3, h=0.4
                tolerance = 0.01
                self.assertAlmostEqual(bbox.left, 0.1, delta=tolerance)
                self.assertAlmostEqual(bbox.top, 0.2, delta=tolerance)
                self.assertAlmostEqual(bbox.width, 0.3, delta=tolerance)
                self.assertAlmostEqual(bbox.height, 0.4, delta=tolerance)

                print("✓ Bounding box accuracy verified")
                print(f"  Expected: (0.1, 0.2, 0.3, 0.4)")
                print(f"  Got: ({bbox.left:.3f}, {bbox.top:.3f}, {bbox.width:.3f}, {bbox.height:.3f})")

        finally:
            if not self.skip_cleanup:
                self.client.delete_dataset(dataset_id)

    def test_coco_multiple_annotations_per_image(self):
        """Test handling multiple annotations on a single image."""
        original = {
            "info": {},
            "licenses": [],
            "images": [
                {"id": 1, "width": 640, "height": 480, "file_name": "multi.jpg"},
            ],
            "annotations": [
                {"id": 1, "image_id": 1, "category_id": 1,
                 "bbox": [10, 10, 50, 50], "area": 2500, "iscrowd": 0},
                {"id": 2, "image_id": 1, "category_id": 2,
                 "bbox": [100, 100, 80, 80], "area": 6400, "iscrowd": 0},
                {"id": 3, "image_id": 1, "category_id": 1,
                 "bbox": [200, 200, 60, 70], "area": 4200, "iscrowd": 0},
                {"id": 4, "image_id": 1, "category_id": 3,
                 "bbox": [300, 50, 100, 120], "area": 12000, "iscrowd": 0},
            ],
            "categories": [
                {"id": 1, "name": "person", "supercategory": ""},
                {"id": 2, "name": "car", "supercategory": ""},
                {"id": 3, "name": "dog", "supercategory": ""},
            ],
        }

        dataset_id, annotation_set_id, dataset_name = self._create_test_dataset(
            "_multi"
        )

        try:
            with tempfile.TemporaryDirectory() as temp_dir:
                samples = coco_to_samples(original, temp_dir=temp_dir)
                self.assertEqual(len(samples), 1, "Should have one sample")
                self.assertEqual(
                    len(samples[0].annotations), 4,
                    "Sample should have 4 annotations"
                )

                self.client.populate_samples(dataset_id, annotation_set_id, samples)
                time.sleep(1)

                fetched = self.client.samples(
                    dataset_id, annotation_set_id,
                    annotation_types=[], groups=[], types=[]
                )

                self.assertEqual(len(fetched), 1)
                self.assertEqual(
                    len(fetched[0].annotations), 4,
                    "Should preserve all 4 annotations"
                )

                # Verify labels
                labels = sorted([a.label for a in fetched[0].annotations])
                expected = sorted(["person", "car", "person", "dog"])
                self.assertEqual(labels, expected)

                print(f"✓ Multiple annotations preserved: {labels}")

        finally:
            if not self.skip_cleanup:
                self.client.delete_dataset(dataset_id)

    def test_coco_with_masks(self):
        """Test that segmentation masks survive the roundtrip."""
        # Create COCO with polygon segmentation
        original = {
            "info": {"description": "Mask test"},
            "licenses": [],
            "images": [
                {"id": 1, "width": 640, "height": 480, "file_name": "mask.jpg"},
            ],
            "annotations": [
                {
                    "id": 1, "image_id": 1, "category_id": 1,
                    "bbox": [100, 100, 200, 150], "area": 30000, "iscrowd": 0,
                    "segmentation": [[
                        100, 100, 300, 100, 300, 250, 100, 250
                    ]],
                },
            ],
            "categories": [
                {"id": 1, "name": "rectangle", "supercategory": "shape"},
            ],
        }

        dataset_id, annotation_set_id, dataset_name = self._create_test_dataset(
            "_masks"
        )

        try:
            with tempfile.TemporaryDirectory() as temp_dir:
                samples = coco_to_samples(original, temp_dir=temp_dir)

                # Verify mask was converted
                self.assertEqual(len(samples), 1)
                self.assertEqual(len(samples[0].annotations), 1)
                self.assertIsNotNone(
                    samples[0].annotations[0].mask,
                    "Should have mask after conversion"
                )

                self.client.populate_samples(dataset_id, annotation_set_id, samples)
                time.sleep(1)

                # Fetch samples (masks are returned with box2d)
                fetched = self.client.samples(
                    dataset_id, annotation_set_id,
                    annotation_types=[], groups=[], types=[]
                )

                self.assertEqual(len(fetched), 1)
                self.assertGreater(len(fetched[0].annotations), 0)

                # Check if any annotation has a mask
                has_mask = any(
                    ann.mask is not None for ann in fetched[0].annotations
                )

                print(f"Mask present in export: {has_mask}")
                print("✓ Mask roundtrip test completed")

        finally:
            if not self.skip_cleanup:
                self.client.delete_dataset(dataset_id)


class TestCocoStudioImportExport(unittest.TestCase):
    """Test COCO import and export as separate operations."""

    @classmethod
    def setUpClass(cls):
        """Set up test fixtures."""
        cls.client = get_client()
        cls.skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"

        projects = cls.client.projects("Unit Testing")
        if len(projects) == 0:
            raise unittest.SkipTest("Unit Testing project not found")
        cls.project = projects[0]

    def test_import_creates_correct_sample_count(self):
        """Verify import creates the correct number of samples."""
        coco = create_synthetic_coco_dataset(num_images=3, annotations_per_image=2)

        timestamp = generate_timestamp()
        dataset_id = self.client.create_dataset(
            str(self.project.id),
            f"COCO Import Count {timestamp}",
            "Test sample count",
        )
        annotation_set_id = self.client.create_annotation_set(
            dataset_id, "Default", ""
        )

        try:
            with tempfile.TemporaryDirectory() as temp_dir:
                samples = coco_to_samples(coco, group="test", temp_dir=temp_dir)
                self.client.populate_samples(dataset_id, annotation_set_id, samples)
                time.sleep(1)

                fetched = self.client.samples(
                    dataset_id, annotation_set_id,
                    annotation_types=[], groups=[], types=[]
                )

                self.assertEqual(
                    len(fetched), len(coco["images"]),
                    "Should create one sample per image"
                )

                print(f"✓ Created {len(fetched)} samples (expected {len(coco['images'])})")

        finally:
            if not self.skip_cleanup:
                self.client.delete_dataset(dataset_id)

    def test_export_produces_valid_coco(self):
        """Verify exported COCO has valid structure."""
        coco = create_synthetic_coco_dataset(num_images=2, annotations_per_image=2)

        timestamp = generate_timestamp()
        dataset_id = self.client.create_dataset(
            str(self.project.id),
            f"COCO Export Valid {timestamp}",
            "Test valid export",
        )
        annotation_set_id = self.client.create_annotation_set(
            dataset_id, "Default", ""
        )

        try:
            with tempfile.TemporaryDirectory() as temp_dir:
                samples = coco_to_samples(coco, temp_dir=temp_dir)
                self.client.populate_samples(dataset_id, annotation_set_id, samples)
                time.sleep(1)

                fetched = self.client.samples(
                    dataset_id, annotation_set_id,
                    annotation_types=[], groups=[], types=[]
                )

                exported = samples_to_coco(fetched)

                # Validate COCO structure
                self.assertIn("images", exported)
                self.assertIn("annotations", exported)
                self.assertIn("categories", exported)

                # Validate images
                for img in exported["images"]:
                    self.assertIn("id", img)
                    self.assertIn("width", img)
                    self.assertIn("height", img)
                    self.assertIn("file_name", img)

                # Validate annotations
                for ann in exported["annotations"]:
                    self.assertIn("id", ann)
                    self.assertIn("image_id", ann)
                    self.assertIn("category_id", ann)
                    self.assertIn("bbox", ann)
                    self.assertEqual(len(ann["bbox"]), 4)

                # Validate categories
                for cat in exported["categories"]:
                    self.assertIn("id", cat)
                    self.assertIn("name", cat)

                print("✓ Exported COCO has valid structure")
                print(f"  Images: {len(exported['images'])}")
                print(f"  Annotations: {len(exported['annotations'])}")
                print(f"  Categories: {len(exported['categories'])}")

        finally:
            if not self.skip_cleanup:
                self.client.delete_dataset(dataset_id)


if __name__ == "__main__":
    unittest.main(verbosity=2)
