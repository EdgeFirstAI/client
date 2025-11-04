# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""
Test fixtures and helper functions for EdgeFirst Client Python tests.

This module provides reusable test utilities to reduce boilerplate and
improve test maintainability.
"""

import time
from pathlib import Path

from PIL import Image, ImageDraw

from edgefirst_client import Annotation, Box2d, Sample, SampleFile


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
) -> Sample:
    """Create sample with circle image and bbox annotation.

    Args:
        image_path: Path to circle image from
            create_test_image_with_circle().
        label_name: Label for the annotation (default: "circle").
        object_id: Object ID for the annotation (default:
            "{label_name}-obj-1").

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

    sample.add_annotation(annotation)
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
