# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""Integration tests for TaskInfo data methods.

The test class creates its own **user-managed** validation session in
``setUpClass`` and uses that session's backing ``BackgroundTask`` as the
task fixture. The session (and thus the task) is deleted in
``tearDownClass`` — no random live tasks on the test account are ever
mutated, so the suite is safe to run on any developer machine.

If ``STUDIO_TEST_TASK_ID`` is set we reuse that task instead of creating
one (and skip the teardown delete because we did not create it).

Requires STUDIO_USERNAME and STUDIO_PASSWORD credentials. Skips when
unavailable. Skips with a clear message when the canonical "Unit
Testing" project lacks the entities needed to create a fixture.
"""

import os
import tempfile
import unittest
from pathlib import Path

from edgefirst_client import TaskID

from test import (
    cleanup_validation_session,
    get_client,
    make_user_managed_validation_session,
)


class TestTaskData(unittest.TestCase):
    """Test suite for TaskInfo data API operations."""

    # When True, ``tearDownClass`` deletes ``_owned_session_id`` (the
    # backing validation session). False when an externally-provided
    # task id is reused — we then do not own its lifecycle.
    _owns_session = False
    _owned_session_id = None

    @classmethod
    def setUpClass(cls):
        """Authenticate and resolve (or create) a task fixture."""
        username = os.environ.get("STUDIO_USERNAME")
        password = os.environ.get("STUDIO_PASSWORD")
        if not username or not password:
            raise unittest.SkipTest(
                "STUDIO_USERNAME and STUDIO_PASSWORD not set; skipping task data tests"
            )

        cls.client = get_client()
        raw_id = os.environ.get("STUDIO_TEST_TASK_ID")
        if raw_id:
            cls.task_id = TaskID(raw_id)
            cls._owns_session = False
            return

        new_session = make_user_managed_validation_session(
            cls.client, name_suffix="task-data"
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
        also removes its backing task, so we don't need a separate task
        cleanup call."""
        if cls._owns_session and cls._owned_session_id is not None:
            cleanup_validation_session(cls.client, cls._owned_session_id)

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
