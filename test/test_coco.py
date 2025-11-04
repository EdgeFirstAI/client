"""
COCO dataset integration tests.

Tests COCO dataset availability and Polars DataFrame integration
(when Polars feature is enabled).
"""

import unittest

from test import get_client


class TestCOCO(unittest.TestCase):
    """Test COCO dataset accessibility and basic functionality."""

    def test_coco_known_label_indices(self):
        """
        Critical test: Verify COCO dataset returns labels in correct order.

        This test ensures that the 80 standard COCO classes are returned
        with the correct indices (0=person, 1=bicycle, ..., 79=toothbrush).
        This is essential for correct object detection and classification.
        """
        # Standard COCO dataset label ordering
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

        # Find Sample Project
        projects = client.projects("Sample Project")
        self.assertGreater(len(projects), 0, "Sample Project should exist")
        project = projects[0]

        # Find COCO dataset
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

        # Retrieve labels and verify correct indices
        labels = dataset.labels(client)
        self.assertEqual(
            len(labels),
            80,
            "COCO dataset must have exactly 80 classes",
        )

        # Verify each label has the correct index and name
        for label in labels:
            self.assertIn(
                label.index,
                coco_labels,
                f"Unknown COCO label index: {label.index}",
            )
            self.assertEqual(
                label.name,
                coco_labels[label.index],
                (
                    f"COCO label index {label.index} should be "
                    f"'{coco_labels[label.index]}' but got '{label.name}'"
                ),
            )

    def test_coco_dataset_accessible(self):
        """Test COCO dataset exists and samples are retrievable."""
        client = get_client()

        # Find Sample Project
        projects = client.projects("Sample Project")
        self.assertGreater(len(projects), 0, "Sample Project should exist")
        project = projects[0]

        # Find COCO dataset
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

        # Verify samples retrieval
        samples = client.samples(dataset.id, None, [], ["val"], [], None)
        self.assertGreater(len(samples), 0, "Should retrieve COCO samples")

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
            "COCO should have annotation sets",
        )
        annotation_set_id = annotation_sets[0].id

        # Get samples as DataFrame with new API (13 columns)
        df = client.samples_dataframe(
            dataset.id,
            annotation_set_id,
            ["val"],
            [],
            None)

        # Verify new schema has 13 columns
        self.assertEqual(
            df.shape[1],
            13,
            "Should have 13 columns in 2025.10 schema")

        # Verify column names
        expected_columns = {
            "name",
            "frame",
            "object_id",
            "label",
            "label_index",
            "group",
            "mask",
            "box2d",
            "box3d",
            "size",
            "location",
            "pose",
            "degradation"}
        actual_columns = set(df.columns)
        self.assertEqual(
            actual_columns,
            expected_columns,
            "Column names should match 2025.10 schema")

        # Get unique by name
        df = df.unique(subset=["name"], keep="first", maintain_order=True)
        self.assertGreater(df.height, 0, "Should have samples in DataFrame")


if __name__ == "__main__":
    unittest.main()
