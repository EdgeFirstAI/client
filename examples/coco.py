# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

from os import environ

from edgefirst_client import Client
from tqdm import tqdm

coco_labels = {
    0: "person",
    1: "bicycle",
    2: "car",
    3: "motorcycle",
    4: "airplane",
    5: "bus",
    6: "train",
    7: "truck",
    8: "boat",
    9: "traffic light",
    10: "fire hydrant",
    11: "stop sign",
    12: "parking meter",
    13: "bench",
    14: "bird",
    15: "cat",
    16: "dog",
    17: "horse",
    18: "sheep",
    19: "cow",
    20: "elephant",
    21: "bear",
    22: "zebra",
    23: "giraffe",
    24: "backpack",
    25: "umbrella",
    26: "handbag",
    27: "tie",
    28: "suitcase",
    29: "frisbee",
    30: "skis",
    31: "snowboard",
    32: "sports ball",
    33: "kite",
    34: "baseball bat",
    35: "baseball glove",
    36: "skateboard",
    37: "surfboard",
    38: "tennis racket",
    39: "bottle",
    40: "wine glass",
    41: "cup",
    42: "fork",
    43: "knife",
    44: "spoon",
    45: "bowl",
    46: "banana",
    47: "apple",
    48: "sandwich",
    49: "orange",
    50: "broccoli",
    51: "carrot",
    52: "hot dog",
    53: "pizza",
    54: "donut",
    55: "cake",
    56: "chair",
    57: "couch",
    58: "potted plant",
    59: "bed",
    60: "dining table",
    61: "toilet",
    62: "tv",
    63: "laptop",
    64: "mouse",
    65: "remote",
    66: "keyboard",
    67: "cell phone",
    68: "microwave",
    69: "oven",
    70: "toaster",
    71: "sink",
    72: "refrigerator",
    73: "book",
    74: "clock",
    75: "vase",
    76: "scissors",
    77: "teddy bear",
    78: "hair drier",
    79: "toothbrush",
}


if __name__ == "__main__":
    client = Client(token=environ.get("STUDIO_TOKEN"))
    project = client.projects("Sample Project")[0]
    dataset = client.datasets(project.id, "COCO")
    # Filter to avoid fetching the COCO People dataset.
    dataset = filter(lambda d: d.name == "COCO", dataset)
    dataset = next(dataset, None)
    assert dataset is not None
    labels = dataset.labels(client)
    assert len(labels) == 80

    # Two passes to reset the label indices to the correct COCO values.
    # First we set them to 1000+index to avoid conflicts, then on the next
    # pass we set them to the correct index.  Finally we verify the values.
    coco_indices = {v: k for k, v in coco_labels.items()}

    for label in tqdm(labels, desc="Indexing #1"):
        index = coco_indices[label.name]
        label.set_index(client, 1000 + index)

    for label in tqdm(labels, desc="Indexing #2"):
        index = coco_indices[label.name]
        label.set_index(client, index)

    for label in dataset.labels(client):
        assert label.name == coco_labels[label.index]
