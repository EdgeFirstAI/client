// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

/// Tests for annotation data types.
///
/// These tests verify Box2d, Box3d, Point2d, PolygonRing, Mask, Location,
/// and Annotation struct construction, equality, and hashability.

import XCTest

@testable import EdgeFirstClient

final class AnnotationTypeTests: XCTestCase {

  // MARK: - Box2d Tests

  /// Test Box2d construction with typical values.
  func testBox2dConstruction() {
    let box = Box2d(left: 10.0, top: 20.0, width: 100.0, height: 50.0)

    XCTAssertEqual(box.left, 10.0)
    XCTAssertEqual(box.top, 20.0)
    XCTAssertEqual(box.width, 100.0)
    XCTAssertEqual(box.height, 50.0)
  }

  /// Test Box2d equality.
  func testBox2dEquality() {
    let box1 = Box2d(left: 10.0, top: 20.0, width: 100.0, height: 50.0)
    let box2 = Box2d(left: 10.0, top: 20.0, width: 100.0, height: 50.0)
    let box3 = Box2d(left: 10.0, top: 20.0, width: 100.0, height: 51.0)

    XCTAssertEqual(box1, box2)
    XCTAssertNotEqual(box1, box3)
  }

  /// Test Box2d hashability.
  func testBox2dHashability() {
    var boxSet: Set<Box2d> = []

    let box1 = Box2d(left: 10.0, top: 20.0, width: 100.0, height: 50.0)
    let box2 = Box2d(left: 20.0, top: 30.0, width: 100.0, height: 50.0)
    let box3 = Box2d(left: 10.0, top: 20.0, width: 100.0, height: 50.0)  // Duplicate

    boxSet.insert(box1)
    boxSet.insert(box2)
    boxSet.insert(box3)

    XCTAssertEqual(boxSet.count, 2)
  }

  /// Test Box2d with zero values.
  func testBox2dZeroValues() {
    let box = Box2d(left: 0.0, top: 0.0, width: 0.0, height: 0.0)

    XCTAssertEqual(box.left, 0.0)
    XCTAssertEqual(box.top, 0.0)
    XCTAssertEqual(box.width, 0.0)
    XCTAssertEqual(box.height, 0.0)
  }

  /// Test Box2d with negative values.
  func testBox2dNegativeValues() {
    let box = Box2d(left: -10.0, top: -20.0, width: 100.0, height: 50.0)

    XCTAssertEqual(box.left, -10.0)
    XCTAssertEqual(box.top, -20.0)
  }

  /// Test Box2d with large values.
  func testBox2dLargeValues() {
    let box = Box2d(left: 10000.0, top: 20000.0, width: 4096.0, height: 2160.0)

    XCTAssertEqual(box.left, 10000.0)
    XCTAssertEqual(box.width, 4096.0)
  }

  /// Test Box2d with fractional values.
  func testBox2dFractionalValues() {
    let box = Box2d(left: 10.5, top: 20.25, width: 100.125, height: 50.875)

    XCTAssertEqual(box.left, 10.5, accuracy: 0.001)
    XCTAssertEqual(box.top, 20.25, accuracy: 0.001)
    XCTAssertEqual(box.width, 100.125, accuracy: 0.001)
    XCTAssertEqual(box.height, 50.875, accuracy: 0.001)
  }

  // MARK: - Box3d Tests

  /// Test Box3d construction with typical values.
  func testBox3dConstruction() {
    let box = Box3d(cx: 1.0, cy: 2.0, cz: 3.0, width: 4.0, height: 5.0, length: 6.0)

    XCTAssertEqual(box.cx, 1.0)
    XCTAssertEqual(box.cy, 2.0)
    XCTAssertEqual(box.cz, 3.0)
    XCTAssertEqual(box.width, 4.0)
    XCTAssertEqual(box.height, 5.0)
    XCTAssertEqual(box.length, 6.0)
  }

  /// Test Box3d equality.
  func testBox3dEquality() {
    let box1 = Box3d(cx: 1.0, cy: 2.0, cz: 3.0, width: 4.0, height: 5.0, length: 6.0)
    let box2 = Box3d(cx: 1.0, cy: 2.0, cz: 3.0, width: 4.0, height: 5.0, length: 6.0)
    let box3 = Box3d(cx: 1.0, cy: 2.0, cz: 3.0, width: 4.0, height: 5.0, length: 7.0)

    XCTAssertEqual(box1, box2)
    XCTAssertNotEqual(box1, box3)
  }

  /// Test Box3d hashability.
  func testBox3dHashability() {
    var boxSet: Set<Box3d> = []

    let box1 = Box3d(cx: 1.0, cy: 2.0, cz: 3.0, width: 4.0, height: 5.0, length: 6.0)
    let box2 = Box3d(cx: 2.0, cy: 3.0, cz: 4.0, width: 4.0, height: 5.0, length: 6.0)

    boxSet.insert(box1)
    boxSet.insert(box2)

    XCTAssertEqual(boxSet.count, 2)
  }

  /// Test Box3d with zero values.
  func testBox3dZeroValues() {
    let box = Box3d(cx: 0.0, cy: 0.0, cz: 0.0, width: 0.0, height: 0.0, length: 0.0)

    XCTAssertEqual(box.cx, 0.0)
    XCTAssertEqual(box.cy, 0.0)
    XCTAssertEqual(box.cz, 0.0)
  }

  /// Test Box3d with negative center coordinates.
  func testBox3dNegativeCenter() {
    let box = Box3d(cx: -1.0, cy: -2.0, cz: -3.0, width: 4.0, height: 5.0, length: 6.0)

    XCTAssertEqual(box.cx, -1.0)
    XCTAssertEqual(box.cy, -2.0)
    XCTAssertEqual(box.cz, -3.0)
  }

  // MARK: - Point2d Tests

  /// Test Point2d construction.
  func testPoint2dConstruction() {
    let point = Point2d(x: 100.0, y: 200.0)

    XCTAssertEqual(point.x, 100.0)
    XCTAssertEqual(point.y, 200.0)
  }

  /// Test Point2d equality.
  func testPoint2dEquality() {
    let point1 = Point2d(x: 100.0, y: 200.0)
    let point2 = Point2d(x: 100.0, y: 200.0)
    let point3 = Point2d(x: 100.0, y: 201.0)

    XCTAssertEqual(point1, point2)
    XCTAssertNotEqual(point1, point3)
  }

  /// Test Point2d hashability.
  func testPoint2dHashability() {
    var pointSet: Set<Point2d> = []

    pointSet.insert(Point2d(x: 0.0, y: 0.0))
    pointSet.insert(Point2d(x: 100.0, y: 100.0))
    pointSet.insert(Point2d(x: 0.0, y: 0.0))  // Duplicate

    XCTAssertEqual(pointSet.count, 2)
  }

  /// Test Point2d at origin.
  func testPoint2dOrigin() {
    let point = Point2d(x: 0.0, y: 0.0)

    XCTAssertEqual(point.x, 0.0)
    XCTAssertEqual(point.y, 0.0)
  }

  /// Test Point2d with negative coordinates.
  func testPoint2dNegative() {
    let point = Point2d(x: -50.0, y: -75.0)

    XCTAssertEqual(point.x, -50.0)
    XCTAssertEqual(point.y, -75.0)
  }

  // MARK: - PolygonRing Tests

  /// Test PolygonRing construction with points.
  func testPolygonRingConstruction() {
    let ring = PolygonRing(points: [
      Point2d(x: 0.0, y: 0.0),
      Point2d(x: 100.0, y: 0.0),
      Point2d(x: 100.0, y: 100.0),
      Point2d(x: 0.0, y: 100.0),
    ])

    XCTAssertEqual(ring.points.count, 4)
  }

  /// Test PolygonRing with triangle.
  func testPolygonRingTriangle() {
    let ring = PolygonRing(points: [
      Point2d(x: 50.0, y: 0.0),
      Point2d(x: 100.0, y: 100.0),
      Point2d(x: 0.0, y: 100.0),
    ])

    XCTAssertEqual(ring.points.count, 3)
    XCTAssertEqual(ring.points[0].x, 50.0)
  }

  /// Test PolygonRing with empty points.
  func testPolygonRingEmpty() {
    let ring = PolygonRing(points: [])

    XCTAssertTrue(ring.points.isEmpty)
  }

  /// Test PolygonRing equality.
  func testPolygonRingEquality() {
    let ring1 = PolygonRing(points: [
      Point2d(x: 0.0, y: 0.0),
      Point2d(x: 100.0, y: 100.0),
    ])
    let ring2 = PolygonRing(points: [
      Point2d(x: 0.0, y: 0.0),
      Point2d(x: 100.0, y: 100.0),
    ])
    let ring3 = PolygonRing(points: [
      Point2d(x: 0.0, y: 0.0),
      Point2d(x: 100.0, y: 101.0),
    ])

    XCTAssertEqual(ring1, ring2)
    XCTAssertNotEqual(ring1, ring3)
  }

  // MARK: - Mask Tests

  /// Test Mask construction with single polygon.
  func testMaskConstruction() {
    let ring = PolygonRing(points: [
      Point2d(x: 0.0, y: 0.0),
      Point2d(x: 100.0, y: 0.0),
      Point2d(x: 100.0, y: 100.0),
      Point2d(x: 0.0, y: 100.0),
    ])
    let mask = Mask(polygon: [ring])

    XCTAssertEqual(mask.polygon.count, 1)
    XCTAssertEqual(mask.polygon[0].points.count, 4)
  }

  /// Test Mask with multiple polygon rings.
  func testMaskMultipleRings() {
    let outerRing = PolygonRing(points: [
      Point2d(x: 0.0, y: 0.0),
      Point2d(x: 200.0, y: 0.0),
      Point2d(x: 200.0, y: 200.0),
      Point2d(x: 0.0, y: 200.0),
    ])
    let innerRing = PolygonRing(points: [
      Point2d(x: 50.0, y: 50.0),
      Point2d(x: 150.0, y: 50.0),
      Point2d(x: 150.0, y: 150.0),
      Point2d(x: 50.0, y: 150.0),
    ])
    let mask = Mask(polygon: [outerRing, innerRing])

    XCTAssertEqual(mask.polygon.count, 2)
  }

  /// Test Mask with empty polygon.
  func testMaskEmpty() {
    let mask = Mask(polygon: [])

    XCTAssertTrue(mask.polygon.isEmpty)
  }

  /// Test Mask equality.
  func testMaskEquality() {
    let ring = PolygonRing(points: [Point2d(x: 0.0, y: 0.0)])
    let mask1 = Mask(polygon: [ring])
    let mask2 = Mask(polygon: [ring])
    let mask3 = Mask(polygon: [])

    XCTAssertEqual(mask1, mask2)
    XCTAssertNotEqual(mask1, mask3)
  }

  /// Test Mask hashability.
  func testMaskHashability() {
    var maskSet: Set<Mask> = []

    let ring1 = PolygonRing(points: [Point2d(x: 0.0, y: 0.0)])
    let ring2 = PolygonRing(points: [Point2d(x: 100.0, y: 100.0)])

    maskSet.insert(Mask(polygon: [ring1]))
    maskSet.insert(Mask(polygon: [ring2]))
    maskSet.insert(Mask(polygon: [ring1]))  // Duplicate

    XCTAssertEqual(maskSet.count, 2)
  }

  // MARK: - GpsData Tests

  /// Test GpsData construction.
  func testGpsDataConstruction() {
    let gps = GpsData(lat: 37.7749, lon: -122.4194)

    XCTAssertEqual(gps.lat, 37.7749, accuracy: 0.0001)
    XCTAssertEqual(gps.lon, -122.4194, accuracy: 0.0001)
  }

  /// Test GpsData equality.
  func testGpsDataEquality() {
    let gps1 = GpsData(lat: 37.7749, lon: -122.4194)
    let gps2 = GpsData(lat: 37.7749, lon: -122.4194)
    let gps3 = GpsData(lat: 37.7750, lon: -122.4194)

    XCTAssertEqual(gps1, gps2)
    XCTAssertNotEqual(gps1, gps3)
  }

  /// Test GpsData hashability.
  func testGpsDataHashability() {
    var gpsSet: Set<GpsData> = []

    gpsSet.insert(GpsData(lat: 0.0, lon: 0.0))
    gpsSet.insert(GpsData(lat: 37.7749, lon: -122.4194))
    gpsSet.insert(GpsData(lat: 0.0, lon: 0.0))  // Duplicate

    XCTAssertEqual(gpsSet.count, 2)
  }

  /// Test GpsData at equator/prime meridian.
  func testGpsDataOrigin() {
    let gps = GpsData(lat: 0.0, lon: 0.0)

    XCTAssertEqual(gps.lat, 0.0)
    XCTAssertEqual(gps.lon, 0.0)
  }

  /// Test GpsData at extreme coordinates.
  func testGpsDataExtremes() {
    // North pole
    let north = GpsData(lat: 90.0, lon: 0.0)
    XCTAssertEqual(north.lat, 90.0)

    // South pole
    let south = GpsData(lat: -90.0, lon: 0.0)
    XCTAssertEqual(south.lat, -90.0)

    // International date line
    let dateline = GpsData(lat: 0.0, lon: 180.0)
    XCTAssertEqual(dateline.lon, 180.0)
  }

  // MARK: - ImuData Tests

  /// Test ImuData construction.
  func testImuDataConstruction() {
    let imu = ImuData(roll: 0.1, pitch: 0.2, yaw: 0.3)

    XCTAssertEqual(imu.roll, 0.1, accuracy: 0.0001)
    XCTAssertEqual(imu.pitch, 0.2, accuracy: 0.0001)
    XCTAssertEqual(imu.yaw, 0.3, accuracy: 0.0001)
  }

  /// Test ImuData equality.
  func testImuDataEquality() {
    let imu1 = ImuData(roll: 0.1, pitch: 0.2, yaw: 0.3)
    let imu2 = ImuData(roll: 0.1, pitch: 0.2, yaw: 0.3)
    let imu3 = ImuData(roll: 0.1, pitch: 0.2, yaw: 0.4)

    XCTAssertEqual(imu1, imu2)
    XCTAssertNotEqual(imu1, imu3)
  }

  /// Test ImuData hashability.
  func testImuDataHashability() {
    var imuSet: Set<ImuData> = []

    imuSet.insert(ImuData(roll: 0.0, pitch: 0.0, yaw: 0.0))
    imuSet.insert(ImuData(roll: 0.1, pitch: 0.2, yaw: 0.3))
    imuSet.insert(ImuData(roll: 0.0, pitch: 0.0, yaw: 0.0))  // Duplicate

    XCTAssertEqual(imuSet.count, 2)
  }

  /// Test ImuData at zero orientation.
  func testImuDataZero() {
    let imu = ImuData(roll: 0.0, pitch: 0.0, yaw: 0.0)

    XCTAssertEqual(imu.roll, 0.0)
    XCTAssertEqual(imu.pitch, 0.0)
    XCTAssertEqual(imu.yaw, 0.0)
  }

  /// Test ImuData with full rotation values.
  func testImuDataFullRotation() {
    let imu = ImuData(roll: 3.14159, pitch: 3.14159, yaw: 3.14159)

    XCTAssertEqual(imu.roll, 3.14159, accuracy: 0.00001)
    XCTAssertEqual(imu.pitch, 3.14159, accuracy: 0.00001)
    XCTAssertEqual(imu.yaw, 3.14159, accuracy: 0.00001)
  }

  /// Test ImuData with negative values.
  func testImuDataNegative() {
    let imu = ImuData(roll: -0.5, pitch: -1.0, yaw: -1.5)

    XCTAssertEqual(imu.roll, -0.5, accuracy: 0.0001)
    XCTAssertEqual(imu.pitch, -1.0, accuracy: 0.0001)
    XCTAssertEqual(imu.yaw, -1.5, accuracy: 0.0001)
  }

  // MARK: - Location Tests

  /// Test Location construction with GPS only.
  func testLocationWithGpsOnly() {
    let gps = GpsData(lat: 37.7749, lon: -122.4194)
    let location = Location(gps: gps, imu: nil)

    XCTAssertNotNil(location.gps)
    XCTAssertNil(location.imu)
    if let lat = location.gps?.lat {
      XCTAssertEqual(lat, 37.7749, accuracy: 0.0001)
    } else {
      XCTFail("Expected GPS lat")
    }
  }

  /// Test Location construction with IMU only.
  func testLocationWithImuOnly() {
    let imu = ImuData(roll: 0.1, pitch: 0.2, yaw: 0.3)
    let location = Location(gps: nil, imu: imu)

    XCTAssertNil(location.gps)
    XCTAssertNotNil(location.imu)
    if let roll = location.imu?.roll {
      XCTAssertEqual(roll, 0.1, accuracy: 0.0001)
    } else {
      XCTFail("Expected IMU roll")
    }
  }

  /// Test Location construction with both GPS and IMU.
  func testLocationWithBoth() {
    let gps = GpsData(lat: 37.7749, lon: -122.4194)
    let imu = ImuData(roll: 0.1, pitch: 0.2, yaw: 0.3)
    let location = Location(gps: gps, imu: imu)

    XCTAssertNotNil(location.gps)
    XCTAssertNotNil(location.imu)
  }

  /// Test Location construction with neither.
  func testLocationEmpty() {
    let location = Location(gps: nil, imu: nil)

    XCTAssertNil(location.gps)
    XCTAssertNil(location.imu)
  }

  /// Test Location equality.
  func testLocationEquality() {
    let gps = GpsData(lat: 37.7749, lon: -122.4194)
    let location1 = Location(gps: gps, imu: nil)
    let location2 = Location(gps: gps, imu: nil)
    let location3 = Location(gps: nil, imu: nil)

    XCTAssertEqual(location1, location2)
    XCTAssertNotEqual(location1, location3)
  }

  // MARK: - AnnotationType Tests

  /// Test AnnotationType enum cases.
  func testAnnotationTypeBox2d() {
    let type = AnnotationType.box2d
    if case .box2d = type {
      // Success
    } else {
      XCTFail("Expected box2d")
    }
  }

  /// Test AnnotationType box3d case.
  func testAnnotationTypeBox3d() {
    let type = AnnotationType.box3d
    if case .box3d = type {
      // Success
    } else {
      XCTFail("Expected box3d")
    }
  }

  /// Test AnnotationType mask case.
  func testAnnotationTypeMask() {
    let type = AnnotationType.mask
    if case .mask = type {
      // Success
    } else {
      XCTFail("Expected mask")
    }
  }

  /// Test AnnotationType equality.
  func testAnnotationTypeEquality() {
    XCTAssertEqual(AnnotationType.box2d, AnnotationType.box2d)
    XCTAssertEqual(AnnotationType.box3d, AnnotationType.box3d)
    XCTAssertEqual(AnnotationType.mask, AnnotationType.mask)
    XCTAssertNotEqual(AnnotationType.box2d, AnnotationType.box3d)
    XCTAssertNotEqual(AnnotationType.box2d, AnnotationType.mask)
  }

  // MARK: - Annotation Tests

  /// Test Annotation construction with Box2d.
  func testAnnotationWithBox2d() {
    let box = Box2d(left: 10.0, top: 20.0, width: 100.0, height: 50.0)
    let annotation = Annotation(
      sampleId: nil,
      name: "image001.jpg",
      sequenceName: nil,
      frameNumber: nil,
      group: "train",
      objectId: "obj-1",
      labelName: "car",
      labelIndex: 0,
      box2d: box,
      box3d: nil,
      mask: nil
    )

    XCTAssertEqual(annotation.name, "image001.jpg")
    XCTAssertEqual(annotation.labelName, "car")
    XCTAssertNotNil(annotation.box2d)
    XCTAssertNil(annotation.box3d)
    XCTAssertNil(annotation.mask)
  }

  /// Test Annotation construction with Box3d.
  func testAnnotationWithBox3d() {
    let box = Box3d(cx: 1.0, cy: 2.0, cz: 3.0, width: 4.0, height: 5.0, length: 6.0)
    let annotation = Annotation(
      sampleId: nil,
      name: "lidar001.pcd",
      sequenceName: "sequence1",
      frameNumber: 42,
      group: "val",
      objectId: "obj-2",
      labelName: "pedestrian",
      labelIndex: 1,
      box2d: nil,
      box3d: box,
      mask: nil
    )

    XCTAssertEqual(annotation.sequenceName, "sequence1")
    XCTAssertEqual(annotation.frameNumber, 42)
    XCTAssertNotNil(annotation.box3d)
    XCTAssertEqual(annotation.box3d?.cx, 1.0)
  }

  /// Test Annotation construction with Mask.
  func testAnnotationWithMask() {
    let ring = PolygonRing(points: [
      Point2d(x: 0.0, y: 0.0),
      Point2d(x: 100.0, y: 0.0),
      Point2d(x: 100.0, y: 100.0),
    ])
    let mask = Mask(polygon: [ring])
    let annotation = Annotation(
      sampleId: nil,
      name: "image002.jpg",
      sequenceName: nil,
      frameNumber: nil,
      group: "test",
      objectId: "obj-3",
      labelName: "road",
      labelIndex: 2,
      box2d: nil,
      box3d: nil,
      mask: mask
    )

    XCTAssertNotNil(annotation.mask)
    XCTAssertEqual(annotation.mask?.polygon.count, 1)
  }

  /// Test Annotation with minimal fields.
  func testAnnotationMinimal() {
    let annotation = Annotation(
      sampleId: nil,
      name: nil,
      sequenceName: nil,
      frameNumber: nil,
      group: nil,
      objectId: nil,
      labelName: nil,
      labelIndex: nil,
      box2d: nil,
      box3d: nil,
      mask: nil
    )

    XCTAssertNil(annotation.name)
    XCTAssertNil(annotation.labelName)
    XCTAssertNil(annotation.box2d)
  }

  /// Test Annotation equality.
  func testAnnotationEquality() {
    let box = Box2d(left: 10.0, top: 20.0, width: 100.0, height: 50.0)
    let ann1 = Annotation(
      sampleId: nil, name: "test.jpg", sequenceName: nil, frameNumber: nil,
      group: nil, objectId: nil, labelName: "car", labelIndex: 0,
      box2d: box, box3d: nil, mask: nil
    )
    let ann2 = Annotation(
      sampleId: nil, name: "test.jpg", sequenceName: nil, frameNumber: nil,
      group: nil, objectId: nil, labelName: "car", labelIndex: 0,
      box2d: box, box3d: nil, mask: nil
    )
    let ann3 = Annotation(
      sampleId: nil, name: "test.jpg", sequenceName: nil, frameNumber: nil,
      group: nil, objectId: nil, labelName: "truck", labelIndex: 1,
      box2d: box, box3d: nil, mask: nil
    )

    XCTAssertEqual(ann1, ann2)
    XCTAssertNotEqual(ann1, ann3)
  }
}
