// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

/// Tests for FileType enum.
///
/// These tests verify FileType case construction, pattern matching,
/// equality, and hashability.

import XCTest

@testable import EdgeFirstClient

final class FileTypeTests: XCTestCase {

  // MARK: - FileType Case Tests

  /// Test FileType.image case.
  func testFileTypeImage() {
    let fileType = FileType.image

    if case .image = fileType {
      // Success
    } else {
      XCTFail("Expected image")
    }
  }

  /// Test FileType.lidarPcd case.
  func testFileTypeLidarPcd() {
    let fileType = FileType.lidarPcd

    if case .lidarPcd = fileType {
      // Success
    } else {
      XCTFail("Expected lidarPcd")
    }
  }

  /// Test FileType.lidarDepth case.
  func testFileTypeLidarDepth() {
    let fileType = FileType.lidarDepth

    if case .lidarDepth = fileType {
      // Success
    } else {
      XCTFail("Expected lidarDepth")
    }
  }

  /// Test FileType.lidarReflect case.
  func testFileTypeLidarReflect() {
    let fileType = FileType.lidarReflect

    if case .lidarReflect = fileType {
      // Success
    } else {
      XCTFail("Expected lidarReflect")
    }
  }

  /// Test FileType.radarPcd case.
  func testFileTypeRadarPcd() {
    let fileType = FileType.radarPcd

    if case .radarPcd = fileType {
      // Success
    } else {
      XCTFail("Expected radarPcd")
    }
  }

  /// Test FileType.radarCube case.
  func testFileTypeRadarCube() {
    let fileType = FileType.radarCube

    if case .radarCube = fileType {
      // Success
    } else {
      XCTFail("Expected radarCube")
    }
  }

  // MARK: - FileType Equality Tests

  /// Test FileType equality for same cases.
  func testFileTypeEquality() {
    XCTAssertEqual(FileType.image, FileType.image)
    XCTAssertEqual(FileType.lidarPcd, FileType.lidarPcd)
    XCTAssertEqual(FileType.lidarDepth, FileType.lidarDepth)
    XCTAssertEqual(FileType.lidarReflect, FileType.lidarReflect)
    XCTAssertEqual(FileType.radarPcd, FileType.radarPcd)
    XCTAssertEqual(FileType.radarCube, FileType.radarCube)
  }

  /// Test FileType inequality for different cases.
  func testFileTypeInequality() {
    XCTAssertNotEqual(FileType.image, FileType.lidarPcd)
    XCTAssertNotEqual(FileType.image, FileType.lidarDepth)
    XCTAssertNotEqual(FileType.lidarPcd, FileType.radarPcd)
    XCTAssertNotEqual(FileType.lidarDepth, FileType.lidarReflect)
    XCTAssertNotEqual(FileType.radarPcd, FileType.radarCube)
  }

  // MARK: - FileType Hashability Tests

  /// Test FileType can be used in sets.
  func testFileTypeHashability() {
    var fileTypeSet: Set<FileType> = []

    fileTypeSet.insert(.image)
    fileTypeSet.insert(.lidarPcd)
    fileTypeSet.insert(.lidarDepth)
    fileTypeSet.insert(.image)  // Duplicate

    XCTAssertEqual(fileTypeSet.count, 3)
  }

  /// Test all FileTypes in set.
  func testAllFileTypesInSet() {
    let allTypes: Set<FileType> = [
      .image,
      .lidarPcd,
      .lidarDepth,
      .lidarReflect,
      .radarPcd,
      .radarCube,
    ]

    XCTAssertEqual(allTypes.count, 6)
    XCTAssertTrue(allTypes.contains(.image))
    XCTAssertTrue(allTypes.contains(.lidarPcd))
    XCTAssertTrue(allTypes.contains(.lidarDepth))
    XCTAssertTrue(allTypes.contains(.lidarReflect))
    XCTAssertTrue(allTypes.contains(.radarPcd))
    XCTAssertTrue(allTypes.contains(.radarCube))
  }

  /// Test FileType as dictionary key.
  func testFileTypeAsDictionaryKey() {
    var extensions: [FileType: String] = [:]

    extensions[.image] = ".jpg"
    extensions[.lidarPcd] = ".pcd"
    extensions[.lidarDepth] = ".png"
    extensions[.lidarReflect] = ".jpg"
    extensions[.radarPcd] = ".pcd"
    extensions[.radarCube] = ".png"

    XCTAssertEqual(extensions[.image], ".jpg")
    XCTAssertEqual(extensions[.lidarPcd], ".pcd")
    XCTAssertEqual(extensions.count, 6)
  }

  // MARK: - FileType Pattern Matching Tests

  /// Test pattern matching all FileType cases.
  func testFileTypePatternMatching() {
    let fileTypes: [FileType] = [
      .image,
      .lidarPcd,
      .lidarDepth,
      .lidarReflect,
      .radarPcd,
      .radarCube,
    ]

    var matchedCases: [String] = []

    for fileType in fileTypes {
      switch fileType {
      case .image:
        matchedCases.append("image")
      case .lidarPcd:
        matchedCases.append("lidarPcd")
      case .lidarDepth:
        matchedCases.append("lidarDepth")
      case .lidarReflect:
        matchedCases.append("lidarReflect")
      case .radarPcd:
        matchedCases.append("radarPcd")
      case .radarCube:
        matchedCases.append("radarCube")
      }
    }

    XCTAssertEqual(matchedCases.count, 6)
    XCTAssertEqual(matchedCases[0], "image")
    XCTAssertEqual(matchedCases[1], "lidarPcd")
    XCTAssertEqual(matchedCases[2], "lidarDepth")
    XCTAssertEqual(matchedCases[3], "lidarReflect")
    XCTAssertEqual(matchedCases[4], "radarPcd")
    XCTAssertEqual(matchedCases[5], "radarCube")
  }

  /// Test exhaustive switch on FileType.
  func testFileTypeExhaustiveSwitch() {
    func describeFileType(_ type: FileType) -> String {
      switch type {
      case .image:
        return "Standard image file"
      case .lidarPcd:
        return "LiDAR point cloud"
      case .lidarDepth:
        return "LiDAR depth image"
      case .lidarReflect:
        return "LiDAR reflectance image"
      case .radarPcd:
        return "Radar point cloud"
      case .radarCube:
        return "Radar cube data"
      }
    }

    XCTAssertEqual(describeFileType(.image), "Standard image file")
    XCTAssertEqual(describeFileType(.lidarPcd), "LiDAR point cloud")
    XCTAssertEqual(describeFileType(.radarCube), "Radar cube data")
  }

  // MARK: - FileType Array Tests

  /// Test FileType in array operations.
  func testFileTypeArrayOperations() {
    let sensorFiles: [FileType] = [.lidarPcd, .lidarDepth, .radarPcd]

    XCTAssertEqual(sensorFiles.count, 3)
    XCTAssertTrue(sensorFiles.contains(.lidarPcd))
    XCTAssertFalse(sensorFiles.contains(.image))

    let filteredLidar = sensorFiles.filter {
      if case .lidarPcd = $0 { return true }
      if case .lidarDepth = $0 { return true }
      if case .lidarReflect = $0 { return true }
      return false
    }

    XCTAssertEqual(filteredLidar.count, 2)
  }

  /// Test FileType grouping.
  func testFileTypeGrouping() {
    let allTypes: [FileType] = [.image, .lidarPcd, .lidarDepth, .lidarReflect, .radarPcd, .radarCube]

    let grouped = Dictionary(grouping: allTypes) { type -> String in
      switch type {
      case .image:
        return "image"
      case .lidarPcd, .lidarDepth, .lidarReflect:
        return "lidar"
      case .radarPcd, .radarCube:
        return "radar"
      }
    }

    XCTAssertEqual(grouped["image"]?.count, 1)
    XCTAssertEqual(grouped["lidar"]?.count, 3)
    XCTAssertEqual(grouped["radar"]?.count, 2)
  }
}
