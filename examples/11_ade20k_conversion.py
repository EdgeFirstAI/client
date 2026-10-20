# SPDX-License-Identifier: Apache-2.0
# Copyright © 2026 Au-Zone Technologies. All Rights Reserved.

"""Convert ADE20K (ADEChallengeData2016) to one offline EdgeFirst dataset.

Example:
  python examples/11_ade20k_conversion.py ~/Datasets/ADEChallengeData2016 \
      --output ade20k/ade20k.arrow --images --link

Each image becomes one row whose ``mask`` is its annotation PNG, unchanged.
Use an output ending in ``.parquet`` to write Parquet instead of Arrow IPC,
and ``--group val`` to convert only the validation split. No EdgeFirst
Studio account or network connection is required.
"""

import argparse
import io
from pathlib import Path

import edgefirst_client as ec
import numpy as np
import polars as pl
from _path import normalize_user_path
from PIL import Image


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", type=Path, help="ADEChallengeData2016 directory")
    parser.add_argument(
        "--output", "-o", type=Path, required=True, help=".arrow or .parquet"
    )
    parser.add_argument("--group", choices=["train", "val"], help="One split only")
    parser.add_argument(
        "--images", action="store_true", help="Stage images beside the output"
    )
    parser.add_argument(
        "--link", action="store_true", help="Symlink staged images (Unix)"
    )
    return parser.parse_args()


def main() -> None:
    args = parse_args()
    output = normalize_user_path(args.output)
    output.parent.mkdir(parents=True, exist_ok=True)

    rows = ec.ade20k_to_arrow(
        normalize_user_path(args.root),
        output,
        group=args.group,
        stage_images=args.images,
        link_images=args.link,
    )
    print(f"Wrote {rows} rows to {output}")

    meta = ec.read_dataset_metadata(output)
    labels = meta.labels
    print(f"mask_interpretation: {meta.mask_interpretation}")
    print(f"ignore_index: {meta.ignore_index} ({labels[meta.ignore_index]})")
    print(f"{len(labels)} labels: {labels[:4]} ...")

    if output.suffix == ".parquet":
        df = pl.read_parquet(output)
    else:
        df = pl.read_ipc(output, memory_map=False)
    print(df.group_by("group").len().sort("group"))

    row = df.row(0, named=True)
    classes = np.asarray(Image.open(io.BytesIO(row["mask"])))
    present = [labels[i] for i in np.unique(classes)]
    print(f"{row['name']}: {classes.shape[1]}x{classes.shape[0]}, classes {present}")


if __name__ == "__main__":
    main()
