// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

/// Tests for snapshot operations.
///
/// These tests verify the client can list and retrieve snapshots
/// from EdgeFirst Studio. Matches Python test patterns in test_snapshots.py.

import XCTest

@testable import EdgeFirstClient

final class SnapshotTests: XCTestCase {

  /// Test snapshots() returns a list of snapshots.
  func testListSnapshots() throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try TestConfig.getClient()
    let snapshots = try client.snapshots(name: nil)

    print("Found \(snapshots.count) snapshots")

    if let first = snapshots.first {
      XCTAssertGreaterThan(first.id.value, 0)
      XCTAssertFalse(first.status.isEmpty, "Snapshot status should not be empty")
      print("First snapshot ID: \(first.id.value)")
      print("First snapshot status: \(first.status)")
    }
  }

  /// Test snapshotsAsync() returns a list of snapshots.
  func testListSnapshotsAsync() async throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try await TestConfig.getClientAsync()
    let snapshots = try await client.snapshotsAsync(name: nil)

    print("Found \(snapshots.count) snapshots (async)")
  }

  /// Test snapshot() retrieves a single snapshot by ID.
  func testGetSnapshotById() throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try TestConfig.getClient()
    let snapshots = try client.snapshots(name: nil)

    guard let first = snapshots.first else {
      print("No snapshots available, skipping ID test")
      return
    }

    // Fetch the same snapshot by ID
    let snapshot = try client.snapshot(id: first.id)

    XCTAssertEqual(snapshot.id.value, first.id.value)
    XCTAssertEqual(snapshot.status, first.status)
    print("Retrieved snapshot: ID=\(snapshot.id.value), status=\(snapshot.status)")
  }

  /// Test snapshotAsync() retrieves a single snapshot by ID.
  func testGetSnapshotByIdAsync() async throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try await TestConfig.getClientAsync()
    let snapshots = try await client.snapshotsAsync(name: nil)

    guard let first = snapshots.first else {
      print("No snapshots available, skipping async ID test")
      return
    }

    // Fetch the same snapshot by ID
    let snapshot = try await client.snapshotAsync(id: first.id)

    XCTAssertEqual(snapshot.id.value, first.id.value)
    XCTAssertEqual(snapshot.status, first.status)
    print("Retrieved snapshot (async): ID=\(snapshot.id.value)")
  }

  /// Test snapshot properties are accessible.
  func testSnapshotProperties() throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try TestConfig.getClient()
    let snapshots = try client.snapshots(name: nil)

    guard let snapshot = snapshots.first else {
      print("No snapshots available, skipping properties test")
      return
    }

    // Verify all properties are accessible
    XCTAssertGreaterThan(snapshot.id.value, 0)
    XCTAssertNotNil(snapshot.description)
    XCTAssertFalse(snapshot.status.isEmpty)
    XCTAssertNotNil(snapshot.path)
    XCTAssertFalse(snapshot.created.isEmpty)

    print("Snapshot properties:")
    print("  ID: \(snapshot.id.value)")
    print("  Description: \(snapshot.description)")
    print("  Status: \(snapshot.status)")
    print("  Path: \(snapshot.path)")
    print("  Created: \(snapshot.created)")
  }

  /// Test snapshot ID consistency between list and get.
  func testSnapshotIdConsistency() throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try TestConfig.getClient()
    let snapshots = try client.snapshots(name: nil)

    guard let first = snapshots.first else {
      print("No snapshots available")
      return
    }

    let snapshot = try client.snapshot(id: first.id)

    // IDs should match exactly
    XCTAssertEqual(snapshot.id.value, first.id.value)

    // All properties should match
    XCTAssertEqual(snapshot.description, first.description)
    XCTAssertEqual(snapshot.status, first.status)
    XCTAssertEqual(snapshot.path, first.path)
    XCTAssertEqual(snapshot.created, first.created)
  }

  /// Test snapshot filtering by name.
  func testSnapshotsWithNameFilter() throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try TestConfig.getClient()

    // Get all snapshots first
    let allSnapshots = try client.snapshots(name: nil)

    // If we have snapshots, test filtering
    if let first = allSnapshots.first {
      // Filter by description (name parameter)
      // Note: This tests the API works, actual filtering depends on server behavior
      let filtered = try client.snapshots(name: first.description)
      print("Filtered snapshots: \(filtered.count) (filtered by description)")
    }
  }

  /// Test async snapshot ID consistency.
  func testSnapshotIdConsistencyAsync() async throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try await TestConfig.getClientAsync()
    let snapshots = try await client.snapshotsAsync(name: nil)

    guard let first = snapshots.first else {
      print("No snapshots available")
      return
    }

    let snapshot = try await client.snapshotAsync(id: first.id)

    // IDs should match exactly
    XCTAssertEqual(snapshot.id.value, first.id.value)
    XCTAssertEqual(snapshot.status, first.status)
  }

  // MARK: - Offline Struct Tests

  /// Test Snapshot struct construction.
  func testSnapshotConstruction() {
    let snapshot = Snapshot(
      id: SnapshotId(value: 100),
      description: "Model v1.0 Release",
      status: "ready",
      path: "/snapshots/v1.0",
      created: "2024-03-15T10:30:00Z"
    )

    XCTAssertEqual(snapshot.id.value, 100)
    XCTAssertEqual(snapshot.description, "Model v1.0 Release")
    XCTAssertEqual(snapshot.status, "ready")
    XCTAssertEqual(snapshot.path, "/snapshots/v1.0")
    XCTAssertEqual(snapshot.created, "2024-03-15T10:30:00Z")
  }

  /// Test Snapshot equality.
  func testSnapshotEquality() {
    let snapshot1 = Snapshot(
      id: SnapshotId(value: 100),
      description: "Test Snapshot",
      status: "ready",
      path: "/path/to/snapshot",
      created: "2024-01-01T00:00:00Z"
    )

    let snapshot2 = Snapshot(
      id: SnapshotId(value: 100),
      description: "Test Snapshot",
      status: "ready",
      path: "/path/to/snapshot",
      created: "2024-01-01T00:00:00Z"
    )

    let snapshot3 = Snapshot(
      id: SnapshotId(value: 101),
      description: "Different Snapshot",
      status: "pending",
      path: "/other/path",
      created: "2024-01-02T00:00:00Z"
    )

    XCTAssertEqual(snapshot1, snapshot2)
    XCTAssertNotEqual(snapshot1, snapshot3)
  }

  /// Test Snapshot hashability.
  func testSnapshotHashability() {
    var snapshotSet: Set<Snapshot> = []

    let snapshot1 = Snapshot(
      id: SnapshotId(value: 100),
      description: "Snapshot 1",
      status: "ready",
      path: "/path1",
      created: "2024-01-01T00:00:00Z"
    )

    let snapshot2 = Snapshot(
      id: SnapshotId(value: 101),
      description: "Snapshot 2",
      status: "pending",
      path: "/path2",
      created: "2024-01-02T00:00:00Z"
    )

    let duplicateSnapshot = Snapshot(
      id: SnapshotId(value: 100),
      description: "Snapshot 1",
      status: "ready",
      path: "/path1",
      created: "2024-01-01T00:00:00Z"
    )

    snapshotSet.insert(snapshot1)
    snapshotSet.insert(snapshot2)
    snapshotSet.insert(duplicateSnapshot)  // Duplicate should not increase count

    XCTAssertEqual(snapshotSet.count, 2)
  }

  // MARK: - SnapshotId Tests

  /// Test SnapshotId construction.
  func testSnapshotIdConstruction() {
    let id = SnapshotId(value: 12345)
    XCTAssertEqual(id.value, 12345)
  }

  /// Test SnapshotId equality.
  func testSnapshotIdEquality() {
    let id1 = SnapshotId(value: 100)
    let id2 = SnapshotId(value: 100)
    let id3 = SnapshotId(value: 200)

    XCTAssertEqual(id1, id2)
    XCTAssertNotEqual(id1, id3)
  }

  /// Test SnapshotId hashability.
  func testSnapshotIdHashability() {
    var idSet: Set<SnapshotId> = []

    idSet.insert(SnapshotId(value: 100))
    idSet.insert(SnapshotId(value: 200))
    idSet.insert(SnapshotId(value: 100))  // Duplicate

    XCTAssertEqual(idSet.count, 2)
  }

  /// Test SnapshotId as dictionary key.
  func testSnapshotIdAsDictionaryKey() {
    var snapshotNames: [SnapshotId: String] = [:]

    let id1 = SnapshotId(value: 100)
    let id2 = SnapshotId(value: 200)

    snapshotNames[id1] = "Model v1.0"
    snapshotNames[id2] = "Model v2.0"

    XCTAssertEqual(snapshotNames[id1], "Model v1.0")
    XCTAssertEqual(snapshotNames[id2], "Model v2.0")
  }

  // MARK: - Snapshot Edge Case Tests

  /// Test Snapshot with various status values.
  func testSnapshotStatusValues() {
    let statuses = ["ready", "pending", "processing", "failed", "cancelled"]

    for status in statuses {
      let snapshot = Snapshot(
        id: SnapshotId(value: 1),
        description: "Test",
        status: status,
        path: "/path",
        created: "2024-01-01T00:00:00Z"
      )

      XCTAssertEqual(snapshot.status, status)
    }
  }

  /// Test Snapshot with empty description.
  func testSnapshotWithEmptyDescription() {
    let snapshot = Snapshot(
      id: SnapshotId(value: 1),
      description: "",
      status: "ready",
      path: "/path",
      created: "2024-01-01T00:00:00Z"
    )

    XCTAssertTrue(snapshot.description.isEmpty)
  }

  /// Test Snapshot with empty path.
  func testSnapshotWithEmptyPath() {
    let snapshot = Snapshot(
      id: SnapshotId(value: 1),
      description: "Test",
      status: "ready",
      path: "",
      created: "2024-01-01T00:00:00Z"
    )

    XCTAssertTrue(snapshot.path.isEmpty)
  }

  /// Test Snapshot with unicode characters.
  func testSnapshotWithUnicode() {
    let snapshot = Snapshot(
      id: SnapshotId(value: 1),
      description: "模型快照 v1.0 - 日本語テスト",
      status: "ready",
      path: "/snapshots/模型/v1.0",
      created: "2024-01-01T00:00:00Z"
    )

    XCTAssertTrue(snapshot.description.contains("模型快照"))
    XCTAssertTrue(snapshot.description.contains("日本語"))
    XCTAssertTrue(snapshot.path.contains("模型"))
  }

  /// Test SnapshotId with zero value.
  func testSnapshotIdZero() {
    let id = SnapshotId(value: 0)
    XCTAssertEqual(id.value, 0)
  }

  /// Test SnapshotId with max UInt64 value.
  func testSnapshotIdMaxValue() {
    let id = SnapshotId(value: UInt64.max)
    XCTAssertEqual(id.value, UInt64.max)
  }
}
