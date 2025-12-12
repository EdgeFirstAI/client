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
}
