# SPDX-License-Identifier: Apache-2.0
# Copyright © 2026 Au-Zone Technologies. All Rights Reserved.

"""Convert extracted VisDrone2019 splits to one offline EdgeFirst dataset.

Example:
  python examples/09_visdrone_conversion.py \
      ~/Datasets/VisDrone2019-DET-train ~/Datasets/VisDrone2019-DET-val \
      --output visdrone-det/visdrone-det.arrow --images

Use an output ending in ``.parquet`` to write Parquet instead of Arrow IPC.
No EdgeFirst Studio account or network connection is required.
"""

import argparse
from pathlib import Path

import edgefirst_client as ec
import polars as pl
from _path import normalize_user_path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "split_dirs",
        nargs="+",
        type=Path,
        help="Extracted VisDrone2019 DET or VID split directories",
    )
    parser.add_argument(
        "--output", "-o", type=Path, required=True, help=".arrow or .parquet"
    )
    parser.add_argument(
        "--images", action="store_true", help="Stage images beside the output"
    )
    parser.add_argument(
        "--link", action="store_true", help="Symlink staged images (Unix)"
    )
    parser.add_argument(
        "--group", help="Group for all samples; omit to infer from names"
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    output = normalize_user_path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)

    rows = ec.visdrone_to_arrow(
        [normalize_user_path(p) for p in args.split_dirs],
        output,
        group=args.group,
        stage_images=args.images,
        link_images=args.link,
    )
    print(f"Wrote {rows} rows to {output}")

    read = pl.read_parquet if output.suffix == ".parquet" else pl.read_ipc
    df = read(output)
    trainable = df.filter(~pl.col("label_index").is_in([0, 11]))
    print(df.group_by("group").len().sort("group"))
    print(f"{trainable.height} boxes after dropping ignored regions and others")


if __name__ == "__main__":
    main()
