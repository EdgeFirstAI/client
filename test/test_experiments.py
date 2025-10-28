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

        # Verify we can fetch each session
        for sess in sessions:
            s = client.validation_session(sess.id)
            self.assertEqual(s.id, sess.id)
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


if __name__ == '__main__':
    unittest.main()
