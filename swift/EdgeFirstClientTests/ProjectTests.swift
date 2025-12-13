// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

/// Tests for project listing operations.
///
/// These tests verify the client can list and retrieve projects
/// from EdgeFirst Studio.

import XCTest

@testable import EdgeFirstClient

final class ProjectTests: XCTestCase {

  /// Test projects() returns a list of projects.
  func testListProjects() throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try TestConfig.getClient()
    let projects = try client.projects(name: nil)

    XCTAssertGreaterThan(
      projects.count,
      0,
      "Should have at least one project"
    )

    if let first = projects.first {
      XCTAssertNotNil(first.id)
      XCTAssertFalse(first.name.isEmpty, "Project name should not be empty")
      print("First project: \(first.name) (ID: \(first.id.value))")
    }
  }

  /// Test projectsAsync() returns a list of projects.
  func testListProjectsAsync() async throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try await TestConfig.getClientAsync()
    let projects = try await client.projectsAsync(name: nil)

    XCTAssertGreaterThan(
      projects.count,
      0,
      "Should have at least one project"
    )
  }

  /// Test projectAsync() retrieves a single project by ID.
  func testGetProjectByIdAsync() async throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try await TestConfig.getClientAsync()
    let projects = try await client.projectsAsync(name: nil)

    guard let first = projects.first else {
      XCTFail("Need at least one project to test")
      return
    }

    // Fetch the same project by ID
    let project = try await client.projectAsync(id: first.id)

    XCTAssertEqual(project.id.value, first.id.value)
    XCTAssertEqual(project.name, first.name)
    print("Retrieved project (async): \(project.name) (ID: \(project.id.value))")
  }

  /// Test projectsAsync() with name filter.
  func testListProjectsByNameAsync() async throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try await TestConfig.getClientAsync()

    // First get all projects
    let allProjects = try await client.projectsAsync(name: nil)
    guard let first = allProjects.first else {
      XCTFail("Need at least one project to test filtering")
      return
    }

    // Filter by the first project's name
    let filtered = try await client.projectsAsync(name: first.name)

    XCTAssertGreaterThan(
      filtered.count,
      0,
      "Should find at least one project with name filter"
    )
    XCTAssertTrue(
      filtered.contains { $0.name == first.name },
      "Filtered results should contain the searched project"
    )
  }

  /// Test project() retrieves a single project by ID.
  func testGetProjectById() throws {
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

    // Fetch the same project by ID
    let project = try client.project(id: first.id)

    XCTAssertEqual(project.id.value, first.id.value)
    XCTAssertEqual(project.name, first.name)
    print("Retrieved project: \(project.name) (ID: \(project.id.value))")
  }

  /// Test projects() with name filter.
  func testListProjectsByName() throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try TestConfig.getClient()

    // First get all projects
    let allProjects = try client.projects(name: nil)
    guard let first = allProjects.first else {
      XCTFail("Need at least one project to test filtering")
      return
    }

    // Filter by the first project's name
    let filtered = try client.projects(name: first.name)

    XCTAssertGreaterThan(
      filtered.count,
      0,
      "Should find at least one project with name filter"
    )
    XCTAssertTrue(
      filtered.contains { $0.name == first.name },
      "Filtered results should contain the searched project"
    )
  }
}
