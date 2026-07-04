# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""
Download Coffee Cup images and export YOLO-format labels.

CLI equivalent:
  edgefirst-client download-dataset ds-145f --groups val --types image --output ./images/

This script downloads via the Python API and writes a flat YOLO/Darknet layout:

  <output>/
    images/<group>/<sample>.jpg
    labels/<group>/<sample>.txt   # class cx cy w h (normalized)

Note: this YOLO export *flattens* the dataset. Each sequence's frames are
collapsed into a single images/<group>/ directory, so the per-sequence folder
hierarchy (sequence directories containing their ordered frames) is lost. That
is fine for YOLO/Darknet training. To preserve full dataset fidelity (sequence
hierarchy + rich annotations), use the EdgeFirst Dataset Format instead:
annotations stored as Arrow (`download-annotations`) with images kept in their
native sequence layout. See ../DATASET_FORMAT.md and example 04.
"""

import shutil
import sys
from argparse import ArgumentParser
from pathlib import Path
from typing import Optional

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from examples import (  # noqa: E402
    COFFEE_CUP_DATASET_ID,
    get_client,
    progress_bar,
)

from edgefirst_client import AnnotationType, FileType  # noqa: E402

try:
    from tqdm import tqdm  # noqa: E402
except ImportError as exc:  # noqa: E402
    raise SystemExit(
        "Missing optional dependency 'tqdm'. Install example deps:\n"
        "    pip install -r examples/requirements.txt"
    ) from exc


def find_image_file(base: Path, sample_name: str) -> Optional[Path]:
    """Locate a downloaded image for ``sample_name`` anywhere under ``base``."""
    for ext in (".jpg", ".png", ".jpeg"):
        matches = list(base.rglob(f"{sample_name}{ext}"))
        if matches:
            return matches[0]
    return None


def save_yolo_annotation(annotation_path: Path, annotations) -> None:
    annotation_path.parent.mkdir(parents=True, exist_ok=True)
    with open(annotation_path, "w", encoding="utf-8") as handle:
        for ann in annotations:
            if ann.box2d is not None:
                box = ann.box2d
                handle.write(
                    f"{ann.label_index} {box.cx} {box.cy} {box.width} {box.height}\n"
                )


def download_group(client, dataset_id, annotation_set_id, group: str, output: str):
    """Download one group into a flat YOLO layout: images/<group>/ + labels/<group>/."""
    out = Path(output)
    raw_dir = out / ".raw" / group
    images_dir = out / "images" / group
    labels_dir = out / "labels" / group
    for directory in (raw_dir, images_dir, labels_dir):
        directory.mkdir(parents=True, exist_ok=True)

    # download_dataset lays files out under nested sequence directories; stage
    # them in .raw/, then flatten image + label pairs into the YOLO layout.
    with tqdm(total=0, desc=f"Downloading {group} images") as bar:
        client.download_dataset(
            dataset_id=dataset_id,
            groups=[group],
            types=[FileType.Image],
            output=str(raw_dir),
            progress=lambda c, t: progress_bar(c, t, bar),
        )

    with tqdm(total=0, desc=f"Fetching {group} samples") as bar:
        samples = client.samples(
            dataset_id=dataset_id,
            annotation_set_id=annotation_set_id,
            annotation_types=[AnnotationType.Box2d],
            groups=[group],
            types=[FileType.Image],
            progress=lambda c, t: progress_bar(c, t, bar),
        )

    for sample in tqdm(samples, desc=f"Organizing {group} (YOLO)"):
        image_path = find_image_file(raw_dir, sample.name)
        if image_path:
            shutil.move(
                str(image_path), str(images_dir / f"{sample.name}{image_path.suffix}")
            )
            save_yolo_annotation(labels_dir / f"{sample.name}.txt", sample.annotations)

    shutil.rmtree(out / ".raw", ignore_errors=True)


def main() -> None:
    parser = ArgumentParser(
        description="Download EdgeFirst dataset images and YOLO labels"
    )
    parser.add_argument(
        "--output",
        type=str,
        default="dataset",
        help="Output directory",
    )
    parser.add_argument(
        "--groups",
        type=str,
        default="val",
        help="Comma-separated groups (train,val)",
    )
    parser.add_argument(
        "--dataset",
        type=str,
        default=COFFEE_CUP_DATASET_ID,
        help="Dataset ID (default: Coffee Cup ds-145f)",
    )
    args = parser.parse_args()

    client = get_client()
    client.verify_token()

    dataset = client.dataset(args.dataset)
    annotation_sets = client.annotation_sets(dataset.id)
    if not annotation_sets:
        raise RuntimeError(f"No annotation sets for {args.dataset}")
    annotation_set_id = annotation_sets[0].id

    for group in args.groups.split(","):
        group = group.strip()
        if group:
            download_group(client, dataset.id, annotation_set_id, group, args.output)

    print(f"Done. Output: {args.output}")


if __name__ == "__main__":
    main()
