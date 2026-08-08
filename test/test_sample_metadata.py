#!/usr/bin/env python3
"""Credential-free tests for Sample GPS/IMU metadata and Annotation label_index.

These exercise the Python bindings added for DE-2863 without constructing a
Client, so they run on the pull-request lane. Studio round-trip coverage lives
in test_datasets.DatasetTest.test_populate_samples_location_pose_label_index_roundtrip.
"""

import unittest

from edgefirst_client import Annotation, GpsData, ImuData, Sample


class TestGpsData(unittest.TestCase):
    def test_valid_constructor(self):
        gps = GpsData(37.7749, -122.4194)
        self.assertAlmostEqual(gps.lat, 37.7749)
        self.assertAlmostEqual(gps.lon, -122.4194)

    def test_invalid_latitude_raises(self):
        with self.assertRaises(ValueError):
            GpsData(91.0, 0.0)

    def test_invalid_longitude_raises(self):
        with self.assertRaises(ValueError):
            GpsData(0.0, 181.0)


class TestImuData(unittest.TestCase):
    def test_valid_constructor(self):
        imu = ImuData(10.0, -5.0, 90.0)
        self.assertAlmostEqual(imu.roll, 10.0)
        self.assertAlmostEqual(imu.pitch, -5.0)
        self.assertAlmostEqual(imu.yaw, 90.0)

    def test_invalid_roll_raises(self):
        with self.assertRaises(ValueError):
            ImuData(181.0, 0.0, 0.0)

    def test_invalid_pitch_raises(self):
        with self.assertRaises(ValueError):
            ImuData(0.0, 91.0, 0.0)


class TestSampleLocationPose(unittest.TestCase):
    def test_set_location_and_pose_roundtrip(self):
        sample = Sample()
        sample.set_location(GpsData(37.7749, -122.4194))
        sample.set_pose(ImuData(10.0, -5.0, 90.0))

        self.assertIsNotNone(sample.location)
        self.assertIsNotNone(sample.pose)
        assert sample.location is not None
        assert sample.pose is not None
        self.assertAlmostEqual(sample.location.lat, 37.7749)
        self.assertAlmostEqual(sample.location.lon, -122.4194)
        self.assertAlmostEqual(sample.pose.roll, 10.0)
        self.assertAlmostEqual(sample.pose.pitch, -5.0)
        self.assertAlmostEqual(sample.pose.yaw, 90.0)

    def test_clear_location_and_pose(self):
        sample = Sample()
        sample.set_location(GpsData(1.0, 2.0))
        sample.set_pose(ImuData(3.0, 4.0, 5.0))
        sample.set_location(None)
        sample.set_pose(None)
        self.assertIsNone(sample.location)
        self.assertIsNone(sample.pose)


class TestAnnotationLabelIndex(unittest.TestCase):
    def test_set_label_index_roundtrip(self):
        ann = Annotation()
        ann.set_label_index(4242)
        self.assertEqual(ann.label_index, 4242)
        ann.set_label_index(None)
        self.assertIsNone(ann.label_index)


if __name__ == "__main__":
    unittest.main()
