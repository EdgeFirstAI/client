# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""Integration tests for TaskInfo chart methods (add_chart, list_charts, get_chart).

These tests verify the TaskInfo chart API functionality including adding charts,
listing charts, and retrieving chart data. Operations require explicit task
authorization; when ``STUDIO_TEST_TASK_ID`` is set we use that, otherwise the
suite auto-discovers a task in the ``Unit Testing`` project so it can still
exercise the wrappers (PyO3 coverage) on any developer machine.

Requires STUDIO_USERNAME and STUDIO_PASSWORD credentials. Skips when unavailable.
"""

import os
import unittest

from edgefirst_client import Parameter, TaskID

from test import get_client


def _autodiscover_task_id(client):
    """Best-effort lookup of any task whose ``task_info`` resolves.

    Returns a ``TaskID`` or ``None``. Mirrors the helper in
    ``test_task_data.py``: prefer a task in the canonical Unit Testing
    project, otherwise fall back to any resolvable task so the PyO3
    chart wrappers still get exercised on fresh dev servers.
    """
    projects = client.projects("Unit Testing")
    target_project_id = projects[0].id if projects else None

    fallback = None
    for task in client.tasks(None, None, None, None):
        try:
            info = client.task_info(task.id)
        except RuntimeError:
            continue
        if target_project_id is not None and info.project_id == target_project_id:
            return task.id
        if fallback is None:
            fallback = task.id
    return fallback


class TestTaskCharts(unittest.TestCase):
    """Test suite for TaskInfo chart API operations."""

    @classmethod
    def setUpClass(cls):
        """Set up authenticated client and resolve a task fixture."""
        username = os.environ.get("STUDIO_USERNAME")
        password = os.environ.get("STUDIO_PASSWORD")

        if not username or not password:
            raise unittest.SkipTest(
                "STUDIO_USERNAME and STUDIO_PASSWORD not set; skipping task chart tests"
            )

        cls.client = get_client()
        raw_id = os.environ.get("STUDIO_TEST_TASK_ID")
        cls.task_id = TaskID(raw_id) if raw_id else _autodiscover_task_id(cls.client)

    def setUp(self):
        if self.task_id is None:
            self.skipTest(
                "no task fixture available (STUDIO_TEST_TASK_ID unset and no "
                "tasks visible in the 'Unit Testing' project)"
            )

    def test_add_list_get_chart_round_trip(self):
        """Add, list, and get task charts with upsert semantics on (group, name)."""
        task_info = self.client.task_info(self.task_id)

        # A simple line chart matching the server's documented schema.
        body = Parameter.object({
            "type": Parameter.string("line"),
            "title": Parameter.string("DE-2565 smoke chart"),
            "series": Parameter.array([
                Parameter.object({
                    "name": Parameter.string("metric"),
                    "x": Parameter.array(
                        [Parameter.integer(1), Parameter.integer(2), Parameter.integer(3)]
                    ),
                    "y": Parameter.array(
                        [Parameter.real(0.1), Parameter.real(0.2), Parameter.real(0.3)]
                    ),
                }),
            ]),
        })

        # Add (or overwrite if exists)
        task_info.add_chart(self.client, "de2565-test", "smoke", body)

        # List - verify chart appears
        listing = task_info.list_charts(self.client, group="de2565-test")
        files = listing.data.get("de2565-test", [])
        # The server stores charts as JSON files keyed by chart name.
        self.assertTrue(
            any("smoke" in f for f in files),
            f"smoke chart not found in {files}",
        )

        # Get - verify body fetched
        fetched = task_info.get_chart(self.client, "de2565-test", "smoke")
        self.assertIsNotNone(fetched)

    def test_list_charts_without_group_filter(self):
        """Exercise list_charts with group=None (covers the optional-arg arm)."""
        task_info = self.client.task_info(self.task_id)
        listing = task_info.list_charts(self.client)
        # We don't assert on specific content; the call itself exercises the
        # group-None path of the PyO3 wrapper.
        self.assertIsNotNone(listing)

    def test_get_chart_rejects_empty_group_locally(self):
        """validate_chart_args fires client-side before the request."""
        task_info = self.client.task_info(self.task_id)
        with self.assertRaises(RuntimeError):
            task_info.get_chart(self.client, "", "smoke")

    def test_add_chart_rejects_empty_name_locally(self):
        """validate_chart_args also rejects empty chart name."""
        task_info = self.client.task_info(self.task_id)
        with self.assertRaises(RuntimeError):
            task_info.add_chart(
                self.client,
                "de2565-test",
                "",
                Parameter.object({"type": Parameter.string("line")}),
            )


if __name__ == "__main__":
    unittest.main()
