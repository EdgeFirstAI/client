# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""
Create annotated samples in an ephemeral sandbox dataset.

Reads Coffee Cup (ds-145f) for reference only. Writes to a temporary dataset
in your project — never mutates the public ds-145f dataset.

CLI upload alternative (after export):
  edgefirst-client upload-dataset <ds-id> --annotations manifest.arrow \\
      --images ./images/ --annotation-set-id <as-id>

Set SKIP_CLEANUP=1 to keep the sandbox dataset for inspection.
"""

import os
import random
import string
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from examples import (  # noqa: E402
    COFFEE_CUP_DATASET_ID,
    EXAMPLES_PROJECT_NAME,
    get_client,
    resolve_project,
)

from edgefirst_client import Annotation, Box2d, Sample, SampleFile  # noqa: E402

try:
    from PIL import Image, ImageDraw  # noqa: E402
except ImportError as exc:  # noqa: E402
    raise SystemExit(
        "Missing optional dependency 'Pillow'. Install example deps:\n"
        "    pip install -r examples/requirements.txt"
    ) from exc


def make_circle_image(path: Path) -> tuple[float, float, float, float, int, int]:
    """Create a 640x480 PNG with a red circle; return norm bbox + dimensions."""
    width, height = 640, 480
    img = Image.new("RGB", (width, height), color="white")
    draw = ImageDraw.Draw(img)
    cx, cy, radius = 150.0, 120.0, 50.0
    draw.ellipse(
        [cx - radius, cy - radius, cx + radius, cy + radius],
        fill="red",
    )
    bbox_x = cx - radius - 5.0
    bbox_y = cy - radius - 5.0
    bbox_w = (radius * 2.0) + 10.0
    bbox_h = (radius * 2.0) + 10.0
    img.save(path, format="PNG")
    return (
        bbox_x / width,
        bbox_y / height,
        bbox_w / width,
        bbox_h / height,
        width,
        height,
    )


def main() -> None:
    client = get_client()
    client.verify_token()

    # Reference read from Coffee Cup (read-only)
    ref = client.dataset(COFFEE_CUP_DATASET_ID)
    ref_labels = client.labels(ref.id)
    print(f"Coffee Cup reference: {ref.name} ({len(ref_labels)} labels)")

    project = resolve_project(client, EXAMPLES_PROJECT_NAME or None)
    print(f"Sandbox project: {project.name} ({project.id})")

    suffix = "".join(random.choices(string.ascii_uppercase + string.digits, k=6))
    dataset_name = f"Example Populate {suffix}"
    dataset_id = client.create_dataset(
        str(project.id),
        dataset_name,
        "DE-2762 examples: populate_samples sandbox",
    )
    annotation_set_id = client.create_annotation_set(
        dataset_id, "Default", "Example annotation set"
    )
    print(f"Created dataset {dataset_id}, annset {annotation_set_id}")

    skip_cleanup = os.getenv("SKIP_CLEANUP", "0") == "1"
    out_dir = Path("target/example_artifacts")
    out_dir.mkdir(parents=True, exist_ok=True)
    timestamp = int(time.time())
    image_path = out_dir / f"example_{timestamp}.png"
    nx, ny, nw, nh, _, _ = make_circle_image(image_path)

    sample = Sample()
    sample.set_image_name(f"example_{timestamp}.png")
    sample.add_file(SampleFile("image", str(image_path)))

    annotation = Annotation()
    annotation.set_label("circle")
    annotation.set_object_id("circle-1")
    annotation.set_box2d(Box2d(nx, ny, nw, nh))
    sample.add_annotation(annotation)

    def progress(current, total):
        print(f"  Upload {current}/{total}")

    results = client.populate_samples(
        dataset_id,
        annotation_set_id,
        [sample],
        progress=progress,
    )
    print(f"populate_samples: {len(results)} result(s), uuid={results[0].uuid}")

    time.sleep(2)
    fetched = client.samples(dataset_id, annotation_set_id)
    names = [s.name for s in fetched]
    assert f"example_{timestamp}" in names or any(
        f"example_{timestamp}" in (s.name or "") for s in fetched
    ), f"Round-trip failed; got names: {names[:5]}"
    print("Round-trip verification OK.")

    if skip_cleanup:
        print("SKIP_CLEANUP=1 — dataset left on server:", dataset_id)
    else:
        client.delete_dataset(dataset_id)
        print("Cleaned up sandbox dataset:", dataset_id)


if __name__ == "__main__":
    main()
