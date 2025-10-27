# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

from argparse import ArgumentParser
from os import environ
from pathlib import Path

from edgefirst_client import AnnotationType, Client, FileType
from tqdm import tqdm

if __name__ == "__main__":
    args = ArgumentParser(
        description="Download EdgeFirst Studio Dataset to COCO format"
    )
    args.add_argument(
        "--output",
        type=str,
        default="dataset",
        help="Output directory")
    args.add_argument(
        "--groups",
        type=str,
        default="val",
        help="Comma-separated list of groups (train,val)",
    )
    args.add_argument("dataset", type=str, help="Dataset name (ds-xxx)")
    args = args.parse_args()

    client = Client(token=environ.get("STUDIO_TOKEN"))
    dataset = client.dataset(args.dataset)
    annotation_set = client.annotation_sets(dataset.id)[0]

    for group in args.groups.split(","):
        Path(f"{args.output}/{group}").mkdir(parents=True, exist_ok=True)

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

        for sample in tqdm(samples, desc=f"Saving {group} Samples"):
            with open(f"{args.output}/{group}/{sample.name}.txt", "w") as f:
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

        with tqdm(total=0, desc=f"Downloading {group} Images") as bar:

            def download_progress(current, total, pbar=bar):
                if total != pbar.total:
                    pbar.reset(total)
                pbar.update(current - pbar.n)

            client.download_dataset(
                dataset_id=dataset.id,
                groups=[group],
                types=[FileType.Image],
                output=f"{args.output}/{group}",
                progress=download_progress,
            )
