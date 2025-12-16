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
