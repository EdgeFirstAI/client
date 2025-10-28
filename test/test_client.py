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
