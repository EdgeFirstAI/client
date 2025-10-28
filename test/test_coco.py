"""
COCO dataset integration tests.

Tests COCO dataset validation including labels, samples, and Polars DataFrame
integration (when Polars feature is enabled).
"""

import unittest

from test import get_client


class TestCOCO(unittest.TestCase):
    """Test COCO dataset validation."""

    def test_coco_dataset(self):
        """Test COCO dataset has correct labels and samples."""
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

        client = get_client()

        # Find Sample Project and COCO dataset
        projects = client.projects("Sample Project")
        self.assertGreater(len(projects), 0, "Sample Project should exist")
        project = projects[0]

        datasets = client.datasets(project.id, "COCO")
        self.assertGreater(len(datasets), 0, "COCO dataset should exist")

        # Filter to get exact "COCO" dataset (not "COCO People")
        dataset = None
        for d in datasets:
            if d.name == "COCO":
                dataset = d
                break
        self.assertIsNotNone(dataset, "COCO dataset should exist")
        assert dataset is not None

        # Verify labels
        labels = dataset.labels(client)
        self.assertEqual(len(labels), 80, "COCO should have 80 labels")

        for label in labels:
            self.assertEqual(
                label.name,
                coco_labels[label.index],
                f"Label {label.index} should be "
                f"{coco_labels[label.index]}")

        # Verify samples retrieval (samples_count not exposed in Python)
        samples = client.samples(dataset.id, None, [], ["val"], [], None)
        self.assertEqual(
            len(samples),
            5000,
            "Should retrieve 5000 samples")

    def test_coco_dataframe(self):
        """Test COCO dataset with Polars DataFrame (feature-gated)."""
        import edgefirst_client as ec

        # Check if Polars feature is enabled at compile time
        if not ec.is_polars_enabled():
            self.skipTest("Polars feature not enabled")

        client = get_client()

        # Find Sample Project and COCO dataset
        projects = client.projects("Sample Project")
        self.assertGreater(len(projects), 0, "Sample Project should exist")
        project = projects[0]

        datasets = client.datasets(project.id, "COCO")
        self.assertGreater(len(datasets), 0, "COCO dataset should exist")

        # Filter to get exact "COCO" dataset
        dataset = None
        for d in datasets:
            if d.name == "COCO":
                dataset = d
                break
        self.assertIsNotNone(dataset, "COCO dataset should exist")
        assert dataset is not None

        # Get annotation set
        annotation_sets = client.annotation_sets(dataset.id)
        self.assertGreater(
            len(annotation_sets),
            0,
            "COCO should have annotation sets")
        annotation_set_id = annotation_sets[0].id

        # Get annotations as DataFrame directly
        df = client.annotations_dataframe(
            annotation_set_id, ["val"], [], None)

        # Get unique by name
        df = df.unique(subset=["name"], keep="first", maintain_order=True)
        self.assertEqual(
            df.height,
            5000,
            "Should have 5000 unique samples")


if __name__ == '__main__':
    unittest.main()
