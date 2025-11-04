"""Capture raw samples.list payload for Deer dataset annotations."""

from __future__ import annotations

import base64
import json
import os
import unittest
from typing import Any, Dict, List, Optional, Set, Tuple

import requests

from test import get_test_data_dir


class TestDeerAnnotationsRaw(unittest.TestCase):
    """Fetch raw JSON for Deer dataset annotations via HTTP."""

    DATASET_ID = "ds-c22"
    ANNOTATION_SET_ID = "as-cc7"
    TYPES = ["box2d", "image"]
    MAX_PAGES = 10

    def setUp(self) -> None:
        """Prepare HTTP session and server configuration."""
        token = os.environ.get("STUDIO_TOKEN")
        username = os.environ.get("STUDIO_USERNAME")
        password = os.environ.get("STUDIO_PASSWORD")
        server = os.environ.get("STUDIO_SERVER")

        if token:
            resolved_server = server or self._server_from_token(token)
            if not resolved_server:
                self.skipTest(
                    "Unable to determine server. Provide STUDIO_SERVER or "
                    "use a token with embedded server metadata."
                )
            base_url = self._normalize_server(resolved_server)
        else:
            if not username or not password:
                self.skipTest(
                    "Provide STUDIO_TOKEN or both STUDIO_USERNAME and "
                    "STUDIO_PASSWORD"
                )
            if not server:
                self.skipTest(
                    "STUDIO_SERVER is required when authenticating with "
                    "username/password"
                )
            base_url = self._normalize_server(server)
            token = self._login(base_url, username, password)

        assert token is not None
        self.base_url = base_url
        self.session = requests.Session()
        self.session.headers.update(
            {
                "Authorization": f"Bearer {token}",
                "Accept": "application/json",
                "User-Agent": "edgefirst-client-tests",
            }
        )

    def tearDown(self) -> None:
        """Close HTTP session."""
        if hasattr(self, "session"):
            self.session.close()

    def _rpc(self, method: str, params: Dict[str, Any]) -> Dict[str, Any]:
        """Call Studio JSON-RPC endpoint and return the result payload."""
        payload = {
            "jsonrpc": "2.0",
            "id": 0,
            "method": method,
            "params": params,
        }
        response = self.session.post(
            f"{self.base_url}/api",
            json=payload,
            timeout=120,
        )
        response.raise_for_status()
        body = response.json()
        self.assertNotIn(
            "error",
            body,
            f"RPC error: {body.get('error')}",
        )
        result = body.get("result")
        self.assertIsNotNone(result, "JSON-RPC response missing 'result'")
        assert isinstance(result, dict)
        return result

    @staticmethod
    def _normalize_server(server: str) -> str:
        """Convert a server identifier or URL into a base URL."""
        if server.startswith(("http://", "https://")):
            return server.rstrip("/")
        return f"https://{server}.edgefirst.studio"

    @staticmethod
    def _server_from_token(token: str) -> Optional[str]:
        """Extract server name from Studio authentication token."""
        parts = token.split(".")
        if len(parts) != 3:
            return None
        payload_segment = parts[1]
        padding = (-len(payload_segment)) % 4
        payload_segment += "=" * padding
        try:
            decoded = base64.b64decode(payload_segment)
            payload = json.loads(decoded.decode("utf-8"))
        except ValueError:
            return None
        server = payload.get("database")
        if isinstance(server, str) and server:
            return server
        return None

    def _login(self, base_url: str, username: str, password: str) -> str:
        """Request an authentication token using username/password."""
        login_payload = {
            "jsonrpc": "2.0",
            "id": 0,
            "method": "auth.login",
            "params": {"username": username, "password": password},
        }
        response = requests.post(
            f"{base_url}/api",
            json=login_payload,
            timeout=120,
        )
        response.raise_for_status()
        body = response.json()
        error = body.get("error")
        if error:
            raise AssertionError(f"auth.login failed: {error}")
        result = body.get("result")
        if not isinstance(result, dict):
            raise AssertionError("auth.login response missing 'result'")
        token = result.get("token")
        if not isinstance(token, str) or not token:
            raise AssertionError("auth.login did not return a token")
        return token

    def _collect_samples(self) -> Tuple[List[Dict[str, Any]], Dict[str, Any]]:
        """Fetch paginated samples for the Deer dataset."""
        samples: List[Dict[str, Any]] = []
        first_page: Optional[Dict[str, Any]] = None
        continue_token: Optional[str] = None

        for _ in range(self.MAX_PAGES):
            params: Dict[str, Any] = {
                "dataset_id": self.DATASET_ID,
                "annotation_set_id": self.ANNOTATION_SET_ID,
                "types": self.TYPES,
            }
            if continue_token:
                params["continue_token"] = continue_token

            result = self._rpc("samples.list", params)
            if first_page is None:
                first_page = result

            raw_samples = result.get("samples", [])
            self.assertIsInstance(raw_samples, list)

            page_samples = [
                item for item in raw_samples if isinstance(item, dict)
            ]
            samples.extend(page_samples)

            token_raw = result.get("continue_token")
            continue_token = token_raw if isinstance(token_raw, str) else None
            if not continue_token:
                break
        else:
            self.fail("Exceeded MAX_PAGES while following pagination tokens")

        self.assertGreater(len(samples), 0, "No samples returned from API")
        assert first_page is not None
        return samples, first_page

    @staticmethod
    def _annotations(sample: Dict[str, Any]) -> List[Dict[str, Any]]:
        """Return annotation dictionaries for a sample."""
        annotations = sample.get("annotations", [])
        if not isinstance(annotations, list):
            return []
        return [
            annotation
            for annotation in annotations
            if isinstance(annotation, dict)
        ]

    def _summarize_annotations(
        self, samples: List[Dict[str, Any]]
    ) -> Tuple[str, int, int, Set[str]]:
        """Collect annotation field statistics for captured samples."""
        annotation_keys: Set[str] = set()
        total_annotations = 0
        for sample in samples:
            for annotation in self._annotations(sample):
                total_annotations += 1
                annotation_keys.update(annotation.keys())

        self.assertGreater(
            total_annotations,
            0,
            "Samples returned without any annotations",
        )

        candidate_fields = [
            "object_id",
            "objectId",
            "object_reference",
            "objectReference",
        ]
        field_name: Optional[str] = None
        for candidate in candidate_fields:
            if candidate in annotation_keys:
                field_name = candidate
                break

        self.assertIsNotNone(
            field_name,
            "No object ID-like field found in annotations. Keys: "
            f"{sorted(annotation_keys)}",
        )

        assert field_name is not None
        non_null_count = sum(
            1
            for sample in samples
            for annotation in self._annotations(sample)
            if annotation.get(field_name) not in (None, "")
        )

        return field_name, total_annotations, non_null_count, annotation_keys

    def test_samples_list_object_id_field(self) -> None:
        """Verify annotation payload exposes an object ID field."""
        samples, first_page = self._collect_samples()
        (
            field_name,
            total_annotations,
            non_null_count,
            annotation_keys,
        ) = self._summarize_annotations(samples)

        self.assertEqual(
            field_name,
            "object_id",
            f"Unexpected object identifier field name: {field_name}",
        )

        capture = {
            "request": {
                "dataset_id": self.DATASET_ID,
                "annotation_set_id": self.ANNOTATION_SET_ID,
                "types": self.TYPES,
            },
            "object_id_field": field_name,
            "total_samples": len(samples),
            "total_annotations": total_annotations,
            "annotations_with_object_id": non_null_count,
            "annotation_keys": sorted(annotation_keys),
            "first_page": first_page,
        }

        output_dir = get_test_data_dir() / "deer"
        output_dir.mkdir(parents=True, exist_ok=True)
        output_path = output_dir / "samples_list_raw.json"
        output_path.write_text(
            json.dumps(capture, indent=2, ensure_ascii=True),
            encoding="utf-8",
        )


if __name__ == "__main__":  # pragma: no cover
    unittest.main()
