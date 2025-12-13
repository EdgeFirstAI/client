// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

/// Tests for basic client operations: version, token, and organization.
///
/// These tests verify the core client functionality including client creation,
/// authentication token management, and organization information retrieval.

import XCTest

@testable import EdgeFirstClient

final class ClientTests: XCTestCase {

  // MARK: - Offline Tests (No Credentials Required)

  /// Test client creation with memory storage without authentication.
  func testClientCreationWithMemoryStorage() throws {
    let client = try Client.withMemoryStorage()
    XCTAssertNotNil(client)
  }

  /// Test url() returns the default server URL.
  func testClientDefaultURL() throws {
    let client = try Client.withMemoryStorage()
    let url = client.url()
    XCTAssertFalse(url.isEmpty, "Default URL should not be empty")
  }

  /// Test client can be configured with test server.
  func testClientWithTestServer() throws {
    let client = try Client.withMemoryStorage()
    let testClient = try client.withServer(name: "test")
    let url = testClient.url()
    XCTAssertFalse(url.isEmpty, "Test server URL should not be empty")
  }

  // MARK: - Online Tests (Require Credentials)

  /// Test login with memory storage using environment credentials.
  func testLoginWithMemoryStorage() throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try TestConfig.getClient()
    XCTAssertNotNil(client)

    // Verify token works by calling verifyToken
    try client.verifyToken()
  }

  /// Test async login with memory storage.
  func testLoginAsync() async throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try await TestConfig.getClientAsync()
    try await client.verifyTokenAsync()
  }

  /// Test organization() returns complete organization details.
  func testOrganization() throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try TestConfig.getClient()
    let org = try client.organization()

    XCTAssertNotNil(org.id)
    XCTAssertFalse(org.name.isEmpty, "Organization name should not be empty")
    XCTAssertNotNil(org.credits)

    print("Organization: \(org.name)")
    print("ID: \(org.id.value)")
    print("Credits: \(org.credits)")
  }

  /// Test organization() async returns complete organization details.
  func testOrganizationAsync() async throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try await TestConfig.getClientAsync()
    let org = try await client.organizationAsync()

    XCTAssertNotNil(org.id)
    XCTAssertFalse(org.name.isEmpty, "Organization name should not be empty")
  }

  /// Test verifyToken() validates the authentication token.
  func testVerifyToken() throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try TestConfig.getClient()

    // Should not throw if token is valid
    try client.verifyToken()
  }

  /// Test verifyTokenAsync() validates the authentication token.
  func testVerifyTokenAsync() async throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try await TestConfig.getClientAsync()

    // Should not throw if token is valid
    try await client.verifyTokenAsync()
  }

  /// Test logout() clears the authentication token.
  func testLogout() throws {
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
    XCTAssertThrowsError(try client.verifyToken()) { error in
      print("Expected error after logout: \(error)")
    }
  }

  /// Test logoutAsync() clears the authentication token.
  func testLogoutAsync() async throws {
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
      print("Expected error after logout (async): \(error)")
    }
  }
}
