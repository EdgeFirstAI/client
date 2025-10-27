# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

from os import environ
from pathlib import Path
from time import sleep
from unittest import TestCase

from edgefirst_client import AnnotationType, Client, Parameter
from polars import read_ipc
from tqdm import tqdm

from examples.coco import coco_labels


def get_test_data_dir():
    """
    Get the test data directory (target/testdata).
    Creates it if it doesn't exist.
    """
    test_dir = Path(__file__).parent / "target" / "testdata"
    test_dir.mkdir(parents=True, exist_ok=True)
    return test_dir


class BasicTest(TestCase):
    def get_client(self):
        return Client(token=environ.get("STUDIO_TOKEN"))

    def test_version(self):
        client = Client()
        version = client.version()
        assert version != ""

    def test_token(self):
        client = self.get_client()
        token = client.token()
        assert token != ""
        print(f"Token: {token}")
        print(f"Token Expiration: {client.token_expiration}")
        print(f"Username: {client.username}")
        print(f"Server: {client.url}")

        sleep(2)  # Wait for 2 seconds to ensure token renewal updates the time

        client.renew_token()
        new_token = client.token()
        assert new_token != ""
        assert token != new_token
        print(f"New Token Expiration: {client.token_expiration}")

    def test_organization(self):
        client = self.get_client()
        org = client.organization()
        assert org is not None
        assert org.id is not None
        assert org.name is not None
        assert org.credits is not None
        print(f"Organization: {org.name}")
        print(f"ID: {org.id.value}")
        print(f"Credits: {org.credits}")

    def test_projects(self):
        client = self.get_client()
        projects = client.projects()
        assert len(projects) > 0

    def test_project(self):
        client = self.get_client()
        project = client.projects("Unit Testing")
        assert project is not None
        assert len(project) == 1
        assert project[0].name == "Unit Testing"

    def test_datasets(self):
        client = self.get_client()
        project = client.projects("Unit Testing")[0]
        datasets = project.datasets(client)
        assert len(datasets) > 0

        for dataset in datasets:
            ds = client.dataset(dataset.id)
            assert ds is not None
            assert ds.name == dataset.name

            ds = client.datasets(project.id, dataset.name)
            assert len(ds) > 0
            ds = ds[0]
            assert dataset.name in ds.name

    def test_labels(self):
        client = self.get_client()
        projects = client.projects("Unit Testing")
        project_id = projects[0].id
        self.assertIsNotNone(project_id)

        dataset = client.datasets(project_id, "Test Labels")[0]
        for label in dataset.labels(client):
            label.remove(client)

        dataset.add_label(client, "test")
        assert len(dataset.labels(client)) == 1

        dataset.remove_label(client, "test")
        assert len(dataset.labels(client)) == 0

    def test_coco_labels(self):
        client = self.get_client()
        project = client.projects("Sample Project")[0]
        dataset = client.datasets(project.id, "COCO")
        # Filter to avoid fetching the COCO People dataset.
        dataset = filter(lambda d: d.name == "COCO", dataset)
        dataset = next(dataset, None)
        assert dataset is not None
        labels = dataset.labels(client)

        assert len(labels) == 80
        for label in labels:
            assert label.name == coco_labels[label.index]

    def test_annotations_set(self):
        client = self.get_client()
        projects = client.projects("Sample Project")
        project_id = projects[0].id
        self.assertIsNotNone(project_id)

        dataset = client.datasets(project_id, "COCO")
        # Filter to avoid fetching the COCO People dataset.
        dataset = filter(lambda d: d.name == "COCO", dataset)
        dataset = next(dataset, None)
        assert dataset is not None

        test_dir = get_test_data_dir()
        arrow_file = test_dir / "coco.arrow"

        with tqdm(total=0, unit="", unit_scale=True, unit_divisor=1000) as bar:

            def progress(current, total):
                if total != bar.total:
                    bar.reset(total)
                bar.update(current - bar.n)

            annotation_set = client.annotation_sets(dataset.id)[0]
            df = client.annotations_dataframe(
                annotation_set_id=annotation_set.id,
                groups=["val"],
                annotation_types=[AnnotationType.Box2d],
                progress=progress,
            )
            df.write_ipc(str(arrow_file))
        df = read_ipc(str(arrow_file))
        assert len(df.unique("name")) == 5000

        # Clean up
        if arrow_file.exists():
            arrow_file.unlink()

    def test_training_sessions(self):
        client = self.get_client()
        project = client.projects("Unit Testing")[0]
        experiment = client.experiments(project.id, "Unit Testing")[0]
        trainers = client.training_sessions(experiment.id)
        assert len(trainers) > 0

        for trainer in trainers:
            t = client.training_session(trainer.id)
            assert t.id.value == trainer.id.value
            assert t.name == trainer.name
            assert t.description == trainer.description
            assert t.dataset_params is not None
            assert t.model_params is not None

        trainer = filter(lambda s: s.name == "modelpack-usermanaged", trainers)
        trainer = next(trainer, None)
        assert trainer is not None

        trainer.set_metrics(client, {"precision": Parameter.real(0.75)})
        metrics = trainer.metrics(client)
        assert metrics["precision"] == Parameter.real(0.75)

    def test_validation_sessions(self):
        client = self.get_client()
        project = client.projects("Unit Testing")[0]
        sessions = client.validation_sessions(project.id)
        assert len(sessions) > 0

        for session in sessions:
            s = client.validation_session(session.id)
            assert s.id.value == session.id.value
            assert s.name == session.name
            assert s.description == session.description

        session = filter(lambda s: s.name == "modelpack-usermanaged", sessions)
        session = next(session, None)
        assert session is not None

        session.set_metrics(client, {"precision": Parameter.real(0.75)})
        metrics = session.metrics(client)
        assert metrics["precision"] == Parameter.real(0.75)

    def test_artifacts(self):
        client = self.get_client()
        project = client.projects("Unit Testing")[0]
        experiment = client.experiments(project.id, "Unit Testing")[0]
        trainer = client.training_sessions(
            experiment.id, "modelpack-960x540")[0]
        assert trainer.uid.startswith("t-")
        artifacts = client.artifacts(trainer.id)
        assert len(artifacts) > 0

        test_dir = get_test_data_dir()

        for artifact in artifacts:
            output_path = test_dir / artifact.name
            client.download_artifact(
                trainer.id, artifact.name, output_path)

            # Clean up downloaded file
            if output_path.exists():
                output_path.unlink()

    def test_checkpoints(self):
        client = self.get_client()
        project = client.projects("Unit Testing")[0]
        experiment = client.experiments(project.id, "Unit Testing")[0]
        trainer = client.training_sessions(
            experiment.id, "modelpack-usermanaged")[0]
        assert trainer.uid.startswith("t-")

        test_dir = get_test_data_dir()
        checkpoint_file = test_dir / "checkpoint_py.txt"

        with open(str(checkpoint_file), "w") as f:
            f.write("Checkpoint from Python")

        trainer.upload_checkpoint(client, str(checkpoint_file))
        ckpt = trainer.download_checkpoint(client, "checkpoint_py.txt")
        assert ckpt == b"Checkpoint from Python"

        # Clean up
        if checkpoint_file.exists():
            checkpoint_file.unlink()

    def test_tasks(self):
        client = self.get_client()
        project = client.projects("Unit Testing")[0]
        tasks = client.tasks("modelpack-usermanaged")
        tasks = [client.task_info(t.id) for t in tasks]
        tasks = filter(lambda t: t.project_id.value == project.id.value, tasks)
        task = next(tasks, None)
        assert task is not None

        t = client.task_status(task.id, "export")
        assert t.status == "export"

        client.set_stages(task.id, [("export", "Export Model")])
        client.update_stage(
            task_id=task.id,
            stage="export",
            status="running",
            message="Exporting the model",
            percentage=25,
        )

        task = client.task_info(task.id)
        assert len(task.stages) == 1
        assert task.stages["export"].message == "Exporting the model"
        assert task.stages["export"].status == "running"
        assert task.stages["export"].percentage == 25

    def test_populate_samples(self):
        """Test populating samples with automatic file upload."""
        import random
        import string
        import time

        from edgefirst_client import Annotation, Box2d, Sample, SampleFile
        from PIL import Image, ImageDraw

        client = self.get_client()

        # Find the Unit Testing project
        projects = client.projects("Unit Testing")
        assert len(projects) > 0
        project = projects[0]

        # Create a temporary test dataset with random suffix
        random_suffix = "".join(
            random.choices(string.ascii_uppercase + string.digits, k=6)
        )
        test_dataset_name = f"Test Populate {random_suffix}"

        print(f"Creating test dataset: {test_dataset_name}")

        dataset_id = client.create_dataset(
            str(project.id),
            test_dataset_name,
            "Automated test: populate_samples verification",
        )

        print(f"Created test dataset: {dataset_id}")

        # Create an annotation set
        print("Creating annotation set...")
        annotation_set_id = client.create_annotation_set(
            dataset_id, "Default", "Default annotation set"
        )

        print(f"Created annotation set: {annotation_set_id}")

        # Get the annotation set
        annotation_sets = client.annotation_sets(dataset_id)
        assert len(annotation_sets) > 0
        annotation_set = annotation_sets[0]

        # Generate a 640x480 PNG image with a red circle
        img_width = 640
        img_height = 480
        img = Image.new("RGB", (img_width, img_height), color="white")
        draw = ImageDraw.Draw(img)

        # Draw a red circle in the top-left quadrant
        center_x = 150.0
        center_y = 120.0
        radius = 50.0

        # PIL ellipse takes (left, top, right, bottom)
        draw.ellipse(
            [
                center_x - radius,
                center_y - radius,
                center_x + radius,
                center_y + radius,
            ],
            fill="red",
        )

        # Calculate bounding box around the circle (with some padding)
        bbox_x = center_x - radius - 5.0
        bbox_y = center_y - radius - 5.0
        bbox_w = (radius * 2.0) + 10.0
        bbox_h = (radius * 2.0) + 10.0

        print(
            f"Generated PNG image with circle at bbox: "
            f"({bbox_x:.1f}, {bbox_y:.1f}, {bbox_w:.1f}, {bbox_h:.1f})"
        )

        # Save to temporary file
        test_dir = get_test_data_dir()
        timestamp = int(time.time())
        test_image_path = test_dir / f"test_populate_{timestamp}.png"
        img.save(str(test_image_path), format="PNG")
        print(f"Test image saved to: {test_image_path}")

        # Create sample with annotation
        sample = Sample()
        sample.set_image_name(f"test_populate_{timestamp}.png")

        # Add file
        sample.add_file(SampleFile("image", str(test_image_path)))

        # Add bounding box annotation with NORMALIZED coordinates
        annotation = Annotation()
        annotation.set_label("circle")
        annotation.set_object_id("circle-obj-1")

        # Normalize coordinates: divide pixel values by image dimensions
        normalized_x = bbox_x / img_width
        normalized_y = bbox_y / img_height
        normalized_w = bbox_w / img_width
        normalized_h = bbox_h / img_height

        print(
            f"Normalized bbox: ({normalized_x:.3f}, {normalized_y:.3f}, "
            f"{normalized_w:.3f}, {normalized_h:.3f})"
        )

        bbox = Box2d(normalized_x, normalized_y, normalized_w, normalized_h)
        annotation.set_box2d(bbox)
        sample.add_annotation(annotation)

        # Populate the sample with progress callback
        def progress(current, total):
            print(f"Upload progress: {current}/{total}")

        try:
            results = client.populate_samples(
                dataset_id, annotation_set.id, [sample], progress=progress
            )

            assert len(results) == 1
            result = results[0]
            assert len(result.urls) == 1
            print(f"✓ Sample populated with UUID: {result.uuid}")

            # Give the server a moment to process the upload
            time.sleep(2)

            # Verify the sample was created by fetching it back
            image_filename = f"test_populate_{timestamp}"
            print(f"Looking for image: {image_filename}")

            samples = client.samples(
                dataset_id,
                annotation_set.id,
                annotation_types=[],
                groups=[],  # Don't filter by group - get all samples
                types=[],
            )

            print(f"Found {len(samples)} samples total")

            # Find the sample by image_name
            created_sample = None
            for s in samples:
                print(
                    f"  Sample: {s.name} UUID: {s.uuid} "
                    f"Dimensions: {s.width}x{s.height}"
                )
                if s.name == image_filename:
                    created_sample = s
                    break

            assert (
                created_sample is not None
            ), f"Sample with image_name '{image_filename}' should exist"

            print(f"✓ Found sample by image_name: {image_filename}")

            # Verify basic properties
            assert created_sample.name == image_filename
            assert (created_sample.group == "train" or
                    created_sample.group is None)

            print("\nSample verification:")
            print(f"  ✓ image_name: {created_sample.name}")
            print(f"  ✓ group: {created_sample.group}")
            print(
                f"  ✓ annotations: {len(created_sample.annotations)} item(s)")

            # Verify annotations are returned correctly
            annotations = created_sample.annotations
            assert len(annotations) == 1, "Should have exactly one annotation"

            annotation = annotations[0]
            assert annotation.label == "circle"
            assert annotation.box2d is not None, (
                "Bounding box should be present")

            returned_bbox = annotation.box2d
            print(
                f"\nReturned bbox (normalized): ({returned_bbox.left:.3f}, "
                f"{returned_bbox.top:.3f}, {returned_bbox.width:.3f}, "
                f"{returned_bbox.height:.3f})"
            )

            # Verify bbox coordinates are approximately correct
            # (within 5% tolerance)
            tolerance = 0.05
            assert abs(returned_bbox.left - normalized_x) < tolerance
            assert abs(returned_bbox.top - normalized_y) < tolerance
            assert abs(returned_bbox.width - normalized_w) < tolerance
            assert abs(returned_bbox.height - normalized_h) < tolerance

            print("✓ Bounding box coordinates verified")

            # Download the image and verify byte-for-byte match
            downloaded_data = created_sample.download(client)
            assert downloaded_data is not None, (
                "Downloaded data should not be None")

            # Read original file
            with open(str(test_image_path), "rb") as f:
                original_data = f.read()

            assert len(downloaded_data) == len(
                original_data
            ), "Downloaded data length should match original"
            assert (
                downloaded_data == original_data
            ), "Downloaded data should match original byte-for-byte"

            print(
                f"✓ Downloaded image matches original "
                f"({len(downloaded_data)} bytes)"
            )

            print("\n✓ Test passed: populate_samples with automatic upload")

        finally:
            # Clean up temporary file
            if test_image_path.exists():
                test_image_path.unlink()

            # Clean up test dataset (always execute, even if test failed)
            print("\nCleaning up test dataset...")
            client.delete_dataset(dataset_id)
            print("  ✓ Deleted test dataset")

    def _download_deer_samples_and_images(
            self, client, deer_dataset, annotation_set, download_dir):
        """Download samples, annotations, and images from Deer dataset."""
        print("Downloading samples from Deer dataset...")
        deer_samples = client.samples(
            deer_dataset.id, annotation_set.id, groups=[])
        print(f"Downloaded {len(deer_samples)} samples")
        assert len(deer_samples) > 0, "Deer dataset should have samples"

        print("Downloading annotations...")
        deer_annotations = client.annotations(annotation_set.id, groups=[])
        print(f"Downloaded {len(deer_annotations)} annotations")

        print("Downloading sample images...")
        downloaded_images = {}
        for idx, sample in enumerate(deer_samples[:5]):  # First 5 samples
            image_data = sample.download(client)
            if image_data:
                image_name = sample.image_name or f"sample_{idx}.jpg"
                image_path = download_dir / image_name
                image_path.write_bytes(image_data)
                downloaded_images[image_name] = image_data
                print(f"  Downloaded: {image_name}")

        print(
            f"Downloaded {len(downloaded_images)} sample images "
            f"for verification"
        )
        return deer_samples, deer_annotations, downloaded_images

    def _prepare_upload_samples(
            self, deer_samples, deer_annotations, download_dir, client):
        """Prepare samples for upload to test dataset."""
        from edgefirst_client import Sample, SampleFile

        print("Preparing samples for upload...")
        upload_samples = []

        for idx, sample in enumerate(deer_samples[:10]):
            new_sample = Sample()
            image_data = sample.download(client)
            if image_data:
                image_name = sample.image_name or f"sample_{idx}.jpg"
                temp_path = download_dir / image_name
                temp_path.write_bytes(image_data)
                new_sample.add_file(SampleFile("image", str(temp_path)))

                sample_annotations = [
                    ann for ann in deer_annotations
                    if ann.name == sample.image_name]
                for ann in sample_annotations:
                    new_sample.add_annotation(ann)

                upload_samples.append(new_sample)

        print(f"Prepared {len(upload_samples)} samples for upload")
        assert len(
            upload_samples) > 0, "Should have samples prepared for upload"
        return upload_samples

    def _verify_uploaded_images(
            self, client, uploaded_samples, downloaded_images):
        """Verify uploaded images match originals byte-for-byte."""
        verified_count = 0
        for original_name, original_data in list(
                downloaded_images.items())[:3]:
            uploaded_sample = next(
                (s for s in uploaded_samples
                 if s.image_name == original_name), None)
            if uploaded_sample:
                uploaded_data = uploaded_sample.download(client)
                if uploaded_data:
                    assert len(original_data) == len(
                        uploaded_data
                    ), f"Image {original_name} should have same size"
                    assert (
                        original_data == uploaded_data
                    ), f"Image {original_name} should match byte-for-byte"
                    verified_count += 1
                    print(f"  ✓ Verified: {original_name}")

        assert verified_count > 0, (
            "Should have verified at least one image")
        print(f"Verified {verified_count} images match byte-for-byte")

    def test_deer_dataset_roundtrip(self):  # type: ignore[misc]
        """
        Test downloading Deer dataset and re-uploading to verify
        data integrity.

        Note: This integration test downloads a dataset, uploads it
        to a new dataset, and verifies byte-for-byte image integrity and
        annotation preservation.
        """
        import random
        import string
        import time

        client = self.get_client()

        # Find the Unit Testing project and Deer dataset (read-only)
        projects = client.projects("Unit Testing")
        assert len(projects) > 0
        project = projects[0]

        datasets = client.datasets(project.id, "Deer")
        deer_dataset = next((d for d in datasets if d.name == "Deer"), None)
        assert deer_dataset is not None, "Deer dataset should exist"
        print(f"Found Deer dataset: {deer_dataset.uid}")

        # Get annotation sets
        annotation_sets = client.annotation_sets(deer_dataset.id)
        assert len(annotation_sets) > 0
        annotation_set = annotation_sets[0]
        print(f"Using annotation set: {annotation_set.uid}")

        # Download data from Deer dataset
        test_dir = get_test_data_dir()
        download_dir = test_dir / f"deer_download_{int(time.time())}"
        download_dir.mkdir(parents=True, exist_ok=True)

        deer_samples, deer_annotations, downloaded_images = (
            self._download_deer_samples_and_images(
                client, deer_dataset, annotation_set, download_dir))

        # Create a test dataset with random suffix to avoid conflicts
        random_suffix = "".join(
            random.choices(string.ascii_uppercase + string.digits, k=6)
        )
        test_dataset_name = f"Deer Test {random_suffix}"
        print(f"Creating test dataset: {test_dataset_name}")

        test_dataset_id = client.create_dataset(
            str(project.id),
            test_dataset_name,
            "Automated test: Deer dataset round-trip verification",
        )
        print(f"Created test dataset: {test_dataset_id}")

        try:
            # Create an annotation set
            print("Creating annotation set...")
            test_annotation_set_id = client.create_annotation_set(
                test_dataset_id, "Default", "Default annotation set"
            )
            print(f"Created annotation set: {test_annotation_set_id}")

            # Copy labels from Deer dataset
            test_dataset = client.dataset(test_dataset_id)
            deer_labels = deer_dataset.labels(client)
            for label in deer_labels:
                test_dataset.add_label(client, label.name)
            print(f"Copied {len(deer_labels)} labels")

            # Prepare and upload samples
            upload_samples = self._prepare_upload_samples(
                deer_samples, deer_annotations, download_dir, client)

            print("Uploading samples to test dataset...")
            results = client.populate_samples(
                test_dataset_id, test_annotation_set_id, upload_samples
            )
            print(f"Uploaded {len(results)} samples")

            # Give the server time to process
            time.sleep(3)

            # Verify uploaded data
            print("Verifying uploaded data...")
            uploaded_samples = client.samples(
                test_dataset_id, test_annotation_set_id, groups=[]
            )
            print(f"Found {len(uploaded_samples)} uploaded samples")
            assert len(uploaded_samples) == len(
                results
            ), "Should have same number of uploaded samples"

            # Verify images match byte-for-byte
            self._verify_uploaded_images(
                client, uploaded_samples, downloaded_images)

            # Verify annotations were uploaded
            uploaded_annotations = client.annotations(
                test_annotation_set_id, groups=[])
            print(f"Found {len(uploaded_annotations)} uploaded annotations")
            assert len(
                uploaded_annotations) > 0, "Should have uploaded annotations"

            print("\n✅ Round-trip test completed successfully!")

        finally:
            # Clean up: Delete the test dataset
            print("Cleaning up test dataset...")
            client.delete_dataset(test_dataset_id)
            print("  ✓ Deleted test dataset")

            # Clean up downloaded files
            import shutil

            if download_dir.exists():
                shutil.rmtree(download_dir)
            print("  ✓ Cleaned up downloaded files")
