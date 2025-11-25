# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

from argparse import ArgumentParser
from os import environ
from pathlib import Path

from edgefirst_client import AnnotationType, Client, FileType
from tqdm import tqdm


def download_dataset_yolo(dataset_id: str, output: str, groups: str):
    """
    Download dataset and save in YOLO format.

    Args:
        dataset_id: Dataset ID (e.g., "ds-xxx")
        output: Output directory path
        groups: Comma-separated list of groups (e.g., "train,val")
    """
    client = Client(token=environ.get("STUDIO_TOKEN"))
    dataset = client.dataset(dataset_id)
    annotation_set = client.annotation_sets(dataset.id)[0]

    for group in groups.split(","):
        Path(f"{output}/{group}").mkdir(parents=True, exist_ok=True)

        with tqdm(total=0, desc=f"Downloading {group} Images") as bar:

            def download_progress(current, total, pbar=bar):
                if total != pbar.total:
                    pbar.reset(total)
                pbar.update(current - pbar.n)

            client.download_dataset(
                dataset_id=dataset.id,
                groups=[group],
                types=[FileType.Image],
                output=f"{output}/{group}",
                progress=download_progress,
            )

        with tqdm(total=0, desc=f"Fetching {group} Samples") as bar:

            def fetch_progress(current, total, pbar=bar):
                if total != pbar.total:
                    pbar.reset(total)
                pbar.update(current - pbar.n)

            samples = client.samples(
                dataset_id=dataset.id,
                annotation_set_id=annotation_set.id,
                annotation_types=[AnnotationType.Box2d],
                groups=[group],
                types=[FileType.Image],
                progress=fetch_progress,
            )

        for sample in tqdm(samples, desc=f"Saving {group} Annotations"):
            # Find the corresponding image file to determine the correct directory
            image_files = list(
                Path(f"{output}/{group}").rglob(f"{sample.name}.jpg")
            ) + list(Path(f"{output}/{group}").rglob(f"{sample.name}.png"))

            if image_files:
                # Save annotation in the same directory as the image
                image_path = image_files[0]
                annotation_path = image_path.with_suffix(".txt")
                annotation_path.parent.mkdir(parents=True, exist_ok=True)

                with open(annotation_path, "w") as f:
                    for ann in sample.annotations:
                        if ann.box2d is not None:
                            f.write(
                                "%s %s %s %s %s\n"
                                % (
                                    ann.label_index,
                                    ann.box2d.cx,
                                    ann.box2d.cy,
                                    ann.box2d.width,
                                    ann.box2d.height,
                                )
                            )


if __name__ == "__main__":
    args = ArgumentParser(
        description="Download EdgeFirst Studio Dataset to COCO format"
    )
    args.add_argument("--output", type=str, default="dataset", help="Output directory")
    args.add_argument(
        "--groups",
        type=str,
        default="val",
        help="Comma-separated list of groups (train,val)",
    )
    args.add_argument("dataset", type=str, help="Dataset name (ds-xxx)")
    args = args.parse_args()

    download_dataset_yolo(args.dataset, args.output, args.groups)
