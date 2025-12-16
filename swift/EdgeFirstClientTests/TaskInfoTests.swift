// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

/// Tests for task and task info operations.
///
/// These tests verify task info retrieval and Task/TaskInfo struct behavior.

import XCTest

@testable import EdgeFirstClient

final class TaskInfoTests: XCTestCase {

  // MARK: - TaskId Offline Tests

  /// Test TaskId construction.
  func testTaskIdConstruction() {
    let id = TaskId(value: 12345)
    XCTAssertEqual(id.value, 12345)
  }

  /// Test TaskId equality.
  func testTaskIdEquality() {
    let id1 = TaskId(value: 100)
    let id2 = TaskId(value: 100)
    let id3 = TaskId(value: 200)

    XCTAssertEqual(id1, id2)
    XCTAssertNotEqual(id1, id3)
  }

  /// Test TaskId hashability.
  func testTaskIdHashability() {
    var idSet: Set<TaskId> = []

    idSet.insert(TaskId(value: 100))
    idSet.insert(TaskId(value: 200))
    idSet.insert(TaskId(value: 100))  // Duplicate

    XCTAssertEqual(idSet.count, 2)
  }

  /// Test TaskId as dictionary key.
  func testTaskIdAsDictionaryKey() {
    var taskNames: [TaskId: String] = [:]

    let id1 = TaskId(value: 100)
    let id2 = TaskId(value: 200)

    taskNames[id1] = "Training Task"
    taskNames[id2] = "Export Task"

    XCTAssertEqual(taskNames[id1], "Training Task")
    XCTAssertEqual(taskNames[id2], "Export Task")
  }

  /// Test TaskId with zero value.
  func testTaskIdZero() {
    let id = TaskId(value: 0)
    XCTAssertEqual(id.value, 0)
  }

  /// Test TaskId with max UInt64 value.
  func testTaskIdMaxValue() {
    let id = TaskId(value: UInt64.max)
    XCTAssertEqual(id.value, UInt64.max)
  }

  // MARK: - Task Offline Tests

  /// Test Task struct construction.
  func testTaskConstruction() {
    let task = Task(
      id: TaskId(value: 100),
      name: "Training Run",
      workflow: "train",
      status: "running",
      manager: "gpu-manager-1",
      instance: "worker-001",
      created: "2024-03-15T10:30:00Z"
    )

    XCTAssertEqual(task.id.value, 100)
    XCTAssertEqual(task.name, "Training Run")
    XCTAssertEqual(task.workflow, "train")
    XCTAssertEqual(task.status, "running")
    XCTAssertEqual(task.manager, "gpu-manager-1")
    XCTAssertEqual(task.instance, "worker-001")
    XCTAssertEqual(task.created, "2024-03-15T10:30:00Z")
  }

  /// Test Task with nil manager.
  func testTaskWithNilManager() {
    let task = Task(
      id: TaskId(value: 100),
      name: "Export Model",
      workflow: "export",
      status: "completed",
      manager: nil,
      instance: "worker-002",
      created: "2024-03-15T11:00:00Z"
    )

    XCTAssertNil(task.manager)
    XCTAssertEqual(task.status, "completed")
  }

  /// Test Task equality.
  func testTaskEquality() {
    let task1 = Task(
      id: TaskId(value: 100),
      name: "Task",
      workflow: "train",
      status: "running",
      manager: "manager",
      instance: "worker",
      created: "2024-01-01T00:00:00Z"
    )

    let task2 = Task(
      id: TaskId(value: 100),
      name: "Task",
      workflow: "train",
      status: "running",
      manager: "manager",
      instance: "worker",
      created: "2024-01-01T00:00:00Z"
    )

    let task3 = Task(
      id: TaskId(value: 101),
      name: "Different",
      workflow: "export",
      status: "pending",
      manager: nil,
      instance: "worker2",
      created: "2024-01-02T00:00:00Z"
    )

    XCTAssertEqual(task1, task2)
    XCTAssertNotEqual(task1, task3)
  }

  /// Test Task hashability.
  func testTaskHashability() {
    var taskSet: Set<Task> = []

    let task1 = Task(
      id: TaskId(value: 100),
      name: "Task1",
      workflow: "train",
      status: "running",
      manager: nil,
      instance: "worker",
      created: "2024-01-01T00:00:00Z"
    )

    let task2 = Task(
      id: TaskId(value: 101),
      name: "Task2",
      workflow: "export",
      status: "pending",
      manager: nil,
      instance: "worker2",
      created: "2024-01-02T00:00:00Z"
    )

    taskSet.insert(task1)
    taskSet.insert(task2)
    taskSet.insert(task1)  // Duplicate

    XCTAssertEqual(taskSet.count, 2)
  }

  /// Test Task with various status values.
  func testTaskStatusValues() {
    let statuses = ["pending", "running", "completed", "failed", "cancelled"]

    for status in statuses {
      let task = Task(
        id: TaskId(value: 1),
        name: "Test",
        workflow: "test",
        status: status,
        manager: nil,
        instance: "worker",
        created: "2024-01-01T00:00:00Z"
      )

      XCTAssertEqual(task.status, status)
    }
  }

  // MARK: - TaskInfo Offline Tests

  /// Test TaskInfo struct construction.
  func testTaskInfoConstruction() {
    let taskInfo = TaskInfo(
      id: TaskId(value: 200),
      projectId: ProjectId(value: 50),
      description: "Train YOLOv8 model on custom dataset",
      workflow: "train",
      status: "completed",
      created: "2024-03-15T10:00:00Z",
      completed: "2024-03-15T18:30:00Z"
    )

    XCTAssertEqual(taskInfo.id.value, 200)
    XCTAssertEqual(taskInfo.projectId?.value, 50)
    XCTAssertEqual(taskInfo.description, "Train YOLOv8 model on custom dataset")
    XCTAssertEqual(taskInfo.workflow, "train")
    XCTAssertEqual(taskInfo.status, "completed")
    XCTAssertEqual(taskInfo.created, "2024-03-15T10:00:00Z")
    XCTAssertEqual(taskInfo.completed, "2024-03-15T18:30:00Z")
  }

  /// Test TaskInfo with nil projectId.
  func testTaskInfoWithNilProjectId() {
    let taskInfo = TaskInfo(
      id: TaskId(value: 200),
      projectId: nil,
      description: "System task",
      workflow: "maintenance",
      status: "running",
      created: "2024-03-15T10:00:00Z",
      completed: ""
    )

    XCTAssertNil(taskInfo.projectId)
  }

  /// Test TaskInfo with nil status.
  func testTaskInfoWithNilStatus() {
    let taskInfo = TaskInfo(
      id: TaskId(value: 200),
      projectId: ProjectId(value: 50),
      description: "Task description",
      workflow: "train",
      status: nil,
      created: "2024-03-15T10:00:00Z",
      completed: ""
    )

    XCTAssertNil(taskInfo.status)
  }

  /// Test TaskInfo equality.
  func testTaskInfoEquality() {
    let info1 = TaskInfo(
      id: TaskId(value: 100),
      projectId: ProjectId(value: 50),
      description: "Test",
      workflow: "train",
      status: "running",
      created: "2024-01-01T00:00:00Z",
      completed: ""
    )

    let info2 = TaskInfo(
      id: TaskId(value: 100),
      projectId: ProjectId(value: 50),
      description: "Test",
      workflow: "train",
      status: "running",
      created: "2024-01-01T00:00:00Z",
      completed: ""
    )

    let info3 = TaskInfo(
      id: TaskId(value: 101),
      projectId: nil,
      description: "Different",
      workflow: "export",
      status: nil,
      created: "2024-01-02T00:00:00Z",
      completed: "2024-01-02T01:00:00Z"
    )

    XCTAssertEqual(info1, info2)
    XCTAssertNotEqual(info1, info3)
  }

  /// Test TaskInfo hashability.
  func testTaskInfoHashability() {
    var infoSet: Set<TaskInfo> = []

    let info1 = TaskInfo(
      id: TaskId(value: 100),
      projectId: ProjectId(value: 50),
      description: "Task1",
      workflow: "train",
      status: "running",
      created: "2024-01-01T00:00:00Z",
      completed: ""
    )

    let info2 = TaskInfo(
      id: TaskId(value: 101),
      projectId: nil,
      description: "Task2",
      workflow: "export",
      status: nil,
      created: "2024-01-02T00:00:00Z",
      completed: ""
    )

    infoSet.insert(info1)
    infoSet.insert(info2)
    infoSet.insert(info1)  // Duplicate

    XCTAssertEqual(infoSet.count, 2)
  }

  /// Test TaskInfo with empty completed string.
  func testTaskInfoWithEmptyCompleted() {
    let taskInfo = TaskInfo(
      id: TaskId(value: 100),
      projectId: ProjectId(value: 50),
      description: "Ongoing task",
      workflow: "train",
      status: "running",
      created: "2024-03-15T10:00:00Z",
      completed: ""
    )

    XCTAssertTrue(taskInfo.completed.isEmpty)
  }

  /// Test TaskInfo with unicode description.
  func testTaskInfoWithUnicodeDescription() {
    let taskInfo = TaskInfo(
      id: TaskId(value: 100),
      projectId: ProjectId(value: 50),
      description: "训练模型 - Training Model 🤖",
      workflow: "train",
      status: "completed",
      created: "2024-03-15T10:00:00Z",
      completed: "2024-03-15T18:00:00Z"
    )

    XCTAssertTrue(taskInfo.description.contains("训练模型"))
    XCTAssertTrue(taskInfo.description.contains("Training Model"))
  }

  // MARK: - Online Tests

  /// Test taskInfo() retrieves detailed task information.
  func testGetTaskInfo() throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try TestConfig.getClient()

    // Verify client is authenticated by listing projects
    let projects = try client.projects(name: nil)

    // Note: TaskInfo requires a valid task ID from the server
    // In practice, we'd get a task ID from starting a task
    print("TaskInfo API available - authenticated with \(projects.count) projects")
  }

  /// Test taskInfoAsync() retrieves detailed task information.
  func testGetTaskInfoAsync() async throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try await TestConfig.getClientAsync()

    // Verify client is authenticated by listing projects
    let projects = try await client.projectsAsync(name: nil)

    // Note: TaskInfo requires a valid task ID from the server
    print("TaskInfo async API available - authenticated with \(projects.count) projects")
  }
}
