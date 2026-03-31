// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

/// Tests for ID types.
///
/// These tests verify ID type value properties, equality, and that IDs retrieved
/// from the API have valid values. Matches Python test patterns in test_ids.py.

import XCTest

@testable import EdgeFirstClient

final class IDFormatTests: XCTestCase {

  // MARK: - ID Value Property Tests (Offline)

  /// Test OrganizationId value property.
  func testOrganizationIdValue() {
    let id = OrganizationId(value: 12345)
    XCTAssertEqual(id.value, 12345)
  }

  /// Test ProjectId value property.
  func testProjectIdValue() {
    let id = ProjectId(value: 42)
    XCTAssertEqual(id.value, 42)
  }

  /// Test DatasetId value property.
  func testDatasetIdValue() {
    let id = DatasetId(value: 1000)
    XCTAssertEqual(id.value, 1000)
  }

  /// Test ExperimentId value property.
  func testExperimentIdValue() {
    let id = ExperimentId(value: 500)
    XCTAssertEqual(id.value, 500)
  }

  /// Test AnnotationSetId value property.
  func testAnnotationSetIdValue() {
    let id = AnnotationSetId(value: 789)
    XCTAssertEqual(id.value, 789)
  }

  /// Test SampleId value property.
  func testSampleIdValue() {
    let id = SampleId(value: 101112)
    XCTAssertEqual(id.value, 101112)
  }

  /// Test TrainingSessionId value property.
  func testTrainingSessionIdValue() {
    let id = TrainingSessionId(value: 999)
    XCTAssertEqual(id.value, 999)
  }

  /// Test ValidationSessionId value property.
  func testValidationSessionIdValue() {
    let id = ValidationSessionId(value: 888)
    XCTAssertEqual(id.value, 888)
  }

  /// Test SnapshotId value property.
  func testSnapshotIdValue() {
    let id = SnapshotId(value: 777)
    XCTAssertEqual(id.value, 777)
  }

  /// Test TaskId value property.
  func testTaskIdValue() {
    let id = TaskId(value: 666)
    XCTAssertEqual(id.value, 666)
  }

  // MARK: - ID Equality Tests (Offline)

  /// Test OrganizationId equality.
  func testOrganizationIdEquality() {
    let id1 = OrganizationId(value: 100)
    let id2 = OrganizationId(value: 100)
    let id3 = OrganizationId(value: 200)
    XCTAssertEqual(id1, id2)
    XCTAssertNotEqual(id1, id3)
  }

  /// Test ProjectId equality.
  func testProjectIdEquality() {
    let id1 = ProjectId(value: 100)
    let id2 = ProjectId(value: 100)
    let id3 = ProjectId(value: 200)
    XCTAssertEqual(id1, id2)
    XCTAssertNotEqual(id1, id3)
  }

  /// Test DatasetId equality.
  func testDatasetIdEquality() {
    let id1 = DatasetId(value: 100)
    let id2 = DatasetId(value: 100)
    let id3 = DatasetId(value: 200)
    XCTAssertEqual(id1, id2)
    XCTAssertNotEqual(id1, id3)
  }

  // MARK: - ID Hashability Tests (Offline)

  /// Test IDs can be used as dictionary keys.
  func testIDsAsHashableKeys() {
    var projectMap: [ProjectId: String] = [:]
    let id1 = ProjectId(value: 1)
    let id2 = ProjectId(value: 2)

    projectMap[id1] = "Project One"
    projectMap[id2] = "Project Two"

    XCTAssertEqual(projectMap[id1], "Project One")
    XCTAssertEqual(projectMap[id2], "Project Two")
    XCTAssertEqual(projectMap.count, 2)
  }

  /// Test IDs can be stored in sets.
  func testIDsInSets() {
    var datasetSet: Set<DatasetId> = []
    let id1 = DatasetId(value: 100)
    let id2 = DatasetId(value: 200)
    let id3 = DatasetId(value: 100)  // Duplicate

    datasetSet.insert(id1)
    datasetSet.insert(id2)
    datasetSet.insert(id3)

    XCTAssertEqual(datasetSet.count, 2)  // Duplicate not added
    XCTAssertTrue(datasetSet.contains(id1))
    XCTAssertTrue(datasetSet.contains(id2))
  }

  // MARK: - Parse/Format Free Function Tests (Offline)

  // -- OrganizationId --

  /// Test parsing an organization ID from its string representation.
  func testParseOrganizationId() throws {
    let id = try parseOrganizationId(s: "org-abc123")
    XCTAssertEqual(id.value, 0xabc123)
  }

  /// Test formatting an organization ID to its string representation.
  func testFormatOrganizationId() {
    let id = OrganizationId(value: 0xabc123)
    XCTAssertEqual(formatOrganizationId(id: id), "org-abc123")
  }

  /// Test round-trip parse/format for organization ID.
  func testParseOrganizationIdRoundTrip() throws {
    let original = OrganizationId(value: 0xdeadbeef)
    let formatted = formatOrganizationId(id: original)
    let parsed = try parseOrganizationId(s: formatted)
    XCTAssertEqual(parsed.value, original.value)
  }

  /// Test parsing an organization ID with an invalid prefix throws an error.
  func testParseOrganizationIdInvalidPrefix() {
    XCTAssertThrowsError(try parseOrganizationId(s: "p-abc123"))
  }

  /// Test parsing an organization ID with invalid hex throws an error.
  func testParseOrganizationIdInvalidHex() {
    XCTAssertThrowsError(try parseOrganizationId(s: "org-xyz"))
  }

  // -- ProjectId --

  /// Test parsing a project ID from its string representation.
  func testParseProjectId() throws {
    let id = try parseProjectId(s: "p-abc123")
    XCTAssertEqual(id.value, 0xabc123)
  }

  /// Test formatting a project ID to its string representation.
  func testFormatProjectId() {
    let id = ProjectId(value: 0xabc123)
    XCTAssertEqual(formatProjectId(id: id), "p-abc123")
  }

  /// Test round-trip parse/format for project ID.
  func testParseProjectIdRoundTrip() throws {
    let original = ProjectId(value: 0xdeadbeef)
    let formatted = formatProjectId(id: original)
    let parsed = try parseProjectId(s: formatted)
    XCTAssertEqual(parsed.value, original.value)
  }

  /// Test parsing a project ID with an invalid prefix throws an error.
  func testParseProjectIdInvalidPrefix() {
    XCTAssertThrowsError(try parseProjectId(s: "ds-abc123"))
  }

  /// Test parsing a project ID with invalid hex throws an error.
  func testParseProjectIdInvalidHex() {
    XCTAssertThrowsError(try parseProjectId(s: "p-xyz"))
  }

  // -- ExperimentId --

  /// Test parsing an experiment ID from its string representation.
  func testParseExperimentId() throws {
    let id = try parseExperimentId(s: "exp-abc123")
    XCTAssertEqual(id.value, 0xabc123)
  }

  /// Test formatting an experiment ID to its string representation.
  func testFormatExperimentId() {
    let id = ExperimentId(value: 0xabc123)
    XCTAssertEqual(formatExperimentId(id: id), "exp-abc123")
  }

  /// Test round-trip parse/format for experiment ID.
  func testParseExperimentIdRoundTrip() throws {
    let original = ExperimentId(value: 0xdeadbeef)
    let formatted = formatExperimentId(id: original)
    let parsed = try parseExperimentId(s: formatted)
    XCTAssertEqual(parsed.value, original.value)
  }

  /// Test parsing an experiment ID with an invalid prefix throws an error.
  func testParseExperimentIdInvalidPrefix() {
    XCTAssertThrowsError(try parseExperimentId(s: "p-abc123"))
  }

  /// Test parsing an experiment ID with invalid hex throws an error.
  func testParseExperimentIdInvalidHex() {
    XCTAssertThrowsError(try parseExperimentId(s: "exp-xyz"))
  }

  // -- TrainingSessionId --

  /// Test parsing a training session ID from its string representation.
  func testParseTrainingSessionId() throws {
    let id = try parseTrainingSessionId(s: "t-abc123")
    XCTAssertEqual(id.value, 0xabc123)
  }

  /// Test formatting a training session ID to its string representation.
  func testFormatTrainingSessionId() {
    let id = TrainingSessionId(value: 0xabc123)
    XCTAssertEqual(formatTrainingSessionId(id: id), "t-abc123")
  }

  /// Test round-trip parse/format for training session ID.
  func testParseTrainingSessionIdRoundTrip() throws {
    let original = TrainingSessionId(value: 0xdeadbeef)
    let formatted = formatTrainingSessionId(id: original)
    let parsed = try parseTrainingSessionId(s: formatted)
    XCTAssertEqual(parsed.value, original.value)
  }

  /// Test parsing a training session ID with an invalid prefix throws an error.
  func testParseTrainingSessionIdInvalidPrefix() {
    XCTAssertThrowsError(try parseTrainingSessionId(s: "v-abc123"))
  }

  /// Test parsing a training session ID with invalid hex throws an error.
  func testParseTrainingSessionIdInvalidHex() {
    XCTAssertThrowsError(try parseTrainingSessionId(s: "t-xyz"))
  }

  // -- ValidationSessionId --

  /// Test parsing a validation session ID from its string representation.
  func testParseValidationSessionId() throws {
    let id = try parseValidationSessionId(s: "v-abc123")
    XCTAssertEqual(id.value, 0xabc123)
  }

  /// Test formatting a validation session ID to its string representation.
  func testFormatValidationSessionId() {
    let id = ValidationSessionId(value: 0xabc123)
    XCTAssertEqual(formatValidationSessionId(id: id), "v-abc123")
  }

  /// Test round-trip parse/format for validation session ID.
  func testParseValidationSessionIdRoundTrip() throws {
    let original = ValidationSessionId(value: 0xdeadbeef)
    let formatted = formatValidationSessionId(id: original)
    let parsed = try parseValidationSessionId(s: formatted)
    XCTAssertEqual(parsed.value, original.value)
  }

  /// Test parsing a validation session ID with an invalid prefix throws an error.
  func testParseValidationSessionIdInvalidPrefix() {
    XCTAssertThrowsError(try parseValidationSessionId(s: "t-abc123"))
  }

  /// Test parsing a validation session ID with invalid hex throws an error.
  func testParseValidationSessionIdInvalidHex() {
    XCTAssertThrowsError(try parseValidationSessionId(s: "v-xyz"))
  }

  // -- SnapshotId --

  /// Test parsing a snapshot ID from its string representation.
  func testParseSnapshotId() throws {
    let id = try parseSnapshotId(s: "ss-abc123")
    XCTAssertEqual(id.value, 0xabc123)
  }

  /// Test formatting a snapshot ID to its string representation.
  func testFormatSnapshotId() {
    let id = SnapshotId(value: 0xabc123)
    XCTAssertEqual(formatSnapshotId(id: id), "ss-abc123")
  }

  /// Test round-trip parse/format for snapshot ID.
  func testParseSnapshotIdRoundTrip() throws {
    let original = SnapshotId(value: 0xdeadbeef)
    let formatted = formatSnapshotId(id: original)
    let parsed = try parseSnapshotId(s: formatted)
    XCTAssertEqual(parsed.value, original.value)
  }

  /// Test parsing a snapshot ID with an invalid prefix throws an error.
  func testParseSnapshotIdInvalidPrefix() {
    XCTAssertThrowsError(try parseSnapshotId(s: "p-abc123"))
  }

  /// Test parsing a snapshot ID with invalid hex throws an error.
  func testParseSnapshotIdInvalidHex() {
    XCTAssertThrowsError(try parseSnapshotId(s: "ss-xyz"))
  }

  // -- TaskId --

  /// Test parsing a task ID from its string representation.
  func testParseTaskId() throws {
    let id = try parseTaskId(s: "task-abc123")
    XCTAssertEqual(id.value, 0xabc123)
  }

  /// Test formatting a task ID to its string representation.
  func testFormatTaskId() {
    let id = TaskId(value: 0xabc123)
    XCTAssertEqual(formatTaskId(id: id), "task-abc123")
  }

  /// Test round-trip parse/format for task ID.
  func testParseTaskIdRoundTrip() throws {
    let original = TaskId(value: 0xdeadbeef)
    let formatted = formatTaskId(id: original)
    let parsed = try parseTaskId(s: formatted)
    XCTAssertEqual(parsed.value, original.value)
  }

  /// Test parsing a task ID with an invalid prefix throws an error.
  func testParseTaskIdInvalidPrefix() {
    XCTAssertThrowsError(try parseTaskId(s: "p-abc123"))
  }

  /// Test parsing a task ID with invalid hex throws an error.
  func testParseTaskIdInvalidHex() {
    XCTAssertThrowsError(try parseTaskId(s: "task-xyz"))
  }

  // -- DatasetId --

  /// Test parsing a dataset ID from its string representation.
  func testParseDatasetId() throws {
    let id = try parseDatasetId(s: "ds-abc123")
    XCTAssertEqual(id.value, 0xabc123)
  }

  /// Test formatting a dataset ID to its string representation.
  func testFormatDatasetId() {
    let id = DatasetId(value: 0xabc123)
    XCTAssertEqual(formatDatasetId(id: id), "ds-abc123")
  }

  /// Test round-trip parse/format for dataset ID.
  func testParseDatasetIdRoundTrip() throws {
    let original = DatasetId(value: 0xdeadbeef)
    let formatted = formatDatasetId(id: original)
    let parsed = try parseDatasetId(s: formatted)
    XCTAssertEqual(parsed.value, original.value)
  }

  /// Test parsing a dataset ID with an invalid prefix throws an error.
  func testParseDatasetIdInvalidPrefix() {
    XCTAssertThrowsError(try parseDatasetId(s: "p-abc123"))
  }

  /// Test parsing a dataset ID with invalid hex throws an error.
  func testParseDatasetIdInvalidHex() {
    XCTAssertThrowsError(try parseDatasetId(s: "ds-xyz"))
  }

  // -- AnnotationSetId --

  /// Test parsing an annotation set ID from its string representation.
  func testParseAnnotationSetId() throws {
    let id = try parseAnnotationSetId(s: "as-abc123")
    XCTAssertEqual(id.value, 0xabc123)
  }

  /// Test formatting an annotation set ID to its string representation.
  func testFormatAnnotationSetId() {
    let id = AnnotationSetId(value: 0xabc123)
    XCTAssertEqual(formatAnnotationSetId(id: id), "as-abc123")
  }

  /// Test round-trip parse/format for annotation set ID.
  func testParseAnnotationSetIdRoundTrip() throws {
    let original = AnnotationSetId(value: 0xdeadbeef)
    let formatted = formatAnnotationSetId(id: original)
    let parsed = try parseAnnotationSetId(s: formatted)
    XCTAssertEqual(parsed.value, original.value)
  }

  /// Test parsing an annotation set ID with an invalid prefix throws an error.
  func testParseAnnotationSetIdInvalidPrefix() {
    XCTAssertThrowsError(try parseAnnotationSetId(s: "ds-abc123"))
  }

  /// Test parsing an annotation set ID with invalid hex throws an error.
  func testParseAnnotationSetIdInvalidHex() {
    XCTAssertThrowsError(try parseAnnotationSetId(s: "as-xyz"))
  }

  // -- SampleId --

  /// Test parsing a sample ID from its string representation.
  func testParseSampleId() throws {
    let id = try parseSampleId(s: "s-abc123")
    XCTAssertEqual(id.value, 0xabc123)
  }

  /// Test formatting a sample ID to its string representation.
  func testFormatSampleId() {
    let id = SampleId(value: 0xabc123)
    XCTAssertEqual(formatSampleId(id: id), "s-abc123")
  }

  /// Test round-trip parse/format for sample ID.
  func testParseSampleIdRoundTrip() throws {
    let original = SampleId(value: 0xdeadbeef)
    let formatted = formatSampleId(id: original)
    let parsed = try parseSampleId(s: formatted)
    XCTAssertEqual(parsed.value, original.value)
  }

  /// Test parsing a sample ID with an invalid prefix throws an error.
  func testParseSampleIdInvalidPrefix() {
    XCTAssertThrowsError(try parseSampleId(s: "ds-abc123"))
  }

  /// Test parsing a sample ID with invalid hex throws an error.
  func testParseSampleIdInvalidHex() {
    XCTAssertThrowsError(try parseSampleId(s: "s-xyz"))
  }

  // -- AppId --

  /// Test parsing an app ID from its string representation.
  func testParseAppId() throws {
    let id = try parseAppId(s: "app-abc123")
    XCTAssertEqual(id.value, 0xabc123)
  }

  /// Test formatting an app ID to its string representation.
  func testFormatAppId() {
    let id = AppId(value: 0xabc123)
    XCTAssertEqual(formatAppId(id: id), "app-abc123")
  }

  /// Test round-trip parse/format for app ID.
  func testParseAppIdRoundTrip() throws {
    let original = AppId(value: 0xdeadbeef)
    let formatted = formatAppId(id: original)
    let parsed = try parseAppId(s: formatted)
    XCTAssertEqual(parsed.value, original.value)
  }

  /// Test parsing an app ID with an invalid prefix throws an error.
  func testParseAppIdInvalidPrefix() {
    XCTAssertThrowsError(try parseAppId(s: "p-abc123"))
  }

  /// Test parsing an app ID with invalid hex throws an error.
  func testParseAppIdInvalidHex() {
    XCTAssertThrowsError(try parseAppId(s: "app-xyz"))
  }

  // -- ImageId --

  /// Test parsing an image ID from its string representation.
  func testParseImageId() throws {
    let id = try parseImageId(s: "im-abc123")
    XCTAssertEqual(id.value, 0xabc123)
  }

  /// Test formatting an image ID to its string representation.
  func testFormatImageId() {
    let id = ImageId(value: 0xabc123)
    XCTAssertEqual(formatImageId(id: id), "im-abc123")
  }

  /// Test round-trip parse/format for image ID.
  func testParseImageIdRoundTrip() throws {
    let original = ImageId(value: 0xdeadbeef)
    let formatted = formatImageId(id: original)
    let parsed = try parseImageId(s: formatted)
    XCTAssertEqual(parsed.value, original.value)
  }

  /// Test parsing an image ID with an invalid prefix throws an error.
  func testParseImageIdInvalidPrefix() {
    XCTAssertThrowsError(try parseImageId(s: "p-abc123"))
  }

  /// Test parsing an image ID with invalid hex throws an error.
  func testParseImageIdInvalidHex() {
    XCTAssertThrowsError(try parseImageId(s: "im-xyz"))
  }

  // -- SequenceId --

  /// Test parsing a sequence ID from its string representation.
  func testParseSequenceId() throws {
    let id = try parseSequenceId(s: "se-abc123")
    XCTAssertEqual(id.value, 0xabc123)
  }

  /// Test formatting a sequence ID to its string representation.
  func testFormatSequenceId() {
    let id = SequenceId(value: 0xabc123)
    XCTAssertEqual(formatSequenceId(id: id), "se-abc123")
  }

  /// Test round-trip parse/format for sequence ID.
  func testParseSequenceIdRoundTrip() throws {
    let original = SequenceId(value: 0xdeadbeef)
    let formatted = formatSequenceId(id: original)
    let parsed = try parseSequenceId(s: formatted)
    XCTAssertEqual(parsed.value, original.value)
  }

  /// Test parsing a sequence ID with an invalid prefix throws an error.
  func testParseSequenceIdInvalidPrefix() {
    XCTAssertThrowsError(try parseSequenceId(s: "p-abc123"))
  }

  /// Test parsing a sequence ID with invalid hex throws an error.
  func testParseSequenceIdInvalidHex() {
    XCTAssertThrowsError(try parseSequenceId(s: "se-xyz"))
  }

  // MARK: - Online ID Tests (Require Credentials)

  /// Test Organization ID has valid non-zero value.
  func testOrganizationIdFormat() throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try TestConfig.getClient()
    let org = try client.organization()

    // Value should be non-zero
    XCTAssertGreaterThan(org.id.value, 0)
    print("Organization ID value: \(org.id.value)")
  }

  /// Test Project ID has valid non-zero value.
  func testProjectIdFormat() throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try TestConfig.getClient()
    let projects = try client.projects(name: nil)
    XCTAssertGreaterThan(projects.count, 0)

    if let project = projects.first {
      XCTAssertGreaterThan(project.id.value, 0)
      print("Project ID value: \(project.id.value)")
    }
  }

  /// Test Dataset ID has valid non-zero value.
  func testDatasetIdFormat() throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try TestConfig.getClient()
    let projects = try client.projects(name: nil)
    guard let project = projects.first else {
      print("No projects available")
      return
    }

    let datasets = try client.datasets(projectId: project.id, name: nil)
    if let dataset = datasets.first {
      XCTAssertGreaterThan(dataset.id.value, 0)
      print("Dataset ID value: \(dataset.id.value)")
    }
  }

  /// Test Experiment ID has valid non-zero value.
  func testExperimentIdFormat() throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try TestConfig.getClient()
    let projects = try client.projects(name: nil)
    guard let project = projects.first else {
      print("No projects available")
      return
    }

    let experiments = try client.experiments(projectId: project.id, name: nil)
    if let experiment = experiments.first {
      XCTAssertGreaterThan(experiment.id.value, 0)
      print("Experiment ID value: \(experiment.id.value)")
    }
  }

  /// Test ID consistency between project retrieval methods.
  func testProjectIdConsistency() throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try TestConfig.getClient()
    let projects = try client.projects(name: nil)
    guard let first = projects.first else {
      XCTFail("Need at least one project to test")
      return
    }

    // Fetch by ID should return same ID value
    let project = try client.project(id: first.id)
    XCTAssertEqual(project.id.value, first.id.value)
    XCTAssertEqual(project.name, first.name)
  }

  /// Test ID consistency between dataset retrieval methods.
  func testDatasetIdConsistency() throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try TestConfig.getClient()
    let projects = try client.projects(name: nil)
    guard let project = projects.first else {
      print("No projects available")
      return
    }

    let datasets = try client.datasets(projectId: project.id, name: nil)
    guard let first = datasets.first else {
      print("No datasets available")
      return
    }

    // Fetch by ID should return same ID value
    let dataset = try client.dataset(id: first.id)
    XCTAssertEqual(dataset.id.value, first.id.value)
    XCTAssertEqual(dataset.name, first.name)
  }

  /// Test ID consistency between experiment retrieval methods (async).
  func testExperimentIdConsistencyAsync() async throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try await TestConfig.getClientAsync()
    let projects = try await client.projectsAsync(name: nil)
    guard let project = projects.first else {
      print("No projects available")
      return
    }

    let experiments = try await client.experimentsAsync(projectId: project.id, name: nil)
    guard let first = experiments.first else {
      print("No experiments available")
      return
    }

    // Fetch by ID should return same ID value
    let experiment = try await client.experimentAsync(id: first.id)
    XCTAssertEqual(experiment.id.value, first.id.value)
    XCTAssertEqual(experiment.name, first.name)
  }

  /// Test AnnotationSet ID from dataset.
  func testAnnotationSetIdFormat() throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try TestConfig.getClient()
    let projects = try client.projects(name: nil)
    guard let project = projects.first else {
      print("No projects available")
      return
    }

    let datasets = try client.datasets(projectId: project.id, name: nil)
    guard let dataset = datasets.first else {
      print("No datasets available")
      return
    }

    let annotationSets = try client.annotationSets(datasetId: dataset.id)
    if let annotationSet = annotationSets.first {
      XCTAssertGreaterThan(annotationSet.id.value, 0)
      print("AnnotationSet ID value: \(annotationSet.id.value)")
    }
  }

  /// Test TrainingSession ID from experiment.
  func testTrainingSessionIdFormat() throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try TestConfig.getClient()
    let projects = try client.projects(name: nil)
    guard let project = projects.first else {
      print("No projects available")
      return
    }

    let experiments = try client.experiments(projectId: project.id, name: nil)
    for experiment in experiments {
      let sessions = try client.trainingSessions(experimentId: experiment.id, name: nil)
      if let session = sessions.first {
        XCTAssertGreaterThan(session.id.value, 0)
        print("TrainingSession ID value: \(session.id.value)")
        break
      }
    }
  }

  /// Test ValidationSession ID from project.
  func testValidationSessionIdFormat() throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try TestConfig.getClient()
    let projects = try client.projects(name: nil)
    guard let project = projects.first else {
      print("No projects available")
      return
    }

    let sessions = try client.validationSessions(projectId: project.id)
    if let session = sessions.first {
      XCTAssertGreaterThan(session.id.value, 0)
      print("ValidationSession ID value: \(session.id.value)")
    }
  }
}
