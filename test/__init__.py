# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""Python test utilities for EdgeFirst Client."""

import decimal  # noqa: F401  # Ensure decimal module is pre-loaded for PyO3

# Ensure PNG encoder/decoder registers before tests create artifacts.
from PIL import PngImagePlugin  # noqa: F401

from os import environ
from pathlib import Path

from edgefirst_client import Client


def get_client():
    """
    Create an authenticated EdgeFirst Studio client for testing.

    Supports authentication via:
    - STUDIO_TOKEN environment variable (direct token)
    - STUDIO_USERNAME and STUDIO_PASSWORD environment variables (login)

    The STUDIO_SERVER environment variable can specify the server instance
    (e.g., "test", "stage", "saas"). Defaults to "saas" if not set.

    Returns:
        Client: Authenticated client instance.

    Raises:
        RuntimeError: If no authentication credentials are available.
    """
    token = environ.get("STUDIO_TOKEN")
    username = environ.get("STUDIO_USERNAME")
    password = environ.get("STUDIO_PASSWORD")
    server = environ.get("STUDIO_SERVER")

    if token:
        return Client(token=token)
    elif username and password:
        return Client(username=username, password=password, server=server)
    else:
        raise RuntimeError(
            "No authentication credentials found. Set STUDIO_TOKEN or "
            "STUDIO_USERNAME and STUDIO_PASSWORD environment variables."
        )


def get_test_data_dir():
    """
    Get the test data directory (target/testdata).
    Creates it if it doesn't exist.

    Returns:
        Path: Path to test data directory.
    """
    test_dir = Path(__file__).parent.parent / "target" / "testdata"
    test_dir.mkdir(parents=True, exist_ok=True)
    return test_dir
