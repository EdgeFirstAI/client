// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

/// Tests for Sample and SampleFile types.
///
/// These tests verify Sample construction with various field combinations,
/// SampleFile creation, and related type operations.

import XCTest

@testable import EdgeFirstClient

final class SampleTests: XCTestCase {

  // MARK: - SampleId Tests

  /// Test SampleId construction.
  func testSampleIdConstruction() {
    let id = SampleId(value: 12345)
    XCTAssertEqual(id.value, 12345)
  }

  /// Test SampleId equality.
  func testSampleIdEquality() {
    let id1 = SampleId(value: 100)
    let id2 = SampleId(value: 100)
    let id3 = SampleId(value: 200)

    XCTAssertEqual(id1, id2)
    XCTAssertNotEqual(id1, id3)
  }

  /// Test SampleId hashability.
  func testSampleIdHashability() {
    var idSet: Set<SampleId> = []

    idSet.insert(SampleId(value: 1))
    idSet.insert(SampleId(value: 2))
    idSet.insert(SampleId(value: 1))  // Duplicate

    XCTAssertEqual(idSet.count, 2)
  }

  // MARK: - SampleFile Tests

  /// Test SampleFile construction with all fields.
  func testSampleFileConstruction() {
    let file = SampleFile(
      fileType: "lidar_pcd",
      url: "https://example.com/file.pcd",
      filename: "scan001.pcd"
    )

    XCTAssertEqual(file.fileType, "lidar_pcd")
    XCTAssertEqual(file.url, "https://example.com/file.pcd")
    XCTAssertEqual(file.filename, "scan001.pcd")
  }

  /// Test SampleFile with URL only (retrieved sample).
  func testSampleFileWithUrlOnly() {
    let file = SampleFile(
      fileType: "image",
      url: "https://example.com/image.jpg",
      filename: nil
    )

    XCTAssertEqual(file.fileType, "image")
    XCTAssertNotNil(file.url)
    XCTAssertNil(file.filename)
  }

  /// Test SampleFile with filename only (populating sample).
  func testSampleFileWithFilenameOnly() {
    let file = SampleFile(
      fileType: "radar_cube",
      url: nil,
      filename: "radar_data.bin"
    )

    XCTAssertEqual(file.fileType, "radar_cube")
    XCTAssertNil(file.url)
    XCTAssertEqual(file.filename, "radar_data.bin")
  }

  /// Test SampleFile equality.
  func testSampleFileEquality() {
    let file1 = SampleFile(fileType: "image", url: "http://a.com", filename: nil)
    let file2 = SampleFile(fileType: "image", url: "http://a.com", filename: nil)
    let file3 = SampleFile(fileType: "image", url: "http://b.com", filename: nil)

    XCTAssertEqual(file1, file2)
    XCTAssertNotEqual(file1, file3)
  }

  /// Test SampleFile hashability.
  func testSampleFileHashability() {
    var fileSet: Set<SampleFile> = []

    fileSet.insert(SampleFile(fileType: "image", url: nil, filename: "a.jpg"))
    fileSet.insert(SampleFile(fileType: "image", url: nil, filename: "b.jpg"))
    fileSet.insert(SampleFile(fileType: "image", url: nil, filename: "a.jpg"))  // Duplicate

    XCTAssertEqual(fileSet.count, 2)
  }

  /// Test SampleFile with various file types.
  func testSampleFileTypes() {
    let types = ["image", "lidar_pcd", "lidar_depth", "lidar_reflectance", "radar_cube"]

    for fileType in types {
      let file = SampleFile(fileType: fileType, url: nil, filename: "test.bin")
      XCTAssertEqual(file.fileType, fileType)
    }
  }

  // MARK: - Sample Minimal Construction Tests

  /// Test Sample construction with minimal fields.
  func testSampleMinimalConstruction() {
    let sample = Sample(
      id: nil,
      group: nil,
      sequenceName: nil,
      sequenceUuid: nil,
      sequenceDescription: nil,
      frameNumber: nil,
      uuid: nil,
      imageName: nil,
      imageUrl: nil,
      width: nil,
      height: nil,
      date: nil,
      source: nil,
      location: nil,
      degradation: nil,
      files: [],
      annotations: []
    )

    XCTAssertNil(sample.id)
    XCTAssertNil(sample.group)
    XCTAssertTrue(sample.files.isEmpty)
    XCTAssertTrue(sample.annotations.isEmpty)
  }

  // MARK: - Sample With Group Tests

  /// Test Sample with train group.
  func testSampleWithTrainGroup() {
    let sample = Sample(
      id: SampleId(value: 1),
      group: "train",
      sequenceName: nil,
      sequenceUuid: nil,
      sequenceDescription: nil,
      frameNumber: nil,
      uuid: nil,
      imageName: "image001.jpg",
      imageUrl: nil,
      width: 1920,
      height: 1080,
      date: nil,
      source: nil,
      location: nil,
      degradation: nil,
      files: [],
      annotations: []
    )

    XCTAssertEqual(sample.group, "train")
    XCTAssertEqual(sample.width, 1920)
    XCTAssertEqual(sample.height, 1080)
  }

  /// Test Sample with val group.
  func testSampleWithValGroup() {
    let sample = Sample(
      id: nil, group: "val", sequenceName: nil, sequenceUuid: nil,
      sequenceDescription: nil, frameNumber: nil, uuid: nil,
      imageName: nil, imageUrl: nil, width: nil, height: nil,
      date: nil, source: nil, location: nil, degradation: nil,
      files: [], annotations: []
    )

    XCTAssertEqual(sample.group, "val")
  }

  /// Test Sample with test group.
  func testSampleWithTestGroup() {
    let sample = Sample(
      id: nil, group: "test", sequenceName: nil, sequenceUuid: nil,
      sequenceDescription: nil, frameNumber: nil, uuid: nil,
      imageName: nil, imageUrl: nil, width: nil, height: nil,
      date: nil, source: nil, location: nil, degradation: nil,
      files: [], annotations: []
    )

    XCTAssertEqual(sample.group, "test")
  }

  // MARK: - Sample With Sequence Tests

  /// Test Sample with sequence information.
  func testSampleWithSequence() {
    let sample = Sample(
      id: SampleId(value: 42),
      group: "train",
      sequenceName: "driving_scene_001",
      sequenceUuid: "550e8400-e29b-41d4-a716-446655440000",
      sequenceDescription: "Highway driving sequence",
      frameNumber: 150,
      uuid: "sample-uuid-123",
      imageName: "frame_150.jpg",
      imageUrl: nil,
      width: 1920,
      height: 1080,
      date: "2024-01-15T10:30:00Z",
      source: "front_camera",
      location: nil,
      degradation: nil,
      files: [],
      annotations: []
    )

    XCTAssertEqual(sample.sequenceName, "driving_scene_001")
    XCTAssertEqual(sample.sequenceUuid, "550e8400-e29b-41d4-a716-446655440000")
    XCTAssertEqual(sample.sequenceDescription, "Highway driving sequence")
    XCTAssertEqual(sample.frameNumber, 150)
    XCTAssertEqual(sample.uuid, "sample-uuid-123")
    XCTAssertEqual(sample.date, "2024-01-15T10:30:00Z")
    XCTAssertEqual(sample.source, "front_camera")
  }

  // MARK: - Sample With Files Tests

  /// Test Sample with single file.
  func testSampleWithSingleFile() {
    let file = SampleFile(fileType: "lidar_pcd", url: nil, filename: "scan.pcd")
    let sample = Sample(
      id: nil, group: nil, sequenceName: nil, sequenceUuid: nil,
      sequenceDescription: nil, frameNumber: nil, uuid: nil,
      imageName: "image.jpg", imageUrl: nil, width: nil, height: nil,
      date: nil, source: nil, location: nil, degradation: nil,
      files: [file],
      annotations: []
    )

    XCTAssertEqual(sample.files.count, 1)
    XCTAssertEqual(sample.files[0].fileType, "lidar_pcd")
  }

  /// Test Sample with multiple files.
  func testSampleWithMultipleFiles() {
    let files = [
      SampleFile(fileType: "lidar_pcd", url: nil, filename: "scan.pcd"),
      SampleFile(fileType: "lidar_depth", url: nil, filename: "depth.png"),
      SampleFile(fileType: "radar_cube", url: nil, filename: "radar.bin"),
    ]
    let sample = Sample(
      id: nil, group: nil, sequenceName: nil, sequenceUuid: nil,
      sequenceDescription: nil, frameNumber: nil, uuid: nil,
      imageName: nil, imageUrl: nil, width: nil, height: nil,
      date: nil, source: nil, location: nil, degradation: nil,
      files: files,
      annotations: []
    )

    XCTAssertEqual(sample.files.count, 3)
  }

  // MARK: - Sample With Annotations Tests

  /// Test Sample with single annotation.
  func testSampleWithSingleAnnotation() {
    let box = Box2d(left: 100, top: 100, width: 50, height: 30)
    let annotation = Annotation(
      sampleId: nil, name: nil, sequenceName: nil, frameNumber: nil,
      group: nil, objectId: "car-1", labelName: "car", labelIndex: 0,
      box2d: box, box3d: nil, mask: nil
    )
    let sample = Sample(
      id: nil, group: nil, sequenceName: nil, sequenceUuid: nil,
      sequenceDescription: nil, frameNumber: nil, uuid: nil,
      imageName: "image.jpg", imageUrl: nil, width: 1920, height: 1080,
      date: nil, source: nil, location: nil, degradation: nil,
      files: [],
      annotations: [annotation]
    )

    XCTAssertEqual(sample.annotations.count, 1)
    XCTAssertEqual(sample.annotations[0].labelName, "car")
  }

  /// Test Sample with multiple annotations.
  func testSampleWithMultipleAnnotations() {
    let annotations = [
      Annotation(
        sampleId: nil, name: nil, sequenceName: nil, frameNumber: nil,
        group: nil, objectId: "car-1", labelName: "car", labelIndex: 0,
        box2d: Box2d(left: 100, top: 100, width: 50, height: 30),
        box3d: nil, mask: nil
      ),
      Annotation(
        sampleId: nil, name: nil, sequenceName: nil, frameNumber: nil,
        group: nil, objectId: "person-1", labelName: "person", labelIndex: 1,
        box2d: Box2d(left: 200, top: 150, width: 40, height: 80),
        box3d: nil, mask: nil
      ),
    ]
    let sample = Sample(
      id: nil, group: nil, sequenceName: nil, sequenceUuid: nil,
      sequenceDescription: nil, frameNumber: nil, uuid: nil,
      imageName: nil, imageUrl: nil, width: nil, height: nil,
      date: nil, source: nil, location: nil, degradation: nil,
      files: [],
      annotations: annotations
    )

    XCTAssertEqual(sample.annotations.count, 2)
  }

  // MARK: - Sample With Location Tests

  /// Test Sample with GPS location.
  func testSampleWithGpsLocation() {
    let gps = GpsData(lat: 37.7749, lon: -122.4194)
    let location = Location(gps: gps, imu: nil)
    let sample = Sample(
      id: nil, group: nil, sequenceName: nil, sequenceUuid: nil,
      sequenceDescription: nil, frameNumber: nil, uuid: nil,
      imageName: nil, imageUrl: nil, width: nil, height: nil,
      date: nil, source: nil, location: location, degradation: nil,
      files: [],
      annotations: []
    )

    XCTAssertNotNil(sample.location)
    if let lat = sample.location?.gps?.lat {
      XCTAssertEqual(lat, 37.7749, accuracy: 0.0001)
    } else {
      XCTFail("Expected GPS lat")
    }
  }

  /// Test Sample with IMU location.
  func testSampleWithImuLocation() {
    let imu = ImuData(roll: 0.1, pitch: 0.2, yaw: 0.3)
    let location = Location(gps: nil, imu: imu)
    let sample = Sample(
      id: nil, group: nil, sequenceName: nil, sequenceUuid: nil,
      sequenceDescription: nil, frameNumber: nil, uuid: nil,
      imageName: nil, imageUrl: nil, width: nil, height: nil,
      date: nil, source: nil, location: location, degradation: nil,
      files: [],
      annotations: []
    )

    XCTAssertNotNil(sample.location)
    if let yaw = sample.location?.imu?.yaw {
      XCTAssertEqual(yaw, 0.3, accuracy: 0.0001)
    } else {
      XCTFail("Expected IMU yaw")
    }
  }

  /// Test Sample with full location (GPS + IMU).
  func testSampleWithFullLocation() {
    let gps = GpsData(lat: 37.7749, lon: -122.4194)
    let imu = ImuData(roll: 0.1, pitch: 0.2, yaw: 0.3)
    let location = Location(gps: gps, imu: imu)
    let sample = Sample(
      id: nil, group: nil, sequenceName: nil, sequenceUuid: nil,
      sequenceDescription: nil, frameNumber: nil, uuid: nil,
      imageName: nil, imageUrl: nil, width: nil, height: nil,
      date: nil, source: nil, location: location, degradation: nil,
      files: [],
      annotations: []
    )

    XCTAssertNotNil(sample.location?.gps)
    XCTAssertNotNil(sample.location?.imu)
  }

  // MARK: - Sample With Degradation Tests

  /// Test Sample with blur degradation.
  func testSampleWithBlurDegradation() {
    let sample = Sample(
      id: nil, group: nil, sequenceName: nil, sequenceUuid: nil,
      sequenceDescription: nil, frameNumber: nil, uuid: nil,
      imageName: nil, imageUrl: nil, width: nil, height: nil,
      date: nil, source: nil, location: nil, degradation: "blur",
      files: [],
      annotations: []
    )

    XCTAssertEqual(sample.degradation, "blur")
  }

  /// Test Sample with occlusion degradation.
  func testSampleWithOcclusionDegradation() {
    let sample = Sample(
      id: nil, group: nil, sequenceName: nil, sequenceUuid: nil,
      sequenceDescription: nil, frameNumber: nil, uuid: nil,
      imageName: nil, imageUrl: nil, width: nil, height: nil,
      date: nil, source: nil, location: nil, degradation: "occlusion",
      files: [],
      annotations: []
    )

    XCTAssertEqual(sample.degradation, "occlusion")
  }

  // MARK: - Sample Equality Tests

  /// Test Sample equality.
  func testSampleEquality() {
    let sample1 = Sample(
      id: SampleId(value: 1), group: "train", sequenceName: nil, sequenceUuid: nil,
      sequenceDescription: nil, frameNumber: nil, uuid: nil,
      imageName: "image.jpg", imageUrl: nil, width: 1920, height: 1080,
      date: nil, source: nil, location: nil, degradation: nil,
      files: [], annotations: []
    )
    let sample2 = Sample(
      id: SampleId(value: 1), group: "train", sequenceName: nil, sequenceUuid: nil,
      sequenceDescription: nil, frameNumber: nil, uuid: nil,
      imageName: "image.jpg", imageUrl: nil, width: 1920, height: 1080,
      date: nil, source: nil, location: nil, degradation: nil,
      files: [], annotations: []
    )
    let sample3 = Sample(
      id: SampleId(value: 2), group: "train", sequenceName: nil, sequenceUuid: nil,
      sequenceDescription: nil, frameNumber: nil, uuid: nil,
      imageName: "image.jpg", imageUrl: nil, width: 1920, height: 1080,
      date: nil, source: nil, location: nil, degradation: nil,
      files: [], annotations: []
    )

    XCTAssertEqual(sample1, sample2)
    XCTAssertNotEqual(sample1, sample3)
  }

  /// Test Sample hashability.
  func testSampleHashability() {
    var sampleSet: Set<Sample> = []

    let sample1 = Sample(
      id: SampleId(value: 1), group: nil, sequenceName: nil, sequenceUuid: nil,
      sequenceDescription: nil, frameNumber: nil, uuid: nil,
      imageName: nil, imageUrl: nil, width: nil, height: nil,
      date: nil, source: nil, location: nil, degradation: nil,
      files: [], annotations: []
    )
    let sample2 = Sample(
      id: SampleId(value: 2), group: nil, sequenceName: nil, sequenceUuid: nil,
      sequenceDescription: nil, frameNumber: nil, uuid: nil,
      imageName: nil, imageUrl: nil, width: nil, height: nil,
      date: nil, source: nil, location: nil, degradation: nil,
      files: [], annotations: []
    )

    sampleSet.insert(sample1)
    sampleSet.insert(sample2)
    sampleSet.insert(sample1)  // Duplicate

    XCTAssertEqual(sampleSet.count, 2)
  }

  // MARK: - Complex Sample Tests

  /// Test fully populated Sample.
  func testFullyPopulatedSample() {
    let gps = GpsData(lat: 37.7749, lon: -122.4194)
    let imu = ImuData(roll: 0.0, pitch: 0.0, yaw: 1.57)
    let location = Location(gps: gps, imu: imu)

    let files = [
      SampleFile(fileType: "lidar_pcd", url: nil, filename: "scan.pcd"),
      SampleFile(fileType: "radar_cube", url: nil, filename: "radar.bin"),
    ]

    let box = Box2d(left: 100, top: 100, width: 200, height: 150)
    let annotation = Annotation(
      sampleId: nil, name: nil, sequenceName: nil, frameNumber: nil,
      group: nil, objectId: "vehicle-1", labelName: "car", labelIndex: 0,
      box2d: box, box3d: nil, mask: nil
    )

    let sample = Sample(
      id: SampleId(value: 12345),
      group: "train",
      sequenceName: "urban_drive_001",
      sequenceUuid: "uuid-123",
      sequenceDescription: "Urban driving sequence",
      frameNumber: 42,
      uuid: "sample-uuid-456",
      imageName: "frame_042.jpg",
      imageUrl: "https://storage.example.com/frame_042.jpg",
      width: 1920,
      height: 1080,
      date: "2024-06-15T14:30:00Z",
      source: "front_camera",
      location: location,
      degradation: "rain",
      files: files,
      annotations: [annotation]
    )

    // Verify all fields
    XCTAssertEqual(sample.id?.value, 12345)
    XCTAssertEqual(sample.group, "train")
    XCTAssertEqual(sample.sequenceName, "urban_drive_001")
    XCTAssertEqual(sample.frameNumber, 42)
    XCTAssertEqual(sample.imageName, "frame_042.jpg")
    XCTAssertEqual(sample.width, 1920)
    XCTAssertEqual(sample.height, 1080)
    XCTAssertEqual(sample.source, "front_camera")
    XCTAssertNotNil(sample.location?.gps)
    XCTAssertNotNil(sample.location?.imu)
    XCTAssertEqual(sample.degradation, "rain")
    XCTAssertEqual(sample.files.count, 2)
    XCTAssertEqual(sample.annotations.count, 1)
  }
}
