# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""Integration tests for ValidationSession data methods (upload_data, download_data, data_list).

These tests verify the ValidationSession data API functionality including
uploading files, listing data, and downloading files. Operations require
explicit session authorization; when
``STUDIO_TEST_VALIDATION_SESSION_ID`` is set we use that, otherwise the
suite auto-discovers a validation session in the ``Unit Testing``
project so it can still exercise the PyO3 wrappers on any developer
machine.

Requires STUDIO_USERNAME and STUDIO_PASSWORD credentials. Skips when unavailable.
"""

import os
import tempfile
import unittest
from pathlib import Path

from test import get_client


def _autodiscover_validation_session(client):
    """Best-effort lookup of any validation session the user can see.

    Returns a ``ValidationSessionID`` or ``None``. Prefer the canonical
    Unit Testing project; otherwise sweep every visible project and
    return the first session found. Lets the test exercise the PyO3
    upload_data / download_data / data_list wrappers without requiring
    developer-side env var setup.
    """

    def first_session(project_id):
        try:
            sessions = client.validation_sessions(project_id)
        except RuntimeError:
            return None
        return sessions[0].id if sessions else None

    # Preference: canonical project.
    projects = client.projects("Unit Testing")
    if projects:
        sid = first_session(projects[0].id)
        if sid is not None:
            return sid

    # Fallback: sweep all visible projects until we find one with a session.
    for project in client.projects(None):
        sid = first_session(project.id)
        if sid is not None:
            return sid

    return None


class TestValData(unittest.TestCase):
    """Test suite for ValidationSession data API operations."""

    @classmethod
    def setUpClass(cls):
        """Set up authenticated client and resolve a session fixture."""
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
        else:
            cls.session_id = _autodiscover_validation_session(cls.client)

    def setUp(self):
        if self.session_id is None:
            self.skipTest(
                "no validation-session fixture available "
                "(STUDIO_TEST_VALIDATION_SESSION_ID unset and no sessions "
                "visible in the 'Unit Testing' project)"
            )

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
