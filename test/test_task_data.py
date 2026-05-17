# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""Integration tests for TaskInfo data methods (upload_data, download_data, data_list).

These tests verify the TaskInfo data API functionality including uploading files,
listing data, and downloading files. Operations require explicit task
authorization; when ``STUDIO_TEST_TASK_ID`` is set we use that, otherwise the
suite auto-discovers a task in the ``Unit Testing`` project so it can still
exercise the wrappers (PyO3 coverage) on any developer machine.

Requires STUDIO_USERNAME and STUDIO_PASSWORD credentials. Skips when unavailable.
"""

import os
import tempfile
import unittest
from pathlib import Path

from edgefirst_client import TaskID

from test import get_client


def _autodiscover_task_id(client):
    """Best-effort lookup: pick any task whose ``task_info`` resolves.

    Returns a ``TaskID`` or ``None``. This lets the test exercise the PyO3
    upload_data / download_data / data_list wrappers when no fixture env
    var is configured, instead of silently skipping (which leaves
    py/lib.rs coverage at ~0 for these methods).

    Preference order:
      1. Any task in the ``Unit Testing`` project (canonical fixture home)
      2. Any task whose ``task_info`` succeeds (so PyO3 paths still run
         even on a fresh dev server where the Unit Testing project is
         empty)

    Some entries in the global task listing point at deleted tasks
    server-side; ``task_info`` returns ``RpcError(101, ...)`` for those.
    Swallow those individually so a single dangling row doesn't block
    the whole auto-discovery.
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


class TestTaskData(unittest.TestCase):
    """Test suite for TaskInfo data API operations."""

    @classmethod
    def setUpClass(cls):
        """Set up authenticated client and resolve a task fixture."""
        username = os.environ.get("STUDIO_USERNAME")
        password = os.environ.get("STUDIO_PASSWORD")

        if not username or not password:
            raise unittest.SkipTest(
                "STUDIO_USERNAME and STUDIO_PASSWORD not set; skipping task data tests"
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

    def test_upload_list_download_round_trip(self):
        """Upload → list → download with byte-for-byte verification."""
        task_info = self.client.task_info(self.task_id)

        with tempfile.TemporaryDirectory() as tmp:
            src = Path(tmp) / "hello.txt"
            payload = b"hello DE-2565\n"
            src.write_bytes(payload)

            # Upload file to task data storage. Pass ``src`` as str to confirm
            # the .pyi `Union[str, Path]` typing matches runtime.
            task_info.upload_data(self.client, str(src), folder="de2565-test")

            # List data and verify file appears.
            listing = task_info.data_list(self.client)
            files = listing.data.get("de2565-test", [])
            self.assertIn("hello.txt", files)

            # Exercise listing getters (PyO3 wrapper code).
            self.assertIsInstance(listing.server, str)
            self.assertIsInstance(listing.organization_uid, str)
            self.assertIsInstance(listing.traces, list)
            self.assertIsInstance(listing.data, dict)

            # Download file and verify byte-for-byte match. Pass dst as Path
            # to cover the other arm of Union[str, Path].
            dst = Path(tmp) / "downloaded.txt"
            task_info.download_data(
                self.client, "hello.txt", dst, folder="de2565-test"
            )
            self.assertEqual(dst.read_bytes(), payload)

    def test_data_list_returns_taskdatalist_with_repr(self):
        """`task_info.data_list` returns a TaskDataList; exercise __repr__."""
        task_info = self.client.task_info(self.task_id)
        listing = task_info.data_list(self.client)
        # __repr__ should be non-empty and mention the class name.
        rep = repr(listing)
        self.assertIn("TaskDataList", rep)

    def test_upload_and_download_emit_progress_events(self):
        """Exercise the progress-callback PyO3 bridge for both upload and
        download. The bridge spins up an mpsc channel, a worker thread, and
        re-enters the GIL on each event to invoke the Python callable — none
        of that is hit when ``progress=None`` (the path the other tests
        take), so this test exists purely to cover those wrapper branches.
        """
        task_info = self.client.task_info(self.task_id)

        with tempfile.TemporaryDirectory() as tmp:
            src = Path(tmp) / "progress.txt"
            payload = b"progress callback probe payload"
            src.write_bytes(payload)

            upload_events = []

            def upload_cb(current, total):
                upload_events.append((current, total))

            task_info.upload_data(
                self.client,
                str(src),
                folder="de2565-test",
                progress=upload_cb,
            )

            # At minimum we should have received the terminal completion
            # event (current == total). Intermediate events are
            # best-effort (try_send may drop them under load).
            self.assertTrue(upload_events, "upload progress callback never fired")
            last_cur, last_tot = upload_events[-1]
            self.assertEqual(last_cur, last_tot)
            self.assertEqual(last_tot, len(payload))

            download_events = []

            def download_cb(current, total, status=None):
                # Exercise the 3-arg callback signature path
                download_events.append((current, total, status))

            dst = Path(tmp) / "progress_dl.txt"
            task_info.download_data(
                self.client,
                "progress.txt",
                dst,
                folder="de2565-test",
                progress=download_cb,
            )
            self.assertEqual(dst.read_bytes(), payload)
            self.assertTrue(
                download_events, "download progress callback never fired"
            )


if __name__ == "__main__":
    unittest.main()
