// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

/// Tests for token storage API.
///
/// These tests verify TokenStorage protocol implementations and custom storage
/// behavior, matching the Python test patterns in test_storage.py.

import XCTest

@testable import EdgeFirstClient

/// Simple in-memory storage for testing the TokenStorage protocol.
final class DictStorage: TokenStorage, @unchecked Sendable {
  private var token: String?

  func store(token: String) {
    self.token = token
  }

  func load() -> String? {
    return token
  }

  func clear() {
    token = nil
  }
}

/// Storage that records method calls for testing.
final class TracingStorage: TokenStorage, @unchecked Sendable {
  private var token: String?
  var calls: [(String, String?)] = []

  func store(token: String) {
    calls.append(("store", token))
    self.token = token
  }

  func load() -> String? {
    calls.append(("load", nil))
    return token
  }

  func clear() {
    calls.append(("clear", nil))
    token = nil
  }
}

final class StorageTests: XCTestCase {

  // MARK: - Custom TokenStorage Protocol Tests

  /// Test DictStorage basic store/load/clear operations.
  func testDictStorageBasicOperations() {
    let storage = DictStorage()

    // Initially empty
    XCTAssertNil(storage.load())

    // Store token
    storage.store(token: "test-token-123")
    XCTAssertEqual(storage.load(), "test-token-123")

    // Clear token
    storage.clear()
    XCTAssertNil(storage.load())
  }

  /// Test that storing overwrites previous token.
  func testDictStorageOverwrite() {
    let storage = DictStorage()

    storage.store(token: "token-1")
    XCTAssertEqual(storage.load(), "token-1")

    storage.store(token: "token-2")
    XCTAssertEqual(storage.load(), "token-2")
  }

  /// Test TracingStorage records method calls correctly.
  func testTracingStorageCalls() {
    let storage = TracingStorage()

    // Load initially (should be nil)
    let initial = storage.load()
    XCTAssertNil(initial)
    XCTAssertEqual(storage.calls.count, 1)
    XCTAssertEqual(storage.calls[0].0, "load")

    // Store a token
    storage.store(token: "traced-token")
    XCTAssertEqual(storage.calls.count, 2)
    XCTAssertEqual(storage.calls[1].0, "store")
    XCTAssertEqual(storage.calls[1].1, "traced-token")

    // Load again
    let loaded = storage.load()
    XCTAssertEqual(loaded, "traced-token")
    XCTAssertEqual(storage.calls.count, 3)

    // Clear
    storage.clear()
    XCTAssertEqual(storage.calls.count, 4)
    XCTAssertEqual(storage.calls[3].0, "clear")
  }

  /// Test separate storage instances are independent.
  func testStorageInstancesIndependent() {
    let storage1 = DictStorage()
    let storage2 = DictStorage()

    storage1.store(token: "token-1")
    storage2.store(token: "token-2")

    XCTAssertEqual(storage1.load(), "token-1")
    XCTAssertEqual(storage2.load(), "token-2")
  }

  // MARK: - Client Memory Storage Tests

  /// Test Client.withMemoryStorage() creates a client with memory storage.
  func testClientWithMemoryStorage() throws {
    let client = try Client.withMemoryStorage()
    XCTAssertNotNil(client)
  }

  /// Test default Client URL is saas.
  func testDefaultClientURLIsSaas() throws {
    let client = try Client.withMemoryStorage()
    XCTAssertEqual(client.url(), "https://edgefirst.studio")
  }

  // MARK: - Storage Authentication Integration Tests

  /// Test login with memory storage stores token and can make authenticated calls.
  func testMemoryStorageAuthenticationRoundtrip() throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try TestConfig.getClient()

    // Verify we can make authenticated API calls
    let org = try client.organization()
    XCTAssertNotNil(org)
    XCTAssertFalse(org.name.isEmpty)
  }

  /// Test logout clears the stored token.
  func testLogoutClearsStorage() throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try TestConfig.getClient()

    // Verify we're logged in first
    try client.verifyToken()

    // Logout should succeed
    try client.logout()

    // After logout, verifyToken should fail
    XCTAssertThrowsError(try client.verifyToken())
  }

  /// Test async logout clears the stored token.
  func testLogoutClearsStorageAsync() async throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try await TestConfig.getClientAsync()

    // Verify we're logged in first
    try await client.verifyTokenAsync()

    // Logout should succeed
    try await client.logoutAsync()

    // After logout, verifyToken should fail
    do {
      try await client.verifyTokenAsync()
      XCTFail("verifyTokenAsync should throw after logout")
    } catch {
      // Expected
    }
  }
}
