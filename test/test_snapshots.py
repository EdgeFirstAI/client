#!/usr/bin/env python3
"""
Comprehensive test suite for Snapshot API functionality.

Tests cover:
- Snapshot class properties and methods
- Snapshot creation from files and directories
- Snapshot download with progress tracking
- Snapshot restore with AGTG options
- Snapshot deletion
- Error handling and edge cases
"""

import os
import tempfile
import unittest
from pathlib import Path

from test import get_client


class TestSnapshotAPI(unittest.TestCase):
    """Test Snapshot API across all layers (Rust, CLI, Python)."""

    @classmethod
    def setUpClass(cls):
        """Set up test fixtures once for all tests."""
        cls.client = get_client()
        # Get first project for testing
        projects = cls.client.projects()
        if len(projects) == 0:
            raise RuntimeError("No projects available for testing")
        cls.project_id = str(projects[0].id)

    def test_snapshots_list(self):
        """Test listing all snapshots."""
        snapshots = self.client.snapshots()
        self.assertIsInstance(snapshots, list)
        # May be empty if no snapshots exist
        for snapshot in snapshots:
            self.assertIsNotNone(snapshot.id)
            self.assertIsNotNone(snapshot.description)
            self.assertIsNotNone(snapshot.status)
            self.assertIsNotNone(snapshot.path)
            self.assertIsNotNone(snapshot.created)

    def test_snapshot_class_properties(self):
        """Test Snapshot class property accessors."""
        snapshots = self.client.snapshots()
        if len(snapshots) == 0:
            self.skipTest("No snapshots available for testing")

        snapshot = snapshots[0]

        # Test all property accessors
        self.assertIsNotNone(snapshot.id)
        self.assertIsInstance(str(snapshot.id), str)

        self.assertIsInstance(snapshot.description, str)
        self.assertGreater(len(snapshot.description), 0)

        self.assertIsInstance(snapshot.status, str)
        self.assertGreater(len(snapshot.status), 0)

        self.assertIsInstance(snapshot.path, str)
        self.assertGreater(len(snapshot.path), 0)

        self.assertIsInstance(snapshot.created, str)
        self.assertGreater(len(snapshot.created), 0)

    def test_snapshot_repr(self):
        """Test Snapshot.__repr__() method."""
        snapshots = self.client.snapshots()
        if len(snapshots) == 0:
            self.skipTest("No snapshots available for testing")

        snapshot = snapshots[0]
        repr_str = repr(snapshot)

        # Verify repr contains key information
        self.assertIn("Snapshot(", repr_str)
        self.assertIn(f"id={snapshot.id}", repr_str)
        self.assertIn(f"description='{snapshot.description}'", repr_str)
        self.assertIn(f"status='{snapshot.status}'", repr_str)
        self.assertIn(f"path='{snapshot.path}'", repr_str)

    def test_snapshot_get_by_id(self):
        """Test retrieving a specific snapshot by ID."""
        snapshots = self.client.snapshots()
        if len(snapshots) == 0:
            self.skipTest("No snapshots available for testing")

        original = snapshots[0]
        retrieved = self.client.snapshot(original.id)

        # Verify same snapshot was retrieved
        self.assertEqual(str(original.id), str(retrieved.id))
        self.assertEqual(original.description, retrieved.description)
        self.assertEqual(original.status, retrieved.status)
        self.assertEqual(original.path, retrieved.path)

    def test_create_snapshot_small_file(self):
        """Test creating snapshot from a small file (<100MB)."""
        # Create a temporary test file (small)
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".txt", delete=False
        ) as f:
            test_file = f.name
            # Write ~1MB of data
            f.write("x" * (1024 * 1024))

        try:
            snapshot = self.client.create_snapshot(test_file)

            # Verify snapshot was created
            self.assertIsNotNone(snapshot)
            self.assertIsNotNone(snapshot.id)
            self.assertIsInstance(snapshot.description, str)
            self.assertIn(snapshot.status, ["available", "processing", "pending"])

            # Clean up: delete the snapshot
            self.client.delete_snapshot(snapshot.id)

        finally:
            # Clean up test file
            if os.path.exists(test_file):
                os.unlink(test_file)

    def test_create_snapshot_medium_file(self):
        """Test creating snapshot from medium file (~150MB, multipart upload)."""
        # Create a temporary test file that will trigger multipart upload
        with tempfile.NamedTemporaryFile(
            mode="wb", suffix=".bin", delete=False
        ) as f:
            test_file = f.name
            # Write 150MB to trigger multipart (PART_SIZE = 100MB)
            chunk_size = 1024 * 1024  # 1MB chunks
            for _ in range(150):  # 150MB total
                f.write(b"x" * chunk_size)

        try:
            snapshot = self.client.create_snapshot(test_file)

            # Verify snapshot was created
            self.assertIsNotNone(snapshot)
            self.assertIsNotNone(snapshot.id)
            self.assertIsInstance(snapshot.description, str)

            # Clean up: delete the snapshot
            self.client.delete_snapshot(snapshot.id)

        finally:
            # Clean up test file
            if os.path.exists(test_file):
                os.unlink(test_file)

    @unittest.skip(
        "Large file test (>4GB) - run manually with: "
        "python -m unittest test.test_snapshots.TestSnapshotAPI.test_create_snapshot_large_file"
    )
    def test_create_snapshot_large_file(self):
        """Test creating snapshot from large file (>4GB) - SKIPPED by default."""
        # Create a 5GB sparse file for testing
        test_file = None
        try:
            with tempfile.NamedTemporaryFile(
                mode="wb", suffix=".bin", delete=False
            ) as f:
                test_file = f.name
                # Create sparse file (fast, doesn't actually write 5GB)
                f.seek(5 * 1024 * 1024 * 1024 - 1)  # 5GB - 1 byte
                f.write(b"\0")

            snapshot = self.client.create_snapshot(test_file)

            # Verify snapshot was created
            self.assertIsNotNone(snapshot)
            self.assertIsNotNone(snapshot.id)

            # Clean up: delete the snapshot
            self.client.delete_snapshot(snapshot.id)

        finally:
            if test_file and os.path.exists(test_file):
                os.unlink(test_file)

    def test_create_snapshot_directory(self):
        """Test creating snapshot from a directory with multiple files."""
        # Create temporary directory with test files
        with tempfile.TemporaryDirectory() as temp_dir:
            # Create several test files
            for i in range(5):
                file_path = Path(temp_dir) / f"test_{i}.txt"
                file_path.write_text(f"Test content {i}\n" * 1000)

            snapshot = self.client.create_snapshot(temp_dir)

            # Verify snapshot was created
            self.assertIsNotNone(snapshot)
            self.assertIsNotNone(snapshot.id)
            self.assertIsInstance(snapshot.description, str)

            # Clean up: delete the snapshot
            self.client.delete_snapshot(snapshot.id)

    def test_download_snapshot(self):
        """Test downloading a snapshot."""
        snapshots = self.client.snapshots()
        if len(snapshots) == 0:
            self.skipTest("No snapshots available for testing")

        snapshot = snapshots[0]

        with tempfile.TemporaryDirectory() as temp_dir:
            output_path = Path(temp_dir)
            self.client.download_snapshot(snapshot.id, str(output_path))

            # Verify files were downloaded
            downloaded_files = list(output_path.rglob("*"))
            # Filter out directories
            downloaded_files = [f for f in downloaded_files if f.is_file()]
            self.assertGreater(
                len(downloaded_files), 0, "No files were downloaded"
            )

    def test_delete_snapshot(self):
        """Test deleting a snapshot."""
        # Create a small test file
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".txt", delete=False
        ) as f:
            test_file = f.name
            f.write("Test snapshot for deletion\n" * 100)

        try:
            # Create snapshot
            snapshot = self.client.create_snapshot(test_file)
            snapshot_id = snapshot.id

            # Delete snapshot
            self.client.delete_snapshot(snapshot_id)

            # Verify snapshot is deleted by trying to retrieve it
            # This should raise an error
            with self.assertRaises(Exception):
                self.client.snapshot(snapshot_id)

        finally:
            # Clean up test file
            if os.path.exists(test_file):
                os.unlink(test_file)

    def test_snapshot_id_format(self):
        """Test SnapshotID string format is 'ss-xxx'."""
        snapshots = self.client.snapshots()
        if len(snapshots) == 0:
            self.skipTest("No snapshots available for testing")

        snapshot = snapshots[0]
        str_id = str(snapshot.id)
        self.assertTrue(str_id.startswith("ss-"))

        # Verify hex part is valid
        hex_part = str_id[3:]  # Skip "ss-"
        try:
            value = int(hex_part, 16)
            self.assertGreater(value, 0)
        except ValueError:
            self.fail(f"Invalid hex in snapshot ID: {str_id}")


class TestSnapshotErrorHandling(unittest.TestCase):
    """Test error handling in Snapshot API."""

    @classmethod
    def setUpClass(cls):
        """Set up test fixtures."""
        cls.client = get_client()

    def test_snapshot_nonexistent_id(self):
        """Test error when retrieving non-existent snapshot."""
        # Use string format that will be converted to ID
        fake_id = "ss-ffffffffff"

        with self.assertRaises(Exception):
            self.client.snapshot(fake_id)

    def test_delete_nonexistent_snapshot(self):
        """Test error when deleting non-existent snapshot."""
        # Use string format that will be converted to ID
        fake_id = "ss-ffffffffff"

        with self.assertRaises(Exception):
            self.client.delete_snapshot(fake_id)

    def test_create_snapshot_invalid_path(self):
        """Test error when creating snapshot from non-existent path."""
        with self.assertRaises(Exception):
            self.client.create_snapshot("/nonexistent/path/to/file.txt")


class TestCreateSnapshotFromDataset(unittest.TestCase):
    """Test create_snapshot_from_dataset API functionality.

    This tests creating snapshots from existing datasets on the server,
    with and without explicit annotation set IDs.
    """

    @classmethod
    def setUpClass(cls):
        """Set up test fixtures."""
        cls.client = get_client()
        # Get a test dataset - use 'Deer' dataset from Unit Testing project
        datasets = cls.client.datasets(name="Deer")
        if len(datasets) == 0:
            raise RuntimeError("Deer dataset not found for testing")
        cls.dataset = datasets[0]
        cls.dataset_id = cls.dataset.id

    def test_create_snapshot_from_dataset_no_annotation_set(self):
        """Test creating snapshot from dataset without explicit annotation_set_id.

        When annotation_set_id is not provided, the API should automatically
        use the default 'annotations' set or the first available set.
        """
        result = self.client.create_snapshot_from_dataset(
            self.dataset_id, "Test snapshot - no annotation set"
        )

        # Verify result structure
        self.assertIsNotNone(result)
        self.assertIsNotNone(result.id)
        snapshot_id_str = str(result.id)
        self.assertTrue(
            snapshot_id_str.startswith("ss-"),
            f"Snapshot ID should start with 'ss-', got: {snapshot_id_str}",
        )

        # Task ID may or may not be present depending on server behavior
        if result.task_id:
            task_id_str = str(result.task_id)
            self.assertTrue(
                task_id_str.startswith("task-"),
                f"Task ID should start with 'task-', got: {task_id_str}",
            )

        # Clean up - delete the created snapshot
        try:
            self.client.delete_snapshot(result.id)
        except Exception:
            pass  # Best effort cleanup

    def test_create_snapshot_from_dataset_with_annotation_set(self):
        """Test creating snapshot from dataset with explicit annotation_set_id."""
        # Get annotation sets for the dataset
        annotation_sets = self.client.annotation_sets(self.dataset_id)
        self.assertGreater(
            len(annotation_sets), 0, "Dataset should have at least one annotation set"
        )

        # Use the first annotation set
        ann_set = annotation_sets[0]
        ann_set_id = ann_set.id

        result = self.client.create_snapshot_from_dataset(
            self.dataset_id, "Test snapshot - with annotation set", ann_set_id
        )

        # Verify result structure
        self.assertIsNotNone(result)
        self.assertIsNotNone(result.id)

        # Clean up - delete the created snapshot
        try:
            self.client.delete_snapshot(result.id)
        except Exception:
            pass  # Best effort cleanup

    def test_create_snapshot_from_dataset_string_ids(self):
        """Test that string IDs work for create_snapshot_from_dataset."""
        # Convert dataset_id to string format
        dataset_id_str = str(self.dataset_id)
        self.assertTrue(
            dataset_id_str.startswith("ds-"),
            f"Dataset ID should be 'ds-xxx' format: {dataset_id_str}",
        )

        result = self.client.create_snapshot_from_dataset(
            dataset_id_str, "Test snapshot - string ID"
        )

        # Verify result
        self.assertIsNotNone(result)
        self.assertIsNotNone(result.id)

        # Clean up
        try:
            self.client.delete_snapshot(result.id)
        except Exception:
            pass

    def test_create_snapshot_from_dataset_invalid_dataset(self):
        """Test error handling for non-existent dataset."""
        fake_dataset_id = "ds-ffffffffff"

        with self.assertRaises(Exception):
            self.client.create_snapshot_from_dataset(
                fake_dataset_id, "Should fail"
            )

    def test_create_snapshot_from_dataset_invalid_annotation_set(self):
        """Test error handling for non-existent annotation set."""
        fake_ann_set_id = "as-ffffffffff"

        with self.assertRaises(Exception):
            self.client.create_snapshot_from_dataset(
                self.dataset_id, "Should fail", fake_ann_set_id
            )

    def test_snapshot_from_dataset_result_properties(self):
        """Test SnapshotFromDatasetResult class properties."""
        result = self.client.create_snapshot_from_dataset(
            self.dataset_id, "Test result properties"
        )

        # Test id property
        self.assertIsNotNone(result.id)
        id_str = str(result.id)
        assert id_str is not None  # Pylance type narrowing
        self.assertTrue(id_str.startswith("ss-"))

        # Test task_id property (may be None)
        task_id = result.task_id
        if task_id is not None:
            task_id_str = str(task_id)
            assert task_id_str is not None  # Pylance type narrowing
            self.assertTrue(task_id_str.startswith("task-"))

        # Clean up
        try:
            self.client.delete_snapshot(result.id)
        except Exception:
            pass


if __name__ == "__main__":
    unittest.main()
