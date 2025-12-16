// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

/// Tests for artifact operations.
///
/// These tests verify the client can retrieve artifacts from training sessions.
/// Matches Python test patterns in test_experiments.py and test_advanced.py.

import XCTest

@testable import EdgeFirstClient

final class ArtifactTests: XCTestCase {

  /// Test artifacts() returns artifacts for a training session.
  func testListArtifacts() throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try TestConfig.getClient()
    let projects = try client.projects(name: nil)

    guard let project = projects.first else {
      XCTFail("Need at least one project to test artifacts")
      return
    }

    let experiments = try client.experiments(projectId: project.id, name: nil)

    // Find an experiment with training sessions
    for experiment in experiments {
      let sessions = try client.trainingSessions(experimentId: experiment.id, name: nil)

      if let session = sessions.first {
        let artifacts = try client.artifacts(trainingSessionId: session.id)

        print("Found \(artifacts.count) artifacts in training session \(session.name)")

        if let first = artifacts.first {
          XCTAssertFalse(first.name.isEmpty, "Artifact name should not be empty")
          XCTAssertFalse(first.modelType.isEmpty, "Artifact model type should not be empty")
          print("First artifact: \(first.name) (type: \(first.modelType))")
        }
        return
      }
    }

    print("No training sessions with artifacts found")
  }

  /// Test artifactsAsync() returns artifacts for a training session.
  func testListArtifactsAsync() async throws {
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

    let experiments = try await client.experimentsAsync(projectId: project.id, name: nil)

    // Find an experiment with training sessions
    for experiment in experiments {
      let sessions = try await client.trainingSessionsAsync(
        experimentId: experiment.id,
        name: nil
      )

      if let session = sessions.first {
        let artifacts = try await client.artifactsAsync(trainingSessionId: session.id)

        print("Found \(artifacts.count) artifacts (async) in training session \(session.name)")
        return
      }
    }

    print("No training sessions found (async)")
  }

  /// Test artifact properties are accessible.
  func testArtifactProperties() throws {
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

      for session in sessions {
        let artifacts = try client.artifacts(trainingSessionId: session.id)

        if let artifact = artifacts.first {
          // Verify properties are accessible
          XCTAssertFalse(artifact.name.isEmpty)
          XCTAssertFalse(artifact.modelType.isEmpty)

          print("Artifact properties:")
          print("  Name: \(artifact.name)")
          print("  Model Type: \(artifact.modelType)")
          return
        }
      }
    }

    print("No artifacts found in any training session")
  }

  /// Test artifact equality.
  func testArtifactEquality() {
    let artifact1 = Artifact(name: "model.onnx", modelType: "onnx")
    let artifact2 = Artifact(name: "model.onnx", modelType: "onnx")
    let artifact3 = Artifact(name: "model.pt", modelType: "pytorch")

    XCTAssertEqual(artifact1, artifact2)
    XCTAssertNotEqual(artifact1, artifact3)
  }

  /// Test artifact hashability.
  func testArtifactHashability() {
    var artifactSet: Set<Artifact> = []

    let artifact1 = Artifact(name: "model1.onnx", modelType: "onnx")
    let artifact2 = Artifact(name: "model2.onnx", modelType: "onnx")
    let artifact3 = Artifact(name: "model1.onnx", modelType: "onnx")  // Duplicate

    artifactSet.insert(artifact1)
    artifactSet.insert(artifact2)
    artifactSet.insert(artifact3)

    XCTAssertEqual(artifactSet.count, 2)
  }

  // MARK: - TrainingSession Artifact Integration Tests

  /// Test training session can be retrieved and has artifacts.
  func testTrainingSessionArtifacts() throws {
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
        // Get the session by ID to verify ID consistency
        let retrieved = try client.trainingSession(id: session.id)
        XCTAssertEqual(retrieved.id.value, session.id.value)
        XCTAssertEqual(retrieved.name, session.name)

        // Get artifacts
        let artifacts = try client.artifacts(trainingSessionId: session.id)
        print("Training session \(session.name) has \(artifacts.count) artifacts")
        return
      }
    }

    print("No training sessions found")
  }

  /// Test async training session artifact retrieval.
  func testTrainingSessionArtifactsAsync() async throws {
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

    for experiment in experiments {
      let sessions = try await client.trainingSessionsAsync(
        experimentId: experiment.id,
        name: nil
      )

      if let session = sessions.first {
        // Get the session by ID async
        let retrieved = try await client.trainingSessionAsync(id: session.id)
        XCTAssertEqual(retrieved.id.value, session.id.value)

        // Get artifacts async
        let artifacts = try await client.artifactsAsync(trainingSessionId: session.id)
        print("Training session \(session.name) has \(artifacts.count) artifacts (async)")
        return
      }
    }

    print("No training sessions found (async)")
  }
}
