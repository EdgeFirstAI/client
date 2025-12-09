#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""
Comprehensive tests for the token storage API.

Tests FileTokenStorage, MemoryTokenStorage, custom Python storage implementations,
and the Client builder methods for storage configuration.
"""

import os
import tempfile
import time
import unittest
import warnings

import edgefirst_client as ec


class TestFileTokenStorage(unittest.TestCase):
    """Test suite for FileTokenStorage class."""

    def test_default_constructor(self):
        """Test FileTokenStorage() uses default platform path."""
        storage = ec.FileTokenStorage()
        path = storage.path
        self.assertIsNotNone(path)
        self.assertGreater(len(str(path)), 0)
        # Should contain EdgeFirst in path (case-insensitive due to platform differences)
        # macOS: ~/Library/Application Support/ai.EdgeFirst.EdgeFirst-Studio/token
        # Linux: ~/.config/edgefirststudio/token
        # Windows: C:\Users\<User>\AppData\Roaming\EdgeFirst\EdgeFirst Studio\token
        self.assertIn("edgefirst", str(path).lower())

    def test_with_path_constructor(self):
        """Test FileTokenStorage.with_path() uses custom path."""
        with tempfile.TemporaryDirectory() as tmpdir:
            custom_path = os.path.join(tmpdir, "custom_token")
            storage = ec.FileTokenStorage.with_path(custom_path)
            self.assertEqual(str(storage.path), custom_path)

    def test_store_load_clear(self):
        """Test basic store/load/clear operations."""
        with tempfile.TemporaryDirectory() as tmpdir:
            token_path = os.path.join(tmpdir, "token")
            storage = ec.FileTokenStorage.with_path(token_path)

            # Initially empty
            self.assertIsNone(storage.load())

            # Store token
            storage.store("test-token-123")
            self.assertEqual(storage.load(), "test-token-123")

            # Verify file exists
            self.assertTrue(os.path.exists(token_path))

            # Clear token
            storage.clear()
            self.assertIsNone(storage.load())

            # File should be removed
            self.assertFalse(os.path.exists(token_path))

    def test_overwrite(self):
        """Test that storing overwrites previous token."""
        with tempfile.TemporaryDirectory() as tmpdir:
            token_path = os.path.join(tmpdir, "token")
            storage = ec.FileTokenStorage.with_path(token_path)

            storage.store("token-1")
            self.assertEqual(storage.load(), "token-1")

            storage.store("token-2")
            self.assertEqual(storage.load(), "token-2")

    def test_creates_parent_directories(self):
        """Test that storage creates parent directories if needed."""
        with tempfile.TemporaryDirectory() as tmpdir:
            nested_path = os.path.join(tmpdir, "nested", "dirs", "token")
            storage = ec.FileTokenStorage.with_path(nested_path)

            storage.store("nested-token")
            self.assertTrue(os.path.exists(nested_path))
            self.assertEqual(storage.load(), "nested-token")

    def test_clear_nonexistent(self):
        """Test that clearing nonexistent file doesn't error."""
        with tempfile.TemporaryDirectory() as tmpdir:
            token_path = os.path.join(tmpdir, "nonexistent_token")
            storage = ec.FileTokenStorage.with_path(token_path)

            # Should not raise
            storage.clear()

    def test_repr(self):
        """Test __repr__ includes path."""
        with tempfile.TemporaryDirectory() as tmpdir:
            token_path = os.path.join(tmpdir, "token")
            storage = ec.FileTokenStorage.with_path(token_path)
            repr_str = repr(storage)
            self.assertIn("FileTokenStorage", repr_str)
            self.assertIn("token", repr_str)


class TestMemoryTokenStorage(unittest.TestCase):
    """Test suite for MemoryTokenStorage class."""

    def test_constructor(self):
        """Test MemoryTokenStorage() constructor."""
        storage = ec.MemoryTokenStorage()
        self.assertIsNotNone(storage)

    def test_store_load_clear(self):
        """Test basic store/load/clear operations."""
        storage = ec.MemoryTokenStorage()

        # Initially empty
        self.assertIsNone(storage.load())

        # Store token
        storage.store("memory-token-456")
        self.assertEqual(storage.load(), "memory-token-456")

        # Clear token
        storage.clear()
        self.assertIsNone(storage.load())

    def test_overwrite(self):
        """Test that storing overwrites previous token."""
        storage = ec.MemoryTokenStorage()

        storage.store("token-a")
        self.assertEqual(storage.load(), "token-a")

        storage.store("token-b")
        self.assertEqual(storage.load(), "token-b")

    def test_repr(self):
        """Test __repr__ format."""
        storage = ec.MemoryTokenStorage()
        self.assertEqual(repr(storage), "MemoryTokenStorage()")


class TestCustomPythonStorage(unittest.TestCase):
    """Test suite for custom Python storage implementations."""

    def test_custom_storage_with_client(self):
        """Test Client.with_storage() accepts custom Python object."""

        class DictStorage:
            """Simple dict-based storage for testing."""

            def __init__(self):
                self._token = None

            def store(self, token):
                self._token = token

            def load(self):
                return self._token

            def clear(self):
                self._token = None

        storage = DictStorage()
        client = ec.Client().with_storage(storage)
        self.assertIsNotNone(client)

    def test_custom_storage_called_correctly(self):
        """Test that custom storage methods are called correctly."""
        calls = []

        class TracingStorage:
            """Storage that records method calls."""

            def __init__(self):
                self._token = None

            def store(self, token):
                calls.append(("store", token))
                self._token = token

            def load(self):
                calls.append(("load",))
                return self._token

            def clear(self):
                calls.append(("clear",))
                self._token = None

        storage = TracingStorage()

        # with_storage should call load() to check for existing token
        ec.Client().with_storage(storage)
        self.assertIn(("load",), calls)


class TestClientStorageBuilder(unittest.TestCase):
    """Test suite for Client storage builder methods."""

    def test_default_client_has_storage(self):
        """Test that default Client() uses file storage."""
        client = ec.Client()
        # Default URL should be saas
        self.assertEqual(client.url, "https://edgefirst.studio")

    def test_with_memory_storage(self):
        """Test Client().with_memory_storage() returns client with memory storage."""
        client = ec.Client().with_memory_storage()
        self.assertIsNotNone(client)
        self.assertEqual(client.url, "https://edgefirst.studio")

    def test_with_no_storage(self):
        """Test Client().with_no_storage() returns client without storage."""
        client = ec.Client().with_no_storage()
        self.assertIsNotNone(client)

    def test_with_storage_file(self):
        """Test Client().with_storage(FileTokenStorage) works."""
        with tempfile.TemporaryDirectory() as tmpdir:
            token_path = os.path.join(tmpdir, "token")
            storage = ec.FileTokenStorage.with_path(token_path)
            client = ec.Client().with_storage(storage)
            self.assertIsNotNone(client)

    def test_with_storage_memory(self):
        """Test Client().with_storage(MemoryTokenStorage) works."""
        storage = ec.MemoryTokenStorage()
        client = ec.Client().with_storage(storage)
        self.assertIsNotNone(client)

    def test_builder_chaining(self):
        """Test that builder methods can be chained."""
        client = ec.Client().with_memory_storage().with_server("test")
        self.assertEqual(client.url, "https://test.edgefirst.studio")


class TestClientServerBuilder(unittest.TestCase):
    """Test suite for Client.with_server() builder method."""

    def test_with_server_test(self):
        """Test with_server('test') maps to test.edgefirst.studio."""
        client = ec.Client().with_server("test")
        self.assertEqual(client.url, "https://test.edgefirst.studio")

    def test_with_server_stage(self):
        """Test with_server('stage') maps to stage.edgefirst.studio."""
        client = ec.Client().with_server("stage")
        self.assertEqual(client.url, "https://stage.edgefirst.studio")

    def test_with_server_dev(self):
        """Test with_server('dev') maps to dev.edgefirst.studio."""
        client = ec.Client().with_server("dev")
        self.assertEqual(client.url, "https://dev.edgefirst.studio")

    def test_with_server_saas(self):
        """Test with_server('saas') maps to edgefirst.studio."""
        client = ec.Client().with_server("saas")
        self.assertEqual(client.url, "https://edgefirst.studio")

    def test_with_server_empty(self):
        """Test with_server('') maps to edgefirst.studio."""
        client = ec.Client().with_server("")
        self.assertEqual(client.url, "https://edgefirst.studio")

    def test_with_server_custom(self):
        """Test with_server('custom') maps to custom.edgefirst.studio."""
        client = ec.Client().with_server("custom")
        self.assertEqual(client.url, "https://custom.edgefirst.studio")


class TestDeprecationWarnings(unittest.TestCase):
    """Test suite for deprecation warnings on Client constructor parameters."""

    def test_server_parameter_deprecated(self):
        """Test that server= parameter emits deprecation warning."""
        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")
            ec.Client(server="test")
            self.assertEqual(len(w), 1)
            self.assertTrue(issubclass(w[0].category, DeprecationWarning))
            self.assertIn("server=", str(w[0].message))
            self.assertIn("with_server", str(w[0].message))

    def test_use_token_file_false_deprecated(self):
        """Test that use_token_file=False emits deprecation warning."""
        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")
            ec.Client(use_token_file=False)
            self.assertEqual(len(w), 1)
            self.assertTrue(issubclass(w[0].category, DeprecationWarning))
            self.assertIn("use_token_file", str(w[0].message))
            self.assertIn("with_memory_storage", str(w[0].message))

    def test_no_warning_for_default_constructor(self):
        """Test that Client() without arguments emits no warning."""
        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")
            ec.Client()
            # Filter for DeprecationWarnings only (ignore other warnings)
            deprecation_warnings = [
                x for x in w if issubclass(x.category, DeprecationWarning)
            ]
            self.assertEqual(len(deprecation_warnings), 0)

    def test_multiple_deprecated_params_emit_multiple_warnings(self):
        """Test that multiple deprecated params emit multiple warnings."""
        with warnings.catch_warnings(record=True) as w:
            warnings.simplefilter("always")
            ec.Client(server="test", use_token_file=False)
            # Filter for DeprecationWarnings only
            deprecation_warnings = [
                x for x in w if issubclass(x.category, DeprecationWarning)
            ]
            self.assertEqual(len(deprecation_warnings), 2)


class TestStorageIntegration(unittest.TestCase):
    """Integration tests for storage with Client operations."""

    def test_file_storage_persists_across_instances(self):
        """Test that token persists across Client instances with same storage path."""
        with tempfile.TemporaryDirectory() as tmpdir:
            token_path = os.path.join(tmpdir, "token")

            # Create storage and manually store a token
            storage = ec.FileTokenStorage.with_path(token_path)
            storage.store("persistent-token")

            # Create a new storage instance pointing to same path
            storage2 = ec.FileTokenStorage.with_path(token_path)
            self.assertEqual(storage2.load(), "persistent-token")

    def test_memory_storage_independent(self):
        """Test that separate MemoryTokenStorage instances are independent."""
        storage1 = ec.MemoryTokenStorage()
        storage2 = ec.MemoryTokenStorage()

        storage1.store("token-1")
        storage2.store("token-2")

        self.assertEqual(storage1.load(), "token-1")
        self.assertEqual(storage2.load(), "token-2")


class TestStorageServerAuthentication(unittest.TestCase):
    """Integration tests for token storage with actual server authentication.

    These tests require STUDIO_SERVER, STUDIO_USERNAME, and STUDIO_PASSWORD
    environment variables to be set. They verify that tokens stored via
    the storage API can be used to authenticate to the server.
    """

    @classmethod
    def setUpClass(cls):
        """Capture credentials from environment before tests."""
        cls.server = os.environ.get("STUDIO_SERVER")
        cls.username = os.environ.get("STUDIO_USERNAME")
        cls.password = os.environ.get("STUDIO_PASSWORD")

        if not all([cls.server, cls.username, cls.password]):
            raise unittest.SkipTest(
                "STUDIO_SERVER, STUDIO_USERNAME, and STUDIO_PASSWORD must be set"
            )

    def test_file_storage_authentication_roundtrip(self):
        """Test login with credentials, store token, then authenticate with stored token."""
        with tempfile.TemporaryDirectory() as tmpdir:
            token_path = os.path.join(tmpdir, "token")
            storage = ec.FileTokenStorage.with_path(token_path)

            # Login with credentials and store token in file storage
            client1 = (
                ec.Client()
                .with_server(self.server)
                .with_storage(storage)
                .with_login(self.username, self.password)
            )

            # Verify we got a valid token and can access the server
            token1 = client1.token()
            self.assertIsNotNone(token1)
            self.assertNotEqual(token1, "")

            # Verify token was persisted to file
            stored_token = storage.load()
            self.assertEqual(stored_token, token1)

            # Create a NEW client with ONLY the stored token (no credentials)
            # This simulates a fresh application start with persisted token
            storage2 = ec.FileTokenStorage.with_path(token_path)
            client2 = ec.Client().with_server(self.server).with_storage(storage2)

            # Verify the new client can authenticate using the stored token
            token2 = client2.token()
            self.assertEqual(token1, token2)

            # Verify we can make authenticated API calls
            org = client2.organization()
            self.assertIsNotNone(org)
            self.assertIsNotNone(org.name)

    def test_memory_storage_authentication_roundtrip(self):
        """Test login stores token in memory storage and can be used for auth."""
        storage = ec.MemoryTokenStorage()

        # Login with credentials and store in memory storage
        client = (
            ec.Client()
            .with_server(self.server)
            .with_storage(storage)
            .with_login(self.username, self.password)
        )

        # Verify token was stored
        token = client.token()
        self.assertIsNotNone(token)
        stored_token = storage.load()
        self.assertEqual(stored_token, token)

        # Verify we can make authenticated API calls
        org = client.organization()
        self.assertIsNotNone(org)

    def test_custom_python_storage_authentication(self):
        """Test custom Python storage implementation works with authentication."""
        calls = []

        class TracingStorage:
            """Custom storage that traces all operations."""

            def __init__(self):
                self._token = None

            def store(self, token):
                calls.append(("store", token))
                self._token = token

            def load(self):
                calls.append(("load",))
                return self._token

            def clear(self):
                calls.append(("clear",))
                self._token = None

        storage = TracingStorage()

        # Login with custom storage
        client = (
            ec.Client()
            .with_server(self.server)
            .with_storage(storage)
            .with_login(self.username, self.password)
        )

        # Verify store was called with the token
        store_calls = [c for c in calls if c[0] == "store"]
        self.assertGreater(len(store_calls), 0)

        # Verify we can authenticate
        token = client.token()
        self.assertIsNotNone(token)
        self.assertEqual(storage._token, token)

    def test_logout_clears_storage(self):
        """Test that logout clears the stored token."""
        with tempfile.TemporaryDirectory() as tmpdir:
            token_path = os.path.join(tmpdir, "token")
            storage = ec.FileTokenStorage.with_path(token_path)

            # Login and store token
            client = (
                ec.Client()
                .with_server(self.server)
                .with_storage(storage)
                .with_login(self.username, self.password)
            )

            # Verify token is stored
            self.assertIsNotNone(storage.load())
            self.assertTrue(os.path.exists(token_path))

            # Logout should clear storage
            client.logout()

            # Verify token is cleared
            self.assertIsNone(storage.load())
            self.assertFalse(os.path.exists(token_path))

    def test_token_renewal_updates_storage(self):
        """Test that renewing token updates the storage."""
        storage = ec.MemoryTokenStorage()

        # Login with credentials
        client = (
            ec.Client()
            .with_server(self.server)
            .with_storage(storage)
            .with_login(self.username, self.password)
        )

        original_token = storage.load()
        self.assertIsNotNone(original_token)

        # Wait for 2 seconds to ensure token renewal updates the expiration time
        # (JWT expiration is at second granularity)
        time.sleep(2)

        # Renew token
        client.renew_token()

        # Verify storage was updated with new token
        new_token = storage.load()
        self.assertIsNotNone(new_token)
        self.assertNotEqual(original_token, new_token)


if __name__ == "__main__":
    unittest.main()
