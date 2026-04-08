# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""
Tests for dataset versioning operations.

These integration tests verify:
- Version tag lifecycle (create, list, get, delete)
- Tagged data fetching (samples, annotations, labels at a specific version)
- Changelog tracking, filtering, and counting
- Version info and summary endpoints
- Tag restore functionality
- Error handling (duplicate tags, invalid names, nonexistent tags)
"""

import os
import random
import string
import tempfile
import time
import unittest
from unittest import TestCase

from test import get_client, get_test_data_dir
from test.fixtures import (
    create_sample_with_circle_annotation,
    create_test_image_with_circle,
)


def _server_supports_versioning(client):
    """Check if the connected server supports versioning APIs.

    Returns False if the server doesn't have the versioning endpoints
    (e.g., running against 'test' server instead of 'dev').
    """
    try:
        # Try listing tags on a nonexistent dataset — if the endpoint
        # exists, we get an error about dataset_id, not a method-not-found
        client.version_tag_list("ds-1")
        return True
    except Exception as e:
        msg = str(e).lower()
        if "method not found" in msg or "unknown method" in msg:
            return False
        # Any other error means the endpoint exists
        return True


def _create_test_dataset(client):
    """Create a temporary test dataset for versioning tests.

    Returns:
        Tuple of (dataset_id, annotation_set_id, project_id).
    """
    projects = client.projects("Unit Testing")
    assert len(projects) > 0
    project = projects[0]

    random_suffix = "".join(random.choices(string.ascii_uppercase + string.digits, k=6))
    test_dataset_name = f"Test Versioning {random_suffix}"

    dataset_id = client.create_dataset(
        str(project.id),
        test_dataset_name,
        "Automated test: versioning verification",
    )

    annotation_set_id = client.create_annotation_set(
        dataset_id, "Default", "Default annotation set"
    )

    return dataset_id, annotation_set_id, str(project.id)


def _populate_samples(client, dataset_id, annotation_set_id, count=3):
    """Populate a dataset with test samples and annotations.

    Args:
        client: Authenticated client.
        dataset_id: Target dataset.
        annotation_set_id: Annotation set for annotations.
        count: Number of samples to create.

    Returns:
        List of sample image names.
    """
    test_data_dir = get_test_data_dir()
    image_names = []

    samples = []
    for i in range(count):
        timestamp = int(time.time() * 1000)
        image_name = f"version_test_{timestamp}_{i}.png"
        image_path = test_data_dir / image_name
        image_names.append(image_name)

        create_test_image_with_circle(
            image_path,
            center_x=100.0 + i * 50,
            center_y=100.0 + i * 30,
        )

        sample = create_sample_with_circle_annotation(image_path, label_name="circle")
        samples.append(sample)

    client.populate_samples(
        dataset_id,
        annotation_set_id,
        samples,
    )

    return image_names


class VersionTagLifecycleTest(TestCase):
    """Test version tag create, list, get, delete operations."""

    def setUp(self):
        self.client = get_client()
        if not _server_supports_versioning(self.client):
            self.skipTest("Server does not support versioning APIs")

    def test_tag_lifecycle(self):
        """Create dataset, add samples, create tag, list/get/delete."""
        client = self.client
        skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"
        dataset_id, annotation_set_id, project_id = _create_test_dataset(client)

        try:
            # Populate some samples so changelog has entries
            _populate_samples(client, dataset_id, annotation_set_id, count=2)

            # Create a tag
            tag = client.version_tag_create(dataset_id, "v1.0", "First version")
            self.assertEqual(tag.name, "v1.0")
            self.assertEqual(tag.description, "First version")
            self.assertGreater(tag.serial, 0)
            self.assertGreater(tag.image_count, 0)
            print(
                f"Created tag: {tag.name} at serial {tag.serial} "
                f"with {tag.image_count} images"
            )

            # List tags
            tags = client.version_tag_list(dataset_id)
            self.assertGreater(len(tags), 0)
            tag_names = [t.name for t in tags]
            self.assertIn("v1.0", tag_names)
            print(f"Listed {len(tags)} tag(s)")

            # Get specific tag
            fetched_tag = client.version_tag_get(dataset_id, "v1.0")
            self.assertEqual(fetched_tag.name, "v1.0")
            self.assertEqual(fetched_tag.serial, tag.serial)
            self.assertEqual(fetched_tag.image_count, tag.image_count)
            print(f"Fetched tag: {fetched_tag.name}")

            # Create a second tag
            tag2 = client.version_tag_create(dataset_id, "v1.1", "Second version")
            self.assertEqual(tag2.name, "v1.1")

            # List should now have 2 tags
            tags = client.version_tag_list(dataset_id)
            self.assertEqual(len(tags), 2)

            # Delete the second tag
            result = client.version_tag_delete(dataset_id, "v1.1")
            assert result is not None
            print(f"Deleted tag: {result}")

            # List should now have 1 tag
            tags = client.version_tag_list(dataset_id)
            self.assertEqual(len(tags), 1)
            self.assertEqual(tags[0].name, "v1.0")

        finally:
            if not skip_cleanup:
                client.delete_dataset(dataset_id)

    def test_duplicate_tag_creation(self):
        """Creating a tag with an existing name should raise an error."""
        client = get_client()
        skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"
        dataset_id, annotation_set_id, _ = _create_test_dataset(client)

        try:
            _populate_samples(client, dataset_id, annotation_set_id, count=1)
            client.version_tag_create(dataset_id, "dup-test")

            with self.assertRaises(Exception) as ctx:
                client.version_tag_create(dataset_id, "dup-test")
            self.assertIn("already exists", str(ctx.exception))
            print(f"Duplicate tag error: {ctx.exception}")

        finally:
            if not skip_cleanup:
                client.delete_dataset(dataset_id)

    def test_invalid_tag_name(self):
        """Tag name with invalid characters should raise an error."""
        client = get_client()
        skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"
        dataset_id, annotation_set_id, _ = _create_test_dataset(client)

        try:
            _populate_samples(client, dataset_id, annotation_set_id, count=1)

            with self.assertRaises(Exception) as ctx:
                client.version_tag_create(dataset_id, "invalid name with spaces")
            self.assertIn("alphanumeric", str(ctx.exception).lower())
            print(f"Invalid name error: {ctx.exception}")

        finally:
            if not skip_cleanup:
                client.delete_dataset(dataset_id)

    def test_nonexistent_tag_get(self):
        """Getting a tag that does not exist should raise an error."""
        client = get_client()
        skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"
        dataset_id, annotation_set_id, _ = _create_test_dataset(client)

        try:
            with self.assertRaises(Exception) as ctx:
                client.version_tag_get(dataset_id, "nonexistent-tag")
            self.assertIn("not found", str(ctx.exception).lower())

        finally:
            if not skip_cleanup:
                client.delete_dataset(dataset_id)


class VersionTaggedDataFetchTest(TestCase):
    """Test fetching data at a specific tagged version."""

    def setUp(self):
        if not _server_supports_versioning(get_client()):
            self.skipTest("Server does not support versioning APIs")

    def test_tagged_vs_head_data(self):
        """Create tag, modify data, verify tagged fetch returns old state."""
        client = get_client()
        skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"
        dataset_id, annotation_set_id, project_id = _create_test_dataset(client)

        try:
            # Phase 1: Populate initial samples
            initial_names = _populate_samples(
                client, dataset_id, annotation_set_id, count=2
            )
            print(f"Populated {len(initial_names)} initial samples")

            # Get initial counts
            initial_count = client.samples_count(dataset_id).total
            print(f"Initial sample count: {initial_count}")

            # Create tag at this state
            tag = client.version_tag_create(
                dataset_id,
                "before-update",
                "Before adding more data",
            )
            print(f"Created tag '{tag.name}' with {tag.image_count} images")
            self.assertEqual(tag.image_count, len(initial_names))

            # Phase 2: Add more samples (modifying HEAD)
            time.sleep(1)  # Ensure unique timestamps
            additional_names = _populate_samples(
                client, dataset_id, annotation_set_id, count=3
            )
            print(f"Added {len(additional_names)} more samples")

            # HEAD should now have more samples
            head_count = client.samples_count(dataset_id).total
            self.assertEqual(
                head_count,
                len(initial_names) + len(additional_names),
            )
            print(f"HEAD sample count: {head_count}")

            # Tagged version should still have original count
            tagged_count = client.samples_count(
                dataset_id, version="before-update"
            ).total
            self.assertEqual(tagged_count, len(initial_names))
            print(f"Tagged sample count: {tagged_count}")

            # Fetch tagged samples
            tagged_samples = client.samples(dataset_id, version="before-update")
            self.assertEqual(len(tagged_samples), len(initial_names))

            # Fetch HEAD samples
            head_samples = client.samples(dataset_id)
            self.assertEqual(
                len(head_samples),
                len(initial_names) + len(additional_names),
            )

            # Verify tagged labels match HEAD labels
            tagged_labels = client.labels(dataset_id, version="before-update")
            head_labels = client.labels(dataset_id)
            self.assertEqual(len(tagged_labels), len(head_labels))

            # Verify tagged annotation sets
            tagged_annsets = client.annotation_sets(dataset_id, version="before-update")
            self.assertGreater(len(tagged_annsets), 0)

            # Verify tagged annotations
            annset_id = tagged_annsets[0].id
            tagged_annotations = client.annotations(annset_id, version="before-update")
            self.assertGreater(len(tagged_annotations), 0)
            # Tagged annotations should match initial count
            head_annotations = client.annotations(annset_id)
            self.assertGreater(
                len(head_annotations),
                len(tagged_annotations),
                "HEAD should have more annotations than tagged",
            )

            print("Tagged vs HEAD data verification passed")

        finally:
            if not skip_cleanup:
                client.delete_dataset(dataset_id)

    def test_download_dataset_with_tag(self):
        """Download dataset at a tagged version."""
        client = get_client()
        skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"
        dataset_id, annotation_set_id, _ = _create_test_dataset(client)

        try:
            # Populate and tag
            _populate_samples(client, dataset_id, annotation_set_id, count=2)
            client.version_tag_create(dataset_id, "dl-test")

            # Add more data to HEAD
            time.sleep(1)
            _populate_samples(client, dataset_id, annotation_set_id, count=2)

            # Download tagged version
            with tempfile.TemporaryDirectory() as tmpdir:
                client.download_dataset(
                    dataset_id,
                    output=tmpdir,
                    version="dl-test",
                )
                # Count downloaded files
                import pathlib

                files = list(pathlib.Path(tmpdir).rglob("*.*"))
                self.assertEqual(
                    len(files),
                    2,
                    "Tagged download should have 2 images",
                )
                print(f"Downloaded {len(files)} files at tag")

        finally:
            if not skip_cleanup:
                client.delete_dataset(dataset_id)


class VersionChangelogTest(TestCase):
    """Test changelog tracking, filtering, and counting."""

    def setUp(self):
        if not _server_supports_versioning(get_client()):
            self.skipTest("Server does not support versioning APIs")

    def test_changelog_entries(self):
        """Verify changelog entries are recorded for operations."""
        client = get_client()
        skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"
        dataset_id, annotation_set_id, _ = _create_test_dataset(client)

        try:
            # Populate samples (generates changelog entries)
            _populate_samples(client, dataset_id, annotation_set_id, count=2)

            # Get current version info
            current = client.version_current(dataset_id)
            self.assertGreater(current.current_serial, 0)
            print(f"Current serial: {current.current_serial}")

            # Get changelog
            changelog = client.version_changelog(dataset_id)
            self.assertGreater(len(changelog.entries), 0)
            self.assertGreater(changelog.count, 0)
            print(f"Changelog has {changelog.count} entries")

            # Verify entry structure
            entry = changelog.entries[0]
            self.assertIsNotNone(entry.serial)
            assert entry.serial is not None
            self.assertIsNotNone(entry.entity_type)
            assert entry.entity_type is not None
            self.assertIsNotNone(entry.operation)
            assert entry.operation is not None
            self.assertIsNotNone(entry.username)
            assert entry.username is not None
            print(
                f"First entry: serial={entry.serial} "
                f"{entry.operation} {entry.entity_type} "
                f"by {entry.username}"
            )

            # Filter changelog by entity type
            filtered = client.version_changelog(dataset_id, entity_types=["image"])
            for e in filtered.entries:
                self.assertEqual(e.entity_type, "image")
            print(f"Filtered to {len(filtered.entries)} image entries")

        finally:
            if not skip_cleanup:
                client.delete_dataset(dataset_id)

    def test_changelog_count(self):
        """Verify version_changelog_count returns correct count."""
        client = get_client()
        skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"
        dataset_id, annotation_set_id, _ = _create_test_dataset(client)

        try:
            _populate_samples(client, dataset_id, annotation_set_id, count=2)

            # Count all entries
            count = client.version_changelog_count(dataset_id)
            self.assertGreater(count, 0)
            print(f"Total changelog entries: {count}")

            # Count should match full changelog response
            changelog = client.version_changelog(dataset_id)
            self.assertEqual(count, changelog.count)

            # Filtered count
            image_count = client.version_changelog_count(
                dataset_id, entity_types=["image"]
            )
            self.assertGreater(image_count, 0)
            self.assertLessEqual(image_count, count)
            print(f"Image changelog entries: {image_count}")

        finally:
            if not skip_cleanup:
                client.delete_dataset(dataset_id)

    def test_changelog_version_range(self):
        """Test changelog filtering by version range."""
        client = get_client()
        skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"
        dataset_id, annotation_set_id, _ = _create_test_dataset(client)

        try:
            _populate_samples(client, dataset_id, annotation_set_id, count=2)
            tag1 = client.version_tag_create(dataset_id, "range-v1")

            time.sleep(1)
            _populate_samples(client, dataset_id, annotation_set_id, count=2)
            tag2 = client.version_tag_create(dataset_id, "range-v2")

            # Query between tags by name
            between = client.version_changelog(
                dataset_id,
                from_version="range-v1",
                to_version="range-v2",
            )
            self.assertGreater(len(between.entries), 0)
            # All entries should be within serial range
            for e in between.entries:
                self.assertGreaterEqual(e.serial, tag1.serial)
                self.assertLessEqual(e.serial, tag2.serial)
            print(f"Range {tag1.serial}-{tag2.serial}: {len(between.entries)} entries")

            # Query by serial number string
            by_serial = client.version_changelog(
                dataset_id,
                from_version=str(tag1.serial),
                to_version=str(tag2.serial),
            )
            self.assertEqual(
                len(by_serial.entries),
                len(between.entries),
            )

        finally:
            if not skip_cleanup:
                client.delete_dataset(dataset_id)

    def test_version_summary(self):
        """Test version_summary and version_summary_recalculate."""
        client = get_client()
        skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"
        dataset_id, annotation_set_id, _ = _create_test_dataset(client)

        try:
            _populate_samples(client, dataset_id, annotation_set_id, count=2)

            # Get summary
            summary = client.version_summary(dataset_id)
            self.assertGreater(summary.image_count, 0)
            self.assertGreater(summary.current_serial, 0)
            print(
                f"Summary: {summary.image_count} images, {summary.label_count} labels"
            )

            # Recalculate should return consistent data
            recalculated = client.version_summary_recalculate(dataset_id)
            self.assertEqual(recalculated.image_count, summary.image_count)
            print("Summary recalculate verified")

        finally:
            if not skip_cleanup:
                client.delete_dataset(dataset_id)

    def test_version_current_no_tags(self):
        """version_current with no tags should have latest_tag=None."""
        client = get_client()
        skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"
        dataset_id, annotation_set_id, _ = _create_test_dataset(client)

        try:
            _populate_samples(client, dataset_id, annotation_set_id, count=1)

            current = client.version_current(dataset_id)
            self.assertIsNone(current.latest_tag)
            self.assertEqual(len(current.tags), 0)
            self.assertGreater(current.current_serial, 0)
            print(
                f"No tags: serial={current.current_serial}, "
                f"latest_tag={current.latest_tag}"
            )

            # After creating a tag, latest_tag should be set
            client.version_tag_create(dataset_id, "first-tag")
            current = client.version_current(dataset_id)
            self.assertIsNotNone(current.latest_tag)
            assert current.latest_tag is not None
            self.assertEqual(current.latest_tag.name, "first-tag")
            self.assertEqual(len(current.tags), 1)
            print(f"With tag: latest={current.latest_tag.name}")

        finally:
            if not skip_cleanup:
                client.delete_dataset(dataset_id)


class VersionTagRestoreTest(TestCase):
    """Test tag restore functionality."""

    def setUp(self):
        if not _server_supports_versioning(get_client()):
            self.skipTest("Server does not support versioning APIs")

    def test_restore_to_tag(self):
        """Create tag, modify, restore, verify state matches tag."""
        client = get_client()
        skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"
        dataset_id, annotation_set_id, _ = _create_test_dataset(client)

        try:
            # Populate initial data
            _populate_samples(client, dataset_id, annotation_set_id, count=2)

            initial_count = client.samples_count(dataset_id).total

            # Create tag
            tag = client.version_tag_create(dataset_id, "restore-point")
            print(f"Created tag at serial {tag.serial} with {tag.image_count} images")

            # Modify dataset (add more samples)
            time.sleep(1)
            _populate_samples(client, dataset_id, annotation_set_id, count=3)
            modified_count = client.samples_count(dataset_id).total
            self.assertGreater(modified_count, initial_count)
            print(f"After modification: {modified_count} samples (was {initial_count})")

            # Restore to tag
            result = client.version_tag_restore(dataset_id, "restore-point")
            self.assertTrue(result.success)
            self.assertGreater(result.new_serial, 0)
            print(f"Restore result: {result.message}")

            # Verify counts match original
            restored_count = client.samples_count(dataset_id).total
            self.assertEqual(restored_count, initial_count)
            print(
                f"After restore: {restored_count} samples "
                f"(matches original {initial_count})"
            )

            # Verify labels and annotation sets restored
            labels = client.labels(dataset_id)
            annsets = client.annotation_sets(dataset_id)
            self.assertGreater(len(labels), 0)
            self.assertGreater(len(annsets), 0)
            print(f"Restored: {len(labels)} labels, {len(annsets)} annotation sets")

        finally:
            if not skip_cleanup:
                client.delete_dataset(dataset_id)


if __name__ == "__main__":
    unittest.main()
