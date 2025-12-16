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

  // MARK: - Artifact Offline Tests

  /// Test Artifact struct construction.
  func testArtifactConstruction() {
    let artifact = Artifact(name: "yolov8n.onnx", modelType: "onnx")

    XCTAssertEqual(artifact.name, "yolov8n.onnx")
    XCTAssertEqual(artifact.modelType, "onnx")
  }

  /// Test Artifact with various model types.
  func testArtifactModelTypes() {
    let modelTypes = ["onnx", "pytorch", "tensorflow", "tflite", "coreml", "openvino"]

    for modelType in modelTypes {
      let artifact = Artifact(name: "model.\(modelType)", modelType: modelType)
      XCTAssertEqual(artifact.modelType, modelType)
    }
  }

  /// Test Artifact with empty name.
  func testArtifactWithEmptyName() {
    let artifact = Artifact(name: "", modelType: "onnx")
    XCTAssertTrue(artifact.name.isEmpty)
  }

  /// Test Artifact with empty model type.
  func testArtifactWithEmptyModelType() {
    let artifact = Artifact(name: "model.bin", modelType: "")
    XCTAssertTrue(artifact.modelType.isEmpty)
  }

  /// Test Artifact with special characters in name.
  func testArtifactWithSpecialCharactersInName() {
    let artifact = Artifact(name: "model_v1.0_2024-03-15.onnx", modelType: "onnx")
    XCTAssertTrue(artifact.name.contains("_"))
    XCTAssertTrue(artifact.name.contains("-"))
    XCTAssertTrue(artifact.name.contains("."))
  }

  /// Test Artifact with unicode name.
  func testArtifactWithUnicodeName() {
    let artifact = Artifact(name: "模型_v1.onnx", modelType: "onnx")
    XCTAssertTrue(artifact.name.contains("模型"))
  }

  /// Test Artifact as dictionary key.
  func testArtifactAsDictionaryKey() {
    var artifactSizes: [Artifact: Int] = [:]

    let artifact1 = Artifact(name: "model1.onnx", modelType: "onnx")
    let artifact2 = Artifact(name: "model2.pt", modelType: "pytorch")

    artifactSizes[artifact1] = 1024000
    artifactSizes[artifact2] = 2048000

    XCTAssertEqual(artifactSizes[artifact1], 1024000)
    XCTAssertEqual(artifactSizes[artifact2], 2048000)
  }

  // MARK: - TrainingSession Offline Tests

  /// Test TrainingSession struct construction.
  func testTrainingSessionConstruction() {
    let session = TrainingSession(
      id: TrainingSessionId(value: 100),
      experimentId: ExperimentId(value: 50),
      name: "YOLOv8 Training",
      description: "Training YOLOv8 on custom dataset",
      model: "yolov8n"
    )

    XCTAssertEqual(session.id.value, 100)
    XCTAssertEqual(session.experimentId.value, 50)
    XCTAssertEqual(session.name, "YOLOv8 Training")
    XCTAssertEqual(session.description, "Training YOLOv8 on custom dataset")
    XCTAssertEqual(session.model, "yolov8n")
  }

  /// Test TrainingSession equality.
  func testTrainingSessionEquality() {
    let session1 = TrainingSession(
      id: TrainingSessionId(value: 100),
      experimentId: ExperimentId(value: 50),
      name: "Training",
      description: "Description",
      model: "model"
    )

    let session2 = TrainingSession(
      id: TrainingSessionId(value: 100),
      experimentId: ExperimentId(value: 50),
      name: "Training",
      description: "Description",
      model: "model"
    )

    let session3 = TrainingSession(
      id: TrainingSessionId(value: 101),
      experimentId: ExperimentId(value: 51),
      name: "Different",
      description: "Other",
      model: "other"
    )

    XCTAssertEqual(session1, session2)
    XCTAssertNotEqual(session1, session3)
  }

  /// Test TrainingSession hashability.
  func testTrainingSessionHashability() {
    var sessionSet: Set<TrainingSession> = []

    let session1 = TrainingSession(
      id: TrainingSessionId(value: 100),
      experimentId: ExperimentId(value: 50),
      name: "Session1",
      description: "Desc1",
      model: "model1"
    )

    let session2 = TrainingSession(
      id: TrainingSessionId(value: 101),
      experimentId: ExperimentId(value: 51),
      name: "Session2",
      description: "Desc2",
      model: "model2"
    )

    sessionSet.insert(session1)
    sessionSet.insert(session2)
    sessionSet.insert(session1)  // Duplicate

    XCTAssertEqual(sessionSet.count, 2)
  }

  /// Test TrainingSessionId construction.
  func testTrainingSessionIdConstruction() {
    let id = TrainingSessionId(value: 12345)
    XCTAssertEqual(id.value, 12345)
  }

  /// Test TrainingSessionId equality.
  func testTrainingSessionIdEquality() {
    let id1 = TrainingSessionId(value: 100)
    let id2 = TrainingSessionId(value: 100)
    let id3 = TrainingSessionId(value: 200)

    XCTAssertEqual(id1, id2)
    XCTAssertNotEqual(id1, id3)
  }

  /// Test TrainingSessionId hashability.
  func testTrainingSessionIdHashability() {
    var idSet: Set<TrainingSessionId> = []

    idSet.insert(TrainingSessionId(value: 100))
    idSet.insert(TrainingSessionId(value: 200))
    idSet.insert(TrainingSessionId(value: 100))  // Duplicate

    XCTAssertEqual(idSet.count, 2)
  }

  // MARK: - ValidationSession Offline Tests

  /// Test ValidationSession struct construction.
  func testValidationSessionConstruction() {
    let session = ValidationSession(
      id: ValidationSessionId(value: 200),
      experimentId: ExperimentId(value: 50),
      trainingSessionId: TrainingSessionId(value: 100),
      datasetId: DatasetId(value: 75),
      annotationSetId: AnnotationSetId(value: 25),
      description: "Validation on test dataset"
    )

    XCTAssertEqual(session.id.value, 200)
    XCTAssertEqual(session.experimentId.value, 50)
    XCTAssertEqual(session.trainingSessionId.value, 100)
    XCTAssertEqual(session.datasetId.value, 75)
    XCTAssertEqual(session.annotationSetId.value, 25)
    XCTAssertEqual(session.description, "Validation on test dataset")
  }

  /// Test ValidationSession equality.
  func testValidationSessionEquality() {
    let session1 = ValidationSession(
      id: ValidationSessionId(value: 100),
      experimentId: ExperimentId(value: 50),
      trainingSessionId: TrainingSessionId(value: 25),
      datasetId: DatasetId(value: 30),
      annotationSetId: AnnotationSetId(value: 10),
      description: "Test"
    )

    let session2 = ValidationSession(
      id: ValidationSessionId(value: 100),
      experimentId: ExperimentId(value: 50),
      trainingSessionId: TrainingSessionId(value: 25),
      datasetId: DatasetId(value: 30),
      annotationSetId: AnnotationSetId(value: 10),
      description: "Test"
    )

    let session3 = ValidationSession(
      id: ValidationSessionId(value: 101),
      experimentId: ExperimentId(value: 51),
      trainingSessionId: TrainingSessionId(value: 26),
      datasetId: DatasetId(value: 31),
      annotationSetId: AnnotationSetId(value: 11),
      description: "Different"
    )

    XCTAssertEqual(session1, session2)
    XCTAssertNotEqual(session1, session3)
  }

  /// Test ValidationSession hashability.
  func testValidationSessionHashability() {
    var sessionSet: Set<ValidationSession> = []

    let session1 = ValidationSession(
      id: ValidationSessionId(value: 100),
      experimentId: ExperimentId(value: 50),
      trainingSessionId: TrainingSessionId(value: 25),
      datasetId: DatasetId(value: 30),
      annotationSetId: AnnotationSetId(value: 10),
      description: "Session1"
    )

    let session2 = ValidationSession(
      id: ValidationSessionId(value: 101),
      experimentId: ExperimentId(value: 51),
      trainingSessionId: TrainingSessionId(value: 26),
      datasetId: DatasetId(value: 31),
      annotationSetId: AnnotationSetId(value: 11),
      description: "Session2"
    )

    sessionSet.insert(session1)
    sessionSet.insert(session2)
    sessionSet.insert(session1)  // Duplicate

    XCTAssertEqual(sessionSet.count, 2)
  }

  /// Test ValidationSessionId construction.
  func testValidationSessionIdConstruction() {
    let id = ValidationSessionId(value: 12345)
    XCTAssertEqual(id.value, 12345)
  }

  /// Test ValidationSessionId equality.
  func testValidationSessionIdEquality() {
    let id1 = ValidationSessionId(value: 100)
    let id2 = ValidationSessionId(value: 100)
    let id3 = ValidationSessionId(value: 200)

    XCTAssertEqual(id1, id2)
    XCTAssertNotEqual(id1, id3)
  }
}
