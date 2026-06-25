# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""Shared helpers for EdgeFirst Client Python examples (DE-2762)."""

from os import environ
from typing import Optional

from edgefirst_client import Client, FileTokenStorage

# Public Coffee Cup dataset on SaaS (https://edgefirst.studio/public/datasets/ds-145f/gallery)
COFFEE_CUP_DATASET_ID = "ds-145f"
COFFEE_CUP_GALLERY_URL = "https://edgefirst.studio/public/datasets/ds-145f/gallery"

# "" or "saas" → https://edgefirst.studio
DEFAULT_SERVER = ""

# Writable project for sandbox examples (06, 07)
EXAMPLES_PROJECT_NAME = environ.get("EXAMPLES_PROJECT_NAME", "")


def get_client() -> Client:
    """
    Create an authenticated EdgeFirst Studio client for examples.

    Authentication priority:
    1. STUDIO_TOKEN environment variable
    2. STUDIO_USERNAME + STUDIO_PASSWORD (+ optional STUDIO_SERVER)
    3. CLI-cached token (``edgefirst-client login``) via default Client()
    """
    token = environ.get("STUDIO_TOKEN")
    username = environ.get("STUDIO_USERNAME")
    password = environ.get("STUDIO_PASSWORD")
    server = environ.get("STUDIO_SERVER", DEFAULT_SERVER)

    if token:
        return Client().with_server(server).with_token(token)
    if username and password:
        return Client().with_server(server).with_login(username, password)
    return Client(server=server or None)


def progress_bar(current: int, total: int, pbar) -> None:
    """Update a tqdm progress bar with current progress."""
    if total != pbar.total:
        pbar.reset(total)
    pbar.update(current - pbar.n)


def token_storage_path():
    """Return the platform default CLI token file path."""
    return FileTokenStorage().path


def resolve_project(client: Client, name: Optional[str] = None):
    """Return the first project matching name, or the first project."""
    if name:
        projects = client.projects(name)
        if projects:
            return projects[0]
    projects = client.projects()
    if not projects:
        raise RuntimeError(
            "No projects found. Log in and ensure your account has a project."
        )
    return projects[0]
