"""
Advanced integration tests for EdgeFirst client.

Tests complex server interactions including task and stage management.
"""

import hashlib
import tempfile
import unittest
import uuid as uuid_lib
from pathlib import Path

from PIL import Image, ImageDraw

from edgefirst_client import Sample, SampleFile

from test import get_client


def generate_sequence_uuid(dataset_id_str, sequence_name):
    """
    Generate v5 UUID using SHA-1 hash of dataset_id/sequence_name.

    This matches the client's UUID generation logic for sequences.
    """
    input_str = f"{dataset_id_str}/{sequence_name}"
    hash_bytes = hashlib.sha1(input_str.encode()).digest()
    uuid_bytes = hash_bytes[:16]
    uuid_obj = uuid_lib.UUID(bytes=uuid_bytes)

    bytes_array = bytearray(uuid_obj.bytes)
    bytes_array[6] = (bytes_array[6] & 0x0f) | 0x50
    bytes_array[8] = (bytes_array[8] & 0x3f) | 0x80

    return str(uuid_lib.UUID(bytes=bytes(bytes_array)))


def create_image_with_text(text):
    """Create a 640x480 white image with large centered black text."""
    img = Image.new('RGB', (640, 480), color='white')
    draw = ImageDraw.Draw(img)

    # Use large font size (approximately 50-75% of image height)
    font_size = 300  # Large enough to occupy ~60% of 480px height
    try:
        # Try to load a TrueType font
        import os
        if os.name == 'posix':  # macOS/Linux
            font_paths = [
                '/System/Library/Fonts/Helvetica.ttc',
                '/Library/Fonts/Arial.ttf',
                '/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf',
            ]
            font = None
            for font_path in font_paths:
                if os.path.exists(font_path):
                    from PIL import ImageFont
                    font = ImageFont.truetype(font_path, font_size)
                    break
            if font is None:
                raise OSError("No suitable font found")
        else:  # Windows
            from PIL import ImageFont
            font = ImageFont.truetype("arial.ttf", font_size)
    except (OSError, ImportError):
        # Fall back to default font
        print("Warning: Could not load TrueType font, using default")
        font = None

    # Get text bounding box to center it
    bbox = draw.textbbox((0, 0), text, font=font)
    text_width = bbox[2] - bbox[0]
    text_height = bbox[3] - bbox[1]

    # Center the text
    x = (640 - text_width) // 2
    y = (480 - text_height) // 2

    # Draw the text in black
    draw.text((x, y), text, fill='black', font=font)

    return img


class TestTasks(unittest.TestCase):
    """Test task and stage operations."""

    def test_tasks(self):
        """Test task listing, status, and stage management."""
        client = get_client()

        # Find Unit Testing project
        projects = client.projects("Unit Testing")
        self.assertGreater(
            len(projects),
            0,
            "Unit Testing project should exist")
        project = projects[0]

        # Get all tasks
        all_tasks = client.tasks(None, None, None, None)

        # Verify we can fetch task info for each
        for task in all_tasks:
            task_info = client.task_info(task.id)
            self.assertIsNotNone(task_info)
            assert task_info is not None

        # Get tasks for specific training session
        tasks = client.tasks("modelpack-usermanaged", None, None, None)

        # Filter to project
        project_tasks = []
        for task in tasks:
            task_info = client.task_info(task.id)
            if task_info.project_id == project.id:
                project_tasks.append(task_info)

        self.assertNotEqual(
            len(project_tasks),
            0,
            "Should have tasks for project")
        task = project_tasks[0]

        # Get task status
        t = client.task_status(task.id, "training")
        self.assertEqual(t.id, task.id)
        self.assertEqual(t.status, "training")

        # Set stages
        stages = [
            ("download", "Downloading Dataset"),
            ("train", "Training Model"),
            ("export", "Exporting Model"),
        ]
        client.set_stages(task.id, stages)

        # Update stage
        client.update_stage(
            task.id,
            "download",
            "running",
            "Downloading dataset",
            50)

        # Verify stage update
        task_updated = client.task_info(task.id)
        self.assertIsNotNone(task_updated.stages)
        assert task_updated.stages is not None


class TestSequences(unittest.TestCase):
    """Test sequence upload and verification."""

    def test_sequence_upload_and_verification(self):
        """Test uploading 2 sequences + 2 non-sequence images (22 total)."""
        client = get_client()

        # Find Unit Testing project
        projects = client.projects("Unit Testing")
        self.assertGreater(
            len(projects),
            0,
            "Unit Testing project should exist")
        project = projects[0]

        # Create test dataset
        dataset_id = client.create_dataset(
            str(project.id),
            "Python Mixed Sequence Test",
            "Test 2 sequences (10 frames each) + 2 non-sequence images")
        self.assertIsNotNone(dataset_id)
        assert dataset_id is not None

        # Create annotation set
        annset_id = client.create_annotation_set(
            dataset_id,
            "Test Annotations",
            None)
        self.assertIsNotNone(annset_id)
        assert annset_id is not None

        dataset_id_str = str(dataset_id)

        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)

            # ===== Sequence 1: Numbers 1-10 =====
            seq1_name = "numbers"
            seq1_uuid = generate_sequence_uuid(dataset_id_str, seq1_name)
            seq1_samples = []

            for i in range(1, 11):
                img_path = temp_path / f"{i}.jpg"
                img = create_image_with_text(str(i))
                img.save(img_path, "JPEG")

                sample = Sample()
                sample.set_sequence_name(seq1_name)
                sample.set_sequence_uuid(seq1_uuid)
                sample.set_frame_number(i)
                sample.add_file(SampleFile(
                    file_type="image",
                    filename=str(img_path)))

                seq1_samples.append(sample)

            results1 = client.populate_samples(
                dataset_id,
                annset_id,
                seq1_samples,
                None)
            self.assertEqual(
                len(results1),
                10,
                "Should upload 10 samples for sequence 1")

            # ===== Sequence 2: Letters A-J =====
            seq2_name = "letters"
            seq2_uuid = generate_sequence_uuid(dataset_id_str, seq2_name)
            seq2_samples = []

            letters = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J']
            for i, letter in enumerate(letters, 1):
                img_path = temp_path / f"{letter}.jpg"
                img = create_image_with_text(letter)
                img.save(img_path, "JPEG")

                sample = Sample()
                sample.set_sequence_name(seq2_name)
                sample.set_sequence_uuid(seq2_uuid)
                sample.set_frame_number(i)
                sample.add_file(SampleFile(
                    file_type="image",
                    filename=str(img_path)))

                seq2_samples.append(sample)

            results2 = client.populate_samples(
                dataset_id,
                annset_id,
                seq2_samples,
                None)
            self.assertEqual(
                len(results2),
                10,
                "Should upload 10 samples for sequence 2")

            # ===== Non-sequence 1: exclamation.jpg =====
            exclamation_path = temp_path / "exclamation.jpg"
            exclamation_img = create_image_with_text("!")
            exclamation_img.save(exclamation_path, "JPEG")

            exclamation_sample = Sample()
            exclamation_sample.add_file(SampleFile(
                file_type="image",
                filename=str(exclamation_path)))

            client.populate_samples(
                dataset_id,
                annset_id,
                [exclamation_sample],
                None)

            # ===== Non-sequence 2: question.png =====
            question_path = temp_path / "question.png"
            question_img = create_image_with_text("?")
            question_img.save(question_path, "PNG")

            question_sample = Sample()
            question_sample.add_file(SampleFile(
                file_type="image",
                filename=str(question_path)))

            client.populate_samples(
                dataset_id,
                annset_id,
                [question_sample],
                None)

            # ===== Verification =====
            all_samples = client.samples(
                dataset_id, None, [], [], [], None)
            self.assertEqual(
                len(all_samples),
                22,
                "Should have 22 total samples")

            # Verify sequence 1
            seq1_samples_result = [
                s for s in all_samples if s.sequence_name == seq1_name]
            self.assertEqual(
                len(seq1_samples_result),
                10,
                "Should have 10 samples in sequence 1")
            self.assertTrue(
                all(s.sequence_uuid == seq1_uuid
                    for s in seq1_samples_result),
                "All sequence 1 samples should have same UUID")
            seq1_frames = sorted(
                [s.frame_number for s in seq1_samples_result
                 if s.frame_number])
            self.assertEqual(
                seq1_frames,
                list(range(1, 11)),
                "Sequence 1 should have frames 1-10")

            # Verify sequence 2
            seq2_samples_result = [
                s for s in all_samples if s.sequence_name == seq2_name]
            self.assertEqual(
                len(seq2_samples_result),
                10,
                "Should have 10 samples in sequence 2")
            self.assertTrue(
                all(s.sequence_uuid == seq2_uuid
                    for s in seq2_samples_result),
                "All sequence 2 samples should have same UUID")
            seq2_frames = sorted(
                [s.frame_number for s in seq2_samples_result
                 if s.frame_number])
            self.assertEqual(
                seq2_frames,
                list(range(1, 11)),
                "Sequence 2 should have frames 1-10")

            # Verify non-sequence samples
            non_seq_samples = [
                s for s in all_samples if s.sequence_name is None]
            self.assertEqual(
                len(non_seq_samples),
                2,
                "Should have 2 non-sequence samples")

            for non_seq in non_seq_samples:
                self.assertIsNone(
                    non_seq.frame_number,
                    "Non-sequence should have no frame number "
                    "(server returns -1 which deserializes to None)")
                self.assertIsNone(
                    non_seq.sequence_uuid,
                    "Non-sequence should have no sequence UUID")
                self.assertIsNone(
                    non_seq.sequence_name,
                    "Non-sequence should have no sequence name")


if __name__ == '__main__':
    unittest.main()
