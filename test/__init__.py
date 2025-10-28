# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""
Python test suite for EdgeFirst Client.

This package contains comprehensive tests for the Python bindings of the
EdgeFirst Studio Client library. Tests are organized by functional area:

- test_client.py: Client initialization, authentication, organization API
- test_parameter.py: Parameter class and type conversions
- test_datasets.py: Dataset operations and roundtrip testing
- test_populate.py: Sample population and data loading

The Rust CLI integration tests (crates/edgefirst-cli/tests/cli.rs) provide
comprehensive coverage of:
- Projects API (list, filter, CRUD)
- Datasets API (list, labels, annotation sets)
- Experiments and training sessions
- Validation sessions
- Tasks API
- Upload/download operations
- Login/logout workflows

These Python tests focus on:
- Python-specific functionality (Parameter magic methods, type conversions)
- Integration scenarios (populate_samples, dataset roundtrip)
- Python bindings verification
"""

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
