#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# Copyright © 2026 Au-Zone Technologies. All Rights Reserved.

"""
Credential-free tests for ``ValidationSession.set_platform``.

A local JSON-RPC server stands in for Studio so the Python binding's input
handling (dict, str, rejected types) and error mapping run without a live
server. The server runs in a child process: the binding blocks while holding
the GIL, so a server thread in this process could never answer.
"""

import base64
import json
import subprocess
import sys
import tempfile
import unittest
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path

import edgefirst_client as ec

SESSION_ID = 2707


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

VALIDATION_SESSION = {
    "id": SESSION_ID,
    "experiment_id": 1,
    "training_session_id": 2,
    "dataset_id": 3,
    "gt_annotation_set_id": 4,
    "description": "set_platform test session",
    "params": {
        "model_params": {"validation": {}},
        "validate_params": {"model": "test-model"},
    },
    "docker_task": {
        "id": SESSION_ID,
        "name": "test-task",
        "type": "edgefirst-validator:2.10.0",
        "status": "running",
        "manage_type": None,
        "instance_type": "test",
        "date": "2026-10-01T00:00:00Z",
    },
}


class _StudioStub(BaseHTTPRequestHandler):
    """Answers ``validate.session.get`` and ``validate.session.set_platform``."""

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        method = body.get("method")
        if method == "validate.session.get":
            reply = {"result": VALIDATION_SESSION}
        elif method == "validate.session.set_platform":
            with open(self.server.record_path, "a") as record:
                record.write(json.dumps(body["params"]) + "\n")
            platform = body["params"]["platform"]
            if isinstance(platform, dict) and platform.get("schema_version") == 3:
                reply = {
                    "error": {"code": 3, "message": "unsupported schema version: 3"}
                }
            else:
                reply = {
                    "result": {
                        "validate_session_id": "v-a93",
                        "platform_instance_id": 0,
                        "detected_platform": platform,
                    }
                }
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


class TestSetPlatform(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.tmp = tempfile.TemporaryDirectory()
        cls.record = Path(cls.tmp.name) / "set_platform.jsonl"
        cls.server = subprocess.Popen(
            [
                sys.executable,
                "-m",
                "test.test_set_platform",
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
        self.session = self.client.validation_session(SESSION_ID)

    def sent(self):
        """The ``set_platform`` params the stub received since ``setUp``."""
        if not self.record.exists():
            return []
        return [json.loads(line) for line in self.record.read_text().splitlines()]

    def test_dict_is_sent_as_a_json_object(self):
        platform = {
            "schema_version": 2,
            "host": {"hostname": "imx95-frdm"},
            "processor": {"cpu": {"cores": 6}},
            "memory": {"total_gib": 7.5},
            "observed": {"fully_accelerated": False},
        }
        self.session.set_platform(platform)
        self.assertEqual(
            self.sent(),
            [{"validate_session_id": SESSION_ID, "platform": platform}],
        )

    def test_str_is_sent_as_text(self):
        text = "schema_version: 2\nhost:\n  hostname: imx95-frdm\n"
        self.session.set_platform(text)
        self.assertEqual(
            self.sent(),
            [{"validate_session_id": SESSION_ID, "platform": text}],
        )

    def test_other_types_raise_type_error_without_a_request(self):
        for value in (["schema_version", 2], 2, None):
            with self.subTest(value=value):
                with self.assertRaises(TypeError):
                    self.session.set_platform(value)
        self.assertEqual(self.sent(), [])

    def test_unrepresentable_dict_values_raise_without_a_request(self):
        with self.assertRaises(TypeError):
            self.session.set_platform({"captured_at": object()})
        with self.assertRaises(ValueError):
            self.session.set_platform({"memory": {"total_gib": float("nan")}})
        self.assertEqual(self.sent(), [])

    def test_server_rejection_raises_runtime_error(self):
        with self.assertRaisesRegex(RuntimeError, "unsupported schema version: 3"):
            self.session.set_platform({"schema_version": 3})


if __name__ == "__main__":
    if len(sys.argv) == 3 and sys.argv[1] == "--serve":
        _serve(sys.argv[2])
    else:
        unittest.main()
