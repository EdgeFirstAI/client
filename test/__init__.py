# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""Python test utilities for EdgeFirst Client."""

import decimal  # noqa: F401  # Ensure decimal module is pre-loaded for PyO3
import time
from os import environ
from pathlib import Path

# Ensure PNG encoder/decoder registers before tests create artifacts.
from PIL import PngImagePlugin  # noqa: F401

from edgefirst_client import Client

# Canonical fixture-bearing project on the Studio test server. The
# integration suites scope every read of "real" entities (projects,
# experiments, training sessions, datasets, …) to this project so a
# misconfigured test never reaches into a user's live workspace.
TEST_PROJECT_NAME = environ.get("STUDIO_TEST_PROJECT", "Unit Testing")


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


def make_user_managed_validation_session(client, name_suffix=""):
    """Create a user-managed validation session in the canonical test project.

    Walks the ``Unit Testing`` project (override via
    ``STUDIO_TEST_PROJECT``) for a training session that has the bits
    ``cloud.server.start`` needs (project, training session, dataset,
    annotation set, model artifact). If everything is present, posts a
    ``cloud.server.start`` with ``is_local=True`` — a **user-managed**
    session: the DB row exists and accepts data uploads / downloads /
    metric updates, but no EC2 instance is provisioned and no validator
    pipeline runs. That gives us a real session to exercise the
    ``upload_data`` / ``download_data`` / ``data_list`` wrappers
    against, with the caller responsible for cleanup via
    :py:meth:`Client.delete_validation_sessions`.

    Args:
        client: Authenticated :py:class:`Client`.
        name_suffix: Short tag baked into the session name so logs/UI
            can attribute the session back to its originating test.

    Returns:
        NewValidationSession on success, or ``None`` if any of the
        required fixtures is missing on the server. The caller should
        :py:meth:`unittest.TestCase.skipTest` in that case rather than
        silently fall back to a stranger's data.
    """
    projects = client.projects(TEST_PROJECT_NAME)
    if not projects:
        return None
    project = projects[0]

    training_session = None
    for exp in client.experiments(project.id):
        sessions = client.training_sessions(exp.id)
        if sessions:
            training_session = sessions[0]
            break
    if training_session is None:
        return None

    artifacts = client.artifacts(training_session.id)
    if not artifacts:
        return None
    # Prefer a model-shaped extension; otherwise fall back to the first
    # artifact so we still exercise the wire path.
    preferred = next(
        (a for a in artifacts if a.name.endswith((".pt", ".onnx", ".tflite"))),
        None,
    )
    model_file = (preferred or artifacts[0]).name

    dp = training_session.dataset_params
    suffix = name_suffix or "fixture"
    return client.start_validation_session(
        project_id=project.id,
        name=f"de2565-test-{suffix}-{int(time.time())}",
        training_session_id=training_session.id,
        model_file=model_file,
        val_type="modelpack",
        params={},
        is_local=True,
        dataset_id=dp.dataset_id,
        annotation_set_id=dp.annotation_set_id,
    )


def cleanup_validation_session(client, session_id):
    """Best-effort delete for a fixture session.

    Used in ``tearDownClass`` so a successful test pass doesn't leak
    stranded sessions; swallows errors because cleanup failures should
    never mask a real test failure.
    """
    if session_id is None:
        return
    try:
        client.delete_validation_sessions([session_id])
    except Exception:  # noqa: BLE001
        pass


def make_user_managed_training_session(client, name_suffix=""):
    """Create a user-managed training session in the canonical test project.

    Walks the ``Unit Testing`` project (override via
    ``STUDIO_TEST_PROJECT``) for an experiment and a dataset with an
    annotation set, then posts a ``cloud.server.start`` with
    ``is_local=True`` — a **user-managed** session: the DB row exists
    and accepts artifact uploads / metric updates, but no cloud
    instance is provisioned and no trainer pipeline runs. The caller
    is responsible for cleanup via
    :py:meth:`Client.delete_training_sessions`.

    The launch relies on the dataset having at least one version tag
    (the client resolves the latest tag when none is named).

    Args:
        client: Authenticated :py:class:`Client`.
        name_suffix: Short tag baked into the session name so logs/UI
            can attribute the session back to its originating test.

    Returns:
        NewTrainingSession on success, or ``None`` if any of the
        required fixtures is missing on the server (no project,
        experiment, dataset, annotation set, or dataset tag). The
        caller should :py:meth:`unittest.TestCase.skipTest` in that
        case rather than silently fall back to a stranger's data.
    """
    projects = client.projects(TEST_PROJECT_NAME)
    if not projects:
        return None
    project = projects[0]

    experiments = client.experiments(project.id)
    if not experiments:
        return None

    dataset = None
    annotation_set = None
    for candidate in client.datasets(project.id):
        annotation_sets = client.annotation_sets(candidate.id)
        if annotation_sets:
            dataset = candidate
            annotation_set = annotation_sets[0]
            break
    if dataset is None:
        return None

    suffix = name_suffix or "fixture"
    name = f"session-mgmt-test-{suffix}-{int(time.time())}"

    def launch(tag_name):
        return client.start_training_session(
            project_id=project.id,
            name=name,
            experiment_id=experiments[0].id,
            trainer_type="modelpack",
            dataset_id=dataset.id,
            annotation_set_id=annotation_set.id,
            params={"epochs": 1},
            tag_name=tag_name,
            is_local=True,
        )

    try:
        return launch(None)
    except Exception as err:  # noqa: BLE001
        # A dataset without version tags cannot resolve a latest tag;
        # retry as an untagged run (empty tag), which the server accepts.
        if "tag" not in str(err):
            raise
        return launch("")


def cleanup_training_session(client, session_id):
    """Best-effort delete for a fixture training session.

    Used in ``tearDownClass`` so a successful test pass doesn't leak
    stranded sessions; swallows errors because cleanup failures should
    never mask a real test failure.
    """
    if session_id is None:
        return
    try:
        client.delete_training_sessions([session_id])
    except Exception:  # noqa: BLE001
        pass
