# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""
Tests for basic client operations: version, token, and organization.

These tests verify the core client functionality including version checking,
authentication token management, and organization information retrieval.
"""

from time import sleep
from unittest import TestCase

from edgefirst_client import Client
from test import get_client


class ClientTest(TestCase):
    """Test suite for basic Client operations."""

    def test_version(self):
        """Test version() returns non-empty string without authentication."""
        client = Client()
        version = client.version()
        self.assertNotEqual(version, "")

    def test_token(self):
        """Test token retrieval and renewal."""
        client = get_client()
        token = client.token()
        self.assertNotEqual(token, "")
        print(f"Token: {token}")
        print(f"Token Expiration: {client.token_expiration}")
        print(f"Username: {client.username}")
        print(f"Server: {client.url}")

        # Wait for 2 seconds to ensure token renewal updates the time
        sleep(2)

        client.renew_token()
        new_token = client.token()
        self.assertNotEqual(new_token, "")
        self.assertNotEqual(token, new_token)
        print(f"New Token Expiration: {client.token_expiration}")

    def test_organization(self):
        """Test organization() returns complete organization details."""
        client = get_client()
        org = client.organization()
        self.assertIsNotNone(org)
        assert org is not None
        self.assertIsNotNone(org.id)
        assert org.id is not None
        self.assertIsNotNone(org.name)
        assert org.name is not None
        self.assertIsNotNone(org.credits)
        assert org.credits is not None
        print(f"Organization: {org.name}")
        print(f"ID: {org.id.value}")
        print(f"Credits: {org.credits}")

    def test_project_by_id(self):
        """Test project() retrieves a single project by ID."""
        client = get_client()

        # First get all projects to find a valid project ID
        projects = client.projects()
        self.assertGreater(len(projects), 0)
        assert len(projects) > 0

        # Get the first project's ID
        first_project = projects[0]
        self.assertIsNotNone(first_project)
        assert first_project is not None
        self.assertIsNotNone(first_project.id)
        assert first_project.id is not None

        # Now retrieve that same project by ID
        project = client.project(first_project.id)
        self.assertIsNotNone(project)
        assert project is not None
        self.assertEqual(project.id.value, first_project.id.value)
        self.assertEqual(project.name, first_project.name)
        print(f"Retrieved project: {project.name} (ID: {project.id.value})")

    def test_with_url_accepts_https(self):
        """with_url should accept an https:// URL and preserve chaining."""
        client = get_client().with_url("https://test.edgefirst.studio")
        self.assertEqual(client.url, "https://test.edgefirst.studio")

    def test_with_url_rejects_insecure_public_host(self):
        """with_url should reject a plain http:// URL for a non-loopback host."""
        with self.assertRaises(Exception):
            get_client().with_url("http://example.com")

    def test_usage_summary(self):
        """usage_summary should return credits/funds/total as floats."""
        client = get_client()
        summary = client.usage_summary()
        self.assertIsInstance(summary.credits, float)
        self.assertIsInstance(summary.funds, float)
        self.assertIsInstance(summary.total, float)

    def test_download_generic_url(self):
        """download() should fetch raw bytes from an absolute URL."""
        client = get_client()
        # Any small, stable, always-public HTTPS resource served by the
        # Studio test server itself avoids relying on third-party uptime.
        data = client.download("https://test.edgefirst.studio/favicon.ico")
        self.assertIsInstance(data, bytes)
        self.assertGreater(len(data), 0)

    def test_download_rejects_relative_url(self):
        """download() should reject a non-absolute URL with a clear error."""
        client = get_client()
        with self.assertRaises(Exception):
            client.download("not-a-url")
