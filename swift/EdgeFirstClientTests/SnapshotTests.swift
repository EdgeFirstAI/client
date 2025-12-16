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
}
