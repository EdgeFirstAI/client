# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""Integration tests for ValidationSession data methods (upload_data, download_data, data_list).

These tests verify the ValidationSession data API functionality including uploading files,
listing data, and downloading files. Testing upload, list, and download operations requires
explicit session authorization and should only run with STUDIO_TEST_VALIDATION_SESSION_ID set.

Requires STUDIO_USERNAME and STUDIO_PASSWORD credentials. Skips when unavailable.
"""

import os
import tempfile
import unittest
from pathlib import Path

from test import get_client


class TestValData(unittest.TestCase):
    """Test suite for ValidationSession data API operations."""

    @classmethod
    def setUpClass(cls):
        """Set up authenticated client for validation data tests."""
        username = os.environ.get("STUDIO_USERNAME")
        password = os.environ.get("STUDIO_PASSWORD")

        if not username or not password:
            raise unittest.SkipTest(
                "STUDIO_USERNAME and STUDIO_PASSWORD not set; "
                "skipping validation data tests"
            )

        cls.client = get_client()
        cls.session_id = os.environ.get("STUDIO_TEST_VALIDATION_SESSION_ID")

    @unittest.skipIf(
        not (os.environ.get("STUDIO_USERNAME") and os.environ.get("STUDIO_PASSWORD")),
        "STUDIO_USERNAME and STUDIO_PASSWORD not set; skipping integration tests",
    )
    @unittest.skipIf(
        not os.environ.get("STUDIO_TEST_VALIDATION_SESSION_ID"),
        "STUDIO_TEST_VALIDATION_SESSION_ID not set; skipping data round-trip test",
    )
    def test_upload_list_download_round_trip(self):
        """Test upload, list, and download of validation data with byte-for-byte verification."""
        session = self.client.validation_session(self.session_id)

        with tempfile.TemporaryDirectory() as tmp:
            src = Path(tmp) / "result.txt"
            payload = b"de2565 val data smoke\n"
            src.write_bytes(payload)

            # Upload file to validation data storage
            session.upload_data(self.client, [("result.txt", str(src))],
                                folder="de2565-test")

            # List data and verify file appears.
            # data_list returns a flat list of relative paths, e.g.
            # ["de2565-test/result.txt", ...]
            listing = session.data_list(self.client)
            files = [
                p.split("/", 1)[1]
                for p in listing
                if p.startswith("de2565-test/")
            ]
            self.assertIn("result.txt", files)

            # Download file and verify byte-for-byte match
            dst = Path(tmp) / "downloaded.txt"
            session.download_data(self.client, "de2565-test/result.txt", str(dst))
            self.assertEqual(dst.read_bytes(), payload)


if __name__ == "__main__":
    unittest.main()
