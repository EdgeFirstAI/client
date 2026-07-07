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
    create_sample_without_annotation,
    create_test_image_with_circle,
    wait_for_label,
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

        _, bbox = create_test_image_with_circle(
            image_path,
            center_x=100.0 + i * 50,
            center_y=100.0 + i * 30,
        )

        # Attach the real bounding box: without box2d/box3d/mask geometry,
        # the server's samples.populate2 handler silently drops the
        # annotation (and never creates/references its label) instead of
        # creating a usable one. See create_sample_with_circle_annotation's
        # docstring for detail.
        sample = create_sample_with_circle_annotation(
            image_path, label_name="circle", box2d=bbox
        )
        samples.append(sample)

    client.populate_samples(
        dataset_id,
        annotation_set_id,
        samples,
    )

    return image_names


def _wait_until_sample_count(
    client, dataset_id, expected_total, timeout=30.0, interval=0.5
):
    """Poll samples_count() until it reaches expected_total.

    image.delete_from_dataset is fire-and-forget on the server (the RPC returns before
    the delete actually completes), so tests must poll for the deletion's effect instead
    of asserting immediately after the call returns. The server now also issues a real
    (synchronous) S3 delete on this path before the count drops, so the timeout has some
    margin over a bare DB round trip.
    """
    deadline = time.time() + timeout
    last_count = None
    while time.time() < deadline:
        result = client.samples_count(dataset_id)
        last_count = result.total
        if last_count == expected_total:
            return result
        time.sleep(interval)
    raise TimeoutError(
        f"samples_count for dataset {dataset_id} did not reach {expected_total} "
        f"within {timeout}s (last observed: {last_count})"
    )


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

    def test_tagged_labels_and_annotation_sets_nonempty(self):
        """Regression test: tag-scoped labels()/annotation_sets() must not
        crash when the tag snapshot actually contains data.

        This is the exact scenario that escaped detection before this fix:
        prior test fixtures never attached box2d geometry to their
        annotations, so the server silently dropped those annotations and
        never created a label at all (not a timing issue — see
        test_annotation_triggered_label_creation_completes below). The tag
        snapshot's label list was therefore always empty, and the
        (now-fixed) deserialization crash never triggered.
        """
        client = get_client()
        skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"
        dataset_id, annotation_set_id, _ = _create_test_dataset(client)

        try:
            # Explicit add_label (not annotation-triggered) guarantees a
            # non-empty label list at tag time, deterministically.
            client.add_label(dataset_id, "circle")
            _populate_samples(client, dataset_id, annotation_set_id, count=2)

            tag = client.version_tag_create(dataset_id, "with-labels")
            self.assertGreater(tag.label_count, 0)

            tagged_labels = client.labels(dataset_id, version="with-labels")
            self.assertGreater(len(tagged_labels), 0)
            self.assertEqual(tagged_labels[0].name, "circle")
            # dataset_id is backfilled by the client from the query context
            # for tag-scoped label.list reads (see Label::backfill_dataset_id
            # in dataset.rs) — it is populated by design, not left None, so
            # callers always get a usable dataset_id regardless of whether
            # the read was tag-scoped or HEAD-scoped.
            self.assertEqual(str(tagged_labels[0].dataset_id), dataset_id)

            tagged_annsets = client.annotation_sets(dataset_id, version="with-labels")
            self.assertGreater(len(tagged_annsets), 0)
            self.assertEqual(tagged_annsets[0].name, "Default")
            self.assertIsNone(tagged_annsets[0].created)

            print(
                f"Tagged fetch with {len(tagged_labels)} label(s), "
                f"{len(tagged_annsets)} annotation set(s) — no crash"
            )
        finally:
            if not skip_cleanup:
                client.delete_dataset(dataset_id)

    def test_annotation_triggered_label_creation_completes(self):
        """Verifies that a label referenced only through an annotation (no
        explicit add_label() call) is created by populate_samples(), using
        polling rather than an immediate check.

        Investigating this test live (see TESTING.md) found that the
        server actually resolves/creates the label row synchronously,
        inside the same samples.populate2 request that inserts the
        annotation — *provided* the annotation carries real geometry
        (box2d/box3d/mask). Annotations with only a label name and no
        geometry are silently dropped server-side and never create or
        reference a label at all, which is what previously made tag
        snapshots taken right after populate_samples() come back with zero
        labels (see create_sample_with_circle_annotation's box2d
        parameter). This test still polls via wait_for_label rather than
        asserting immediacy, since the API makes no documented guarantee
        of synchronous visibility and a fixed assumption would be fragile.
        """
        client = get_client()
        skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"
        dataset_id, annotation_set_id, _ = _create_test_dataset(client)

        try:
            _populate_samples(client, dataset_id, annotation_set_id, count=1)
            label = wait_for_label(client, dataset_id, "circle", timeout=5.0)
            self.assertEqual(label.name, "circle")
            print(
                f"Label '{label.name}' appeared after populate_samples (synchronous creation confirmed)"
            )
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

    def test_changelog_records_edits_after_tag(self):
        """Every edit after a tag is created must still be recorded in the
        changelog, distinct from the tag-creation entry itself."""
        client = get_client()
        skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"
        dataset_id, annotation_set_id, _ = _create_test_dataset(client)

        try:
            _populate_samples(client, dataset_id, annotation_set_id, count=1)
            serial_before_tag = client.version_current(dataset_id).current_serial

            client.version_tag_create(dataset_id, "checkpoint")
            serial_after_tag = client.version_current(dataset_id).current_serial
            self.assertGreater(serial_after_tag, serial_before_tag)

            time.sleep(1)
            _populate_samples(client, dataset_id, annotation_set_id, count=1)
            serial_after_edit = client.version_current(dataset_id).current_serial
            self.assertGreater(serial_after_edit, serial_after_tag)

            entries = client.version_changelog(
                dataset_id, from_version=str(serial_after_tag)
            )
            self.assertGreater(len(entries.entries), 0)
            # from_version is inclusive (server: GetDatasetChangelog filters
            # "serial >= fromSerial"), matching the >= convention already
            # exercised by test_changelog_version_range. The tag-creation
            # entry itself lands at serial_after_tag, so it is legitimately
            # included alongside the post-tag edit entries.
            self.assertTrue(all(e.serial >= serial_after_tag for e in entries.entries))
            print(
                f"serial before tag={serial_before_tag}, after tag={serial_after_tag}, "
                f"after edit={serial_after_edit}, entries since tag={len(entries.entries)}"
            )
        finally:
            if not skip_cleanup:
                client.delete_dataset(dataset_id)


class VersionTagRestoreTest(TestCase):
    """Test tag restore functionality."""

    def setUp(self):
        if not _server_supports_versioning(get_client()):
            self.skipTest("Server does not support versioning APIs")

    def test_restore_to_tag(self):
        """Create tag, modify, restore, verify state matches tag.

        Previously tracked as DE-2790 (server-side, dve-database):
        restore_tag_to_head() reverts image state by toggling the
        images.tagged boolean column, but database.ListSamplesCount/
        ListSamples (the HEAD-path samples queries) did not filter on
        images.tagged, so samples_count() after a restore still reflected
        every image ever added to the dataset, not the tag's snapshot
        count. This has been fixed and deployed on the test server —
        verified below by asserting the correct reverted count.
        """
        client = get_client()
        skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"
        dataset_id, annotation_set_id, _ = _create_test_dataset(client)

        try:
            client.add_label(dataset_id, "circle")
            _populate_samples(client, dataset_id, annotation_set_id, count=2)
            initial_count = client.samples_count(dataset_id).total

            tag = client.version_tag_create(dataset_id, "restore-point")
            print(f"Created tag at serial {tag.serial} with {tag.image_count} images")

            time.sleep(1)
            _populate_samples(client, dataset_id, annotation_set_id, count=3)
            modified_count = client.samples_count(dataset_id).total
            self.assertGreater(modified_count, initial_count)
            print(f"After modification: {modified_count} samples (was {initial_count})")

            result = client.version_tag_restore(dataset_id, "restore-point")
            self.assertTrue(result.success)
            self.assertGreater(result.new_serial, 0)
            print(f"Restore result: {result.message}")

            # DE-2790 fixed: image count is now correctly reverted by restore.
            restored_count = client.samples_count(dataset_id).total
            self.assertEqual(
                restored_count,
                initial_count,
                "Restore should revert the sample count to the tag's snapshot.",
            )
            print(
                f"After restore: {restored_count} samples "
                f"(matches original {initial_count})"
            )

            # Labels and annotation sets are also correctly reverted by restore
            # (restore_tag_to_head deletes and re-inserts these tables).
            labels = client.labels(dataset_id)
            annsets = client.annotation_sets(dataset_id)
            self.assertGreater(len(labels), 0)
            self.assertGreater(len(annsets), 0)
            print(f"Restored: {len(labels)} labels, {len(annsets)} annotation sets")

        finally:
            if not skip_cleanup:
                client.delete_dataset(dataset_id)


class VersionEditAfterTagTest(TestCase):
    """Test editing annotations/labels after tagging, and fetching each
    historical tag back correctly (not just the most recent one)."""

    def setUp(self):
        if not _server_supports_versioning(get_client()):
            self.skipTest("Server does not support versioning APIs")

    def test_edit_annotation_after_tag_does_not_change_tagged_view(self):
        """A true in-place annotation edit made after tagging must not
        retroactively change what the tag returns — tags are immutable
        snapshots. Uses add_annotations_bulk/delete_annotations_bulk for a
        real edit, now that both are exposed to Python."""
        from edgefirst_client import AnnotationSetID, ServerAnnotation

        client = get_client()
        skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"
        dataset_id, annotation_set_id, _ = _create_test_dataset(client)

        try:
            client.add_label(dataset_id, "circle")
            client.add_label(dataset_id, "square")
            _populate_samples(client, dataset_id, annotation_set_id, count=1)

            client.version_tag_create(dataset_id, "pre-edit")

            tagged_annotations_before = client.annotations(
                annotation_set_id, version="pre-edit"
            )
            self.assertEqual(len(tagged_annotations_before), 1)
            self.assertEqual(tagged_annotations_before[0].label, "circle")

            head_annotations_before = client.annotations(annotation_set_id)
            sample_id = head_annotations_before[0].sample_id
            original_box = head_annotations_before[0].box2d

            # True in-place edit: delete the "circle" box annotation for this
            # sample, then add a "square" one back in its place.
            time.sleep(1)
            client.delete_annotations_bulk(annotation_set_id, ["box"], [sample_id])

            # The server's annotation.add_bulk RPC resolves the label from
            # label_id (label_index/label_name alone are not honored, per
            # live testing and matching the CLI's import-coco --update path
            # in edgefirst_client::coco::studio::update_coco_annotations).
            label_ids = {label.name: label.id for label in client.labels(dataset_id)}
            new_annotation = ServerAnnotation(
                label_id=label_ids["square"],
                label_name="square",
                annotation_type="box",
                x=original_box.left,
                y=original_box.top,
                w=original_box.width,
                h=original_box.height,
                score=1.0,
                image_id=sample_id.value,
                annotation_set_id=AnnotationSetID(annotation_set_id).value,
            )
            client.add_annotations_bulk(annotation_set_id, [new_annotation])

            head_annotations_after = client.annotations(annotation_set_id)
            self.assertEqual(len(head_annotations_after), 1)
            self.assertEqual(head_annotations_after[0].label, "square")

            # The tag must still reflect the original label.
            tagged_annotations_after = client.annotations(
                annotation_set_id, version="pre-edit"
            )
            self.assertEqual(len(tagged_annotations_after), 1)
            self.assertEqual(tagged_annotations_after[0].label, "circle")
            print(
                f"HEAD annotation now '{head_annotations_after[0].label}', "
                f"tag 'pre-edit' still '{tagged_annotations_after[0].label}'"
            )
        finally:
            if not skip_cleanup:
                client.delete_dataset(dataset_id)

    def test_fetch_back_multiple_historical_tags(self):
        """Create three tags at three different states; verify each can
        still be fetched independently by name, not just the newest."""
        client = get_client()
        skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"
        dataset_id, annotation_set_id, _ = _create_test_dataset(client)

        try:
            _populate_samples(client, dataset_id, annotation_set_id, count=1)
            client.version_tag_create(dataset_id, "v1")
            v1_count = client.samples_count(dataset_id, version="v1").total

            time.sleep(1)
            _populate_samples(client, dataset_id, annotation_set_id, count=2)
            client.version_tag_create(dataset_id, "v2")
            v2_count = client.samples_count(dataset_id, version="v2").total

            time.sleep(1)
            _populate_samples(client, dataset_id, annotation_set_id, count=3)
            client.version_tag_create(dataset_id, "v3")
            v3_count = client.samples_count(dataset_id, version="v3").total

            self.assertEqual(v1_count, 1)
            self.assertEqual(v2_count, 3)
            self.assertEqual(v3_count, 6)

            # Fetch the OLDEST tag back after creating newer ones — proves
            # tags aren't just "the latest", each is independently addressable.
            v1_recheck = client.samples_count(dataset_id, version="v1").total
            self.assertEqual(
                v1_recheck,
                1,
                "Oldest tag must still be fetchable after newer tags exist",
            )

            tags = client.version_tag_list(dataset_id)
            self.assertEqual(len(tags), 3)
            tag_names = {t.name for t in tags}
            self.assertEqual(tag_names, {"v1", "v2", "v3"})

            print(
                f"v1={v1_count}, v2={v2_count}, v3={v3_count}; oldest tag re-fetched as {v1_recheck}"
            )
        finally:
            if not skip_cleanup:
                client.delete_dataset(dataset_id)

    def test_head_reflects_latest_after_tagging_and_editing(self):
        """HEAD reads (no version param) must always reflect the current
        live state, regardless of how many tags exist or when they were
        created — tags never "pin" HEAD."""
        client = get_client()
        skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"
        dataset_id, annotation_set_id, _ = _create_test_dataset(client)

        try:
            _populate_samples(client, dataset_id, annotation_set_id, count=2)
            client.version_tag_create(dataset_id, "checkpoint")

            time.sleep(1)
            _populate_samples(client, dataset_id, annotation_set_id, count=4)

            head_count = client.samples_count(dataset_id).total
            tagged_count = client.samples_count(dataset_id, version="checkpoint").total

            self.assertEqual(head_count, 6)
            self.assertEqual(tagged_count, 2)
            self.assertNotEqual(head_count, tagged_count)

            current = client.version_current(dataset_id)
            self.assertEqual(
                current.current_serial, client.version_changelog_count(dataset_id)
            )
            print(
                f"HEAD={head_count}, tagged='checkpoint'={tagged_count}, current_serial={current.current_serial}"
            )
        finally:
            if not skip_cleanup:
                client.delete_dataset(dataset_id)


class VersionDeleteSampleTest(TestCase):
    """Test deleting samples from a dataset via ``client.delete_samples()``,
    including round-trips through tag-restore past a hard-deleted sample.

    ``image.delete_from_dataset`` (wrapped by ``delete_samples()``) is
    fire-and-forget on the server: the RPC returns once the request is
    accepted, before the delete has actually completed. All tests below
    poll via ``_wait_until_sample_count`` instead of asserting immediately
    after the call returns.

    The restore-focused tests below additionally depend on a server-side fix
    landing on whatever server ``STUDIO_SERVER`` points the test run at:
    before that fix, ``version.tag.restore`` could not survive a tag whose
    image was hard-deleted since — an annotated sample's restore threw a
    foreign-key violation and aborted the whole restore, while an
    unannotated sample's restore silently omitted it (no error, but the
    sample never came back). ``_restore_tag_or_skip_if_unfixed`` below
    detects the annotated case's exact failure signature and skips rather
    than failing red for a known, already-tracked server gap; the
    unannotated case has no exception to catch (it fails a plain count
    assertion instead), so its failure message spells out the same caveat.

    Once the fix is deployed: a hard-deleted image is resurrected reusing
    its ORIGINAL id on restore (not a new one), so ``target_id`` stays valid
    across the delete+restore round trip. annotation_set_id does NOT survive
    restore — every restore deletes and recreates annotation sets with new
    ids, independent of this fix — so tests re-fetch it after restoring.
    """

    def setUp(self):
        self.client = get_client()
        if not _server_supports_versioning(self.client):
            self.skipTest("Server does not support versioning APIs")

    def _restore_tag_or_skip_if_unfixed(self, dataset_id, tag_name):
        """version_tag_restore(), skipping (not failing) if the server
        rejects it with the exact foreign-key-violation signature of a
        hard-deleted image the server-side fix hasn't landed for yet. See
        class docstring."""
        try:
            return self.client.version_tag_restore(dataset_id, tag_name)
        except Exception as e:
            msg = str(e).lower()
            if "annotations_image_id_fkey" in msg or "foreign key" in msg:
                self.skipTest(
                    "Server rejected the restore with a foreign-key "
                    "violation — the exact known symptom of a server "
                    "without the tag-restore-survives-hard-deleted-images "
                    "fix (see class docstring). Not a client bug."
                )
            raise

    def test_delete_annotated_sample_then_restore_brings_it_back(self):
        """Tag, delete an annotated sample, verify it's gone at HEAD but
        still present at the tag, restore the tag, verify the sample and
        its annotation are back at HEAD."""
        client = self.client
        skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"
        dataset_id, annotation_set_id, _ = _create_test_dataset(client)

        try:
            _populate_samples(client, dataset_id, annotation_set_id, count=3)
            client.version_tag_create(dataset_id, "pre-delete")

            samples_before = client.samples(dataset_id, annotation_set_id)
            self.assertEqual(len(samples_before), 3)
            target = samples_before[0]
            target_id = target.id
            self.assertIsNotNone(target_id)
            self.assertEqual(
                len(target.annotations),
                1,
                "Populated sample should have one annotation",
            )

            tagged_before = client.samples(
                dataset_id, annotation_set_id, version="pre-delete"
            )
            tagged_ids_before = {s.id for s in tagged_before}
            self.assertIn(target_id, tagged_ids_before)

            initial_count = client.samples_count(dataset_id).total
            client.delete_samples(dataset_id, [target_id])

            _wait_until_sample_count(client, dataset_id, initial_count - 1)

            # HEAD no longer shows the deleted sample or its annotation.
            head_samples = client.samples(dataset_id, annotation_set_id)
            head_ids = {s.id for s in head_samples}
            self.assertNotIn(target_id, head_ids)

            head_annotations = client.annotations(annotation_set_id)
            self.assertTrue(
                all(a.sample_id != target_id for a in head_annotations),
                "Deleted sample's annotation must not remain at HEAD",
            )

            # Tags are immutable snapshots — this must hold true regardless
            # of the server-side restore fix's status.
            tagged_after = client.samples(
                dataset_id, annotation_set_id, version="pre-delete"
            )
            tagged_ids_after = {s.id for s in tagged_after}
            self.assertIn(
                target_id,
                tagged_ids_after,
                "Tag must still show the sample even after it was hard-deleted at HEAD",
            )
            tagged_annotations = client.annotations(
                annotation_set_id, version="pre-delete"
            )
            self.assertTrue(
                any(a.sample_id == target_id for a in tagged_annotations),
                "Tag must still show the deleted sample's annotation",
            )

            result = self._restore_tag_or_skip_if_unfixed(dataset_id, "pre-delete")
            self.assertTrue(result.success)

            # The resurrected image reuses its original id, so target_id is
            # still the right thing to look for. annotation_set_id does NOT
            # survive restore (annotation sets are always recreated with new
            # ids), so re-fetch it rather than reusing the pre-delete one.
            restored_samples = client.samples(dataset_id)
            restored_ids = {s.id for s in restored_samples}
            self.assertIn(
                target_id,
                restored_ids,
                "Restored HEAD should include the resurrected sample",
            )

            restored_annotation_sets = client.annotation_sets(dataset_id)
            self.assertEqual(len(restored_annotation_sets), 1)
            restored_annotation_set_id = restored_annotation_sets[0].id
            restored_annotations = client.annotations(restored_annotation_set_id)
            self.assertTrue(
                any(a.sample_id == target_id for a in restored_annotations),
                "Restored HEAD should include the resurrected sample's annotation",
            )
        finally:
            if not skip_cleanup:
                client.delete_dataset(dataset_id)

    def test_restore_same_tag_twice_after_delete_is_idempotent(self):
        """Restoring the same tag a second time immediately after the first
        must not mint a second, duplicate resurrection of the same
        originally-deleted sample."""
        client = self.client
        skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"
        dataset_id, annotation_set_id, _ = _create_test_dataset(client)

        try:
            _populate_samples(client, dataset_id, annotation_set_id, count=2)
            client.version_tag_create(dataset_id, "pre-delete")

            samples = client.samples(dataset_id, annotation_set_id)
            target_id = samples[0].id
            initial_count = client.samples_count(dataset_id).total

            client.delete_samples(dataset_id, [target_id])
            _wait_until_sample_count(client, dataset_id, initial_count - 1)

            self._restore_tag_or_skip_if_unfixed(dataset_id, "pre-delete")
            _wait_until_sample_count(client, dataset_id, initial_count)

            # Restore the same tag again, right away.
            result_again = self._restore_tag_or_skip_if_unfixed(
                dataset_id, "pre-delete"
            )
            self.assertTrue(result_again.success)

            final_count = client.samples_count(dataset_id).total
            self.assertEqual(
                final_count,
                initial_count,
                "Restoring the same tag twice must not duplicate the "
                "resurrected sample",
            )
        finally:
            if not skip_cleanup:
                client.delete_dataset(dataset_id)

    def test_delete_unannotated_sample_then_restore_brings_it_back(self):
        """Same round trip as the annotated test, but for a sample with NO
        annotation at all — the other failure mode named in the original
        bug report (silent omission on restore, rather than an FK-violation
        abort, since there's no annotation to violate a foreign key over)."""
        client = self.client
        skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"
        dataset_id, annotation_set_id, _ = _create_test_dataset(client)

        try:
            test_data_dir = get_test_data_dir()
            image_path = (
                test_data_dir
                / f"version_test_unannotated_{int(time.time() * 1000)}.png"
            )
            create_test_image_with_circle(image_path, center_x=100.0, center_y=100.0)
            sample = create_sample_without_annotation(image_path)
            client.populate_samples(dataset_id, annotation_set_id, [sample])

            client.version_tag_create(dataset_id, "pre-delete")

            samples_before = client.samples(dataset_id)
            self.assertEqual(len(samples_before), 1)
            target_id = samples_before[0].id
            initial_count = client.samples_count(dataset_id).total

            client.delete_samples(dataset_id, [target_id])
            _wait_until_sample_count(client, dataset_id, initial_count - 1)

            head_samples = client.samples(dataset_id)
            self.assertNotIn(target_id, {s.id for s in head_samples})

            tagged_samples = client.samples(dataset_id, version="pre-delete")
            self.assertIn(target_id, {s.id for s in tagged_samples})

            result = self._restore_tag_or_skip_if_unfixed(dataset_id, "pre-delete")
            self.assertTrue(result.success)

            restored_count = client.samples_count(dataset_id).total
            self.assertEqual(
                restored_count,
                initial_count,
                "Restore should bring the deleted unannotated sample back "
                "(if this fails without an exception, the server may be "
                "silently omitting it — the exact pre-fix symptom for an "
                "unannotated sample described in the class docstring)",
            )
            self.assertIn(target_id, {s.id for s in client.samples(dataset_id)})
        finally:
            if not skip_cleanup:
                client.delete_dataset(dataset_id)

    def test_delete_multiple_samples_bulk(self):
        """Bulk-delete 2 of 4 samples in a single call; verify only the
        targeted samples are gone and the other 2 remain. No tag/restore
        involved, so this test has no dependency on the server-side fix."""
        client = self.client
        skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"
        dataset_id, annotation_set_id, _ = _create_test_dataset(client)

        try:
            _populate_samples(client, dataset_id, annotation_set_id, count=4)
            samples = client.samples(dataset_id, annotation_set_id)
            self.assertEqual(len(samples), 4)
            all_ids = [s.id for s in samples]
            to_delete = all_ids[:2]
            to_keep = all_ids[2:]

            initial_count = client.samples_count(dataset_id).total
            client.delete_samples(dataset_id, to_delete)

            _wait_until_sample_count(client, dataset_id, initial_count - 2)

            remaining_samples = client.samples(dataset_id, annotation_set_id)
            remaining_ids = {s.id for s in remaining_samples}

            for deleted_id in to_delete:
                self.assertNotIn(deleted_id, remaining_ids)
            for kept_id in to_keep:
                self.assertIn(kept_id, remaining_ids)

            print(
                f"Deleted {len(to_delete)} samples, {len(remaining_ids)} remain "
                f"(expected {len(to_keep)})"
            )
        finally:
            if not skip_cleanup:
                client.delete_dataset(dataset_id)


class VersionDatasetIdTypingTest(TestCase):
    """Verify VersionTag/ChangelogEntry/DatasetSummary/VersionCurrentResponse
    expose dataset_id as a DatasetID, not a raw int (PR #34 review comment)."""

    def setUp(self):
        if not _server_supports_versioning(get_client()):
            self.skipTest("Server does not support versioning APIs")

    def test_dataset_id_is_datasetid_not_int(self):
        """dataset_id fields should compare equal to the DatasetID used to
        create the resources, and should not be a plain int."""
        import edgefirst_client as ec

        client = get_client()
        skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"
        dataset_id, annotation_set_id, _ = _create_test_dataset(client)
        # create_dataset() returns a plain str; wrap it once so we can
        # compare against the DatasetID objects returned by the versioning
        # APIs (DatasetID.__eq__ only accepts another DatasetID).
        expected_id = ec.DatasetID(dataset_id)

        try:
            _populate_samples(client, dataset_id, annotation_set_id, count=1)

            tag = client.version_tag_create(dataset_id, "typed-tag")
            self.assertIsInstance(tag.dataset_id, ec.DatasetID)
            self.assertEqual(tag.dataset_id, expected_id)

            changelog = client.version_changelog(dataset_id)
            self.assertGreater(len(changelog.entries), 0)
            self.assertIsInstance(changelog.entries[0].dataset_id, ec.DatasetID)
            self.assertEqual(changelog.entries[0].dataset_id, expected_id)

            summary = client.version_summary(dataset_id)
            self.assertIsInstance(summary.dataset_id, ec.DatasetID)
            self.assertEqual(summary.dataset_id, expected_id)

            current = client.version_current(dataset_id)
            self.assertIsInstance(current.dataset_id, ec.DatasetID)
            self.assertEqual(current.dataset_id, expected_id)

            print("dataset_id typing verified as DatasetID on all 4 types")
        finally:
            if not skip_cleanup:
                client.delete_dataset(dataset_id)


class VersionDatasetConvenienceMethodsTest(TestCase):
    """Verify Dataset-level convenience wrappers for the versioning RPCs
    (previously only exposed on Client), mirroring labels()/samples()/
    annotation_sets()/samples_count()'s existing Client+Dataset symmetry."""

    def setUp(self):
        if not _server_supports_versioning(get_client()):
            self.skipTest("Server does not support versioning APIs")

    def test_dataset_level_version_methods_match_client_level(self):
        """Dataset.version_*() should behave identically to the equivalent
        client.version_*(dataset_id) calls."""
        client = get_client()
        skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"
        dataset_id, annotation_set_id, _ = _create_test_dataset(client)

        try:
            _populate_samples(client, dataset_id, annotation_set_id, count=2)
            dataset = client.dataset(dataset_id)

            tag = dataset.version_tag_create("dataset-level-tag", "via Dataset")
            self.assertEqual(tag.name, "dataset-level-tag")
            self.assertEqual(tag.description, "via Dataset")

            fetched = dataset.version_tag_get("dataset-level-tag")
            self.assertEqual(fetched.serial, tag.serial)

            tags = dataset.version_tag_list()
            self.assertEqual(len(tags), 1)
            self.assertEqual(tags[0].name, "dataset-level-tag")

            changelog = dataset.version_changelog()
            self.assertEqual(
                len(changelog.entries), client.version_changelog(dataset_id).count
            )

            count = dataset.version_changelog_count()
            self.assertEqual(count, client.version_changelog_count(dataset_id))

            current = dataset.version_current()
            self.assertEqual(
                current.current_serial,
                client.version_current(dataset_id).current_serial,
            )

            summary = dataset.version_summary()
            self.assertEqual(summary.image_count, 2)

            recalculated = dataset.version_summary_recalculate()
            self.assertEqual(recalculated.image_count, summary.image_count)

            time.sleep(1)
            _populate_samples(client, dataset_id, annotation_set_id, count=1)
            result = dataset.version_tag_restore("dataset-level-tag")
            self.assertTrue(result.success)
            self.assertEqual(client.samples_count(dataset_id).total, 2)

            deleted = dataset.version_tag_delete("dataset-level-tag")
            self.assertIsNotNone(deleted)
            self.assertEqual(len(dataset.version_tag_list()), 0)

            print("Dataset-level version_*() methods match Client-level equivalents")
        finally:
            if not skip_cleanup:
                client.delete_dataset(dataset_id)


if __name__ == "__main__":
    unittest.main()
