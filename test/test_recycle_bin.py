#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# Copyright © 2026 Au-Zone Technologies. All Rights Reserved.

"""
Credential-free tests for ``Client.recycle_bin`` and
``Client.purge_recycle_bin``.

A local JSON-RPC server stands in for Studio so the Python binding's item
conversion runs without a live server. Purging cannot be undone, so the tests
check that every rejected input fails before any request is sent. The server
runs in a child process: the binding blocks while holding the GIL, so a
server thread in this process could never answer.
"""

import base64
import datetime
import json
import subprocess
import sys
import tempfile
import unittest
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path

import edgefirst_client as ec


def _b64(data: bytes) -> str:
    return base64.b64encode(data).decode().rstrip("=")


# Unsigned JWT with server="test" and a far-future exp, so the client never
# tries to renew it. The client does not verify the signature.
FAKE_JWT = ".".join(
    (
        _b64(b'{"alg":"none","typ":"JWT"}'),
        _b64(b'{"server":"test","exp":2000000000}'),
        _b64(b"sig"),
    )
)

RECYCLE_LIST = {
    "projects": [
        {
            "id": 0x2,
            "name": "Old project",
            "description": "",
            "deleted": "2026-10-01T09:00:00.123456789-06:00",
        }
    ],
    "datasets": [
        {
            "id": 0x1A,
            "name": "Deer copy",
            "description": "",
            "project_id": 0x2,
            "deleted": "2026-10-02T08:00:00Z",
        }
    ],
    "annotation_sets": None,
    "experiments": [],
    "training_sessions": None,
    "automated_tasks": [],
    "validate_sessions": [
        {
            "id": 0x5,
            "description": "val run",
            "dataset_id": 0x1A,
            "project_id": 0x2,
            "deleted": "2026-10-02T07:00:00Z",
            "params": {},
            "results": None,
        }
    ],
}

# The stub rejects a purge of this item, as Studio does for an item that is
# not deleted or that the user cannot write.
REJECTED_ITEM = "ds-bad"


class _StudioStub(BaseHTTPRequestHandler):
    """Answers ``recycle.list`` and ``recycle.purge``."""

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        method = body.get("method")
        if method == "recycle.list":
            reply = {"result": RECYCLE_LIST}
        elif method == "recycle.purge":
            with open(self.server.record_path, "a") as record:
                record.write(json.dumps(body["params"]) + "\n")
            if REJECTED_ITEM in body["params"]["item_ids"]:
                reply = {"error": {"code": 3, "message": "item is not deleted"}}
            else:
                reply = {"result": {}}
        else:
            reply = {"error": {"code": 101, "message": f"unexpected {method}"}}

        payload = json.dumps({"jsonrpc": "2.0", "id": "0", **reply}).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(payload)))
        self.end_headers()
        self.wfile.write(payload)

    def log_message(self, *args):
        pass


def _serve(record_path: str) -> None:
    """Child-process entry: serve the stub and print the bound port."""
    server = HTTPServer(("127.0.0.1", 0), _StudioStub)
    server.record_path = record_path
    print(server.server_address[1], flush=True)
    server.serve_forever()


class TestRecycleBin(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.record = Path(cls.tmp.name) / "purge.jsonl"
        cls.server = subprocess.Popen(
            [
                sys.executable,
                "-m",
                "test.test_recycle_bin",
                "--serve",
                str(cls.record),
            ],
            cwd=Path(__file__).resolve().parents[1],
            stdout=subprocess.PIPE,
            text=True,
        )
        port = int(cls.server.stdout.readline())
        cls.client = (
            ec.Client()
            .with_memory_storage()
            .with_token(FAKE_JWT)
            .with_url(f"http://127.0.0.1:{port}")
        )

    @classmethod
    def tearDownClass(cls):
        cls.server.terminate()
        cls.server.wait()
        cls.server.stdout.close()
        cls.tmp.cleanup()

    def setUp(self):
        self.record.unlink(missing_ok=True)

    def purges(self):
        """The ``recycle.purge`` item lists the stub received since ``setUp``."""
        if not self.record.exists():
            return []
        return [
            json.loads(line)["item_ids"]
            for line in self.record.read_text().splitlines()
        ]

    def test_recycle_bin_lists_items_most_recently_deleted_first(self):
        items = self.client.recycle_bin()
        self.assertEqual(
            [(item.id, item.name) for item in items],
            [("ds-1a", "Deer copy"), ("v-5", "val run"), ("p-2", "Old project")],
        )
        self.assertEqual(
            items[0].deleted,
            datetime.datetime(2026, 10, 2, 8, 0, tzinfo=datetime.timezone.utc),
        )

    def test_listed_item_ids_can_be_purged(self):
        self.client.purge_recycle_bin([item.id for item in self.client.recycle_bin()])
        self.assertEqual(self.purges(), [["ds-1a", "v-5", "p-2"]])

    def test_strings_and_id_objects_are_sent_as_prefixed_ids(self):
        self.client.purge_recycle_bin(
            [
                "exp-3",
                ec.ProjectID(0x2),
                ec.DatasetID(0x1A),
                ec.AnnotationSetID(0x4),
                ec.ExperimentID(0x3),
                ec.TrainingSessionID(0x6),
                ec.ValidationSessionID(0x5),
            ]
        )
        self.assertEqual(
            self.purges(), [["exp-3", "p-2", "ds-1a", "as-4", "exp-3", "t-6", "v-5"]]
        )

    def test_bare_integers_raise_without_a_request(self):
        with self.assertRaisesRegex(RuntimeError, "prefixed ID string"):
            self.client.purge_recycle_bin([0x1A])
        self.assertEqual(self.purges(), [])

    def test_unsupported_objects_raise_without_a_request(self):
        for value in (None, 1.5, ec.OrganizationID(0x1), {"id": "ds-1a"}):
            with self.subTest(value=value):
                with self.assertRaisesRegex(RuntimeError, "prefixed ID string"):
                    self.client.purge_recycle_bin([value])
        self.assertEqual(self.purges(), [])

    def test_unknown_prefixes_raise_without_a_request(self):
        for value in ("org-1", "s-1a", "1a", "", "ds"):
            with self.subTest(value=value):
                with self.assertRaisesRegex(RuntimeError, "not a recycle bin item ID"):
                    self.client.purge_recycle_bin([value])
        self.assertEqual(self.purges(), [])

    def test_malformed_ids_raise_without_a_request(self):
        for value in ("ds-xyz", "p-", "v-1-2"):
            with self.subTest(value=value):
                with self.assertRaises(RuntimeError):
                    self.client.purge_recycle_bin([value])
        self.assertEqual(self.purges(), [])

    def test_one_invalid_item_fails_the_batch_without_a_request(self):
        with self.assertRaises(RuntimeError):
            self.client.purge_recycle_bin(["ds-1a", ec.DatasetID(0x1B), 7, "v-5"])
        self.assertEqual(self.purges(), [])

    def test_empty_list_sends_no_request(self):
        self.client.purge_recycle_bin([])
        self.assertEqual(self.purges(), [])

    def test_server_rejection_raises_runtime_error(self):
        with self.assertRaisesRegex(RuntimeError, "item is not deleted"):
            self.client.purge_recycle_bin(["ds-1a", REJECTED_ITEM])


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "--serve":
        _serve(sys.argv[2])
    else:
        unittest.main()
