// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

/// Tests for entity types.
///
/// These tests verify Label, TrainingSession, ValidationSession, Task,
/// TaskInfo, Stage, and various ID types construction and operations.

import XCTest

@testable import EdgeFirstClient

final class EntityTests: XCTestCase {

  // MARK: - Label Tests

  /// Test Label construction.
  func testLabelConstruction() {
    let label = Label(id: 1, name: "car")

    XCTAssertEqual(label.id, 1)
    XCTAssertEqual(label.name, "car")
  }

  /// Test Label equality.
  func testLabelEquality() {
    let label1 = Label(id: 1, name: "car")
    let label2 = Label(id: 1, name: "car")
    let label3 = Label(id: 2, name: "car")

    XCTAssertEqual(label1, label2)
    XCTAssertNotEqual(label1, label3)
  }

  /// Test Label hashability.
  func testLabelHashability() {
    var labelSet: Set<Label> = []

    labelSet.insert(Label(id: 1, name: "car"))
    labelSet.insert(Label(id: 2, name: "truck"))
    labelSet.insert(Label(id: 1, name: "car"))  // Duplicate

    XCTAssertEqual(labelSet.count, 2)
  }

  /// Test Label with various names.
  func testLabelNames() {
    let labels = [
      Label(id: 0, name: "person"),
      Label(id: 1, name: "bicycle"),
      Label(id: 2, name: "car"),
      Label(id: 3, name: "motorcycle"),
      Label(id: 4, name: "airplane"),
    ]

    XCTAssertEqual(labels.count, 5)
    XCTAssertEqual(labels[0].name, "person")
    XCTAssertEqual(labels[4].name, "airplane")
  }

  /// Test Label with empty name.
  func testLabelEmptyName() {
    let label = Label(id: 0, name: "")

    XCTAssertTrue(label.name.isEmpty)
  }

  // MARK: - TrainingSession Tests

  /// Test TrainingSession construction.
  func testTrainingSessionConstruction() {
    let session = TrainingSession(
      id: TrainingSessionId(value: 100),
      experimentId: ExperimentId(value: 50),
      name: "yolov5-training",
      description: "YOLOv5 object detection training",
      model: "yolov5s"
    )

    XCTAssertEqual(session.id.value, 100)
    XCTAssertEqual(session.experimentId.value, 50)
    XCTAssertEqual(session.name, "yolov5-training")
    XCTAssertEqual(session.description, "YOLOv5 object detection training")
    XCTAssertEqual(session.model, "yolov5s")
  }

  /// Test TrainingSession equality.
  func testTrainingSessionEquality() {
    let session1 = TrainingSession(
      id: TrainingSessionId(value: 1),
      experimentId: ExperimentId(value: 1),
      name: "session1",
      description: "desc",
      model: "model"
    )
    let session2 = TrainingSession(
      id: TrainingSessionId(value: 1),
      experimentId: ExperimentId(value: 1),
      name: "session1",
      description: "desc",
      model: "model"
    )
    let session3 = TrainingSession(
      id: TrainingSessionId(value: 2),
      experimentId: ExperimentId(value: 1),
      name: "session1",
      description: "desc",
      model: "model"
    )

    XCTAssertEqual(session1, session2)
    XCTAssertNotEqual(session1, session3)
  }

  /// Test TrainingSession hashability.
  func testTrainingSessionHashability() {
    var sessionSet: Set<TrainingSession> = []

    sessionSet.insert(
      TrainingSession(
        id: TrainingSessionId(value: 1),
        experimentId: ExperimentId(value: 1),
        name: "s1", description: "", model: ""
      ))
    sessionSet.insert(
      TrainingSession(
        id: TrainingSessionId(value: 2),
        experimentId: ExperimentId(value: 1),
        name: "s2", description: "", model: ""
      ))

    XCTAssertEqual(sessionSet.count, 2)
  }

  // MARK: - ValidationSession Tests
  //
  // `ValidationSession` is now a uniffi `Object` (not a Record), so it lacks
  // field-based initializers and value-equality. Behaviour is exercised end
  // to end in `ClientTests.swift` and in the Python integration suite under
  // `test/test_val_data.py`.

  // MARK: - Task Tests

  /// Test Task construction.
  func testTaskConstruction() {
    let task = Task(
      id: TaskId(value: 500),
      name: "export-dataset",
      workflow: "dataset-export",
      status: "running",
      manager: "worker-1",
      instance: "i-12345",
      created: "2024-01-15T10:00:00Z"
    )

    XCTAssertEqual(task.id.value, 500)
    XCTAssertEqual(task.name, "export-dataset")
    XCTAssertEqual(task.workflow, "dataset-export")
    XCTAssertEqual(task.status, "running")
    XCTAssertEqual(task.manager, "worker-1")
    XCTAssertEqual(task.instance, "i-12345")
    XCTAssertEqual(task.created, "2024-01-15T10:00:00Z")
  }

  /// Test Task with nil manager.
  func testTaskWithNilManager() {
    let task = Task(
      id: TaskId(value: 1),
      name: "task",
      workflow: "workflow",
      status: "pending",
      manager: nil,
      instance: "instance",
      created: "2024-01-01"
    )

    XCTAssertNil(task.manager)
  }

  /// Test Task equality.
  func testTaskEquality() {
    let task1 = Task(
      id: TaskId(value: 1), name: "task", workflow: "wf",
      status: "done", manager: nil, instance: "i", created: "2024"
    )
    let task2 = Task(
      id: TaskId(value: 1), name: "task", workflow: "wf",
      status: "done", manager: nil, instance: "i", created: "2024"
    )
    let task3 = Task(
      id: TaskId(value: 2), name: "task", workflow: "wf",
      status: "done", manager: nil, instance: "i", created: "2024"
    )

    XCTAssertEqual(task1, task2)
    XCTAssertNotEqual(task1, task3)
  }

  // MARK: - TaskInfo Tests
  //
  // `TaskInfo` is now a uniffi `Object` (not a Record) and exposes the
  // task data, chart, and download APIs as instance methods. Field-based
  // construction is therefore not available in Swift; end-to-end behaviour
  // is covered by `TaskInfoTests.testGetTaskInfo` and the Python
  // integration suite under `test/test_task_data.py` /
  // `test/test_task_charts.py`.

  // MARK: - Stage Tests

  /// Test Stage construction.
  func testStageConstruction() {
    let stage = Stage(
      stage: "training",
      status: "in_progress",
      message: "Epoch 50/100",
      percentage: 50
    )

    XCTAssertEqual(stage.stage, "training")
    XCTAssertEqual(stage.status, "in_progress")
    XCTAssertEqual(stage.message, "Epoch 50/100")
    XCTAssertEqual(stage.percentage, 50)
  }

  /// Test Stage with nil optional fields.
  func testStageWithNilFields() {
    let stage = Stage(
      stage: "initialization",
      status: nil,
      message: nil,
      percentage: 0
    )

    XCTAssertEqual(stage.stage, "initialization")
    XCTAssertNil(stage.status)
    XCTAssertNil(stage.message)
    XCTAssertEqual(stage.percentage, 0)
  }

  /// Test Stage equality.
  func testStageEquality() {
    let stage1 = Stage(stage: "test", status: "ok", message: nil, percentage: 100)
    let stage2 = Stage(stage: "test", status: "ok", message: nil, percentage: 100)
    let stage3 = Stage(stage: "test", status: "ok", message: nil, percentage: 50)

    XCTAssertEqual(stage1, stage2)
    XCTAssertNotEqual(stage1, stage3)
  }

  /// Test Stage percentage range.
  func testStagePercentageRange() {
    let stageZero = Stage(stage: "start", status: nil, message: nil, percentage: 0)
    let stageMax = Stage(stage: "end", status: nil, message: nil, percentage: 100)

    XCTAssertEqual(stageZero.percentage, 0)
    XCTAssertEqual(stageMax.percentage, 100)
  }

  // MARK: - SequenceId Tests

  /// Test SequenceId construction.
  func testSequenceIdConstruction() {
    let id = SequenceId(value: 12345)
    XCTAssertEqual(id.value, 12345)
  }

  /// Test SequenceId equality.
  func testSequenceIdEquality() {
    let id1 = SequenceId(value: 100)
    let id2 = SequenceId(value: 100)
    let id3 = SequenceId(value: 200)

    XCTAssertEqual(id1, id2)
    XCTAssertNotEqual(id1, id3)
  }

  /// Test SequenceId hashability.
  func testSequenceIdHashability() {
    var idSet: Set<SequenceId> = []

    idSet.insert(SequenceId(value: 1))
    idSet.insert(SequenceId(value: 2))
    idSet.insert(SequenceId(value: 1))  // Duplicate

    XCTAssertEqual(idSet.count, 2)
  }

  // MARK: - ImageId Tests

  /// Test ImageId construction.
  func testImageIdConstruction() {
    let id = ImageId(value: 54321)
    XCTAssertEqual(id.value, 54321)
  }

  /// Test ImageId equality.
  func testImageIdEquality() {
    let id1 = ImageId(value: 100)
    let id2 = ImageId(value: 100)
    let id3 = ImageId(value: 200)

    XCTAssertEqual(id1, id2)
    XCTAssertNotEqual(id1, id3)
  }

  /// Test ImageId hashability.
  func testImageIdHashability() {
    var idSet: Set<ImageId> = []

    idSet.insert(ImageId(value: 1))
    idSet.insert(ImageId(value: 2))
    idSet.insert(ImageId(value: 1))  // Duplicate

    XCTAssertEqual(idSet.count, 2)
  }

  // MARK: - AppId Tests

  /// Test AppId construction.
  func testAppIdConstruction() {
    let id = AppId(value: 99999)
    XCTAssertEqual(id.value, 99999)
  }

  /// Test AppId equality.
  func testAppIdEquality() {
    let id1 = AppId(value: 100)
    let id2 = AppId(value: 100)
    let id3 = AppId(value: 200)

    XCTAssertEqual(id1, id2)
    XCTAssertNotEqual(id1, id3)
  }

  // MARK: - TrainingSessionId Tests

  /// Test TrainingSessionId construction.
  func testTrainingSessionIdConstruction() {
    let id = TrainingSessionId(value: 123)
    XCTAssertEqual(id.value, 123)
  }

  /// Test TrainingSessionId equality.
  func testTrainingSessionIdEquality() {
    let id1 = TrainingSessionId(value: 100)
    let id2 = TrainingSessionId(value: 100)
    let id3 = TrainingSessionId(value: 200)

    XCTAssertEqual(id1, id2)
    XCTAssertNotEqual(id1, id3)
  }

  // MARK: - ValidationSessionId Tests

  /// Test ValidationSessionId construction.
  func testValidationSessionIdConstruction() {
    let id = ValidationSessionId(value: 456)
    XCTAssertEqual(id.value, 456)
  }

  /// Test ValidationSessionId equality.
  func testValidationSessionIdEquality() {
    let id1 = ValidationSessionId(value: 100)
    let id2 = ValidationSessionId(value: 100)
    let id3 = ValidationSessionId(value: 200)

    XCTAssertEqual(id1, id2)
    XCTAssertNotEqual(id1, id3)
  }

  // MARK: - Complex Entity Relationships Tests

  /// Test related entities share consistent IDs across Record types.
  ///
  /// `ValidationSession` is no longer a Record (it carries instance methods)
  /// so this relationship check focuses on the Record-typed IDs that wire
  /// the entities together; end-to-end cross-entity coverage lives in the
  /// online suite.
  func testEntityRelationships() {
    let experimentId = ExperimentId(value: 10)
    let trainingSessionId = TrainingSessionId(value: 20)

    let trainingSession = TrainingSession(
      id: trainingSessionId,
      experimentId: experimentId,
      name: "training",
      description: "",
      model: "model"
    )

    // Verify the Record-level IDs survive round-trip construction.
    XCTAssertEqual(trainingSession.experimentId.value, experimentId.value)
    XCTAssertEqual(trainingSession.id.value, trainingSessionId.value)
  }
}
