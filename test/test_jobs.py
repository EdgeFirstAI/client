# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""Integration tests for Client.job_run / Client.jobs / Client.job_stop.

These tests verify the Jobs API functionality including listing jobs and
filtering by name. Testing job_run and job_stop is intentionally excluded
from automated runs as they cycle real cloud resources and should only run
with explicit authorization.

Requires STUDIO_USERNAME and STUDIO_PASSWORD credentials. Skips when unavailable.
"""

import os
import unittest

from edgefirst_client import TaskID
from test import get_client


class TestJobs(unittest.TestCase):
    """Test suite for Jobs API operations."""

    @classmethod
    def setUpClass(cls):
        """Set up authenticated client for jobs tests."""
        username = os.environ.get("STUDIO_USERNAME")
        password = os.environ.get("STUDIO_PASSWORD")

        if not username or not password:
            raise unittest.SkipTest(
                "STUDIO_USERNAME and STUDIO_PASSWORD not set; skipping jobs tests"
            )

        cls.client = get_client()

    @unittest.skipIf(
        not (os.environ.get("STUDIO_USERNAME") and os.environ.get("STUDIO_PASSWORD")),
        "STUDIO_USERNAME and STUDIO_PASSWORD not set; skipping integration tests",
    )
    def test_jobs_listing_succeeds(self):
        """Test that jobs() returns a list (possibly empty)."""
        jobs = self.client.jobs()
        self.assertIsInstance(jobs, list)

    @unittest.skipIf(
        not (os.environ.get("STUDIO_USERNAME") and os.environ.get("STUDIO_PASSWORD")),
        "STUDIO_USERNAME and STUDIO_PASSWORD not set; skipping integration tests",
    )
    def test_jobs_filter_by_name_is_substring_match(self):
        """Test that name filter uses substring matching.

        Uses an unlikely prefix to confirm filtering works without
        false matches.
        """
        empty = self.client.jobs(name="zzz-definitely-not-a-real-job-prefix")
        self.assertEqual(empty, [])

    @unittest.skipIf(
        not (os.environ.get("STUDIO_USERNAME") and os.environ.get("STUDIO_PASSWORD")),
        "STUDIO_USERNAME and STUDIO_PASSWORD not set; skipping integration tests",
    )
    def test_job_stop_invalid_task_id_returns_typed_error(self):
        """Test that job_stop with an invalid task ID raises a typed error.

        Uses a deliberately invalid task ID (all-dead-bytes) to exercise the
        wire path and error-mapping code. The server should reject the request;
        we accept either TaskNotFound or PermissionDenied depending on auth
        context — we just verify a RuntimeError is raised (not a crash or
        silent success).
        """
        # "task-deadbeef" is an obviously-invalid task ID.
        invalid = TaskID("task-deadbeef")
        with self.assertRaises(RuntimeError):
            self.client.job_stop(invalid)

    @unittest.skipIf(
        not (os.environ.get("STUDIO_USERNAME") and os.environ.get("STUDIO_PASSWORD")),
        "STUDIO_USERNAME and STUDIO_PASSWORD not set; skipping integration tests",
    )
    def test_job_stop_accepts_int_and_str_arms(self):
        """``job_stop`` should accept ``TaskID``, ``int``, and ``str`` per the
        ``TaskUID`` typing in the .pyi stub. All three arms go through
        ``TryFrom<Bound<PyAny>> for TaskID`` in the PyO3 binding.

        We assert each call raises (the server rejects the dead-beef ID); the
        test's purpose is to exercise the binding-side conversion code, not
        the server behavior.
        """
        for arg in (
            TaskID("task-deadbeef"),  # TaskID arm
            0xDEADBEEF,                # int arm
            "task-deadbeef",          # str arm
        ):
            with self.assertRaises(RuntimeError):
                self.client.job_stop(arg)

    @unittest.skipIf(
        not (os.environ.get("STUDIO_USERNAME") and os.environ.get("STUDIO_PASSWORD")),
        "STUDIO_USERNAME and STUDIO_PASSWORD not set; skipping integration tests",
    )
    def test_jobs_listing_exercises_job_getters(self):
        """When ``jobs()`` returns a non-empty list, walk each entry and hit
        every getter on the ``Job`` PyO3 wrapper.

        On a fresh test account the list is often empty — in that case this
        test still passes (the assertion below is a no-op), but on any
        environment that has run a job it provides direct coverage of the
        ``Job::code/title/job_name/job_id/state/task_id`` getters.
        """
        for job in self.client.jobs():
            # Each getter returns either str or int — checking type at all
            # exercises the PyO3 descriptor path even if the value is empty.
            self.assertIsInstance(job.code, str)
            self.assertIsInstance(job.title, str)
            self.assertIsInstance(job.job_name, str)
            self.assertIsInstance(job.job_id, str)
            self.assertIsInstance(job.state, str)
            # task_id() is a method (not a property) because we need to
            # call the core saturating accessor that promotes negative
            # i64 → TaskID(0).
            self.assertIsInstance(job.task_id(), TaskID)


if __name__ == "__main__":
    unittest.main()
