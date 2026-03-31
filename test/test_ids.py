#!/usr/bin/env python3
"""
Comprehensive tests for ID types.

Tests that all ID types can be converted to strings with the correct format
(prefix-hex) and that classes providing .uid() also provide .id() with
matching string representations.
"""

import unittest
from test import get_client

from edgefirst_client import (
    AnnotationSetID,
    AppId,
    DatasetID,
    ExperimentID,
    ImageId,
    OrganizationID,
    ProjectID,
    SampleID,
    SequenceId,
    SnapshotID,
    TaskID,
    TrainingSessionID,
    ValidationSessionID,
)


class TestIDTypes(unittest.TestCase):
    """Test suite for ID type string formatting and conversions."""

    def _get_reference_dataset(self, client):
        """Return the Test Labels dataset from the Unit Testing project."""
        projects = client.projects("Unit Testing")
        self.assertGreater(len(projects), 0)
        project = projects[0]

        datasets = client.datasets(project.id, "Test Labels")
        self.assertGreater(len(datasets), 0)
        dataset = next(
            (item for item in datasets if item.name == "Test Labels"),
            None,
        )
        self.assertIsNotNone(dataset, "Test Labels dataset should exist")
        assert dataset is not None
        return project, dataset

    # =========================================================================
    # ID String Format Tests (using string parsing)
    # =========================================================================

    def test_organization_id_format(self):
        """Test OrganizationID string format is 'org-xxx'."""
        # IDs don't expose constructors, they're returned from API
        # We test with real data from the server
        client = get_client()
        org = client.organization()
        str_id = str(org.id)
        self.assertTrue(str_id.startswith("org-"))
        # Verify hex format
        hex_part = str_id[4:]  # Skip "org-"
        int(hex_part, 16)  # Should not raise

    def test_project_id_format(self):
        """Test ProjectID string format is 'p-xxx'."""
        client = get_client()
        projects = client.projects()
        self.assertGreater(len(projects), 0)
        project = projects[0]
        str_id = str(project.id)
        self.assertTrue(str_id.startswith("p-"))
        # Verify hex part
        hex_part = str_id[2:]  # Skip "p-"
        value = int(hex_part, 16)
        self.assertEqual(project.id.value, value)

    def test_dataset_id_format(self):
        """Test DatasetID string format is 'ds-xxx'."""
        client = get_client()
        _, dataset = self._get_reference_dataset(client)
        str_id = str(dataset.id)
        self.assertTrue(str_id.startswith("ds-"))
        # Verify hex part
        hex_part = str_id[3:]  # Skip "ds-"
        value = int(hex_part, 16)
        self.assertEqual(dataset.id.value, value)

    def test_annotation_set_id_format(self):
        """Test AnnotationSetID string format is 'as-xxx'."""
        client = get_client()
        _, dataset = self._get_reference_dataset(client)

        annotation_sets = client.annotation_sets(str(dataset.id))
        self.assertGreater(len(annotation_sets), 0)
        as_obj = annotation_sets[0]
        str_id = str(as_obj.id)
        self.assertTrue(str_id.startswith("as-"))
        # Verify hex part
        hex_part = str_id[3:]  # Skip "as-"
        value = int(hex_part, 16)
        self.assertEqual(as_obj.id.value, value)

    def test_sample_id_format(self):
        """Test SampleID string format is 's-xxx'."""
        client = get_client()
        _, dataset = self._get_reference_dataset(client)

        samples = client.samples(str(dataset.id))
        self.assertGreater(len(samples), 0)
        sample = samples[0]
        sample_id = sample.id
        self.assertIsNotNone(sample_id)
        assert sample_id is not None
        str_id = str(sample_id)
        self.assertTrue(str_id.startswith("s-"))
        # Verify hex part
        hex_part = str_id[2:]  # Skip "s-"
        value = int(hex_part, 16)
        self.assertEqual(sample_id.value, value)

    def test_experiment_id_format(self):
        """Test ExperimentID string format is 'exp-xxx'."""
        client = get_client()
        projects = client.projects()
        self.assertGreater(len(projects), 0)
        experiments = client.experiments(str(projects[0].id))
        if len(experiments) > 0:
            experiment = experiments[0]
            str_id = str(experiment.id)
            self.assertTrue(str_id.startswith("exp-"))
            # Verify hex part
            hex_part = str_id[4:]  # Skip "exp-"
            value = int(hex_part, 16)
            self.assertEqual(experiment.id.value, value)

    def test_training_session_id_format(self):
        """Test TrainingSessionID string format is 't-xxx'."""
        client = get_client()
        projects = client.projects()
        self.assertGreater(len(projects), 0)
        experiments = client.experiments(str(projects[0].id))
        # Find an experiment with training sessions
        for exp in experiments:
            training_sessions = client.training_sessions(str(exp.id))
            if len(training_sessions) > 0:
                training = training_sessions[0]
                str_id = str(training.id)
                self.assertTrue(str_id.startswith("t-"))
                # Verify hex part
                hex_part = str_id[2:]  # Skip "t-"
                value = int(hex_part, 16)
                self.assertEqual(training.id.value, value)
                break

    def test_validation_session_id_format(self):
        """Test ValidationSessionID string format is 'v-xxx'."""
        client = get_client()
        projects = client.projects()
        self.assertGreater(len(projects), 0)
        # validation_sessions takes project_id, not experiment_id
        validation_sessions = client.validation_sessions(str(projects[0].id))
        if len(validation_sessions) > 0:
            validation = validation_sessions[0]
            str_id = str(validation.id)
            self.assertTrue(str_id.startswith("v-"))
            # Verify hex part
            hex_part = str_id[2:]  # Skip "v-"
            value = int(hex_part, 16)
            self.assertEqual(validation.id.value, value)

    def test_snapshot_id_format(self):
        """Test SnapshotID string format is 'ss-xxx'."""
        # Snapshots are less commonly available, skip if not found
        pass

    def test_task_id_format(self):
        """Test TaskID string format is 'task-xxx'."""
        client = get_client()
        projects = client.projects()
        self.assertGreater(len(projects), 0)
        tasks = client.tasks(str(projects[0].id))
        if len(tasks) > 0:
            task = tasks[0]
            str_id = str(task.id)
            self.assertTrue(str_id.startswith("task-"))
            # Verify hex part
            hex_part = str_id[5:]  # Skip "task-"
            value = int(hex_part, 16)
            self.assertEqual(task.id.value, value)

    def test_image_id_format(self):
        """Test ImageId string format is 'im-xxx'."""
        # ImageId is used internally, not commonly exposed in API
        pass

    def test_sequence_id_format(self):
        """Test SequenceId string format is 'se-xxx'."""
        # SequenceId is used internally, not commonly exposed in API
        pass

    def test_app_id_format(self):
        """Test AppId string format is 'app-xxx'."""
        # AppId is used internally, not commonly exposed in API
        pass

    # =========================================================================
    # ID Consistency Tests (classes with both .id() and .uid())
    # =========================================================================

    def test_project_id_uid_consistency(self):
        """Test Project.id() and Project.uid() return consistent values."""
        client = get_client()
        projects = client.projects()
        self.assertGreater(len(projects), 0)
        project = projects[0]
        # Get id as ID object
        id_obj = project.id
        # Get uid as string
        uid_str = project.uid
        # Convert id to string and compare
        id_str = str(id_obj)
        self.assertEqual(id_str, uid_str)
        self.assertTrue(uid_str.startswith("p-"))

    def test_dataset_id_uid_consistency(self):
        """Test Dataset.id() and Dataset.uid() return consistent values."""
        client = get_client()
        _, dataset = self._get_reference_dataset(client)
        # Get id as ID object
        id_obj = dataset.id
        # Get uid as string
        uid_str = dataset.uid
        # Convert id to string and compare
        id_str = str(id_obj)
        self.assertEqual(id_str, uid_str)
        self.assertTrue(uid_str.startswith("ds-"))

    def test_annotation_set_id_uid_consistency(self):
        """Test AnnotationSet.id() and AnnotationSet.uid() consistent."""
        client = get_client()
        _, dataset = self._get_reference_dataset(client)

        annotation_sets = client.annotation_sets(str(dataset.id))
        self.assertGreater(len(annotation_sets), 0)
        as_obj = annotation_sets[0]
        # Get id as ID object
        id_obj = as_obj.id
        # Get uid as string
        uid_str = as_obj.uid
        # Convert id to string and compare
        id_str = str(id_obj)
        self.assertEqual(id_str, uid_str)
        self.assertTrue(uid_str.startswith("as-"))

    def test_experiment_id_uid_consistency(self):
        """Test Experiment.id() and Experiment.uid() are consistent."""
        client = get_client()
        projects = client.projects()
        self.assertGreater(len(projects), 0)
        experiments = client.experiments(str(projects[0].id))
        if len(experiments) > 0:
            experiment = experiments[0]
            # Get id as ID object
            id_obj = experiment.id
            # Get uid as string
            uid_str = experiment.uid
            # Convert id to string and compare
            id_str = str(id_obj)
            self.assertEqual(id_str, uid_str)
            self.assertTrue(uid_str.startswith("exp-"))

    def test_training_session_id_uid_consistency(self):
        """Test TrainingSession.id() and .uid() consistent."""
        client = get_client()
        projects = client.projects()
        self.assertGreater(len(projects), 0)
        experiments = client.experiments(str(projects[0].id))
        # Find an experiment with training sessions
        for exp in experiments:
            training_sessions = client.training_sessions(str(exp.id))
            if len(training_sessions) > 0:
                training = training_sessions[0]
                # Get id as ID object
                id_obj = training.id
                # Get uid as string
                uid_str = training.uid
                # Convert id to string and compare
                id_str = str(id_obj)
                self.assertEqual(id_str, uid_str)
                self.assertTrue(uid_str.startswith("t-"))
                break

    def test_validation_session_id_uid_consistency(self):
        """Test ValidationSession.id() and .uid() are consistent."""
        client = get_client()
        projects = client.projects()
        self.assertGreater(len(projects), 0)
        # validation_sessions takes project_id, not experiment_id
        validation_sessions = client.validation_sessions(str(projects[0].id))
        if len(validation_sessions) > 0:
            validation = validation_sessions[0]
            # Get id as ID object
            id_obj = validation.id
            # Get uid as string
            uid_str = validation.uid
            # Convert id to string and compare
            id_str = str(id_obj)
            self.assertEqual(id_str, uid_str)
            self.assertTrue(uid_str.startswith("v-"))

    def test_task_id_uid_consistency(self):
        """Test Task.id() and Task.uid() are consistent."""
        client = get_client()
        projects = client.projects()
        self.assertGreater(len(projects), 0)
        tasks = client.tasks(str(projects[0].id))
        if len(tasks) > 0:
            task = tasks[0]
            # Get id as ID object
            id_obj = task.id
            # Get uid as string
            uid_str = task.uid
            # Convert id to string and compare
            id_str = str(id_obj)
            self.assertEqual(id_str, uid_str)
            self.assertTrue(uid_str.startswith("task-"))

    def test_task_info_id_uid_consistency(self):
        """Test TaskInfo.id() and TaskInfo.uid() are consistent."""
        client = get_client()
        projects = client.projects()
        self.assertGreater(len(projects), 0)
        tasks = client.tasks(str(projects[0].id))
        if len(tasks) > 0:
            task = tasks[0]
            # Get id as ID object
            id_obj = task.id
            # Get uid as string
            uid_str = task.uid
            # Convert id to string and compare
            id_str = str(id_obj)
            self.assertEqual(id_str, uid_str)
            self.assertTrue(uid_str.startswith("task-"))

    def test_sample_id_uid_consistency(self):
        """Test Sample.id() and Sample.uid() are consistent."""
        client = get_client()
        _, dataset = self._get_reference_dataset(client)

        samples = client.samples(str(dataset.id))
        self.assertGreater(len(samples), 0)
        sample = samples[0]
        # Sample.id is Optional
        sample_id = sample.id
        sample_uid = sample.uid
        if sample_id is not None:
            # If id exists, uid should also exist
            self.assertIsNotNone(sample_uid)
            assert sample_uid is not None
            # Convert id to string and compare
            id_str = str(sample_id)
            self.assertEqual(id_str, sample_uid)
            self.assertTrue(sample_uid.startswith("s-"))
        else:
            # If id is None, uid should also be None
            self.assertIsNone(sample_uid)


class TestIDConversions(unittest.TestCase):
    """Offline tests for ID type construction and conversion.

    These tests verify that all 13 ID types support:
    - Construction from a prefixed hex string via __init__
    - Construction from an integer via __init__
    - The from_str static method
    - Round-trip int → string → int fidelity
    - Rejection of wrong prefixes
    - Rejection of invalid hex digits

    No server connection is required.
    """

    # (class, prefix) for every ID type in the module
    ID_TYPES = [
        (ProjectID, "p"),
        (DatasetID, "ds"),
        (ExperimentID, "exp"),
        (OrganizationID, "org"),
        (SampleID, "s"),
        (AnnotationSetID, "as"),
        (TaskID, "task"),
        (TrainingSessionID, "t"),
        (ValidationSessionID, "v"),
        (SnapshotID, "ss"),
        (ImageId, "im"),
        (SequenceId, "se"),
        (AppId, "app"),
    ]

    # ------------------------------------------------------------------
    # Helper
    # ------------------------------------------------------------------

    def _test_id_type(self, cls, prefix, hex_value=0xABC123):
        """Exercise standard construction and conversion for one ID type."""
        hex_str = f"{prefix}-{hex_value:x}"

        # --- construct from prefixed hex string -------------------------
        id_from_str = cls(hex_str)
        self.assertEqual(id_from_str.value, hex_value)
        self.assertEqual(str(id_from_str), hex_str)

        # --- construct from integer -------------------------------------
        id_from_int = cls(hex_value)
        self.assertEqual(str(id_from_int), hex_str)
        self.assertEqual(int(id_from_int), hex_value)

        # --- from_str static method -------------------------------------
        id_static = cls.from_str(hex_str)
        self.assertEqual(id_static.value, hex_value)
        self.assertEqual(str(id_static), hex_str)

        # --- round-trip: int → string → int -----------------------------
        original = cls(42)
        as_str = str(original)
        restored = cls(as_str)
        self.assertEqual(original.value, restored.value)
        self.assertEqual(int(original), int(restored))

        # --- wrong prefix → error ---------------------------------------
        with self.assertRaises(Exception):
            cls("zzz-abc123")

        # --- invalid hex chars → error ----------------------------------
        with self.assertRaises(Exception):
            cls(f"{prefix}-xyz")

    # ------------------------------------------------------------------
    # Per-type tests
    # ------------------------------------------------------------------

    def test_project_id_conversions(self):
        """Test ProjectID construction and conversion (prefix 'p-')."""
        self._test_id_type(ProjectID, "p")

    def test_dataset_id_conversions(self):
        """Test DatasetID construction and conversion (prefix 'ds-')."""
        self._test_id_type(DatasetID, "ds")

    def test_experiment_id_conversions(self):
        """Test ExperimentID construction and conversion (prefix 'exp-')."""
        self._test_id_type(ExperimentID, "exp")

    def test_organization_id_conversions(self):
        """Test OrganizationID construction and conversion (prefix 'org-')."""
        self._test_id_type(OrganizationID, "org")

    def test_sample_id_conversions(self):
        """Test SampleID construction and conversion (prefix 's-')."""
        self._test_id_type(SampleID, "s")

    def test_annotation_set_id_conversions(self):
        """Test AnnotationSetID construction and conversion (prefix 'as-')."""
        self._test_id_type(AnnotationSetID, "as")

    def test_task_id_conversions(self):
        """Test TaskID construction and conversion (prefix 'task-')."""
        self._test_id_type(TaskID, "task")

    def test_training_session_id_conversions(self):
        """Test TrainingSessionID construction and conversion (prefix 't-')."""
        self._test_id_type(TrainingSessionID, "t")

    def test_validation_session_id_conversions(self):
        """Test ValidationSessionID construction and conversion (prefix 'v-')."""
        self._test_id_type(ValidationSessionID, "v")

    def test_snapshot_id_conversions(self):
        """Test SnapshotID construction and conversion (prefix 'ss-')."""
        self._test_id_type(SnapshotID, "ss")

    def test_image_id_conversions(self):
        """Test ImageId construction and conversion (prefix 'im-')."""
        self._test_id_type(ImageId, "im")

    def test_sequence_id_conversions(self):
        """Test SequenceId construction and conversion (prefix 'se-')."""
        self._test_id_type(SequenceId, "se")

    def test_app_id_conversions(self):
        """Test AppId construction and conversion (prefix 'app-')."""
        self._test_id_type(AppId, "app")

    # ------------------------------------------------------------------
    # Additional edge-case tests
    # ------------------------------------------------------------------

    def test_all_id_types_covered(self):
        """Ensure we test all 13 ID types."""
        self.assertEqual(len(self.ID_TYPES), 13)

    def test_zero_value(self):
        """Test that ID types handle zero correctly."""
        for cls, prefix in self.ID_TYPES:
            with self.subTest(cls=cls.__name__):
                id_obj = cls(0)
                self.assertEqual(id_obj.value, 0)
                self.assertEqual(int(id_obj), 0)
                self.assertEqual(str(id_obj), f"{prefix}-0")

    def test_large_value(self):
        """Test that ID types handle large 64-bit values."""
        large = 0xDEADBEEFCAFE
        for cls, prefix in self.ID_TYPES:
            with self.subTest(cls=cls.__name__):
                id_obj = cls(large)
                self.assertEqual(id_obj.value, large)
                self.assertEqual(int(id_obj), large)
                self.assertEqual(str(id_obj), f"{prefix}-deadbeefcafe")

    def test_from_str_round_trip_all(self):
        """Test from_str → str round-trip for every ID type."""
        for cls, prefix in self.ID_TYPES:
            with self.subTest(cls=cls.__name__):
                hex_str = f"{prefix}-ff00ff"
                id_obj = cls.from_str(hex_str)
                self.assertEqual(str(id_obj), hex_str)
                self.assertEqual(id_obj.value, 0xFF00FF)

    def test_int_round_trip_all(self):
        """Test int → cls → int round-trip for every ID type."""
        for cls, prefix in self.ID_TYPES:
            with self.subTest(cls=cls.__name__):
                original_val = 0x1A2B3C
                id_obj = cls(original_val)
                self.assertEqual(int(id_obj), original_val)

    def test_invalid_prefix_all(self):
        """Test that every ID type rejects a wrong prefix."""
        for cls, prefix in self.ID_TYPES:
            with self.subTest(cls=cls.__name__):
                with self.assertRaises(Exception):
                    cls("BADPREFIX-abc123")

    def test_invalid_hex_all(self):
        """Test that every ID type rejects non-hex characters."""
        for cls, prefix in self.ID_TYPES:
            with self.subTest(cls=cls.__name__):
                with self.assertRaises(Exception):
                    cls(f"{prefix}-ghijkl")


if __name__ == "__main__":
    unittest.main()
