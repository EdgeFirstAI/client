// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

/// Tests for experiment listing operations.
///
/// These tests verify the client can list and retrieve experiments
/// from EdgeFirst Studio projects.

import XCTest

@testable import EdgeFirstClient

final class ExperimentTests: XCTestCase {

  /// Test experiments() returns a list of experiments for a project.
  func testListExperiments() throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try TestConfig.getClient()
    let projects = try client.projects(name: nil)

    guard let project = projects.first else {
      XCTFail("Need at least one project to test experiments")
      return
    }

    let experiments = try client.experiments(projectId: project.id, name: nil)

    print("Found \(experiments.count) experiments in project \(project.name)")

    if let first = experiments.first {
      XCTAssertNotNil(first.id)
      XCTAssertFalse(first.name.isEmpty, "Experiment name should not be empty")
      print("First experiment: \(first.name)")
    }
  }

  /// Test experimentsAsync() returns a list of experiments.
  func testListExperimentsAsync() async throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try await TestConfig.getClientAsync()
    let projects = try await client.projectsAsync(name: nil)

    guard let project = projects.first else {
      XCTFail("Need at least one project")
      return
    }

    let experiments = try await client.experimentsAsync(
      projectId: project.id,
      name: nil
    )
    print("Found \(experiments.count) experiments (async)")
  }

  /// Test experimentAsync() retrieves a single experiment by ID.
  func testGetExperimentByIdAsync() async throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try await TestConfig.getClientAsync()
    let projects = try await client.projectsAsync(name: nil)

    guard let project = projects.first else {
      XCTFail("Need at least one project to test")
      return
    }

    let experiments = try await client.experimentsAsync(projectId: project.id, name: nil)

    guard let first = experiments.first else {
      print("No experiments in project \(project.name), skipping ID test (async)")
      return
    }

    // Fetch the same experiment by ID
    let experiment = try await client.experimentAsync(id: first.id)

    XCTAssertEqual(experiment.id.value, first.id.value)
    XCTAssertEqual(experiment.name, first.name)
    print("Retrieved experiment (async): \(experiment.name) (ID: \(experiment.id.value))")
  }

  /// Test trainingSessionsAsync() returns training sessions for an experiment.
  func testTrainingSessionsAsync() async throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try await TestConfig.getClientAsync()
    let projects = try await client.projectsAsync(name: nil)

    guard let project = projects.first else {
      XCTFail("Need at least one project to test")
      return
    }

    let experiments = try await client.experimentsAsync(projectId: project.id, name: nil)

    guard let experiment = experiments.first else {
      print("No experiments in project \(project.name), skipping training sessions test (async)")
      return
    }

    let sessions = try await client.trainingSessionsAsync(
      experimentId: experiment.id,
      name: nil
    )

    print("Found \(sessions.count) training sessions (async) in experiment \(experiment.name)")

    if let first = sessions.first {
      XCTAssertNotNil(first.id)
      XCTAssertFalse(first.name.isEmpty, "Training session name should not be empty")
    }
  }

  /// Test validationSessionsAsync() returns validation sessions for a project.
  func testValidationSessionsAsync() async throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try await TestConfig.getClientAsync()
    let projects = try await client.projectsAsync(name: nil)

    guard let project = projects.first else {
      XCTFail("Need at least one project to test")
      return
    }

    let sessions = try await client.validationSessionsAsync(projectId: project.id)

    print("Found \(sessions.count) validation sessions (async) in project \(project.name)")

    if let first = sessions.first {
      XCTAssertNotNil(first.id)
    }
  }

  /// Test experiment() retrieves a single experiment by ID.
  func testGetExperimentById() throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try TestConfig.getClient()
    let projects = try client.projects(name: nil)

    guard let project = projects.first else {
      XCTFail("Need at least one project to test")
      return
    }

    let experiments = try client.experiments(projectId: project.id, name: nil)

    guard let first = experiments.first else {
      print("No experiments in project \(project.name), skipping ID test")
      return
    }

    // Fetch the same experiment by ID
    let experiment = try client.experiment(id: first.id)

    XCTAssertEqual(experiment.id.value, first.id.value)
    XCTAssertEqual(experiment.name, first.name)
    print("Retrieved experiment: \(experiment.name) (ID: \(experiment.id.value))")
  }

  /// Test trainingSessions() returns training sessions for an experiment.
  func testTrainingSessions() throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try TestConfig.getClient()
    let projects = try client.projects(name: nil)

    guard let project = projects.first else {
      XCTFail("Need at least one project to test")
      return
    }

    let experiments = try client.experiments(projectId: project.id, name: nil)

    guard let experiment = experiments.first else {
      print("No experiments in project \(project.name), skipping training sessions test")
      return
    }

    let sessions = try client.trainingSessions(
      experimentId: experiment.id,
      name: nil
    )

    print("Found \(sessions.count) training sessions in experiment \(experiment.name)")

    if let first = sessions.first {
      XCTAssertNotNil(first.id)
      XCTAssertFalse(first.name.isEmpty, "Training session name should not be empty")
      print("First training session: \(first.name)")
    }
  }

  /// Test validationSessions() returns validation sessions for a project.
  func testValidationSessions() throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try TestConfig.getClient()
    let projects = try client.projects(name: nil)

    guard let project = projects.first else {
      XCTFail("Need at least one project to test")
      return
    }

    let sessions = try client.validationSessions(projectId: project.id)

    print("Found \(sessions.count) validation sessions in project \(project.name)")

    if let first = sessions.first {
      XCTAssertNotNil(first.id)
      print("First validation session ID: \(first.id.value)")
    }
  }
}
