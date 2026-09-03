# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""Convert a standard COCO directory to one offline EdgeFirst dataset.

Example:
  python examples/08_coco_conversion.py ~/Datasets/COCO \
      --output coco/coco.arrow --images ~/Datasets/COCO --link

Use an output ending in ``.parquet`` to write Parquet instead of Arrow IPC.
No EdgeFirst Studio account or network connection is required.
"""

import argparse
from pathlib import Path

import edgefirst_client as ec
import polars as pl


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "coco_path",
        type=Path,
        help="COCO JSON/ZIP or extracted root containing annotations/ and split folders",
    )
    parser.add_argument(
        "--output",
        "-o",
        type=Path,
        required=True,
        help="Output .arrow or .parquet annotation file",
    )
    parser.add_argument(
        "--images",
        type=Path,
        help="Image root to stage beside the output (usually the COCO root)",
    )
    parser.add_argument(
        "--link",
        action="store_true",
        help="Symlink staged images instead of copying them",
    )
    parser.add_argument(
        "--group",
        help="Set one group explicitly; omit for train/val inference from a directory",
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    args.output.parent.mkdir(parents=True, exist_ok=True)

    rows = ec.coco_to_arrow(
        args.coco_path,
        args.output,
        group=args.group,
        images_dir=args.images,
        link_images=args.link,
    )
    print(f"Wrote {rows} rows to {args.output}")

    if args.output.suffix == ".parquet":
        dataframe = pl.read_parquet(args.output)
    else:
        dataframe = pl.read_ipc(args.output)

    sample_keys = ["name"]
    if "group" in dataframe.columns:
        sample_keys.insert(0, "group")
        split_summary = (
            dataframe.select(sample_keys).unique().group_by("group").len().sort("group")
        )
        print("Unique samples by group:")
        print(split_summary)

    print(f"Unique samples: {dataframe.select(sample_keys).unique().height}")
    print(f"Columns: {', '.join(dataframe.columns)}")


if __name__ == "__main__":
    main()
