# SPDX-License-Identifier: Apache-2.0
# Copyright © 2026 Au-Zone Technologies. All Rights Reserved.

"""Convert COCO-Stuff semantic segmentation annotations to an offline EdgeFirst dataset.

COCO-Stuff stores one annotation per class per image, each a compressed RLE
region (with ``iscrowd`` 0). By default the regions are stored losslessly as
1-bit masks in the ``mask`` column, one row per class region with its
``label``. ``--rle-to-polygon`` traces them to contours in the ``polygon``
column instead, which is lossy.

Example:
  python examples/10_coco_stuff_conversion.py \
      ~/Datasets/COCO/annotations/stuff_val2017.json \
      --output stuff-val/stuff-val.arrow --group val --images ~/Datasets/COCO/val2017

Upload the result with ``edgefirst-client upload-dataset``. EdgeFirst Studio
stores segmentation only as polygons, so the upload traces each labelled
1-bit mask to a polygon and prints a warning; the Arrow file keeps the
original masks. No EdgeFirst Studio account or network connection is needed
for the conversion itself.
"""

import argparse
from pathlib import Path

import edgefirst_client as ec
import polars as pl
from _path import normalize_user_path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "coco_path", type=Path, help="COCO-Stuff annotation JSON, ZIP or directory"
    )
    parser.add_argument(
        "--output", "-o", type=Path, required=True, help=".arrow or .parquet"
    )
    parser.add_argument("--group", help="Group for all samples, e.g. val")
    parser.add_argument(
        "--images", type=Path, help="Image directory to stage beside the output"
    )
    parser.add_argument(
        "--link", action="store_true", help="Symlink staged images (Unix)"
    )
    parser.add_argument(
        "--rle-to-polygon",
        action="store_true",
        help="Trace RLE regions to polygons (lossy) instead of storing masks",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    output = normalize_user_path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)

    rows = ec.coco_to_arrow(
        normalize_user_path(args.coco_path),
        output,
        group=args.group,
        images_dir=normalize_user_path(args.images) if args.images else None,
        link_images=args.link,
        rle_to_polygon=args.rle_to_polygon,
    )
    print(f"Wrote {rows} rows to {output}")

    read = pl.read_parquet if output.suffix == ".parquet" else pl.read_ipc
    df = read(output)
    print(f"{df['name'].n_unique()} images, {df['label'].n_unique()} labels")
    for column in ("mask", "polygon"):
        # Columns that are entirely null are not written.
        count = df[column].is_not_null().sum() if column in df.columns else 0
        print(f"{count} {column} rows")
    print(df.group_by("label").len().sort("len", descending=True).head(10))


if __name__ == "__main__":
    main()
