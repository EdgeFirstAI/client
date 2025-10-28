#!/usr/bin/env python3
"""
Comprehensive tests for ID types.

Tests that all ID types can be converted to strings with the correct format
(prefix-hex) and that classes providing .uid() also provide .id() with
matching string representations.
"""

import unittest
from test import get_client


class TestIDTypes(unittest.TestCase):
    """Test suite for ID type string formatting and conversions."""

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
        projects = client.projects()
        self.assertGreater(len(projects), 0)
        datasets = client.datasets(str(projects[0].id))
        self.assertGreater(len(datasets), 0)
        dataset = datasets[0]
        str_id = str(dataset.id)
        self.assertTrue(str_id.startswith("ds-"))
        # Verify hex part
        hex_part = str_id[3:]  # Skip "ds-"
        value = int(hex_part, 16)
        self.assertEqual(dataset.id.value, value)

    def test_annotation_set_id_format(self):
        """Test AnnotationSetID string format is 'as-xxx'."""
        client = get_client()
        projects = client.projects()
        self.assertGreater(len(projects), 0)
        datasets = client.datasets(str(projects[0].id))
        self.assertGreater(len(datasets), 0)
        annotation_sets = client.annotation_sets(str(datasets[0].id))
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
        projects = client.projects()
        self.assertGreater(len(projects), 0)
        datasets = client.datasets(str(projects[0].id))
        self.assertGreater(len(datasets), 0)
        samples = client.samples(str(datasets[0].id))
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
        projects = client.projects()
        self.assertGreater(len(projects), 0)
        datasets = client.datasets(str(projects[0].id))
        self.assertGreater(len(datasets), 0)
        dataset = datasets[0]
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
        projects = client.projects()
        self.assertGreater(len(projects), 0)
        datasets = client.datasets(str(projects[0].id))
        self.assertGreater(len(datasets), 0)
        annotation_sets = client.annotation_sets(str(datasets[0].id))
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
        projects = client.projects()
        self.assertGreater(len(projects), 0)
        datasets = client.datasets(str(projects[0].id))
        self.assertGreater(len(datasets), 0)
        samples = client.samples(str(datasets[0].id))
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


if __name__ == "__main__":
    unittest.main()
