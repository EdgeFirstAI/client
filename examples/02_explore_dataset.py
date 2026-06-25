# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""
Explore the Coffee Cup public dataset (ds-145f).

CLI equivalent:
  edgefirst-client dataset ds-145f --annotation-sets --labels --groups

Gallery: https://edgefirst.studio/public/datasets/ds-145f/gallery
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from examples import COFFEE_CUP_DATASET_ID, COFFEE_CUP_GALLERY_URL, get_client  # noqa: E402

from edgefirst_client import AnnotationType, FileType  # noqa: E402


def main() -> None:
    client = get_client()
    client.verify_token()

    dataset = client.dataset(COFFEE_CUP_DATASET_ID)
    print(f"Dataset: {dataset.name} ({dataset.id})")
    print(f"Gallery: {COFFEE_CUP_GALLERY_URL}")
    print()

    annotation_sets = client.annotation_sets(dataset.id)
    print(f"Annotation sets ({len(annotation_sets)}):")
    for annset in annotation_sets:
        print(f"  [{annset.id}] {annset.name}")
    print()

    labels = client.labels(dataset.id)
    print(f"Labels ({len(labels)}):")
    for label in labels[:10]:
        print(f"  index={label.index} name={label.name!r}")
    if len(labels) > 10:
        print(f"  … and {len(labels) - 10} more")
    print()

    groups = client.groups(dataset.id)
    print(f"Groups ({len(groups)}):")
    for group in groups:
        print(f"  {group.name} (id={group.id})")
    print()

    if annotation_sets:
        count = client.samples_count(
            dataset.id,
            annotation_sets[0].id,
            annotation_types=[AnnotationType.Box2d],
            groups=[],
            types=[FileType.Image],
        )
        print(f"Sample count (box2d, default annset): {count.total}")


if __name__ == "__main__":
    main()
