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

    Returns:
        Client: Authenticated client instance using STUDIO_TOKEN from
            environment.
    """
    return Client(token=environ.get("STUDIO_TOKEN"))


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
