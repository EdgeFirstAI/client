"""
Advanced integration tests for EdgeFirst client.

Tests complex server interactions including task and stage management.
"""

import unittest

from test import get_client


class TestTasks(unittest.TestCase):
    """Test task and stage operations."""

    def test_tasks(self):
        """Test task listing, status, and stage management."""
        client = get_client()

        # Find Unit Testing project
        projects = client.projects("Unit Testing")
        self.assertGreater(
            len(projects),
            0,
            "Unit Testing project should exist")
        project = projects[0]

        # Get all tasks
        all_tasks = client.tasks(None, None, None, None)

        # Verify we can fetch task info for each
        for task in all_tasks:
            task_info = client.task_info(task.id)
            self.assertIsNotNone(task_info)
            assert task_info is not None

        # Get tasks for specific training session
        tasks = client.tasks("modelpack-usermanaged", None, None, None)

        # Filter to project
        project_tasks = []
        for task in tasks:
            task_info = client.task_info(task.id)
            if task_info.project_id == project.id:
                project_tasks.append(task_info)

        self.assertNotEqual(
            len(project_tasks),
            0,
            "Should have tasks for project")
        task = project_tasks[0]

        # Get task status
        t = client.task_status(task.id, "training")
        self.assertEqual(t.id, task.id)
        self.assertEqual(t.status, "training")

        # Set stages
        stages = [
            ("download", "Downloading Dataset"),
            ("train", "Training Model"),
            ("export", "Exporting Model"),
        ]
        client.set_stages(task.id, stages)

        # Update stage
        client.update_stage(
            task.id,
            "download",
            "running",
            "Downloading dataset",
            50)

        # Verify stage update
        task_updated = client.task_info(task.id)
        self.assertIsNotNone(task_updated.stages)
        assert task_updated.stages is not None


if __name__ == '__main__':
    unittest.main()
