# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""Integration tests for TaskInfo chart methods (add_chart, list_charts, get_chart).

These tests verify the TaskInfo chart API functionality including adding charts,
listing charts, and retrieving chart data. Testing these operations requires explicit
task authorization and should only run with STUDIO_TEST_TASK_ID set.

Requires STUDIO_USERNAME and STUDIO_PASSWORD credentials. Skips when unavailable.
"""

import os
import unittest

from edgefirst_client import Parameter, TaskID

from test import get_client


class TestTaskCharts(unittest.TestCase):
    """Test suite for TaskInfo chart API operations."""

    @classmethod
    def setUpClass(cls):
        """Set up authenticated client for task chart tests."""
        username = os.environ.get("STUDIO_USERNAME")
        password = os.environ.get("STUDIO_PASSWORD")

        if not username or not password:
            raise unittest.SkipTest(
                "STUDIO_USERNAME and STUDIO_PASSWORD not set; skipping task chart tests"
            )

        cls.client = get_client()
        raw_id = os.environ.get("STUDIO_TEST_TASK_ID")
        cls.task_id = TaskID(raw_id) if raw_id else None

    @unittest.skipIf(
        not (os.environ.get("STUDIO_USERNAME") and os.environ.get("STUDIO_PASSWORD")),
        "STUDIO_USERNAME and STUDIO_PASSWORD not set; skipping integration tests",
    )
    @unittest.skipIf(
        not os.environ.get("STUDIO_TEST_TASK_ID"),
        "STUDIO_TEST_TASK_ID not set; skipping chart round-trip test",
    )
    def test_add_list_get_chart_round_trip(self):
        """Test add, list, and get of task charts with upsert semantics on (group, name)."""
        task_info = self.client.task_info(self.task_id)

        # A simple line chart matching the server's documented schema.
        body = Parameter.object({
            "type": Parameter.string("line"),
            "title": Parameter.string("DE-2565 smoke chart"),
            "series": Parameter.array([
                Parameter.object({
                    "name": Parameter.string("metric"),
                    "x": Parameter.array([Parameter.integer(1), Parameter.integer(2), Parameter.integer(3)]),
                    "y": Parameter.array([Parameter.real(0.1), Parameter.real(0.2), Parameter.real(0.3)]),
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


if __name__ == "__main__":
    unittest.main()
