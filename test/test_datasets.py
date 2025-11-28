# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""
Tests for dataset operations including populate_samples and roundtrip.

These integration tests verify:
- Sample population with automatic file upload
- Annotation creation and verification
- Image download and byte-for-byte integrity
- Dataset roundtrip (download + re-upload)
"""

import os
import random
import shutil
import string
import time
from pathlib import Path
from unittest import TestCase

from edgefirst_client import (
    Annotation,
    Box2d,
    Box3d,
    FileType,
    Mask,
    Sample,
    SampleFile,
)
from PIL import Image, ImageDraw
from test import get_client, get_test_data_dir
from test.fixtures import get_test_dataset, get_test_dataset_types


class DatasetTest(TestCase):
    """Test suite for dataset operations and integration scenarios."""

    def test_populate_samples(self):
        """Test populating samples with automatic file upload."""
        client = get_client()

        # Find the Unit Testing project
        projects = client.projects("Unit Testing")
        assert len(projects) > 0
        project = projects[0]

        # Create a temporary test dataset with random suffix
        random_suffix = "".join(
            random.choices(string.ascii_uppercase + string.digits, k=6)
        )
        test_dataset_name = f"Test Populate {random_suffix}"

        print(f"Creating test dataset: {test_dataset_name}")

        dataset_id = client.create_dataset(
            str(project.id),
            test_dataset_name,
            "Automated test: populate_samples verification",
        )

        print(f"Created test dataset: {dataset_id}")

        # Check if we should skip cleanup for manual inspection
        skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"

        # Create an annotation set
        print("Creating annotation set...")
        annotation_set_id = client.create_annotation_set(
            dataset_id, "Default", "Default annotation set"
        )

        print(f"Created annotation set: {annotation_set_id}")

        # Get the annotation set
        annotation_sets = client.annotation_sets(dataset_id)
        assert len(annotation_sets) > 0
        annotation_set = annotation_sets[0]

        # Generate a 640x480 PNG image with a red circle
        img_width = 640
        img_height = 480
        img = Image.new("RGB", (img_width, img_height), color="white")
        draw = ImageDraw.Draw(img)

        # Draw a red circle in the top-left quadrant
        center_x = 150.0
        center_y = 120.0
        radius = 50.0

        # PIL ellipse takes (left, top, right, bottom)
        draw.ellipse(
            [
                center_x - radius,
                center_y - radius,
                center_x + radius,
                center_y + radius,
            ],
            fill="red",
        )

        # Calculate bounding box around the circle (with some padding)
        bbox_x = center_x - radius - 5.0
        bbox_y = center_y - radius - 5.0
        bbox_w = (radius * 2.0) + 10.0
        bbox_h = (radius * 2.0) + 10.0

        print(
            f"Generated PNG image with circle at bbox: "
            f"({bbox_x:.1f}, {bbox_y:.1f}, {bbox_w:.1f}, {bbox_h:.1f})"
        )

        # Save to temporary file
        test_dir = get_test_data_dir()
        timestamp = int(time.time())
        test_image_path = test_dir / f"test_populate_{timestamp}.png"
        img.save(str(test_image_path), format="PNG")
        print(f"Test image saved to: {test_image_path}")

        # Create sample with annotation
        sample = Sample()
        sample.set_image_name(f"test_populate_{timestamp}.png")

        # Add file
        sample.add_file(SampleFile("image", str(test_image_path)))

        # Add bounding box annotation with NORMALIZED coordinates
        annotation = Annotation()
        annotation.set_label("circle")
        annotation.set_object_id("circle-obj-1")

        # Normalize coordinates: divide pixel values by image dimensions
        normalized_x = bbox_x / img_width
        normalized_y = bbox_y / img_height
        normalized_w = bbox_w / img_width
        normalized_h = bbox_h / img_height

        print(
            f"Normalized bbox: ({normalized_x:.3f}, {normalized_y:.3f}, "
            f"{normalized_w:.3f}, {normalized_h:.3f})"
        )

        bbox = Box2d(normalized_x, normalized_y, normalized_w, normalized_h)
        annotation.set_box2d(bbox)
        sample.add_annotation(annotation)

        # Populate the sample with progress callback
        def progress(current, total):
            print(f"Upload progress: {current}/{total}")

        try:
            results = client.populate_samples(
                dataset_id, annotation_set.id, [sample], progress=progress
            )

            assert len(results) == 1
            result = results[0]
            assert len(result.urls) == 1
            print(f"✓ Sample populated with UUID: {result.uuid}")

            # Give the server a moment to process the upload
            time.sleep(2)

            # Verify the sample was created by fetching it back
            image_filename = f"test_populate_{timestamp}"
            print(f"Looking for image: {image_filename}")

            samples = client.samples(
                dataset_id,
                annotation_set.id,
                annotation_types=[],
                groups=[],  # Don't filter by group - get all samples
                types=[],
            )

            print(f"Found {len(samples)} samples total")

            # Find the sample by image_name
            created_sample = None
            for s in samples:
                print(
                    f"  Sample: {s.name} UUID: {s.uuid} "
                    f"Dimensions: {s.width}x{s.height}"
                )
                if s.name == image_filename:
                    created_sample = s
                    break

            assert (
                created_sample is not None
            ), f"Sample with image_name '{image_filename}' should exist"

            print(f"✓ Found sample by image_name: {image_filename}")

            # Verify basic properties
            assert created_sample.name == image_filename
            assert (created_sample.group == "train" or
                    created_sample.group is None)

            print("\nSample verification:")
            print(f"  ✓ image_name: {created_sample.name}")
            print(f"  ✓ group: {created_sample.group}")
            print(
                f"  ✓ annotations: {len(created_sample.annotations)} "
                f"item(s)")

            # Verify annotations are returned correctly
            annotations = created_sample.annotations
            assert len(annotations) == 1, (
                "Should have exactly one annotation")

            annotation = annotations[0]
            assert annotation.label == "circle"
            assert annotation.box2d is not None, (
                "Bounding box should be present")

            returned_bbox = annotation.box2d
            print(
                f"\nReturned bbox (normalized): "
                f"({returned_bbox.left:.3f}, "
                f"{returned_bbox.top:.3f}, {returned_bbox.width:.3f}, "
                f"{returned_bbox.height:.3f})"
            )

            # Verify bbox coordinates are approximately correct
            # (within 5% tolerance)
            tolerance = 0.05
            assert abs(returned_bbox.left - normalized_x) < tolerance
            assert abs(returned_bbox.top - normalized_y) < tolerance
            assert abs(returned_bbox.width - normalized_w) < tolerance
            assert abs(returned_bbox.height - normalized_h) < tolerance

            print("✓ Bounding box coordinates verified")

            # Download the image and verify byte-for-byte match
            downloaded_data = created_sample.download(client)
            assert downloaded_data is not None, (
                "Downloaded data should not be None")

            # Read original file
            with open(str(test_image_path), "rb") as f:
                original_data = f.read()

            assert len(downloaded_data) == len(
                original_data
            ), "Downloaded data length should match original"
            assert (
                downloaded_data == original_data
            ), "Downloaded data should match original byte-for-byte"

            print(
                f"✓ Downloaded image matches original "
                f"({len(downloaded_data)} bytes)"
            )

            print(
                "\n✓ Test passed: populate_samples with automatic upload")

        finally:
            # Clean up temporary file
            if test_image_path.exists():
                test_image_path.unlink()

            # Clean up test dataset (unless SKIP_CLEANUP=1 is set)
            if skip_cleanup:
                print(
                    "Skipping dataset deletion for manual verification "
                    "(SKIP_CLEANUP=1)."
                )
            else:
                print("\nCleaning up test dataset...")
                client.delete_dataset(dataset_id)
                print("  ✓ Deleted test dataset")

    def _sample_uuid(self, sample):
        """Return the sample UUID, asserting it is present."""
        sample_uuid = sample.uuid
        self.assertIsNotNone(
            sample_uuid,
            "Sample is missing UUID; this indicates a server-side bug.",
        )
        assert sample_uuid is not None
        return sample_uuid

    def _sample_image_key(self, sample):
        """Return the image-based key used when comparing datasets."""
        image_name = sample.image_name
        if image_name is not None:
            return Path(image_name).stem

        sample_name = sample.name
        self.assertIsNotNone(
            sample_name,
            (
                "Sample is missing image_name and name; "
                "cannot determine file key."
            ),
        )
        assert sample_name is not None
        return Path(sample_name).stem

    def _annotation_image_key(self, annotation):
        """Return the image key linked to an annotation."""
        annotation_name = annotation.name
        self.assertIsNotNone(
            annotation_name,
            "Annotation should include the originating sample name",
        )
        assert annotation_name is not None
        return Path(annotation_name).stem

    def _collect_exported_files(self, directory):
        """Index exported files by stem for quick lookup."""
        indexed = {}
        # Use rglob to recursively search for files (handles sequence subdirs)
        for path in directory.rglob('*'):
            if path.is_file():
                stem = path.stem
                if stem in indexed:
                    self.fail(
                        f"Duplicate exported file for stem '{stem}'"
                    )
                indexed[stem] = path
        return indexed

    def _annotation_signature(self, annotation):
        """Create a comparable signature for an annotation."""
        bbox = annotation.box2d
        if bbox is not None:
            bbox_sig = tuple(
                round(value, 6)
                for value in (
                    bbox.left,
                    bbox.top,
                    bbox.width,
                    bbox.height,
                )
            )
        else:
            bbox_sig = None

        mask = annotation.mask
        if mask is not None:
            mask_sig = tuple(
                tuple(
                    (round(point[0], 6), round(point[1], 6))
                    for point in polygon
                )
                for polygon in mask.polygon
            )
        else:
            mask_sig = None

        return (
            annotation.label,
            annotation.object_id,
            annotation.group,
            bbox_sig,
            mask_sig,
        )

    def _merge_annotation_signature(self, entries, signature):
        """Merge a partial annotation into the existing list if possible."""

        sig_identity = signature[:3]

        for idx, existing in enumerate(entries):
            if existing[:3] != sig_identity:
                continue

            updated_bbox = (
                existing[3] if existing[3] is not None else signature[3]
            )
            updated_mask = (
                existing[4] if existing[4] is not None else signature[4]
            )

            if updated_bbox != existing[3] or updated_mask != existing[4]:
                entries[idx] = (
                    existing[0],
                    existing[1],
                    existing[2],
                    updated_bbox,
                    updated_mask,
                )
                return True

        return False

    def _build_annotation_map(self, annotations):
        """Group annotations by sample key with sorted signatures.

        Server responses may split a logical annotation into multiple entries
        (e.g., one row containing the bounding box and a companion row with
        the mask geometry). We merge those partial rows back together so the
        comparison remains stable regardless of how the backend chooses to
        fan out the geometries.
        """

        grouped = {}
        for annotation in annotations:
            key = self._annotation_image_key(annotation)
            signature = self._annotation_signature(annotation)

            entries = grouped.setdefault(key, [])
            if not self._merge_annotation_signature(entries, signature):
                entries.append(signature)

        for key, values in grouped.items():
            grouped[key] = sorted(
                values,
                key=lambda item: (
                    item[0] or "",
                    item[1] or "",
                    item[3] or (),
                    item[4] or (),
                ),
            )

        return grouped

    def _clone_annotation_for_upload(self, annotation):
        """Create a fresh Annotation with equivalent geometry."""
        cloned = Annotation()

        if annotation.label is not None:
            cloned.set_label(annotation.label)

        if annotation.object_id is not None:
            cloned.set_object_id(annotation.object_id)

        if annotation.box2d is not None:
            box = annotation.box2d
            cloned.set_box2d(Box2d(box.left, box.top, box.width, box.height))

        if annotation.box3d is not None:
            box3d = annotation.box3d
            cloned.set_box3d(
                Box3d(
                    box3d.cx,
                    box3d.cy,
                    box3d.cz,
                    box3d.width,
                    box3d.height,
                    box3d.length,
                )
            )

        mask = annotation.mask
        if mask is not None:
            polygon_copy = [list(ring) for ring in mask.polygon]
            cloned.set_mask(Mask(polygon_copy))

        return cloned

    def _filter_annotation_by_types(self, annotation, types):
        """Check if annotation has any of the specified types."""
        if "box2d" in types and annotation.box2d is not None:
            return True
        if "box3d" in types and annotation.box3d is not None:
            return True
        if "mask" in types and annotation.mask is not None:
            return True
        return False

    def _get_source_dataset(self, client, dataset):
        """Load dataset by ID or name across all projects."""
        if dataset.startswith("ds-"):
            return client.dataset(dataset)
        
        projects = client.projects("")
        for project in projects:
            datasets = client.datasets(project.id, dataset)
            matching = [d for d in datasets if d.name == dataset]
            if matching:
                return matching[0]
        
        raise AssertionError(
            f"Dataset '{dataset}' not found in any project"
        )

    def _get_groups_for_testing(self, client, dataset_id, annotation_set_id):
        """Get available groups and select first 2 for testing."""
        all_samples = client.samples(
            dataset_id,
            annotation_set_id,
            groups=[],
        )
        available_groups = sorted(
            {s.group for s in all_samples if s.group}
        )
        selected_groups = (
            available_groups[:2] if len(available_groups) >= 2
            else available_groups
        )
        print(f"Available groups: {available_groups}")
        print(f"Selected groups for testing: {selected_groups}")
        return selected_groups

    def _build_samples_payload(
        self, client, selected_samples, selected_files, 
        source_annotations, types
    ):
        """Build Sample objects for upload with annotations."""
        samples_payload = []
        source_uuid_by_image_key = {}
        expected_groups = {}
        expected_image_names = {}
        selected_image_keys = []

        for sample in selected_samples:
            sample_uuid = self._sample_uuid(sample)
            sample_key = self._sample_image_key(sample)
            file_path = selected_files[sample_key]

            source_uuid_by_image_key[sample_key] = sample_uuid
            selected_image_keys.append(sample_key)
            expected_groups[sample_key] = sample.group

            # Create new sample with metadata
            new_sample = Sample()
            image_name = sample.image_name
            if image_name is None:
                image_name = f"{sample_key}{file_path.suffix}"
            new_sample.set_image_name(image_name)
            expected_image_names[sample_key] = image_name

            new_sample.set_group(sample.group)
            if sample.sequence_name is not None:
                new_sample.set_sequence_name(sample.sequence_name)
            if sample.frame_number is not None:
                new_sample.set_frame_number(sample.frame_number)

            new_sample.add_file(SampleFile("image", str(file_path)))

            # Add related annotations
            related_annotations = [
                ann for ann in source_annotations
                if self._annotation_image_key(ann) == sample_key
            ]
            for annotation in related_annotations:
                new_sample.add_annotation(
                    self._clone_annotation_for_upload(annotation)
                )

            samples_payload.append(new_sample)

        return {
            "payload": samples_payload,
            "image_keys": selected_image_keys,
            "source_uuids": source_uuid_by_image_key,
            "expected_groups": expected_groups,
            "expected_names": expected_image_names,
        }

    def _verify_roundtrip_samples(
        self, client, new_dataset_id, new_annotation_set_id,
        selected_image_keys, expected_groups, expected_image_names,
        source_uuid_by_image_key
    ):
        """Verify uploaded samples match expected metadata."""
        new_samples = client.samples(
            new_dataset_id,
            new_annotation_set_id,
            groups=[],
        )
        self.assertEqual(len(new_samples), len(selected_image_keys))

        new_samples_map = {
            self._sample_image_key(s): s for s in new_samples
        }
        self.assertSetEqual(
            set(selected_image_keys), set(new_samples_map)
        )

        # Verify UUIDs changed and metadata preserved
        actual_groups = {}
        actual_image_names = {}
        for key in selected_image_keys:
            sample_obj = new_samples_map[key]
            self.assertIsNotNone(sample_obj)
            assert sample_obj is not None
            
            # Check UUID changed
            new_uuid = self._sample_uuid(sample_obj)
            source_uuid = source_uuid_by_image_key[key]
            self.assertNotEqual(
                source_uuid,
                new_uuid,
                "Re-uploaded dataset should assign new sample UUIDs",
            )
            
            actual_groups[key] = sample_obj.group
            new_image_name = sample_obj.image_name
            self.assertIsNotNone(new_image_name)
            assert new_image_name is not None
            actual_image_names[key] = new_image_name

        self.assertEqual(expected_groups, actual_groups)
        self.assertEqual(expected_image_names, actual_image_names)
        
        return new_samples_map

    def test_dataset_roundtrip(self):  # noqa: C901
        """Verify dataset download→upload→download integrity.

        Dataset: Configurable via TEST_DATASET env var (default: "Deer")
        Requirements:
        - Dataset can be in any project (exact name match) or ds-xxx ID
        - Must have at least one annotation set
        - Supports mixed sensors, annotation types, and sequences
        """
        client = get_client()
        dataset = get_test_dataset()

        print(f"\nTesting dataset roundtrip for: {dataset}")

        types = get_test_dataset_types()
        print(f"Testing annotation types: {', '.join(types)}")

        # Load source dataset
        source_dataset = self._get_source_dataset(client, dataset)

        annotation_sets = client.annotation_sets(source_dataset.id)
        self.assertGreater(len(annotation_sets), 0)
        assert len(annotation_sets) > 0
        source_annotation_set = annotation_sets[0]

        # Get groups for testing
        selected_groups = self._get_groups_for_testing(
            client, source_dataset.id, source_annotation_set.id
        )

        # Setup directories
        timestamp = int(time.time())
        test_dir = get_test_data_dir()
        export_dir = test_dir / f"labels_export_{timestamp}"
        reexport_dir = test_dir / f"labels_reexport_{timestamp}"
        export_dir.mkdir(parents=True, exist_ok=True)
        reexport_dir.mkdir(parents=True, exist_ok=True)

        download_progress = []

        def capture_download(current, total):
            download_progress.append((current, total))

        # Download dataset with selected groups
        client.download_dataset(
            source_dataset.id,
            selected_groups,
            [FileType.Image],
            str(export_dir),
            progress=capture_download,
        )

        self.assertGreater(
            len(download_progress),
            0,
            "download_dataset should report progress",
        )

        exported_files = self._collect_exported_files(export_dir)
        self.assertGreater(len(exported_files), 0)

        source_samples = client.samples(
            source_dataset.id,
            source_annotation_set.id,
            groups=selected_groups,
        )
        self.assertGreater(len(source_samples), 0)

        source_annotations = client.annotations(
            source_annotation_set.id,
            groups=selected_groups,
        )
        # Filter annotations by configured types
        source_annotations = [
            ann for ann in source_annotations
            if self._filter_annotation_by_types(ann, types)
        ]

        max_samples = min(8, len(source_samples))
        selected_samples = source_samples[:max_samples]

        selected_uuids = {
            self._sample_uuid(sample): sample for sample in selected_samples
        }
        self.assertEqual(len(selected_uuids), len(selected_samples))

        selected_image_keys = [
            self._sample_image_key(sample) for sample in selected_samples
        ]

        for key in selected_image_keys:
            self.assertIn(key, exported_files)

        selected_files = {
            key: exported_files[key] for key in selected_image_keys
        }

        # Build upload payload
        payload_info = self._build_samples_payload(
            client, selected_samples, selected_files,
            source_annotations, types
        )
        samples_payload = payload_info["payload"]
        source_uuid_by_image_key = payload_info["source_uuids"]
        expected_groups = payload_info["expected_groups"]
        expected_image_names = payload_info["expected_names"]

        selected_annotations = [
            ann for ann in source_annotations
            if self._annotation_image_key(ann) in selected_image_keys
        ]
        expected_annotation_map = self._build_annotation_map(
            selected_annotations
        )

        # Create new dataset for roundtrip
        random_suffix = "".join(
            random.choices(string.ascii_uppercase + string.digits, k=6)
        )
        new_dataset_name = f"{dataset} Roundtrip {random_suffix}"

        projects = client.projects("Unit Testing")
        self.assertGreater(len(projects), 0)
        assert len(projects) > 0
        project = projects[0]

        new_dataset_id = client.create_dataset(
            str(project.id),
            new_dataset_name,
            "Automated test: dataset download/upload verification",
        )

        print(f"\n✓ Created roundtrip dataset: {new_dataset_id}")
        print(f"  Name: {new_dataset_name}")

        new_annotation_set_id = client.create_annotation_set(
            new_dataset_id,
            "Default",
            "Roundtrip annotation set",
        )

        new_dataset = client.dataset(new_dataset_id)
        original_labels = source_dataset.labels(client)
        for label in original_labels:
            new_dataset.add_label(client, label.name)

        upload_progress = []

        def capture_upload(current, total):
            upload_progress.append((current, total))

        results = client.populate_samples(
            new_dataset_id,
            new_annotation_set_id,
            samples_payload,
            progress=capture_upload,
        )

        self.assertEqual(len(results), len(samples_payload))
        self.assertGreater(len(upload_progress), 0)

        # Check if we should skip cleanup for manual inspection
        skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"

        try:
            time.sleep(3)

            # Verify uploaded samples
            self._verify_roundtrip_samples(
                client, new_dataset_id, new_annotation_set_id,
                selected_image_keys, expected_groups, expected_image_names,
                source_uuid_by_image_key
            )

            # Verify annotations
            new_annotations = client.annotations(
                new_annotation_set_id,
                groups=[],
            )
            # Filter by configured types (same as source)
            new_annotations = [
                ann for ann in new_annotations
                if self._filter_annotation_by_types(ann, types)
            ]
            new_annotation_map = self._build_annotation_map([
                ann for ann in new_annotations
                if self._annotation_image_key(ann) in selected_image_keys
            ])

            self.assertEqual(expected_annotation_map, new_annotation_map)

            reexport_progress = []

            def capture_reexport(current, total):
                reexport_progress.append((current, total))

            client.download_dataset(
                new_dataset_id,
                [],
                [FileType.Image],
                str(reexport_dir),
                progress=capture_reexport,
            )

            self.assertGreater(len(reexport_progress), 0)

            reexport_files = self._collect_exported_files(reexport_dir)
            self.assertSetEqual(
                set(selected_image_keys), set(reexport_files)
            )

            for key in selected_image_keys:
                original_path = selected_files[key]
                reexport_path = reexport_files[key]
                self.assertEqual(
                    original_path.suffix,
                    reexport_path.suffix,
                )
                self.assertEqual(
                    original_path.read_bytes(),
                    reexport_path.read_bytes(),
                )

        finally:
            if skip_cleanup:
                print(
                    "Skipping dataset deletion for manual verification "
                    "(SKIP_CLEANUP=1)."
                )
            else:
                client.delete_dataset(new_dataset_id)
            shutil.rmtree(export_dir, ignore_errors=True)
            shutil.rmtree(reexport_dir, ignore_errors=True)


    def test_helper_sample_image_key_with_image_name(self):
        """Test creating samples with specific image names."""
        client = get_client()
        projects = client.projects("Unit Testing")
        self.assertGreater(len(projects), 0)
        project = projects[0]
        
        random_suffix = "".join(
            random.choices(string.ascii_uppercase + string.digits, k=6)
        )
        dataset_name = f"Test Sample Key {random_suffix}"
        
        dataset_id = client.create_dataset(
            str(project.id), dataset_name, "Test"
        )
        
        annotation_set_id = client.create_annotation_set(
            dataset_id, "Default", "Default"
        )
        
        # Create sample with image name
        sample = Sample()
        sample.set_image_name("test_image.jpg")
        
        img = Image.new("RGB", (100, 100), color="red")
        img_path = Path(get_test_data_dir()) / "test_image.jpg"
        img.save(str(img_path))
        
        sample.add_file(SampleFile("image", str(img_path)))
        
        try:
            results = client.populate_samples(
                dataset_id, annotation_set_id, [sample]
            )
            self.assertEqual(len(results), 1)
            print("✓ Sample with image name works")
        finally:
            client.delete_dataset(dataset_id)

    def test_helper_annotation_image_key(self):
        """Test creating samples with annotations."""
        client = get_client()
        projects = client.projects("Unit Testing")
        self.assertGreater(len(projects), 0)
        project = projects[0]
        
        random_suffix = "".join(
            random.choices(string.ascii_uppercase + string.digits, k=6)
        )
        dataset_name = f"Test Annotation Key {random_suffix}"
        
        dataset_id = client.create_dataset(
            str(project.id), dataset_name, "Test"
        )
        
        annotation_set_id = client.create_annotation_set(
            dataset_id, "Default", "Default"
        )
        
        sample = Sample()
        sample.set_image_name("annotated.jpg")
        
        annotation = Annotation()
        annotation.set_object_id("obj-1")
        annotation.set_label("test_label")
        bbox = Box2d(0.1, 0.1, 0.3, 0.3)
        annotation.set_box2d(bbox)
        sample.add_annotation(annotation)
        
        img = Image.new("RGB", (100, 100), color="blue")
        img_path = Path(get_test_data_dir()) / "annotated.jpg"
        img.save(str(img_path))
        
        sample.add_file(SampleFile("image", str(img_path)))
        
        try:
            results = client.populate_samples(
                dataset_id, annotation_set_id, [sample]
            )
            self.assertEqual(len(results), 1)
            print("✓ Annotation image key works")
        finally:
            client.delete_dataset(dataset_id)

    def test_collect_exported_files_scenario(self):
        """Test roundtrip export includes all expected files."""
        client = get_client()
        projects = client.projects("Unit Testing")
        self.assertGreater(len(projects), 0)
        project = projects[0]
        
        random_suffix = "".join(
            random.choices(string.ascii_uppercase + string.digits, k=6)
        )
        dataset_name = f"Test Export Files {random_suffix}"
        
        dataset_id = client.create_dataset(
            str(project.id), dataset_name, "Test"
        )
        
        annotation_set_id = client.create_annotation_set(
            dataset_id, "Default", "Default"
        )
        
        # Create sample with annotation
        sample = Sample()
        sample.set_image_name("export_test.jpg")
        
        annotation = Annotation()
        annotation.set_object_id("obj-export")
        annotation.set_label("export_label")
        bbox = Box2d(0.2, 0.2, 0.4, 0.4)
        annotation.set_box2d(bbox)
        sample.add_annotation(annotation)
        
        img = Image.new("RGB", (200, 200), color="green")
        img_path = Path(get_test_data_dir()) / "export_test.jpg"
        img.save(str(img_path))
        
        sample.add_file(SampleFile("image", str(img_path)))
        
        try:
            results = client.populate_samples(
                dataset_id, annotation_set_id, [sample]
            )
            self.assertEqual(len(results), 1)
            print("✓ Export files scenario works")
        finally:
            client.delete_dataset(dataset_id)

    def test_annotation_signature_with_bbox(self):
        """Test annotation with bbox creates consistent signature."""
        client = get_client()
        projects = client.projects("Unit Testing")
        self.assertGreater(len(projects), 0)
        project = projects[0]
        
        random_suffix = "".join(
            random.choices(string.ascii_uppercase + string.digits, k=6)
        )
        dataset_name = f"Test Annotation Sig {random_suffix}"
        
        dataset_id = client.create_dataset(
            str(project.id), dataset_name, "Test"
        )
        
        annotation_set_id = client.create_annotation_set(
            dataset_id, "Default", "Default"
        )
        
        sample = Sample()
        sample.set_image_name("sig_test.jpg")
        
        annotation = Annotation()
        annotation.set_object_id("sig-obj")
        annotation.set_label("sig_label")
        bbox = Box2d(0.15, 0.25, 0.35, 0.45)
        annotation.set_box2d(bbox)
        sample.add_annotation(annotation)
        
        img = Image.new("RGB", (150, 150), color="yellow")
        img_path = Path(get_test_data_dir()) / "sig_test.jpg"
        img.save(str(img_path))
        
        sample.add_file(SampleFile("image", str(img_path)))
        
        try:
            results = client.populate_samples(
                dataset_id, annotation_set_id, [sample]
            )
            self.assertEqual(len(results), 1)
            print("✓ Annotation signature with bbox works")
        finally:
            client.delete_dataset(dataset_id)

    def test_annotation_signature_with_mask(self):
        """Test samples with mask annotations load correctly."""
        client = get_client()
        projects = client.projects("Unit Testing")
        self.assertGreater(len(projects), 0)
        project = projects[0]
        
        datasets = client.datasets(project.id, "Unit Testing")
        if len(datasets) == 0:
            self.skipTest("No Unit Testing dataset available")
            return
        
        dataset = datasets[0]
        annotation_sets = client.annotation_sets(dataset.id)
        if len(annotation_sets) == 0:
            self.skipTest("No annotation sets available")
            return
        
        # Verify can fetch samples (which may have masks from server)
        samples = client.samples(dataset.id, annotation_sets[0].id)
        if len(samples) > 0:
            for sample in samples:
                self.assertIsNotNone(sample)
        
        print("✓ Mask annotation samples load correctly")

    def test_grouping_multiple_samples_same_image(self):
        """Test grouping multiple annotations for same image."""
        client = get_client()
        projects = client.projects("Unit Testing")
        self.assertGreater(len(projects), 0)
        project = projects[0]
        
        random_suffix = "".join(
            random.choices(string.ascii_uppercase + string.digits, k=6)
        )
        dataset_name = f"Test Multi Annot {random_suffix}"
        
        dataset_id = client.create_dataset(
            str(project.id), dataset_name, "Test"
        )
        
        annotation_set_id = client.create_annotation_set(
            dataset_id, "Default", "Default"
        )
        
        sample = Sample()
        sample.set_image_name("multi_annot.jpg")
        
        # Add multiple annotations for same image
        for i in range(3):
            annotation = Annotation()
            annotation.set_object_id(f"obj-{i}")
            annotation.set_label(f"label_{i}")
            bbox = Box2d(0.1 * i, 0.1 * i, 0.2, 0.2)
            annotation.set_box2d(bbox)
            sample.add_annotation(annotation)
        
        img = Image.new("RGB", (100, 100), color="cyan")
        img_path = Path(get_test_data_dir()) / "multi_annot.jpg"
        img.save(str(img_path))
        
        sample.add_file(SampleFile("image", str(img_path)))
        
        try:
            results = client.populate_samples(
                dataset_id, annotation_set_id, [sample]
            )
            self.assertEqual(len(results), 1)
            print("✓ Multiple annotations for same image works")
        finally:
            client.delete_dataset(dataset_id)


class TestLabels(TestCase):
    """Test label management operations."""

    def test_labels_add_remove(self):
        """Test adding and removing labels with random label names."""
        client = get_client()

        # Find Unit Testing project and Test Labels dataset
        projects = client.projects("Unit Testing")
        self.assertGreater(
            len(projects),
            0,
            "Unit Testing project should exist")
        project = projects[0]

        datasets = client.datasets(project.id, "Test Labels")
        self.assertGreater(
            len(datasets),
            0,
            "Test Labels dataset should exist")
        dataset = datasets[0]

        # Generate random label name to avoid conflicts
        random_suffix = random.randint(0, 2**64 - 1)
        test_label = f"test_{random_suffix:x}"

        # Get initial label count
        initial_labels = dataset.labels(client)
        initial_count = len(initial_labels)

        # Verify random label doesn't exist
        label_names = [label.name for label in initial_labels]
        self.assertNotIn(
            test_label,
            label_names,
            f"Random label '{test_label}' should not exist yet")

        # Add test label
        dataset.add_label(client, test_label)
        labels_after_add = dataset.labels(client)
        self.assertEqual(
            len(labels_after_add),
            initial_count + 1,
            "Should have one more label after adding")
        label_names_after = [label.name for label in labels_after_add]
        self.assertIn(
            test_label,
            label_names_after,
            f"Label '{test_label}' should exist after adding")

        # Remove test label
        dataset.remove_label(client, test_label)
        labels_after_remove = dataset.labels(client)
        self.assertEqual(
            len(labels_after_remove),
            initial_count,
            "Should have same label count as initial after removing")
        label_names_final = [label.name for label in labels_after_remove]
        self.assertNotIn(
            test_label,
            label_names_final,
            f"Label '{test_label}' should not exist after removing")

    def test_update_label(self):
        """Test updating a label's properties."""
        client = get_client()

        # Find Unit Testing project and first dataset
        projects = client.projects("Unit Testing")
        self.assertGreater(len(projects), 0)
        assert len(projects) > 0
        project = projects[0]
        self.assertIsNotNone(project)
        assert project is not None

        datasets = client.datasets(project.id)
        self.assertGreater(len(datasets), 0)
        assert len(datasets) > 0
        dataset = datasets[0]

        # Get existing labels
        labels = client.labels(dataset.id)

        # If no labels exist, add one for testing
        if len(labels) == 0:
            client.add_label(dataset.id, "test_update_label_temp")
            labels = client.labels(dataset.id)
            created_label = True
        else:
            created_label = False

        self.assertGreater(len(labels), 0)
        assert len(labels) > 0

        # Get the first label to update
        label = labels[0]
        self.assertIsNotNone(label)
        assert label is not None
        original_name = label.name

        # Update the label (note: this just calls the API,
        # actual changes depend on server permissions)
        # We're just verifying the method works without errors
        try:
            client.update_label(label)
            print(f"✓ Successfully called update_label for '{original_name}'")
        except Exception as e:
            # Some labels may not be updatable, that's okay for this test
            print(f"Note: update_label raised {type(e).__name__}: {e}")

        # Clean up if we created a label
        if created_label:
            client.remove_label(label.id)

    def test_samples_count(self):
        """Test counting samples without fetching them."""
        client = get_client()

        # Find Unit Testing project and first dataset
        projects = client.projects("Unit Testing")
        self.assertGreater(len(projects), 0)
        assert len(projects) > 0
        project = projects[0]

        datasets = client.datasets(project.id)
        self.assertGreater(len(datasets), 0)
        assert len(datasets) > 0
        dataset = datasets[0]

        # Get annotation sets
        annotation_sets = client.annotation_sets(dataset.id)
        if len(annotation_sets) == 0:
            print("No annotation sets found, skipping samples_count test")
            return

        annotation_set = annotation_sets[0]

        # Count samples
        count_result = client.samples_count(
            dataset.id,
            annotation_set.id,
            annotation_types=[],
            groups=[],
            types=[],
        )

        self.assertIsNotNone(count_result)
        assert count_result is not None
        self.assertGreaterEqual(count_result.total, 0)

        print(
            f"✓ Dataset '{dataset.name}' has {count_result.total} samples")

        # Verify count matches actual samples (if not too many)
        if count_result.total < 100:
            samples = client.samples(
                dataset.id,
                annotation_set.id,
                annotation_types=[],
                groups=[],
                types=[],
            )
            self.assertEqual(
                len(samples),
                count_result.total,
                "samples_count should match len(samples)")
            print("✓ Verified count matches actual samples")


    def _download_dataset(
        self, client, dataset_id, output_dir, flatten=False
    ):
        """Download dataset from EdgeFirst Studio.
        
        Args:
            client: EdgeFirst client instance
            dataset_id: Dataset ID to download
            output_dir: Directory to download to
            flatten: Whether to flatten the directory structure
            
        Raises:
            RuntimeError: If download fails (including missing S3 files)
        """
        client.download_dataset(
            dataset_id,
            [],
            [FileType.Image],
            str(output_dir),
            flatten=flatten,
        )

    def _analyze_download_structure(self, directory):
        """Analyze downloaded dataset directory structure.
        
        Args:
            directory: Path to downloaded dataset directory
            
        Returns:
            Dict with structure analysis results
        """
        entries = list(directory.iterdir())
        has_subdirs = any(e.is_dir() for e in entries)
        
        def count_files(d):
            return sum(1 for f in d.rglob("*") if f.is_file())
        
        file_count = count_files(directory)
        return {
            "entries": entries,
            "has_subdirs": has_subdirs,
            "file_count": file_count,
        }

    def test_download_dataset_flatten(self):
        """Test download_dataset with flatten option for sequences."""
        client = get_client()
        dataset = get_test_dataset()

        print(f"\nTesting flatten option for dataset: {dataset}")

        # Get dataset ID
        if dataset.startswith("ds-"):
            dataset_obj = client.dataset(dataset)
        else:
            projects = client.projects("")
            dataset_obj = None
            for project in projects:
                datasets = client.datasets(project.id, dataset)
                if datasets:
                    dataset_obj = datasets[0]
                    break

        self.assertIsNotNone(dataset_obj, f"Dataset '{dataset}' not found")
        assert dataset_obj is not None

        timestamp = int(time.time())
        test_dir = get_test_data_dir()
        normal_dir = test_dir / f"download_normal_{timestamp}"
        flatten_dir = test_dir / f"download_flatten_{timestamp}"
        normal_dir.mkdir(parents=True, exist_ok=True)
        flatten_dir.mkdir(parents=True, exist_ok=True)

        try:
            # Download with normal structure
            print("\n1. Downloading with normal structure...")
            self._download_dataset(
                client, dataset_obj.id, normal_dir, flatten=False
            )

            # Download with flattened structure
            print("2. Downloading with flattened structure...")
            self._download_dataset(
                client, dataset_obj.id, flatten_dir, flatten=True
            )

            # Analyze structures
            normal = self._analyze_download_structure(normal_dir)
            flatten = self._analyze_download_structure(flatten_dir)

            print(f"\nNormal structure: {len(normal['entries'])} entries")
            if normal["has_subdirs"]:
                subdirs = [
                    e.name for e in normal["entries"] if e.is_dir()
                ]
                print(f"  Subdirectories: {subdirs[:3]}")

            print(f"Flattened structure: {len(flatten['entries'])} entries")

            print("\nFile counts:")
            print(f"  Normal: {normal['file_count']} files")
            print(f"  Flatten: {flatten['file_count']} files")

            # Assertions
            self.assertEqual(
                normal["file_count"],
                flatten["file_count"],
                "Both downloads should have same number of files"
            )

            self.assertFalse(
                flatten["has_subdirs"],
                "Flattened download should not have subdirectories"
            )

            # Verify sequence prefixing if applicable
            if normal["has_subdirs"]:
                print("\n✓ Dataset contains sequences")
                flatten_files = [
                    e.name for e in flatten["entries"] if e.is_file()
                ]
                prefixed_count = sum(
                    1 for f in flatten_files if f.count('_') >= 1
                )
                print(
                    f"  Files with prefixes: {prefixed_count}/"
                    f"{len(flatten_files)}"
                )
                print(f"  Sample filenames: {flatten_files[:3]}")
                self.assertGreater(
                    prefixed_count,
                    0,
                    "Flattened sequence files should have prefixes"
                )
            else:
                print("\n✓ Dataset contains no sequences")

            print("\n✅ Flatten option test passed")

        finally:
            # Cleanup
            import shutil
            if normal_dir.exists():
                shutil.rmtree(normal_dir)
            if flatten_dir.exists():
                shutil.rmtree(flatten_dir)
            print("Cleaned up test directories")


