# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""Integration tests for ValidationSession data methods.

Each test class creates its own **user-managed** validation session in
``setUpClass`` (via :py:meth:`Client.start_validation_session` with
``is_local=True``) and deletes it in ``tearDownClass``. The session is
fully usable for the upload / list / download wrappers under test, but
no EC2 instance is provisioned and no validator pipeline runs, so the
suite has no side effects beyond the session row it creates and tears
down.

If ``STUDIO_TEST_VALIDATION_SESSION_ID`` is set the tests reuse that
existing session instead of creating one (and skip the teardown delete
to avoid removing a fixture they did not own).

Requires STUDIO_USERNAME and STUDIO_PASSWORD credentials. Skips when
unavailable. Skips with a clear message when the canonical "Unit
Testing" project lacks the entities needed to create a fixture.
"""

import os
import tempfile
import unittest
from pathlib import Path

from test import (
    cleanup_validation_session,
    get_client,
    make_user_managed_validation_session,
)


class TestValData(unittest.TestCase):
    """Test suite for ValidationSession data API operations."""

    # When True, ``tearDownClass`` deletes ``session_id``. False when an
    # externally-provided fixture is reused (we did not create it, so we
    # do not own its lifecycle).
    _owns_session = False

    @classmethod
    def setUpClass(cls):
        """Authenticate and resolve (or create) a session fixture."""
        username = os.environ.get("STUDIO_USERNAME")
        password = os.environ.get("STUDIO_PASSWORD")
        if not username or not password:
            raise unittest.SkipTest(
                "STUDIO_USERNAME and STUDIO_PASSWORD not set; "
                "skipping validation data tests"
            )

        cls.client = get_client()
        raw_id = os.environ.get("STUDIO_TEST_VALIDATION_SESSION_ID")
        if raw_id:
            cls.session_id = raw_id
            cls._owns_session = False
            return

        new_session = make_user_managed_validation_session(
            cls.client, name_suffix="val-data"
        )
        if new_session is None or new_session.session_id is None:
            raise unittest.SkipTest(
                "no validation-session fixture available: the 'Unit Testing' "
                "project is missing a training session with model artifacts. "
                "Set STUDIO_TEST_VALIDATION_SESSION_ID to reuse an existing "
                "session instead."
            )
        cls.session_id = new_session.session_id
        cls._owns_session = True

    @classmethod
    def tearDownClass(cls):
        """Delete the session we created (if any)."""
        if cls._owns_session and getattr(cls, "session_id", None) is not None:
            cleanup_validation_session(cls.client, cls.session_id)

    def test_upload_list_download_round_trip(self):
        """Upload → list → download with byte-for-byte verification."""
        session = self.client.validation_session(self.session_id)

        with tempfile.TemporaryDirectory() as tmp:
            src = Path(tmp) / "result.txt"
            payload = b"de2565 val data smoke\n"
            src.write_bytes(payload)

            # Upload using a (filename, str-path) tuple — verifies the
            # str-arm of the .pyi `Tuple[str, Union[str, Path]]` typing.
            session.upload_data(
                self.client,
                [("result.txt", str(src))],
                folder="de2565-test",
            )

            # data_list returns a flat list of relative paths, e.g.
            # ["de2565-test/result.txt", ...]
            listing = session.data_list(self.client)
            files = [
                p.split("/", 1)[1]
                for p in listing
                if p.startswith("de2565-test/")
            ]
            self.assertIn("result.txt", files)

            # Download to a Path (verifies the Path-arm of Union[str, Path]).
            dst = Path(tmp) / "downloaded.txt"
            session.download_data(self.client, "de2565-test/result.txt", dst)
            self.assertEqual(dst.read_bytes(), payload)

    def test_upload_with_path_arm(self):
        """Also exercise the Path-arm of the (filename, path) tuple typing."""
        session = self.client.validation_session(self.session_id)
        with tempfile.TemporaryDirectory() as tmp:
            src = Path(tmp) / "path_arm.txt"
            src.write_bytes(b"path arm payload")
            # Pass src as Path (not str)
            session.upload_data(
                self.client,
                [("path_arm.txt", src)],
                folder="de2565-test",
            )

    def test_upload_and_download_emit_progress_events(self):
        """Cover the progress-callback PyO3 bridge for ValidationSession.

        Mirrors the equivalent test in ``test_task_data`` — the wrapper
        bridge code (mpsc channel + worker thread + re-entering the GIL
        per event) is only exercised when ``progress`` is non-None.
        """
        session = self.client.validation_session(self.session_id)

        with tempfile.TemporaryDirectory() as tmp:
            src = Path(tmp) / "progress.txt"
            payload = b"validation progress probe payload"
            src.write_bytes(payload)

            upload_events = []

            def upload_cb(current, total):
                upload_events.append((current, total))

            session.upload_data(
                self.client,
                [("progress.txt", str(src))],
                folder="de2565-test",
                progress=upload_cb,
            )
            self.assertTrue(upload_events, "upload progress callback never fired")
            last_cur, last_tot = upload_events[-1]
            self.assertEqual(last_cur, last_tot)
            self.assertEqual(last_tot, len(payload))

            download_events = []

            def download_cb(current, total, status=None):
                download_events.append((current, total, status))

            dst = Path(tmp) / "progress_dl.txt"
            session.download_data(
                self.client,
                "de2565-test/progress.txt",
                dst,
                progress=download_cb,
            )
            self.assertEqual(dst.read_bytes(), payload)
            self.assertTrue(
                download_events, "download progress callback never fired"
            )


if __name__ == "__main__":
    unittest.main()
