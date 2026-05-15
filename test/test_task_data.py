# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""Integration tests for TaskInfo data methods (upload_data, download_data, data_list).

These tests verify the TaskInfo data API functionality including uploading files,
listing data, and downloading files. Testing upload, list, and download operations
requires explicit task authorization and should only run with STUDIO_TEST_TASK_ID set.

Requires STUDIO_USERNAME and STUDIO_PASSWORD credentials. Skips when unavailable.
"""

import os
import tempfile
import unittest
from pathlib import Path

from edgefirst_client import TaskID
from test import get_client


class TestTaskData(unittest.TestCase):
    """Test suite for TaskInfo data API operations."""

    @classmethod
    def setUpClass(cls):
        """Set up authenticated client for task data tests."""
        username = os.environ.get("STUDIO_USERNAME")
        password = os.environ.get("STUDIO_PASSWORD")

        if not username or not password:
            raise unittest.SkipTest(
                "STUDIO_USERNAME and STUDIO_PASSWORD not set; skipping task data tests"
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
        "STUDIO_TEST_TASK_ID not set; skipping data round-trip test",
    )
    def test_upload_list_download_round_trip(self):
        """Test upload, list, and download of task data with byte-for-byte verification."""
        task_info = self.client.task_info(self.task_id)

        with tempfile.TemporaryDirectory() as tmp:
            src = Path(tmp) / "hello.txt"
            payload = b"hello DE-2565\n"
            src.write_bytes(payload)

            # Upload file to task data storage
            task_info.upload_data(self.client, str(src), folder="de2565-test")

            # List data and verify file appears
            listing = task_info.data_list(self.client)
            files = listing.data.get("de2565-test", [])
            self.assertIn("hello.txt", files)

            # Download file and verify byte-for-byte match
            dst = Path(tmp) / "downloaded.txt"
            task_info.download_data(
                self.client, "hello.txt", str(dst), folder="de2565-test"
            )
            self.assertEqual(dst.read_bytes(), payload)


if __name__ == "__main__":
    unittest.main()
