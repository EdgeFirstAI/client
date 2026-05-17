# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""Integration tests for TaskInfo chart methods.

The test class creates its own **user-managed** validation session in
``setUpClass`` and uses that session's backing ``BackgroundTask`` as the
task fixture. The session (and thus the task) is deleted in
``tearDownClass`` — no random live tasks on the test account are ever
mutated, so the chart-mutating tests are safe to run on any developer
machine.

If ``STUDIO_TEST_TASK_ID`` is set we reuse that task instead of creating
one (and skip the teardown delete because we did not create it).

Requires STUDIO_USERNAME and STUDIO_PASSWORD credentials. Skips when
unavailable. Skips with a clear message when the canonical "Unit
Testing" project lacks the entities needed to create a fixture.
"""

import os
import unittest

from edgefirst_client import Parameter, TaskID

from test import (
    cleanup_validation_session,
    get_client,
    make_user_managed_validation_session,
)


class TestTaskCharts(unittest.TestCase):
    """Test suite for TaskInfo chart API operations."""

    _owns_session = False
    _owned_session_id = None

    @classmethod
    def setUpClass(cls):
        """Authenticate and resolve (or create) a task fixture."""
        username = os.environ.get("STUDIO_USERNAME")
        password = os.environ.get("STUDIO_PASSWORD")
        if not username or not password:
            raise unittest.SkipTest(
                "STUDIO_USERNAME and STUDIO_PASSWORD not set; skipping task chart tests"
            )

        cls.client = get_client()
        raw_id = os.environ.get("STUDIO_TEST_TASK_ID")
        if raw_id:
            cls.task_id = TaskID(raw_id)
            cls._owns_session = False
            return

        new_session = make_user_managed_validation_session(
            cls.client, name_suffix="task-charts"
        )
        if new_session is None:
            raise unittest.SkipTest(
                "no task fixture available: the 'Unit Testing' project is "
                "missing a training session with model artifacts. Set "
                "STUDIO_TEST_TASK_ID to reuse an existing task instead."
            )
        cls.task_id = new_session.task_id
        cls._owned_session_id = new_session.session_id
        cls._owns_session = True

    @classmethod
    def tearDownClass(cls):
        """Delete the session we created (if any). Deleting the session
        also removes its backing task, so a single delete cleans up
        both the chart fixture and the data-test fixture."""
        if cls._owns_session and cls._owned_session_id is not None:
            cleanup_validation_session(cls.client, cls._owned_session_id)

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
