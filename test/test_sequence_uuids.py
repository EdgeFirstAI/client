#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# Copyright © 2026 Au-Zone Technologies. All Rights Reserved.

"""
Credential-free tests for ``sequence_uuid`` and
``Client.assign_sequence_uuids``.

A local JSON-RPC server stands in for Studio and answers ``sequences.list``
for dataset ``ds-1``. The server runs in a child process: the binding blocks
while holding the GIL, so a server thread in this process could never answer.
"""

import base64
import json
import subprocess
import sys
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

# The sequence_uuid derived from dataset ds-1 and the name "seqA".
SEQ_A = "8882ff41-caf7-512e-b999-970290aa1bd1"

DATASET_SEQUENCES = [
    {"id": 7, "name": "seqC", "uuid": "studio-c"},
    {"id": 8, "name": "seqD", "uuid": "studio-d"},
]


class _StudioStub(BaseHTTPRequestHandler):
    """Answers ``sequences.list`` for dataset 1."""

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        method = body.get("method")
        if method == "sequences.list" and body["params"].get("dataset_id") == 1:
            reply = {"result": DATASET_SEQUENCES}
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


def _serve() -> None:
    """Child-process entry: serve the stub and print the bound port."""
    server = HTTPServer(("127.0.0.1", 0), _StudioStub)
    print(server.server_address[1], flush=True)
    server.serve_forever()


def _sample(name=None, uuid=None):
    sample = ec.Sample()
    sample.set_sequence_name(name)
    sample.set_sequence_uuid(uuid)
    return sample


class TestSequenceUuid(unittest.TestCase):
    def test_derived_from_dataset_and_name(self):
        self.assertEqual(ec.sequence_uuid(ec.DatasetID(1), "seqA"), SEQ_A)
        self.assertEqual(ec.sequence_uuid("ds-1", "seqA"), SEQ_A)
        self.assertNotEqual(ec.sequence_uuid("ds-2", "seqA"), SEQ_A)


class TestAssignSequenceUuids(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.server = subprocess.Popen(
            [sys.executable, "-m", "test.test_sequence_uuids", "--serve"],
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

    def test_samples_get_their_sequence_in_the_dataset(self):
        samples = [
            _sample("seqA"),
            _sample("seqC"),
            _sample("seqD", "studio-d"),
            _sample("seqA", "copied-uuid"),
            _sample(),
        ]
        self.client.assign_sequence_uuids("ds-1", samples)
        self.assertEqual(
            [s.sequence_uuid for s in samples],
            [SEQ_A, "studio-c", "studio-d", SEQ_A, None],
        )

    def test_foreign_uuid_without_name_raises(self):
        samples = [_sample(uuid="copied-uuid")]
        with self.assertRaisesRegex(RuntimeError, "copied-uuid"):
            self.client.assign_sequence_uuids(ec.DatasetID(1), samples)
        self.assertEqual(samples[0].sequence_uuid, "copied-uuid")


if __name__ == "__main__":
    if sys.argv[1:] == ["--serve"]:
        _serve()
    else:
        unittest.main()
