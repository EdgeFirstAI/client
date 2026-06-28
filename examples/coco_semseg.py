# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.
"""
Publish a COCO Semantic Segmentation dataset to EdgeFirst Studio.

Usage
-----
    STUDIO_TOKEN=<token> python examples/coco_semseg.py

Or step-by-step:
    # 1. Convert COCO Stuff annotations to Arrow (once, offline)
    edgefirst-client coco-to-arrow annotations/stuff_train2017.json \\
        -o coco2017_semseg_train.arrow --group train

    edgefirst-client coco-to-arrow annotations/stuff_val2017.json \\
        -o coco2017_semseg_val.arrow --group val

    # 2. Combine and upload (this script handles both steps)

Background: Instance vs Semantic Segmentation
----------------------------------------------
COCO ships two segmentation annotation flavours:

  instances_train2017.json  — **instance segmentation**
    • 80 "thing" categories (person, car, cat, …)
    • One annotation per object instance; polygon segmentation (iscrowd=0)
      or RLE for crowds (iscrowd=1)
    • Multiple annotations of the same class per image are distinct objects

  stuff_train2017.json      — **semantic segmentation material**
    • 92 "stuff" categories (sky, road, grass, …) with IDs 92–183
    • One annotation per class per image; always compressed RLE (iscrowd=0)
    • All regions of one class in an image are merged into a single mask

At the EdgeFirst Arrow level both formats share the same schema.  The
distinction is in how the training iterator consumes the data:

  • Instance seg trainer   — each annotation is a separate object mask.
  • Semantic seg trainer   — annotations are composited into a single
    H×W pixel-label map (class index per pixel), which is exactly what
    `EdgeFirstDatabase.read_mask()` in modelpack already does.

So the upload to Studio is identical; the semantic/instance split lives
entirely in the training iterator.

Arrow file: RLE → polygon (default)
----------------------------------------------
COCO Stuff uses compressed RLE even with iscrowd=0 (unlike the COCO spec
which says iscrowd=0 should use polygons).  The default ``coco-to-arrow``
behaviour decodes all RLE annotations to polygon contours stored in the
``polygon`` column.  This is the preferred representation for ground-truth:
training iterators composite per-class polygons into a semantic label map
on the fly without any pre-rasterisation.

Use ``--to-masks`` only when downstream consumers need pixel-perfect raster
masks (e.g. storing model prediction outputs alongside ground-truth).
"""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path

from edgefirst_client import Client

# ---------------------------------------------------------------------------
# Configuration — adjust to your environment
# ---------------------------------------------------------------------------

STUDIO_TOKEN = os.environ.get("STUDIO_TOKEN")
STUDIO_PROJECT = "Unit Testing"          # Project name in Studio
DATASET_NAME = "COCO Semantic Seg"       # Dataset name to create/update
COCO_DIR = Path("~/Dataset/COCO").expanduser()  # Local COCO root

TRAIN_JSON = COCO_DIR / "annotations" / "stuff_train2017.json"
VAL_JSON   = COCO_DIR / "annotations" / "stuff_val2017.json"
TRAIN_IMAGES = COCO_DIR / "train2017"
VAL_IMAGES   = COCO_DIR / "val2017"

TRAIN_ARROW = COCO_DIR / "coco2017_semseg_train.arrow"
VAL_ARROW   = COCO_DIR / "coco2017_semseg_val.arrow"

# ---------------------------------------------------------------------------
# Step 1: Convert COCO Stuff JSON → Arrow (skip if already done)
# ---------------------------------------------------------------------------

CLI = "edgefirst-client"

def convert_if_needed(json_path: Path, arrow_path: Path, group: str) -> None:
    if arrow_path.exists():
        print(f"  Skipping (already exists): {arrow_path.name}")
        return
    print(f"  Converting {json_path.name} → {arrow_path.name} …")
    subprocess.run(
        [
            CLI, "coco-to-arrow",
            str(json_path),
            "-o", str(arrow_path),
            "--group", group,
            # RLE is decoded to polygon contours by default (no flag needed)
        ],
        check=True,
    )


# ---------------------------------------------------------------------------
# Step 2: Upload to EdgeFirst Studio
# ---------------------------------------------------------------------------

def main() -> None:
    if not STUDIO_TOKEN:
        sys.exit("Set STUDIO_TOKEN environment variable before running.")

    # --- 1. Convert ---
    print("Step 1: Converting COCO Stuff annotations to Arrow format")
    convert_if_needed(TRAIN_JSON, TRAIN_ARROW, "train")
    convert_if_needed(VAL_JSON,   VAL_ARROW,   "val")

    # --- 2. Upload images + annotations ---
    print("\nStep 2: Uploading to EdgeFirst Studio")
    client = Client(token=STUDIO_TOKEN)

    projects = client.projects(STUDIO_PROJECT)
    if not projects:
        sys.exit(f"Project '{STUDIO_PROJECT}' not found in Studio.")
    project = projects[0]
    print(f"  Project: {project.name} ({project.id})")

    # Import each split.  import-coco handles dataset creation on first call
    # and annotation updates on subsequent calls.
    #
    # Note: import-coco always stores segmentation as polygon contours.
    # The --to-masks flag is only available through coco-to-arrow for local
    # Arrow files; it does not affect the Studio upload path.
    for json_path, image_dir, group in [
        (TRAIN_JSON, TRAIN_IMAGES, "train"),
        (VAL_JSON,   VAL_IMAGES,   "val"),
    ]:
        print(f"\n  Importing {group} split …")
        subprocess.run(
            [
                CLI, "import-coco",
                str(json_path.parent.parent),  # COCO root (contains images/)
                "--project", project.id,
                "--name", DATASET_NAME,
                "--group", group,
            ],
            check=True,
        )

    print(f"\n[OK] Done. Dataset '{DATASET_NAME}' uploaded to project '{STUDIO_PROJECT}'.")
    print("  The semantic segmentation arrow files are also available locally:")
    print(f"    {TRAIN_ARROW}")
    print(f"    {VAL_ARROW}")
    print()
    print("Training note:")
    print("  Use EdgeFirstDatabase(path, classes, shape, group, with_masks=True)")
    print("  in modelpack.  read_mask() composites all per-class polygon")
    print("  annotations into a single H×W pixel-label map automatically.")


if __name__ == "__main__":
    main()
