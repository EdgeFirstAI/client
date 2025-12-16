// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

/// Tests for dataset listing operations.
///
/// These tests verify the client can list and retrieve datasets
/// from EdgeFirst Studio projects.

import XCTest

@testable import EdgeFirstClient

final class DatasetTests: XCTestCase {

  /// Test datasets() returns a list of datasets for a project.
  func testListDatasets() throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try TestConfig.getClient()
    let projects = try client.projects(name: nil)

    guard let project = projects.first else {
      XCTFail("Need at least one project to test datasets")
      return
    }

    let datasets = try client.datasets(projectId: project.id, name: nil)

    // Datasets may be empty for some projects, so just verify the call works
    print("Found \(datasets.count) datasets in project \(project.name)")

    if let first = datasets.first {
      XCTAssertNotNil(first.id)
      XCTAssertFalse(first.name.isEmpty, "Dataset name should not be empty")
      print("First dataset: \(first.name)")
    }
  }

  /// Test datasetsAsync() returns a list of datasets.
  func testListDatasetsAsync() async throws {
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

    let datasets = try await client.datasetsAsync(
      projectId: project.id,
      name: nil
    )
    print("Found \(datasets.count) datasets (async)")
  }

  /// Test datasetAsync() retrieves a single dataset by ID.
  func testGetDatasetByIdAsync() async throws {
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

    let datasets = try await client.datasetsAsync(projectId: project.id, name: nil)

    guard let first = datasets.first else {
      print("No datasets in project \(project.name), skipping ID test (async)")
      return
    }

    // Fetch the same dataset by ID
    let dataset = try await client.datasetAsync(id: first.id)

    XCTAssertEqual(dataset.id.value, first.id.value)
    XCTAssertEqual(dataset.name, first.name)
    print("Retrieved dataset (async): \(dataset.name) (ID: \(dataset.id.value))")
  }

  /// Test annotationSetsAsync() returns annotation sets for a dataset.
  func testAnnotationSetsAsync() async throws {
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

    let datasets = try await client.datasetsAsync(projectId: project.id, name: nil)

    guard let dataset = datasets.first else {
      print("No datasets in project \(project.name), skipping annotation set test (async)")
      return
    }

    let annotationSets = try await client.annotationSetsAsync(datasetId: dataset.id)

    print("Found \(annotationSets.count) annotation sets (async) in dataset \(dataset.name)")

    if let first = annotationSets.first {
      XCTAssertNotNil(first.id)
      XCTAssertFalse(first.name.isEmpty, "Annotation set name should not be empty")
    }
  }

  /// Test labelsAsync() returns labels for a dataset.
  func testLabelsAsync() async throws {
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

    let datasets = try await client.datasetsAsync(projectId: project.id, name: nil)

    guard let dataset = datasets.first else {
      print("No datasets in project \(project.name), skipping labels test (async)")
      return
    }

    let labels = try await client.labelsAsync(datasetId: dataset.id)

    print("Found \(labels.count) labels (async) in dataset \(dataset.name)")

    if let first = labels.first {
      XCTAssertNotNil(first.id)
      XCTAssertFalse(first.name.isEmpty, "Label name should not be empty")
    }
  }

  /// Test dataset() retrieves a single dataset by ID.
  func testGetDatasetById() throws {
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

    let datasets = try client.datasets(projectId: project.id, name: nil)

    guard let first = datasets.first else {
      print("No datasets in project \(project.name), skipping ID test")
      return
    }

    // Fetch the same dataset by ID
    let dataset = try client.dataset(id: first.id)

    XCTAssertEqual(dataset.id.value, first.id.value)
    XCTAssertEqual(dataset.name, first.name)
    print("Retrieved dataset: \(dataset.name) (ID: \(dataset.id.value))")
  }

  /// Test annotationSets() returns annotation sets for a dataset.
  func testAnnotationSets() throws {
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

    let datasets = try client.datasets(projectId: project.id, name: nil)

    guard let dataset = datasets.first else {
      print("No datasets in project \(project.name), skipping annotation set test")
      return
    }

    let annotationSets = try client.annotationSets(datasetId: dataset.id)

    print("Found \(annotationSets.count) annotation sets in dataset \(dataset.name)")

    if let first = annotationSets.first {
      XCTAssertNotNil(first.id)
      XCTAssertFalse(first.name.isEmpty, "Annotation set name should not be empty")
      print("First annotation set: \(first.name)")
    }
  }

  /// Test labels() returns labels for a dataset.
  func testLabels() throws {
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

    let datasets = try client.datasets(projectId: project.id, name: nil)

    guard let dataset = datasets.first else {
      print("No datasets in project \(project.name), skipping labels test")
      return
    }

    let labels = try client.labels(datasetId: dataset.id)

    print("Found \(labels.count) labels in dataset \(dataset.name)")

    if let first = labels.first {
      XCTAssertNotNil(first.id)
      XCTAssertFalse(first.name.isEmpty, "Label name should not be empty")
      print("First label: \(first.name)")
    }
  }

  // MARK: - Offline Struct Tests

  /// Test DatasetId construction.
  func testDatasetIdConstruction() {
    let id = DatasetId(value: 12345)
    XCTAssertEqual(id.value, 12345)
  }

  /// Test DatasetId equality.
  func testDatasetIdEquality() {
    let id1 = DatasetId(value: 100)
    let id2 = DatasetId(value: 100)
    let id3 = DatasetId(value: 200)

    XCTAssertEqual(id1, id2)
    XCTAssertNotEqual(id1, id3)
  }

  /// Test DatasetId hashability.
  func testDatasetIdHashability() {
    var idSet: Set<DatasetId> = []

    idSet.insert(DatasetId(value: 100))
    idSet.insert(DatasetId(value: 200))
    idSet.insert(DatasetId(value: 100))  // Duplicate

    XCTAssertEqual(idSet.count, 2)
  }

  /// Test AnnotationSetId construction.
  func testAnnotationSetIdConstruction() {
    let id = AnnotationSetId(value: 67890)
    XCTAssertEqual(id.value, 67890)
  }

  /// Test AnnotationSetId equality.
  func testAnnotationSetIdEquality() {
    let id1 = AnnotationSetId(value: 100)
    let id2 = AnnotationSetId(value: 100)
    let id3 = AnnotationSetId(value: 200)

    XCTAssertEqual(id1, id2)
    XCTAssertNotEqual(id1, id3)
  }

  /// Test Label struct construction.
  func testLabelConstruction() {
    let label = Label(id: 1, name: "person")

    XCTAssertEqual(label.id, 1)
    XCTAssertEqual(label.name, "person")
  }

  /// Test Label equality.
  func testLabelEquality() {
    let label1 = Label(id: 1, name: "car")
    let label2 = Label(id: 1, name: "car")
    let label3 = Label(id: 2, name: "truck")

    XCTAssertEqual(label1, label2)
    XCTAssertNotEqual(label1, label3)
  }

  /// Test Label hashability.
  func testLabelHashability() {
    var labelSet: Set<Label> = []

    let label1 = Label(id: 1, name: "person")
    let label2 = Label(id: 2, name: "car")
    let label3 = Label(id: 1, name: "person")  // Duplicate

    labelSet.insert(label1)
    labelSet.insert(label2)
    labelSet.insert(label3)

    XCTAssertEqual(labelSet.count, 2)
  }

  /// Test Label with empty name.
  func testLabelWithEmptyName() {
    let label = Label(id: 1, name: "")
    XCTAssertTrue(label.name.isEmpty)
  }

  /// Test Label with unicode name.
  func testLabelWithUnicodeName() {
    let label = Label(id: 1, name: "行人 - Pedestrian")
    XCTAssertTrue(label.name.contains("行人"))
    XCTAssertTrue(label.name.contains("Pedestrian"))
  }

  /// Test AnnotationSet struct construction.
  func testAnnotationSetConstruction() {
    let annotationSet = AnnotationSet(
      id: AnnotationSetId(value: 500),
      datasetId: DatasetId(value: 100),
      name: "Ground Truth",
      description: "Main annotation set for ground truth labels",
      created: "2024-02-01T08:00:00Z"
    )

    XCTAssertEqual(annotationSet.id.value, 500)
    XCTAssertEqual(annotationSet.datasetId.value, 100)
    XCTAssertEqual(annotationSet.name, "Ground Truth")
    XCTAssertEqual(annotationSet.description, "Main annotation set for ground truth labels")
    XCTAssertEqual(annotationSet.created, "2024-02-01T08:00:00Z")
  }

  /// Test AnnotationSet equality.
  func testAnnotationSetEquality() {
    let set1 = AnnotationSet(
      id: AnnotationSetId(value: 100),
      datasetId: DatasetId(value: 50),
      name: "Test",
      description: "Test description",
      created: "2024-01-01T00:00:00Z"
    )

    let set2 = AnnotationSet(
      id: AnnotationSetId(value: 100),
      datasetId: DatasetId(value: 50),
      name: "Test",
      description: "Test description",
      created: "2024-01-01T00:00:00Z"
    )

    let set3 = AnnotationSet(
      id: AnnotationSetId(value: 101),
      datasetId: DatasetId(value: 51),
      name: "Different",
      description: "Different description",
      created: "2024-01-02T00:00:00Z"
    )

    XCTAssertEqual(set1, set2)
    XCTAssertNotEqual(set1, set3)
  }

  /// Test AnnotationSet hashability.
  func testAnnotationSetHashability() {
    var setCollection: Set<AnnotationSet> = []

    let set1 = AnnotationSet(
      id: AnnotationSetId(value: 100),
      datasetId: DatasetId(value: 50),
      name: "Set1",
      description: "First set",
      created: "2024-01-01T00:00:00Z"
    )

    let set2 = AnnotationSet(
      id: AnnotationSetId(value: 101),
      datasetId: DatasetId(value: 51),
      name: "Set2",
      description: "Second set",
      created: "2024-01-02T00:00:00Z"
    )

    setCollection.insert(set1)
    setCollection.insert(set2)
    setCollection.insert(set1)  // Duplicate

    XCTAssertEqual(setCollection.count, 2)
  }

  /// Test DatasetId with zero value.
  func testDatasetIdZero() {
    let id = DatasetId(value: 0)
    XCTAssertEqual(id.value, 0)
  }

  /// Test DatasetId with max UInt64 value.
  func testDatasetIdMaxValue() {
    let id = DatasetId(value: UInt64.max)
    XCTAssertEqual(id.value, UInt64.max)
  }

  /// Test Label ID max value.
  func testLabelIdMaxValue() {
    let label = Label(id: UInt64.max, name: "test")
    XCTAssertEqual(label.id, UInt64.max)
  }
}
