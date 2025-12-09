# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

from argparse import ArgumentParser
from os import environ
from pathlib import Path

from edgefirst_client import AnnotationType, Client, FileType
from tqdm import tqdm


def progress(current, total, pbar):
    """Update progress bar with current progress."""
    if total != pbar.total:
        pbar.reset(total)
    pbar.update(current - pbar.n)


def download_images(client, dataset_id, group, output):
    """Download images for a specific group."""
    with tqdm(total=0, desc=f"Downloading {group} Images") as bar:
        client.download_dataset(
            dataset_id=dataset_id,
            groups=[group],
            types=[FileType.Image],
            output=f"{output}/{group}",
            progress=lambda c, t: progress(c, t, bar),
        )


def fetch_samples(client, dataset_id, annotation_set_id, group):
    """Fetch samples with annotations for a specific group."""
    with tqdm(total=0, desc=f"Fetching {group} Samples") as bar:
        return client.samples(
            dataset_id=dataset_id,
            annotation_set_id=annotation_set_id,
            annotation_types=[AnnotationType.Box2d],
            groups=[group],
            types=[FileType.Image],
            progress=lambda c, t: progress(c, t, bar),
        )


def find_image_file(output, group, sample_name):
    """Find the corresponding image file for a sample."""
    image_files = list(Path(f"{output}/{group}").rglob(f"{sample_name}.jpg")) + list(
        Path(f"{output}/{group}").rglob(f"{sample_name}.png")
    )
    return image_files[0] if image_files else None


def save_yolo_annotation(annotation_path, annotations):
    """Save annotations in YOLO format to a file."""
    annotation_path.parent.mkdir(parents=True, exist_ok=True)
    with open(annotation_path, "w") as f:
        for ann in annotations:
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


def process_group_samples(samples, output, group):
    """Process and save annotations for all samples in a group."""
    for sample in tqdm(samples, desc=f"Saving {group} Annotations"):
        image_path = find_image_file(output, group, sample.name)
        if image_path:
            annotation_path = image_path.with_suffix(".txt")
            save_yolo_annotation(annotation_path, sample.annotations)


def download_dataset_yolo(dataset_id: str, output: str, groups: str, client=None):
    """
    Download dataset and save in YOLO format.

    Args:
        dataset_id: Dataset ID (e.g., "ds-xxx")
        output: Output directory path
        groups: Comma-separated list of groups (e.g., "train,val")
        client: Optional Client instance. If not provided, creates one using
                STUDIO_TOKEN, STUDIO_USERNAME/STUDIO_PASSWORD, or stored token.
    """
    if client is None:
        token = environ.get("STUDIO_TOKEN")
        username = environ.get("STUDIO_USERNAME")
        password = environ.get("STUDIO_PASSWORD")
        server = environ.get("STUDIO_SERVER", "")

        if token:
            client = Client().with_server(server).with_token(token)
        elif username and password:
            client = Client().with_server(server).with_login(username, password)
        else:
            # Use default client with stored token
            client = Client().with_server(server)

    dataset = client.dataset(dataset_id)
    annotation_set = client.annotation_sets(dataset.id)[0]

    for group in groups.split(","):
        Path(f"{output}/{group}").mkdir(parents=True, exist_ok=True)
        download_images(client, dataset.id, group, output)
        samples = fetch_samples(client, dataset.id, annotation_set.id, group)
        process_group_samples(samples, output, group)


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
