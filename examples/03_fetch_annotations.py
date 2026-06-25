# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""
Fetch annotations from Coffee Cup (ds-145f) via the Python API.

CLI equivalent (JSON export):
  edgefirst-client download-annotations <as-id> out.json --groups train,val

This example uses client.samples() for the object model with progress.
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from examples import COFFEE_CUP_DATASET_ID, get_client, progress_bar  # noqa: E402

from edgefirst_client import AnnotationType, FileType  # noqa: E402

try:
    from tqdm import tqdm  # noqa: E402
except ImportError as exc:  # noqa: E402
    raise SystemExit(
        "Missing optional dependency 'tqdm'. Install example deps:\n"
        "    pip install -r examples/requirements.txt"
    ) from exc


def main() -> None:
    client = get_client()
    client.verify_token()

    dataset = client.dataset(COFFEE_CUP_DATASET_ID)
    annotation_sets = client.annotation_sets(dataset.id)
    if not annotation_sets:
        raise RuntimeError("Coffee Cup has no annotation sets")
    annotation_set_id = annotation_sets[0].id
    print(f"Using annotation set: {annotation_set_id}")

    groups = ["train", "val"]
    with tqdm(total=0, desc="Fetching samples") as bar:
        samples = client.samples(
            dataset_id=dataset.id,
            annotation_set_id=annotation_set_id,
            annotation_types=[AnnotationType.Box2d],
            groups=groups,
            types=[FileType.Image],
            progress=lambda c, t: progress_bar(c, t, bar),
        )

    print(f"Fetched {len(samples)} samples from groups {groups}")

    annotated = [s for s in samples if s.annotations]
    print(f"Samples with annotations: {len(annotated)}")

    if annotated:
        sample = annotated[0]
        print(f"\nFirst annotated sample: {sample.name} (group={sample.group})")
        for i, ann in enumerate(sample.annotations[:3]):
            box = ann.box2d
            box_str = (
                f"cx={box.cx:.4f} cy={box.cy:.4f} w={box.width:.4f} h={box.height:.4f}"
                if box
                else "none"
            )
            print(f"  [{i}] label={ann.label!r} box2d: {box_str}")

    # Flat annotations API (alternative)
    with tqdm(total=0, desc="Fetching flat annotations") as bar:
        flat = client.annotations(
            annotation_set_id=annotation_set_id,
            groups=["val"],
            annotation_types=[AnnotationType.Box2d],
            progress=lambda c, t: progress_bar(c, t, bar),
        )
    print(f"\nFlat annotations (val, box2d): {len(flat)} rows")


if __name__ == "__main__":
    main()
