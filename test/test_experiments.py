"""
Experiment integration tests.

Tests training sessions, validation sessions, artifacts, and checkpoints
including metrics, file uploads/downloads, and artifact management.
"""

import tempfile
import unittest
from pathlib import Path

from test import get_client, get_test_data_dir


class TestTrainingSession(unittest.TestCase):
    """Test training session operations."""

    def test_training_session_metrics_and_files(self):
        """Test setting metrics and uploading/downloading files."""
        from edgefirst_client import Parameter

        client = get_client()

        # Find Unit Testing project and experiment
        projects = client.projects("Unit Testing")
        self.assertGreater(
            len(projects),
            0,
            "Unit Testing project should exist")
        project = projects[0]

        experiments = client.experiments(project.id, "Unit Testing")
        self.assertGreater(
            len(experiments),
            0,
            "Unit Testing experiment should exist")
        experiment = experiments[0]

        # Get training sessions
        sessions = client.training_sessions(
            experiment.id, "modelpack-usermanaged")
        self.assertNotEqual(
            len(sessions),
            0,
            "Training sessions should exist")
        session = sessions[0]

        # Set metrics
        metrics = {
            "epochs": Parameter.integer(10),
            "loss": Parameter.real(0.05),
            "model": Parameter.string("modelpack"),
        }
        session.set_metrics(client, metrics)

        # Verify metrics (server may normalize types)
        updated_metrics = session.metrics(client)
        self.assertEqual(len(updated_metrics), 3, "Should have 3 metrics")

        # Server returns epochs as Real, not Integer
        self.assertEqual(
            updated_metrics["epochs"],
            Parameter.real(10))
        self.assertEqual(updated_metrics["loss"], Parameter.real(0.05))

        # Verify model metric preserved as string
        self.assertIn("model", updated_metrics)
        # Use string comparison instead of Parameter comparison
        self.assertEqual(
            updated_metrics["model"],
            "modelpack"
        )

        # Upload file
        with tempfile.NamedTemporaryFile(
                mode='w',
                delete=False,
                suffix='.txt') as f:
            f.write("background")
            labels_path = Path(f.name)

        try:
            session.upload(
                client, [("artifacts/labels.txt", labels_path)])

            # Download and verify
            labels_content = session.download(client, "artifacts/labels.txt")
            self.assertEqual(
                labels_content,
                "background",
                "Downloaded content should match uploaded")
        finally:
            # Clean up
            if labels_path.exists():
                labels_path.unlink()


class TestValidate(unittest.TestCase):
    """Test validation session operations."""

    def test_validation_session(self):
        """Test validation session metrics."""
        from edgefirst_client import Parameter

        client = get_client()

        # Find Unit Testing project
        projects = client.projects("Unit Testing")
        self.assertGreater(
            len(projects),
            0,
            "Unit Testing project should exist")
        project = projects[0]

        # Get validation sessions
        sessions = client.validation_sessions(project.id)

        # Verify we can fetch each session using ID object
        for sess in sessions:
            s = client.validation_session(sess.id)
            self.assertEqual(s.id, sess.id)
            self.assertEqual(s.description, sess.description)
        
        # Verify we can fetch sessions using string ID (user's use case)
        if sessions:
            sess = sessions[0]
            session_id_str = str(sess.id)
            s = client.validation_session(session_id_str)
            self.assertEqual(str(s.id), session_id_str)
            self.assertEqual(s.description, sess.description)

        # Find modelpack-usermanaged session
        session = None
        for s in sessions:
            if s.name == "modelpack-usermanaged":
                session = s
                break
        self.assertIsNotNone(
            session,
            "modelpack-usermanaged validation session should exist")
        assert session is not None

        # Set and verify metrics
        metrics = {"accuracy": Parameter.real(0.95)}
        session.set_metrics(client, metrics)

        retrieved_metrics = session.metrics(client)
        self.assertEqual(
            retrieved_metrics["accuracy"],
            Parameter.real(0.95))


class TestArtifacts(unittest.TestCase):
    """Test artifact download operations."""

    def test_artifacts(self):
        """Test downloading artifacts from training session."""
        client = get_client()

        # Find Unit Testing project and experiment
        projects = client.projects("Unit Testing")
        self.assertGreater(
            len(projects),
            0,
            "Unit Testing project should exist")
        project = projects[0]

        experiments = client.experiments(project.id, "Unit Testing")
        self.assertGreater(
            len(experiments),
            0,
            "Unit Testing experiment should exist")
        experiment = experiments[0]

        # Get training session
        trainers = client.training_sessions(
            experiment.id, "modelpack-960x540")
        self.assertGreater(
            len(trainers),
            0,
            "modelpack-960x540 training session should exist")
        trainer = trainers[0]

        # Get artifacts
        artifacts = client.artifacts(trainer.id)
        self.assertGreater(len(artifacts), 0, "Should have artifacts")

        test_dir = get_test_data_dir()

        # Download each artifact
        for artifact in artifacts:
            output_path = test_dir / artifact.name
            client.download_artifact(
                trainer.id, artifact.name, output_path, None)

            # Verify download
            self.assertTrue(
                output_path.exists(),
                f"Artifact {artifact.name} should be downloaded")

            # Clean up
            if output_path.exists():
                output_path.unlink()

        # Test non-existent artifact
        fake_path = test_dir / "fakefile.txt"
        with self.assertRaises(Exception):
            client.download_artifact(
                trainer.id, "fakefile.txt", fake_path, None)
        self.assertFalse(
            fake_path.exists(),
            "Fake file should not be created")


class TestCheckpoints(unittest.TestCase):
    """Test checkpoint upload/download operations."""

    def test_checkpoints(self):
        """Test uploading and downloading checkpoints."""
        client = get_client()

        # Find Unit Testing project and experiment
        projects = client.projects("Unit Testing")
        self.assertGreater(
            len(projects),
            0,
            "Unit Testing project should exist")
        project = projects[0]

        experiments = client.experiments(project.id, "Unit Testing")
        self.assertGreater(
            len(experiments),
            0,
            "Unit Testing experiment should exist")
        experiment = experiments[0]

        # Get training session
        trainers = client.training_sessions(
            experiment.id, "modelpack-usermanaged")
        self.assertGreater(
            len(trainers),
            0,
            "modelpack-usermanaged training session should exist")
        trainer = trainers[0]

        test_dir = get_test_data_dir()
        checkpoint_path = test_dir / "checkpoint.txt"
        checkpoint2_path = test_dir / "checkpoint2.txt"

        try:
            # Create checkpoint file
            checkpoint_path.write_text("Test Checkpoint")

            # Upload checkpoint
            trainer.upload(
                client,
                [("checkpoints/checkpoint.txt", checkpoint_path)])

            # Download checkpoint
            client.download_checkpoint(
                trainer.id,
                "checkpoint.txt",
                checkpoint2_path,
                None)

            # Verify content
            content = checkpoint2_path.read_text()
            self.assertEqual(
                content,
                "Test Checkpoint",
                "Downloaded checkpoint should match uploaded")

            # Test non-existent checkpoint
            fake_path = test_dir / "fakefile.txt"
            with self.assertRaises(Exception):
                client.download_checkpoint(
                    trainer.id, "fakefile.txt", fake_path, None)
            self.assertFalse(
                fake_path.exists(),
                "Fake file should not be created")

        finally:
            # Clean up
            if checkpoint_path.exists():
                checkpoint_path.unlink()
            if checkpoint2_path.exists():
                checkpoint2_path.unlink()


class TestSchemas(unittest.TestCase):
    """Test trainer and validator schema queries."""

    def test_trainer_schemas_and_schema(self):
        """The trainer catalog is non-empty and each schema parses."""
        client = get_client()

        schemas = client.trainer_schemas()
        self.assertGreater(
            len(schemas),
            0,
            "Server should report at least one trainer type")

        known_types = {
            "group", "slider", "select", "bool", "int", "float", "text",
            "date", "project", "dataset", "trainer", "upload", "info",
            "unknown",
        }

        def check_fields(fields):
            for field in fields:
                if field.field_type is not None:
                    self.assertIn(field.field_type, known_types)
                check_fields(field.children)
                for option in field.options:
                    check_fields(option.children)

        for info in schemas:
            self.assertNotEqual(info.name, "")
            fields = client.trainer_schema(info.schema_type or info.name)
            check_fields(fields)

    def test_validator_schemas(self):
        """Validator schemas parse with inline field descriptors."""
        client = get_client()

        schemas = client.validator_schemas()
        self.assertGreater(
            len(schemas),
            0,
            "Server should report at least one validator schema")
        for schema in schemas:
            self.assertNotEqual(schema.schema_type, "")
            for field in schema.schema:
                # Touch the accessors to prove the descriptors parse.
                _ = (field.name, field.field_type, field.default)


class TestTrainingSessionLifecycle(unittest.TestCase):
    """Test launching, updating, and deleting training sessions."""

    def test_training_session_lifecycle(self):
        """Launch a user-managed session, rename it, and delete it."""
        from test import (
            cleanup_training_session,
            make_user_managed_training_session,
        )

        client = get_client()
        new_session = make_user_managed_training_session(client, "lifecycle")
        if new_session is None:
            self.skipTest("test project lacks training-launch fixtures")
        self.assertIsNotNone(new_session.session_id)
        assert new_session.session_id is not None

        session_id = new_session.session_id
        experiment_id = client.training_session(session_id).experiment_id
        try:
            updated = client.update_training_session(
                session_id,
                name="session-mgmt-renamed",
                description="lifecycle test description")
            self.assertEqual(updated.name, "session-mgmt-renamed")
            self.assertEqual(
                updated.description, "lifecycle test description")

            # Update only the name; the description must be preserved.
            updated = client.update_training_session(
                session_id, name="session-mgmt-renamed-2")
            self.assertEqual(updated.name, "session-mgmt-renamed-2")
            self.assertEqual(
                updated.description, "lifecycle test description")
        finally:
            cleanup_training_session(client, session_id)

        # The delete is a soft delete on the server, so a direct get can
        # still resolve; the session must vanish from listings though.
        remaining = [
            s.id.value for s in client.training_sessions(experiment_id)
        ]
        self.assertNotIn(session_id.value, remaining)


class TestValidationSessionManagement(unittest.TestCase):
    """Test validation session update and independent deletion."""

    def test_update_validation_session(self):
        """Rename a fixture validation session and verify the change."""
        from test import (
            cleanup_validation_session,
            make_user_managed_validation_session,
        )

        client = get_client()
        new_session = make_user_managed_validation_session(client, "update")
        if new_session is None:
            self.skipTest("test project lacks validation fixtures")
        self.assertIsNotNone(new_session.session_id)
        assert new_session.session_id is not None

        session_id = new_session.session_id
        try:
            updated = client.update_validation_session(
                session_id,
                name="session-mgmt-val-renamed",
                description="validation update test")
            self.assertEqual(updated.name, "session-mgmt-val-renamed")
            self.assertEqual(updated.description, "validation update test")
        finally:
            cleanup_validation_session(client, session_id)

    def test_delete_validation_session_keeps_training_session(self):
        """Deleting a validation session must not cascade upward."""
        from test import make_user_managed_validation_session

        client = get_client()
        new_session = make_user_managed_validation_session(
            client, "nocascade")
        if new_session is None:
            self.skipTest("test project lacks validation fixtures")
        self.assertIsNotNone(new_session.session_id)
        assert new_session.session_id is not None

        session = client.validation_session(new_session.session_id)
        training_session_id = session.training_session_id

        client.delete_validation_sessions([new_session.session_id])

        # The parent training session must survive the deletion.
        parent = client.training_session(training_session_id)
        self.assertEqual(parent.id.value, training_session_id.value)


if __name__ == '__main__':
    unittest.main()
