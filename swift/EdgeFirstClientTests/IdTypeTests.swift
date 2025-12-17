// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

/// Tests for all ID types in the EdgeFirst Client SDK.
///
/// These tests verify construction, equality, hashability, and edge cases
/// for all identifier types: ProjectId, ExperimentId, DatasetId, etc.

import XCTest

@testable import EdgeFirstClient

final class IdTypeTests: XCTestCase {

  // MARK: - ProjectId Tests

  /// Test ProjectId construction.
  func testProjectIdConstruction() {
    let id = ProjectId(value: 12345)
    XCTAssertEqual(id.value, 12345)
  }

  /// Test ProjectId equality.
  func testProjectIdEquality() {
    let id1 = ProjectId(value: 100)
    let id2 = ProjectId(value: 100)
    let id3 = ProjectId(value: 200)

    XCTAssertEqual(id1, id2)
    XCTAssertNotEqual(id1, id3)
  }

  /// Test ProjectId hashability.
  func testProjectIdHashability() {
    var idSet: Set<ProjectId> = []

    idSet.insert(ProjectId(value: 100))
    idSet.insert(ProjectId(value: 200))
    idSet.insert(ProjectId(value: 100))  // Duplicate

    XCTAssertEqual(idSet.count, 2)
  }

  /// Test ProjectId as dictionary key.
  func testProjectIdAsDictionaryKey() {
    var projectNames: [ProjectId: String] = [:]

    let id1 = ProjectId(value: 100)
    let id2 = ProjectId(value: 200)

    projectNames[id1] = "Project Alpha"
    projectNames[id2] = "Project Beta"

    XCTAssertEqual(projectNames[id1], "Project Alpha")
    XCTAssertEqual(projectNames[id2], "Project Beta")
  }

  /// Test ProjectId with zero value.
  func testProjectIdZero() {
    let id = ProjectId(value: 0)
    XCTAssertEqual(id.value, 0)
  }

  /// Test ProjectId with max UInt64 value.
  func testProjectIdMaxValue() {
    let id = ProjectId(value: UInt64.max)
    XCTAssertEqual(id.value, UInt64.max)
  }

  // MARK: - ExperimentId Tests

  /// Test ExperimentId construction.
  func testExperimentIdConstruction() {
    let id = ExperimentId(value: 54321)
    XCTAssertEqual(id.value, 54321)
  }

  /// Test ExperimentId equality.
  func testExperimentIdEquality() {
    let id1 = ExperimentId(value: 100)
    let id2 = ExperimentId(value: 100)
    let id3 = ExperimentId(value: 200)

    XCTAssertEqual(id1, id2)
    XCTAssertNotEqual(id1, id3)
  }

  /// Test ExperimentId hashability.
  func testExperimentIdHashability() {
    var idSet: Set<ExperimentId> = []

    idSet.insert(ExperimentId(value: 100))
    idSet.insert(ExperimentId(value: 200))
    idSet.insert(ExperimentId(value: 100))  // Duplicate

    XCTAssertEqual(idSet.count, 2)
  }

  /// Test ExperimentId as dictionary key.
  func testExperimentIdAsDictionaryKey() {
    var experimentNames: [ExperimentId: String] = [:]

    let id1 = ExperimentId(value: 100)
    let id2 = ExperimentId(value: 200)

    experimentNames[id1] = "YOLOv8 Training"
    experimentNames[id2] = "ResNet Experiment"

    XCTAssertEqual(experimentNames[id1], "YOLOv8 Training")
    XCTAssertEqual(experimentNames[id2], "ResNet Experiment")
  }

  /// Test ExperimentId with zero value.
  func testExperimentIdZero() {
    let id = ExperimentId(value: 0)
    XCTAssertEqual(id.value, 0)
  }

  /// Test ExperimentId with max UInt64 value.
  func testExperimentIdMaxValue() {
    let id = ExperimentId(value: UInt64.max)
    XCTAssertEqual(id.value, UInt64.max)
  }

  // MARK: - OrganizationId Tests

  /// Test OrganizationId construction.
  func testOrganizationIdConstruction() {
    let id = OrganizationId(value: 99999)
    XCTAssertEqual(id.value, 99999)
  }

  /// Test OrganizationId equality.
  func testOrganizationIdEquality() {
    let id1 = OrganizationId(value: 100)
    let id2 = OrganizationId(value: 100)
    let id3 = OrganizationId(value: 200)

    XCTAssertEqual(id1, id2)
    XCTAssertNotEqual(id1, id3)
  }

  /// Test OrganizationId hashability.
  func testOrganizationIdHashability() {
    var idSet: Set<OrganizationId> = []

    idSet.insert(OrganizationId(value: 100))
    idSet.insert(OrganizationId(value: 200))
    idSet.insert(OrganizationId(value: 100))  // Duplicate

    XCTAssertEqual(idSet.count, 2)
  }

  /// Test OrganizationId as dictionary key.
  func testOrganizationIdAsDictionaryKey() {
    var orgNames: [OrganizationId: String] = [:]

    let id1 = OrganizationId(value: 100)
    let id2 = OrganizationId(value: 200)

    orgNames[id1] = "Acme Corp"
    orgNames[id2] = "Tech Inc"

    XCTAssertEqual(orgNames[id1], "Acme Corp")
    XCTAssertEqual(orgNames[id2], "Tech Inc")
  }

  /// Test OrganizationId with zero value.
  func testOrganizationIdZero() {
    let id = OrganizationId(value: 0)
    XCTAssertEqual(id.value, 0)
  }

  /// Test OrganizationId with max UInt64 value.
  func testOrganizationIdMaxValue() {
    let id = OrganizationId(value: UInt64.max)
    XCTAssertEqual(id.value, UInt64.max)
  }

  // MARK: - ID Type Cross Comparison Tests

  /// Test that different ID types with same value are not comparable.
  func testDifferentIdTypesNotComparable() {
    // These should be different types, so this tests type safety
    let projectId = ProjectId(value: 100)
    let experimentId = ExperimentId(value: 100)
    let datasetId = DatasetId(value: 100)
    let snapshotId = SnapshotId(value: 100)

    // They can't be compared directly (type mismatch), but we can verify they exist
    XCTAssertEqual(projectId.value, 100)
    XCTAssertEqual(experimentId.value, 100)
    XCTAssertEqual(datasetId.value, 100)
    XCTAssertEqual(snapshotId.value, 100)
  }

  /// Test IDs in mixed collections.
  func testIdsInMixedCollections() {
    var projectSet: Set<ProjectId> = []
    var experimentSet: Set<ExperimentId> = []
    var datasetSet: Set<DatasetId> = []

    // Add same values to different sets
    projectSet.insert(ProjectId(value: 100))
    experimentSet.insert(ExperimentId(value: 100))
    datasetSet.insert(DatasetId(value: 100))

    // Each set should have 1 element
    XCTAssertEqual(projectSet.count, 1)
    XCTAssertEqual(experimentSet.count, 1)
    XCTAssertEqual(datasetSet.count, 1)
  }

  // MARK: - ID Value Range Tests

  /// Test various ID values.
  func testIdValueRanges() {
    let values: [UInt64] = [0, 1, 100, 1000, 10000, 100000, UInt64.max / 2, UInt64.max]

    for value in values {
      let projectId = ProjectId(value: value)
      let experimentId = ExperimentId(value: value)
      let datasetId = DatasetId(value: value)

      XCTAssertEqual(projectId.value, value)
      XCTAssertEqual(experimentId.value, value)
      XCTAssertEqual(datasetId.value, value)
    }
  }

  // MARK: - ID Hash Value Tests

  /// Test that equal IDs have the same hash.
  func testEqualIdsHaveSameHash() {
    let id1 = ProjectId(value: 100)
    let id2 = ProjectId(value: 100)

    XCTAssertEqual(id1.hashValue, id2.hashValue)
  }

  /// Test that different IDs typically have different hashes.
  func testDifferentIdsHaveDifferentHashes() {
    let id1 = ProjectId(value: 100)
    let id2 = ProjectId(value: 200)

    // While hash collisions are possible, for sequential values they should differ
    XCTAssertNotEqual(id1.hashValue, id2.hashValue)
  }

  // MARK: - SampleId Tests

  /// Test SampleId construction.
  func testSampleIdConstruction() {
    let id = SampleId(value: 77777)
    XCTAssertEqual(id.value, 77777)
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

    idSet.insert(SampleId(value: 100))
    idSet.insert(SampleId(value: 200))
    idSet.insert(SampleId(value: 100))  // Duplicate

    XCTAssertEqual(idSet.count, 2)
  }

  // MARK: - AnnotationSetId Tests (Additional)

  /// Test AnnotationSetId as dictionary key.
  func testAnnotationSetIdAsDictionaryKey() {
    var setNames: [AnnotationSetId: String] = [:]

    let id1 = AnnotationSetId(value: 100)
    let id2 = AnnotationSetId(value: 200)

    setNames[id1] = "Ground Truth"
    setNames[id2] = "Predictions"

    XCTAssertEqual(setNames[id1], "Ground Truth")
    XCTAssertEqual(setNames[id2], "Predictions")
  }

  /// Test AnnotationSetId with zero value.
  func testAnnotationSetIdZero() {
    let id = AnnotationSetId(value: 0)
    XCTAssertEqual(id.value, 0)
  }

  /// Test AnnotationSetId with max UInt64 value.
  func testAnnotationSetIdMaxValue() {
    let id = AnnotationSetId(value: UInt64.max)
    XCTAssertEqual(id.value, UInt64.max)
  }

  // MARK: - All IDs Sendable Tests

  /// Test that all ID types are Sendable (can be used across concurrency domains).
  func testIdsSendableCompliance() async {
    // Create IDs on main task
    let projectId = ProjectId(value: 100)
    let experimentId = ExperimentId(value: 200)
    let datasetId = DatasetId(value: 300)
    let snapshotId = SnapshotId(value: 400)
    let taskIdVal = TaskId(value: 500)

    // Verify Sendable by using in async context
    // All ID types should be usable across concurrency boundaries
    let result =
      projectId.value + experimentId.value + datasetId.value + snapshotId.value + taskIdVal.value

    XCTAssertEqual(result, 100 + 200 + 300 + 400 + 500)
  }
}
