#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# Copyright © 2026 Au-Zone Technologies. All Rights Reserved.

"""
Credential-free tests for the dataset group summary, split and bulk
assignment methods of ``Client``.

A local JSON-RPC server stands in for Studio, records every request and
serves them back on ``GET /requests``. The server runs in a child process:
the binding blocks while holding the GIL, so a server thread in this process
could never answer.
"""

import base64
import functools
import json
import shutil
import subprocess
import sys
import tempfile
import unittest
import urllib.request
import uuid
from http.server import BaseHTTPRequestHandler, HTTPServer
from os import environ
from pathlib import Path

import edgefirst_client as ec
from edgefirst_client import Sample, SampleFile
from PIL import Image

from test import delete_and_purge_dataset, get_client, get_test_data_dir


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

RESULTS = {
    "groups.summary": {
        "total": 10,
        "ungrouped": 2,
        "groups": [{"id": 5, "name": "train", "images": 6, "sequences": 3}],
        "mixed_sequences": 1,
    },
    "groups.split": {
        "seed": "abc",
        "scope": "ungrouped",
        "groups": [
            {
                "id": None,
                "name": "train",
                "created": True,
                "target_percent": 70.0,
                "achieved_percent": 70.0,
                "images": 7,
                "sequences": 2,
            }
        ],
        "ungrouped": 0,
        "skipped_mixed_sequences": 0,
    },
    "samples.set_group": {"group_id": None, "group_created": False, "updated": 3},
    "groups.create": [
        {"id": 7, "dataset_id": 1, "name": "alpha"},
        {"id": 8, "dataset_id": 1, "name": "beta"},
    ],
    "groups.update": {"id": 7, "dataset_id": 1, "name": "gamma"},
    "groups.delete": {"group_id": 7, "ungrouped": 4},
}

REQUESTS = []


class _StudioStub(BaseHTTPRequestHandler):
    def _send(self, payload):
        data = json.dumps(payload).encode()
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_GET(self):
        self._send(REQUESTS)

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        method = body.get("method")
        REQUESTS.append({"method": method, "params": body.get("params")})
        if method in RESULTS:
            reply = {"result": RESULTS[method]}
        else:
            reply = {"error": {"code": 101, "message": f"unexpected {method}"}}
        self._send({"jsonrpc": "2.0", "id": "0", **reply})

    def log_message(self, *args):
        pass


def _serve() -> None:
    """Child-process entry: serve the stub and print the bound port."""
    server = HTTPServer(("127.0.0.1", 0), _StudioStub)
    print(server.server_address[1], flush=True)
    server.serve_forever()


class TestGroupsWire(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.server = subprocess.Popen(
            [sys.executable, "-m", "test.test_groups", "--serve"],
            cwd=Path(__file__).resolve().parents[1],
            stdout=subprocess.PIPE,
            text=True,
        )
        cls.port = int(cls.server.stdout.readline())
        cls.client = (
            ec.Client()
            .with_memory_storage()
            .with_token(FAKE_JWT)
            .with_url(f"http://127.0.0.1:{cls.port}")
        )

    @classmethod
    def tearDownClass(cls):
        cls.server.terminate()
        cls.server.wait()
        cls.server.stdout.close()

    def _requests(self):
        url = f"http://127.0.0.1:{self.port}/requests"
        with urllib.request.urlopen(url) as resp:
            return json.load(resp)

    def _last(self, method):
        return [r for r in self._requests() if r["method"] == method][-1]["params"]

    def _count(self):
        return len(self._requests())

    def test_split_groups_dict_order_and_scope(self):
        report = self.client.split_groups(
            "ds-1", {"train": 70, "val": 20.0, "test": 10}, dry_run=True
        )
        params = self._last("groups.split")
        self.assertEqual(
            params["groups"],
            [
                {"name": "train", "percent": 70.0},
                {"name": "val", "percent": 20.0},
                {"name": "test", "percent": 10.0},
            ],
        )
        self.assertEqual(params["scope"], "ungrouped")
        self.assertTrue(params["dry_run"])
        self.assertNotIn("from_groups", params)
        self.assertIsNone(report.groups[0].id)
        self.assertTrue(report.groups[0].created)
        self.assertEqual(report.seed, "abc")

    def test_split_groups_list_of_tuples_with_groups_scope(self):
        self.client.split_groups(
            1,
            [("a", 50.0), ("b", 50.0)],
            scope="groups",
            from_groups=["train"],
            include_ungrouped=False,
            seed="s1",
        )
        params = self._last("groups.split")
        self.assertEqual(params["scope"], "groups")
        self.assertEqual(params["from_groups"], ["train"])
        self.assertIs(params["include_ungrouped"], False)
        self.assertEqual(params["seed"], "s1")

    def test_split_groups_groups_scope_requires_from(self):
        before = self._count()
        with self.assertRaises(ValueError):
            self.client.split_groups("ds-1", {"train": 100.0}, scope="groups")
        with self.assertRaises(ValueError):
            self.client.split_groups(
                "ds-1", {"train": 100.0}, scope="all", from_groups=["x"]
            )
        with self.assertRaises(ValueError):
            self.client.split_groups("ds-1", {"train": 100.0}, scope="bogus")
        self.assertEqual(self._count(), before)

    def test_set_samples_group_unassign(self):
        updated = self.client.set_samples_group("ds-1", None, sequence_uuids=["u1"])
        self.assertEqual(updated, 3)
        params = self._last("samples.set_group")
        self.assertIn("group", params)
        self.assertIsNone(params["group"])
        self.assertEqual(params["sequence_uuids"], ["u1"])

    def test_set_samples_group_with_sample_ids(self):
        self.client.set_samples_group("ds-1", "train", sample_ids=[1, "s-2"])
        params = self._last("samples.set_group")
        self.assertEqual(params["group"], "train")
        self.assertEqual(params["sample_ids"], [1, 2])

    def test_set_samples_group_requires_one_selector(self):
        before = self._count()
        with self.assertRaises(ValueError):
            self.client.set_samples_group("ds-1", "train")
        with self.assertRaises(ValueError):
            self.client.set_samples_group(
                "ds-1", "train", sample_ids=[1], sequence_uuids=["u"]
            )
        self.assertEqual(self._count(), before)

    def test_invalid_inputs_raise_value_error_without_a_request(self):
        before = self._count()
        with self.assertRaises(ValueError):
            self.client.split_groups("ds-1", {"train": -5.0})
        with self.assertRaises(ValueError):
            self.client.split_groups("ds-1", {})
        with self.assertRaises(ValueError):
            self.client.set_samples_group("ds-1", "train", sequence_uuids=["  "])
        self.assertEqual(self._count(), before)

    def test_group_summary_fields(self):
        summary = self.client.group_summary("ds-1")
        self.assertEqual(summary.total, 10)
        self.assertEqual(summary.ungrouped, 2)
        self.assertEqual(summary.mixed_sequences, 1)
        group = summary.groups[0]
        self.assertEqual((group.id, group.name), (5, "train"))
        self.assertEqual((group.images, group.sequences), (6, 3))
        self.assertIn("mixed_sequences", repr(summary))

    def test_create_groups(self):
        groups = self.client.create_groups("ds-1", ["alpha", "beta"])
        params = self._last("groups.create")
        self.assertEqual(params["dataset_id"], 1)
        self.assertEqual(params["group_names"], ["alpha", "beta"])
        self.assertEqual(params["group_splits"], [0.0, 0.0])
        self.assertEqual([(g.id, g.name) for g in groups], [(7, "alpha"), (8, "beta")])

    def test_rename_group_returns_group(self):
        group = self.client.rename_group(7, "gamma")
        params = self._last("groups.update")
        self.assertEqual(params["group_id"], 7)
        self.assertEqual(params["name"], "gamma")
        self.assertEqual((group.id, group.name), (7, "gamma"))

    def test_delete_group_returns_ungrouped_count(self):
        ungrouped = self.client.delete_group(7)
        self.assertEqual(self._last("groups.delete")["group_id"], 7)
        self.assertEqual(ungrouped, 4)


def _skip_on_old_server(test):
    """Skips a test when the server predates the group methods."""

    @functools.wraps(test)
    def wrapper(self):
        try:
            test(self)
        except RuntimeError as err:
            if "is not supported by this Studio server" in str(err):
                self.skipTest(str(err))
            raise

    return wrapper


@unittest.skipUnless(
    environ.get("STUDIO_TOKEN")
    or (environ.get("STUDIO_USERNAME") and environ.get("STUDIO_PASSWORD")),
    "Studio credentials (STUDIO_*) not set",
)
class TestGroupsStudio(unittest.TestCase):
    """Acceptance flow against a Studio server that has the group methods."""

    SEQUENCES = 6
    LOOSE = 6

    @classmethod
    def setUpClass(cls):
        cls.client = get_client()
        project = cls.client.projects("Unit Testing")[0]
        name = f"Test Groups {uuid.uuid4().hex[:8]}"
        cls.dataset_id = cls.client.create_dataset(
            str(project.id), name, "Automated test: group splits"
        )
        try:
            cls._populate()
        except BaseException:
            delete_and_purge_dataset(cls.client, cls.dataset_id)
            raise

    @classmethod
    def tearDownClass(cls):
        delete_and_purge_dataset(cls.client, cls.dataset_id)

    @classmethod
    def _populate(cls):
        annotation_set_id = cls.client.create_annotation_set(
            cls.dataset_id, "Default", "Default annotation set"
        )
        images = Path(tempfile.mkdtemp(prefix="groups_", dir=get_test_data_dir()))
        try:
            samples = []
            index = 0

            def add(image_name, sequence=None, frame=None):
                nonlocal index
                path = images / image_name
                Image.new("RGB", (16, 16), (index % 256, 0, 0)).save(path, "PNG")
                index += 1
                sample = Sample()
                sample.set_image_name(image_name)
                if sequence is not None:
                    sample.set_sequence_name(sequence)
                    sample.set_frame_number(frame)
                sample.add_file(SampleFile("image", str(path)))
                samples.append(sample)

            for seq in range(cls.SEQUENCES):
                for frame in range(3 + seq):
                    add(f"seq{seq:02d}_{frame:03d}.png", f"seq{seq:02d}", frame)
            for i in range(cls.LOOSE):
                add(f"loose_{i:02d}.png")
            cls.total = len(samples)
            cls.client.populate_samples(cls.dataset_id, annotation_set_id, samples)
        finally:
            shutil.rmtree(images, ignore_errors=True)

    def _assignment(self):
        samples = {}
        for s in self.client.samples(self.dataset_id):
            samples[s.image_name] = (s.group, s.sequence_uuid)
        return samples

    @_skip_on_old_server
    def test_split_keeps_sequences_whole_and_is_reproducible(self):
        client = self.client
        targets = {"train": 70, "val": 20, "test": 10}

        before = client.group_summary(self.dataset_id)
        self.assertEqual(before.total, self.total)
        self.assertEqual(before.ungrouped, self.total)

        report = client.split_groups(
            self.dataset_id, targets, seed="py-seed", scope="all"
        )
        self.assertEqual(report.seed, "py-seed")
        self.assertEqual(sum(g.images for g in report.groups), self.total)

        summary = client.group_summary(self.dataset_id)
        self.assertEqual(summary.mixed_sequences, 0)
        self.assertEqual(summary.ungrouped, 0)
        self.assertEqual(sum(g.images for g in summary.groups), self.total)

        first = self._assignment()
        self.assertEqual(len(first), self.total)
        groups_by_sequence = {}
        for group, uuid_ in first.values():
            if uuid_ is not None:
                groups_by_sequence.setdefault(uuid_, set()).add(group)
        self.assertEqual(len(groups_by_sequence), self.SEQUENCES)
        for uuid_, groups in groups_by_sequence.items():
            self.assertEqual(len(groups), 1, f"sequence {uuid_} spans {groups}")

        client.split_groups(self.dataset_id, targets, seed="py-seed", scope="all")
        self.assertEqual(self._assignment(), first)

        # Moving a whole sequence, then unassigning one loose sample.
        moved_uuid = next(u for g, u in first.values() if u is not None and g != "val")
        client.set_samples_group(self.dataset_id, "val", sequence_uuids=[moved_uuid])
        after = self._assignment()
        self.assertTrue(
            all(g == "val" for g, u in after.values() if u == moved_uuid)
        )

        loose = next(
            s for s in client.samples(self.dataset_id) if s.sequence_uuid is None
        )
        client.set_samples_group(self.dataset_id, None, sample_ids=[loose.id])
        self.assertIsNone(self._assignment()[loose.image_name][0])
        self.assertEqual(client.group_summary(self.dataset_id).ungrouped, 1)


if __name__ == "__main__":
    if sys.argv[1:] == ["--serve"]:
        _serve()
    else:
        unittest.main()
