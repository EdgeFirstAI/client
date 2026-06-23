# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

"""
Polars and Pandas workflows with Coffee Cup (ds-145f).

Hybrid CLI + Python:
  edgefirst-client download-annotations <as-id> coffee_cup.arrow --groups val
  python -c "import polars as pl; print(pl.read_ipc('coffee_cup.arrow'))"

Native API:
  client.samples_dataframe(dataset_id, annotation_set_id, groups=["val"])
"""

import os
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from examples import COFFEE_CUP_DATASET_ID, get_client  # noqa: E402

import edgefirst_client as ec  # noqa: E402
import polars as pl  # noqa: E402


def cli_download_arrow(annotation_set_id: str, output: Path, groups: str) -> None:
    """Download annotations via bundled edgefirst-client CLI."""
    cmd = [
        "edgefirst-client",
        "download-annotations",
        str(annotation_set_id),
        str(output),
        "--groups",
        groups,
    ]
    print("CLI:", " ".join(cmd))
    subprocess.run(cmd, check=True)


def main() -> None:
    if not ec.is_polars_enabled():
        print("Polars support not enabled in this build.")
        sys.exit(1)

    client = get_client()
    client.verify_token()

    dataset = client.dataset(COFFEE_CUP_DATASET_ID)
    annotation_sets = client.annotation_sets(dataset.id)
    if not annotation_sets:
        raise RuntimeError("Coffee Cup has no annotation sets")
    annotation_set_id = annotation_sets[0].id

    groups = ["val"]
    arrow_path = Path("coffee_cup.arrow")

    # --- Step A: CLI Arrow export (optional if file already exists) ---
    if os.environ.get("SKIP_CLI_DOWNLOAD") != "1":
        try:
            cli_download_arrow(annotation_set_id, arrow_path, "val")
        except (subprocess.CalledProcessError, FileNotFoundError) as exc:
            print("CLI download skipped:", exc)
            print("Set SKIP_CLI_DOWNLOAD=1 or run edgefirst-client manually.")
    elif arrow_path.exists():
        print(f"Using existing {arrow_path}")

    # --- Step B: Load Arrow in Polars ---
    if arrow_path.exists():
        df_cli = pl.read_ipc(arrow_path)
        print(f"\nCLI Arrow: {df_cli.shape[0]} rows, columns: {df_cli.columns}")
        if "label" in df_cli.columns:
            counts = df_cli.group_by("label").len().sort("len", descending=True)
            print("Label counts (top 5):")
            print(counts.head(5))

        try:
            import pandas as pd  # noqa: WPS433

            df_pd = pd.read_feather(arrow_path)
            print(f"\nPandas feather: {len(df_pd)} rows")
        except ImportError:
            print("\nPandas/pyarrow not installed; skipping Pandas path.")

    # --- Step C: Native API samples_dataframe ---
    df_api = client.samples_dataframe(
        dataset.id,
        annotation_set_id,
        groups,
        [],
        None,
    )
    print(f"\nAPI samples_dataframe: {df_api.shape[0]} rows")
    print(f"Columns: {df_api.columns}")

    if "label" in df_api.columns:
        unique_samples = df_api.unique(
            subset=["name"], keep="first", maintain_order=True
        )
        print(f"Unique samples by name: {unique_samples.height}")

    if "box2d" in df_api.columns:
        with_boxes = df_api.filter(pl.col("box2d").is_not_null())
        print(f"Rows with box2d: {with_boxes.height}")


if __name__ == "__main__":
    main()
