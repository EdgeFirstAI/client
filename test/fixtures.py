# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""
Test fixtures and helper functions for EdgeFirst Client Python tests.

This module provides reusable test utilities to reduce boilerplate and
improve test maintainability.
"""

import os
import time
from pathlib import Path
from typing import Optional

from PIL import Image, ImageDraw

from edgefirst_client import Annotation, Box2d, Sample, SampleFile


def get_test_dataset() -> str:
    """Get the test dataset identifier from environment or default to 'Deer'.

    Can be a dataset name (exact match) or dataset ID (ds-xxx format).

    Returns:
        Dataset identifier from TEST_DATASET env var, or "Deer" if not set.
    """
    return os.getenv("TEST_DATASET", "Deer")


def get_test_dataset_types() -> list[str]:
    """Get annotation types to test from environment or default.

    Returns:
        List of annotation types from TEST_DATASET_TYPES env var
        (comma-separated), or ["box2d", "box3d", "mask"] if not set.
    """
    types_str = os.getenv("TEST_DATASET_TYPES", "box2d,box3d,mask")
    return [t.strip() for t in types_str.split(",")]


def create_test_image_with_circle(
    output_path: Path,
    img_width: int = 640,
    img_height: int = 480,
    center_x: float = 150.0,
    center_y: float = 120.0,
    radius: float = 50.0,
) -> tuple:
    """Create test image with red circle.

    Returns (image, normalized_bbox) tuple.

    Args:
        output_path: Path to save the generated PNG image.
        img_width: Image width in pixels.
        img_height: Image height in pixels.
        center_x: Circle center X coordinate (pixels).
        center_y: Circle center Y coordinate (pixels).
        radius: Circle radius (pixels).

    Returns:
        Tuple of (Image, Box2d) where Box2d contains normalized
        coordinates for the bounding box around the circle.
    """
    # Create white background image
    img = Image.new("RGB", (img_width, img_height), color="white")
    draw = ImageDraw.Draw(img)

    # Draw red circle
    draw.ellipse(
        [
            center_x - radius,
            center_y - radius,
            center_x + radius,
            center_y + radius,
        ],
        fill="red",
    )

    # Calculate bounding box with padding
    bbox_x = center_x - radius - 5.0
    bbox_y = center_y - radius - 5.0
    bbox_w = (radius * 2.0) + 10.0
    bbox_h = (radius * 2.0) + 10.0

    # Save to file
    img.save(str(output_path), format="PNG")

    # Normalize coordinates (0.0-1.0 range)
    normalized_bbox = Box2d(
        bbox_x / img_width,
        bbox_y / img_height,
        bbox_w / img_width,
        bbox_h / img_height,
    )

    return img, normalized_bbox


def create_sample_with_circle_annotation(
    image_path: Path,
    label_name: str = "circle",
    object_id=None,
    box2d: Optional[Box2d] = None,
) -> Sample:
    """Create sample with circle image and bbox annotation.

    Args:
        image_path: Path to circle image from
            create_test_image_with_circle().
        label_name: Label for the annotation (default: "circle").
        object_id: Object ID for the annotation (default:
            "{label_name}-obj-1").
        box2d: Normalized bounding box for the annotation, typically the
            second element returned by create_test_image_with_circle().
            Without this, the annotation carries only a label name and no
            geometry (box2d/box3d/mask); the server's samples.populate2
            handler only routes an annotation into the label/annotation
            pipeline when one of those geometry fields is present, so a
            geometry-less annotation is silently dropped server-side and
            never creates or references a label row at all. Pass the
            bbox whenever the test needs the label/annotation to actually
            exist and be queryable (e.g. via labels() or annotations()).

    Returns:
        Sample with image file and annotation.
    """
    if object_id is None:
        object_id = f"{label_name}-obj-1"

    sample = Sample()
    sample.set_image_name(image_path.name)
    sample.add_file(SampleFile("image", str(image_path)))

    annotation = Annotation()
    annotation.set_label(label_name)
    annotation.set_object_id(object_id)
    if box2d is not None:
        annotation.set_box2d(box2d)

    sample.add_annotation(annotation)
    return sample


def create_sample_without_annotation(image_path: Path) -> Sample:
    """Create a sample with an image file and no annotation at all.

    Distinct from calling create_sample_with_circle_annotation(box2d=None):
    that still attaches a geometry-less Annotation (silently dropped
    server-side, per that function's docstring). This helper attaches no
    Annotation whatsoever, for tests that specifically need a genuinely
    unannotated sample (e.g. covering the "delete an unannotated sample"
    path, distinct from "delete an annotated sample").

    Args:
        image_path: Path to an image file, e.g. from
            create_test_image_with_circle().

    Returns:
        Sample with only an image file, no annotations.
    """
    sample = Sample()
    sample.set_image_name(image_path.name)
    sample.add_file(SampleFile("image", str(image_path)))
    return sample


def get_unique_test_name(prefix: str = "test") -> str:
    """Generate a unique test name using timestamp.

    Args:
        prefix: Prefix for the test name (default: "test").

    Returns:
        Unique test name like "test_populate_1698614400".
    """
    timestamp = int(time.time())
    return f"{prefix}_{timestamp}"


def get_unique_image_filename(
    prefix: str = "test",
    extension: str = ".png",
) -> str:
    """Generate a unique image filename using timestamp.

    Args:
        prefix: Prefix for the filename (default: "test").
        extension: File extension (default: ".png").

    Returns:
        Unique filename like "test_populate_1698614400.png".
    """
    timestamp = int(time.time())
    return f"{prefix}_{timestamp}{extension}"


def wait_for_label(client, dataset_id: str, name: str, timeout: float = 5.0):
    """Poll client.labels() until a label with the given name appears.

    Label rows created by an annotation reference (rather than an explicit
    add_label() call) have been observed to resolve synchronously within
    populate_samples() on the current server, provided the annotation
    carries real geometry (box2d/box3d/mask) — see
    create_sample_with_circle_annotation's box2d parameter. The API makes
    no documented guarantee of synchronous visibility, though, so use this
    poll instead of a fixed sleep or an immediate check when a test needs
    to observe label creation triggered by populating annotations.

    Args:
        client: Authenticated client.
        dataset_id: Dataset to check.
        name: Label name to wait for.
        timeout: Maximum seconds to wait.

    Returns:
        The matching Label object.

    Raises:
        TimeoutError: If the label does not appear within timeout.
    """
    deadline = time.time() + timeout
    while time.time() < deadline:
        labels = client.labels(dataset_id)
        for label in labels:
            if label.name == name:
                return label
        time.sleep(0.25)
    raise TimeoutError(
        f"Label '{name}' did not appear on dataset {dataset_id} within {timeout}s"
    )
