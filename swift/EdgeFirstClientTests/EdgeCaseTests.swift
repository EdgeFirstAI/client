// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

/// Tests for edge cases and boundary conditions.
///
/// These tests verify that types handle extreme values, empty strings,
/// unicode characters, and other edge cases correctly.

import XCTest

@testable import EdgeFirstClient

final class EdgeCaseTests: XCTestCase {

  // MARK: - Parameter Edge Case Tests

  /// Test Parameter with empty string.
  func testParameterWithEmptyString() {
    let param = Parameter.string(value: "")

    if case .string(let value) = param {
      XCTAssertTrue(value.isEmpty)
    } else {
      XCTFail("Expected string parameter")
    }
  }

  /// Test Parameter with unicode string.
  func testParameterWithUnicodeString() {
    let unicodeString = "Hello 世界 🌍 مرحبا"
    let param = Parameter.string(value: unicodeString)

    if case .string(let value) = param {
      XCTAssertEqual(value, unicodeString)
    } else {
      XCTFail("Expected string parameter")
    }
  }

  /// Test Parameter with very long string.
  func testParameterWithVeryLongString() {
    let longString = String(repeating: "x", count: 100_000)
    let param = Parameter.string(value: longString)

    if case .string(let value) = param {
      XCTAssertEqual(value.count, 100_000)
    } else {
      XCTFail("Expected string parameter")
    }
  }

  /// Test Parameter integer with minimum value.
  func testParameterIntegerMinValue() {
    let param = Parameter.integer(value: Int64.min)

    if case .integer(let value) = param {
      XCTAssertEqual(value, Int64.min)
    } else {
      XCTFail("Expected integer parameter")
    }
  }

  /// Test Parameter integer with maximum value.
  func testParameterIntegerMaxValue() {
    let param = Parameter.integer(value: Int64.max)

    if case .integer(let value) = param {
      XCTAssertEqual(value, Int64.max)
    } else {
      XCTFail("Expected integer parameter")
    }
  }

  /// Test Parameter integer with zero.
  func testParameterIntegerZero() {
    let param = Parameter.integer(value: 0)

    if case .integer(let value) = param {
      XCTAssertEqual(value, 0)
    } else {
      XCTFail("Expected integer parameter")
    }
  }

  /// Test Parameter real with very small value.
  func testParameterRealVerySmall() {
    let param = Parameter.real(value: Double.leastNormalMagnitude)

    if case .real(let value) = param {
      XCTAssertEqual(value, Double.leastNormalMagnitude)
    } else {
      XCTFail("Expected real parameter")
    }
  }

  /// Test Parameter real with very large value.
  func testParameterRealVeryLarge() {
    let param = Parameter.real(value: Double.greatestFiniteMagnitude)

    if case .real(let value) = param {
      XCTAssertEqual(value, Double.greatestFiniteMagnitude)
    } else {
      XCTFail("Expected real parameter")
    }
  }

  /// Test Parameter real with negative infinity.
  func testParameterRealNegativeInfinity() {
    let param = Parameter.real(value: -.infinity)

    if case .real(let value) = param {
      XCTAssertTrue(value.isInfinite)
      XCTAssertTrue(value < 0)
    } else {
      XCTFail("Expected real parameter")
    }
  }

  /// Test Parameter real with positive infinity.
  func testParameterRealPositiveInfinity() {
    let param = Parameter.real(value: .infinity)

    if case .real(let value) = param {
      XCTAssertTrue(value.isInfinite)
      XCTAssertTrue(value > 0)
    } else {
      XCTFail("Expected real parameter")
    }
  }

  /// Test Parameter real with NaN.
  func testParameterRealNaN() {
    let param = Parameter.real(value: .nan)

    if case .real(let value) = param {
      XCTAssertTrue(value.isNaN)
    } else {
      XCTFail("Expected real parameter")
    }
  }

  /// Test Parameter array with empty array.
  func testParameterEmptyArray() {
    let param = Parameter.array(values: [])

    if case .array(let values) = param {
      XCTAssertTrue(values.isEmpty)
    } else {
      XCTFail("Expected array parameter")
    }
  }

  /// Test Parameter array with large array.
  func testParameterLargeArray() {
    let largeArray = (0..<1000).map { Parameter.integer(value: Int64($0)) }
    let param = Parameter.array(values: largeArray)

    if case .array(let values) = param {
      XCTAssertEqual(values.count, 1000)
    } else {
      XCTFail("Expected array parameter")
    }
  }

  /// Test Parameter object with empty dictionary.
  func testParameterEmptyObject() {
    let param = Parameter.object(entries: [:])

    if case .object(let entries) = param {
      XCTAssertTrue(entries.isEmpty)
    } else {
      XCTFail("Expected object parameter")
    }
  }

  /// Test Parameter object with unicode keys.
  func testParameterObjectUnicodeKeys() {
    let unicodeDict: [String: Parameter] = [
      "键": .string(value: "value1"),
      "مفتاح": .string(value: "value2"),
      "🔑": .string(value: "value3"),
    ]
    let param = Parameter.object(entries: unicodeDict)

    if case .object(let entries) = param {
      XCTAssertEqual(entries.count, 3)
      XCTAssertNotNil(entries["键"])
      XCTAssertNotNil(entries["مفتاح"])
      XCTAssertNotNil(entries["🔑"])
    } else {
      XCTFail("Expected object parameter")
    }
  }

  // MARK: - ID Edge Case Tests

  /// Test ID with zero value.
  func testIdZeroValue() {
    let id = ProjectId(value: 0)
    XCTAssertEqual(id.value, 0)
  }

  /// Test ID with maximum UInt64 value.
  func testIdMaxValue() {
    let id = ProjectId(value: UInt64.max)
    XCTAssertEqual(id.value, UInt64.max)
  }

  /// Test multiple ID types with same value.
  func testDifferentIdTypesWithSameValue() {
    let projectId = ProjectId(value: 100)
    let datasetId = DatasetId(value: 100)
    let experimentId = ExperimentId(value: 100)

    // Same numeric value but different types
    XCTAssertEqual(projectId.value, datasetId.value)
    XCTAssertEqual(datasetId.value, experimentId.value)
  }

  // MARK: - Box2d Edge Case Tests

  /// Test Box2d with zero dimensions.
  func testBox2dZeroDimensions() {
    let box = Box2d(left: 100.0, top: 100.0, width: 0.0, height: 0.0)

    XCTAssertEqual(box.width, 0.0)
    XCTAssertEqual(box.height, 0.0)
  }

  /// Test Box2d with negative coordinates.
  func testBox2dNegativeCoordinates() {
    let box = Box2d(left: -50.0, top: -50.0, width: 100.0, height: 100.0)

    XCTAssertEqual(box.left, -50.0)
    XCTAssertEqual(box.top, -50.0)
  }

  /// Test Box2d with very large coordinates.
  func testBox2dVeryLargeCoordinates() {
    let largeValue: Float = 1e15
    let box = Box2d(left: largeValue, top: largeValue, width: largeValue, height: largeValue)

    XCTAssertEqual(box.left, largeValue)
    XCTAssertEqual(box.top, largeValue)
    XCTAssertEqual(box.width, largeValue)
    XCTAssertEqual(box.height, largeValue)
  }

  /// Test Box2d with very small (epsilon) dimensions.
  func testBox2dVerySmallDimensions() {
    let epsilon: Float = 1e-15
    let box = Box2d(left: 0.0, top: 0.0, width: epsilon, height: epsilon)

    XCTAssertEqual(box.width, epsilon)
    XCTAssertEqual(box.height, epsilon)
  }

  // MARK: - Box3d Edge Case Tests

  /// Test Box3d with zero dimensions.
  func testBox3dZeroDimensions() {
    let box = Box3d(
      cx: 0.0, cy: 0.0, cz: 0.0,
      width: 0.0, height: 0.0, length: 0.0
    )

    XCTAssertEqual(box.width, 0.0)
    XCTAssertEqual(box.height, 0.0)
    XCTAssertEqual(box.length, 0.0)
  }

  /// Test Box3d with negative center coordinates.
  func testBox3dNegativeCoordinates() {
    let box = Box3d(
      cx: -10.0, cy: -20.0, cz: -30.0,
      width: 5.0, height: 5.0, length: 5.0
    )

    XCTAssertEqual(box.cx, -10.0)
    XCTAssertEqual(box.cy, -20.0)
    XCTAssertEqual(box.cz, -30.0)
  }

  /// Test Box3d with very large coordinates.
  func testBox3dVeryLargeCoordinates() {
    let largeValue: Float = 1e10
    let box = Box3d(
      cx: largeValue, cy: largeValue, cz: largeValue,
      width: largeValue, height: largeValue, length: largeValue
    )

    XCTAssertEqual(box.cx, largeValue)
    XCTAssertEqual(box.cy, largeValue)
    XCTAssertEqual(box.cz, largeValue)
  }

  // MARK: - Location Edge Case Tests

  /// Test Location at equator/prime meridian intersection.
  func testLocationEquatorPrimeMeridian() {
    let gps = GpsData(lat: 0.0, lon: 0.0)
    let location = Location(gps: gps, imu: nil)

    XCTAssertEqual(location.gps?.lat, 0.0)
    XCTAssertEqual(location.gps?.lon, 0.0)
  }

  /// Test Location at extreme latitudes.
  func testLocationExtremeLatitudes() {
    let northPole = GpsData(lat: 90.0, lon: 0.0)
    let southPole = GpsData(lat: -90.0, lon: 0.0)

    let locNorth = Location(gps: northPole, imu: nil)
    let locSouth = Location(gps: southPole, imu: nil)

    XCTAssertEqual(locNorth.gps?.lat, 90.0)
    XCTAssertEqual(locSouth.gps?.lat, -90.0)
  }

  /// Test Location at international date line.
  func testLocationDateLine() {
    let eastDateLine = GpsData(lat: 0.0, lon: 180.0)
    let westDateLine = GpsData(lat: 0.0, lon: -180.0)

    let locEast = Location(gps: eastDateLine, imu: nil)
    let locWest = Location(gps: westDateLine, imu: nil)

    XCTAssertEqual(locEast.gps?.lon, 180.0)
    XCTAssertEqual(locWest.gps?.lon, -180.0)
  }

  // MARK: - GpsData Edge Case Tests

  /// Test GpsData at origin.
  func testGpsDataOrigin() {
    let gps = GpsData(lat: 0.0, lon: 0.0)

    XCTAssertEqual(gps.lat, 0.0)
    XCTAssertEqual(gps.lon, 0.0)
  }

  /// Test GpsData with extreme coordinates.
  func testGpsDataExtremeCoordinates() {
    let gps = GpsData(lat: 90.0, lon: 180.0)

    XCTAssertEqual(gps.lat, 90.0)
    XCTAssertEqual(gps.lon, 180.0)
  }

  /// Test GpsData with negative coordinates.
  func testGpsDataNegativeCoordinates() {
    let gps = GpsData(lat: -45.0, lon: -122.0)

    XCTAssertEqual(gps.lat, -45.0)
    XCTAssertEqual(gps.lon, -122.0)
  }

  // MARK: - ImuData Edge Case Tests

  /// Test ImuData at zero orientation.
  func testImuDataZeroOrientation() {
    let imu = ImuData(roll: 0.0, pitch: 0.0, yaw: 0.0)

    XCTAssertEqual(imu.roll, 0.0)
    XCTAssertEqual(imu.pitch, 0.0)
    XCTAssertEqual(imu.yaw, 0.0)
  }

  /// Test ImuData with PI values.
  func testImuDataPiValues() {
    let imu = ImuData(roll: .pi, pitch: .pi / 2, yaw: 2 * .pi)

    XCTAssertEqual(imu.roll, .pi, accuracy: 0.0001)
    XCTAssertEqual(imu.pitch, .pi / 2, accuracy: 0.0001)
    XCTAssertEqual(imu.yaw, 2 * .pi, accuracy: 0.0001)
  }

  /// Test ImuData with negative angles.
  func testImuDataNegativeAngles() {
    let imu = ImuData(roll: -.pi, pitch: -.pi / 2, yaw: -.pi)

    XCTAssertEqual(imu.roll, -.pi, accuracy: 0.0001)
    XCTAssertEqual(imu.pitch, -.pi / 2, accuracy: 0.0001)
    XCTAssertEqual(imu.yaw, -.pi, accuracy: 0.0001)
  }

  // MARK: - Label Edge Case Tests

  /// Test Label with zero ID.
  func testLabelZeroId() {
    let label = Label(id: 0, name: "background")
    XCTAssertEqual(label.id, 0)
  }

  /// Test Label with maximum UInt64 ID.
  func testLabelMaxId() {
    let label = Label(id: UInt64.max, name: "label")
    XCTAssertEqual(label.id, UInt64.max)
  }

  /// Test Label with unicode name.
  func testLabelUnicodeName() {
    let label = Label(id: 1, name: "人 🚗 車")
    XCTAssertEqual(label.name, "人 🚗 車")
  }

  /// Test Label with whitespace name.
  func testLabelWhitespaceName() {
    let label = Label(id: 1, name: "   ")
    XCTAssertEqual(label.name, "   ")
  }

  /// Test Label with newline in name.
  func testLabelNewlineName() {
    let label = Label(id: 1, name: "line1\nline2")
    XCTAssertTrue(label.name.contains("\n"))
  }

  // MARK: - Stage Edge Case Tests

  /// Test Stage with maximum percentage.
  func testStageMaxPercentage() {
    let stage = Stage(stage: "complete", status: "done", message: nil, percentage: 100)
    XCTAssertEqual(stage.percentage, 100)
  }

  /// Test Stage with percentage over 100.
  func testStageOverMaxPercentage() {
    // Some systems might allow > 100%
    let stage = Stage(stage: "overdrive", status: nil, message: nil, percentage: 150)
    XCTAssertEqual(stage.percentage, 150)
  }

  /// Test Stage with zero percentage.
  func testStageZeroPercentage() {
    let stage = Stage(stage: "error", status: nil, message: nil, percentage: 0)
    XCTAssertEqual(stage.percentage, 0)
  }

  /// Test Stage with max UInt8 percentage.
  func testStageMaxPercentage255() {
    // UInt8 max value
    let stage = Stage(stage: "max", status: nil, message: nil, percentage: UInt8.max)
    XCTAssertEqual(stage.percentage, 255)
  }

  // MARK: - ClientError Edge Case Tests

  /// Test ClientError with empty message.
  func testClientErrorEmptyMessage() {
    let error = ClientError.InternalError(message: "")
    if case .InternalError(let msg) = error {
      XCTAssertTrue(msg.isEmpty)
    } else {
      XCTFail("Expected InternalError")
    }
  }

  /// Test ClientError with null character in message.
  func testClientErrorNullCharacter() {
    let error = ClientError.NetworkError(message: "before\0after")
    if case .NetworkError(let msg) = error {
      XCTAssertTrue(msg.contains("\0"))
    } else {
      XCTFail("Expected NetworkError")
    }
  }

  /// Test ClientError with very long message.
  func testClientErrorVeryLongMessage() {
    let longMessage = String(repeating: "error ", count: 10_000)
    let error = ClientError.StorageError(message: longMessage)
    if case .StorageError(let msg) = error {
      XCTAssertTrue(msg.count > 50_000)
    } else {
      XCTFail("Expected StorageError")
    }
  }

  // MARK: - Point2d Edge Case Tests

  /// Test Point2d at origin.
  func testPoint2dOrigin() {
    let point = Point2d(x: 0.0, y: 0.0)
    XCTAssertEqual(point.x, 0.0)
    XCTAssertEqual(point.y, 0.0)
  }

  /// Test Point2d with very large coordinates.
  func testPoint2dVeryLarge() {
    let point = Point2d(x: 1e15, y: 1e15)
    XCTAssertEqual(point.x, 1e15)
    XCTAssertEqual(point.y, 1e15)
  }

  /// Test Point2d with negative coordinates.
  func testPoint2dNegative() {
    let point = Point2d(x: -1000.0, y: -2000.0)
    XCTAssertEqual(point.x, -1000.0)
    XCTAssertEqual(point.y, -2000.0)
  }

  // MARK: - Mask Edge Case Tests

  /// Test Mask with empty polygon array.
  func testMaskEmptyPolygon() {
    let mask = Mask(polygon: [])

    XCTAssertTrue(mask.polygon.isEmpty)
  }

  /// Test Mask with single point polygon.
  func testMaskSinglePointPolygon() {
    let point = Point2d(x: 100.0, y: 100.0)
    let ring = PolygonRing(points: [point])
    let mask = Mask(polygon: [ring])

    XCTAssertEqual(mask.polygon.count, 1)
    XCTAssertEqual(mask.polygon[0].points.count, 1)
  }

  /// Test Mask with complex polygon.
  func testMaskComplexPolygon() {
    // Create a triangle
    let points = [
      Point2d(x: 0.0, y: 0.0),
      Point2d(x: 100.0, y: 0.0),
      Point2d(x: 50.0, y: 100.0),
    ]
    let ring = PolygonRing(points: points)
    let mask = Mask(polygon: [ring])

    XCTAssertEqual(mask.polygon.count, 1)
    XCTAssertEqual(mask.polygon[0].points.count, 3)
  }

  /// Test Mask with multiple polygon rings.
  func testMaskMultipleRings() {
    let ring1 = PolygonRing(points: [Point2d(x: 0.0, y: 0.0), Point2d(x: 10.0, y: 10.0)])
    let ring2 = PolygonRing(points: [Point2d(x: 20.0, y: 20.0), Point2d(x: 30.0, y: 30.0)])
    let mask = Mask(polygon: [ring1, ring2])

    XCTAssertEqual(mask.polygon.count, 2)
  }

  // MARK: - Collection Type Tests

  /// Test array of mixed Parameter types.
  func testMixedParameterArray() {
    let mixedArray: [Parameter] = [
      .integer(value: 42),
      .real(value: 3.14),
      .boolean(value: true),
      .string(value: "hello"),
      .array(values: [.integer(value: 1), .integer(value: 2)]),
      .object(entries: ["key": .string(value: "value")]),
    ]

    XCTAssertEqual(mixedArray.count, 6)
  }

  /// Test deeply nested Parameter structure.
  func testDeeplyNestedParameter() {
    // Create 10 levels of nesting
    var nested: Parameter = .integer(value: 42)
    for _ in 0..<10 {
      nested = .array(values: [nested])
    }

    // Verify we can access the structure
    if case .array(let arr) = nested {
      XCTAssertEqual(arr.count, 1)
    } else {
      XCTFail("Expected array at top level")
    }
  }

  /// Test Parameter object with many keys.
  func testParameterManyKeys() {
    var dict: [String: Parameter] = [:]
    for i in 0..<1000 {
      dict["key_\(i)"] = .integer(value: Int64(i))
    }

    let param = Parameter.object(entries: dict)

    if case .object(let entries) = param {
      XCTAssertEqual(entries.count, 1000)
    } else {
      XCTFail("Expected object parameter")
    }
  }
}
