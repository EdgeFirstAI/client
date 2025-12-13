// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

/// Test utilities for EdgeFirst Client Swift SDK.
///
/// Provides authenticated client creation and test configuration
/// matching the Python test utilities in test/__init__.py.

import Foundation

@testable import EdgeFirstClient

/// Test configuration errors.
enum TestConfigError: Error, LocalizedError {
  case missingCredentials(String)

  var errorDescription: String? {
    switch self {
    case .missingCredentials(let message):
      return message
    }
  }
}

/// Test configuration and utilities.
///
/// Supports authentication via:
/// - STUDIO_TOKEN environment variable (direct token)
/// - STUDIO_USERNAME and STUDIO_PASSWORD environment variables (login)
///
/// The STUDIO_SERVER environment variable can specify the server instance
/// (e.g., "test", "stage", "saas"). Defaults to "test" if not set.
enum TestConfig {
  /// Server name from environment or "test" default.
  static var server: String {
    ProcessInfo.processInfo.environment["STUDIO_SERVER"] ?? "test"
  }

  /// Username from environment.
  static var username: String? {
    ProcessInfo.processInfo.environment["STUDIO_USERNAME"]
  }

  /// Password from environment.
  static var password: String? {
    ProcessInfo.processInfo.environment["STUDIO_PASSWORD"]
  }

  /// Token from environment.
  static var token: String? {
    ProcessInfo.processInfo.environment["STUDIO_TOKEN"]
  }

  /// Check if credentials are available for live tests.
  static var hasCredentials: Bool {
    token != nil || (username != nil && password != nil)
  }

  /// Create an authenticated EdgeFirst Studio client for testing.
  ///
  /// - Returns: Authenticated client instance.
  /// - Throws: TestConfigError if no credentials are available.
  static func getClient() throws -> Client {
    let client = try Client.withMemoryStorage()

    if let token = token {
      return try client.withServer(name: server).withToken(token: token)
    } else if let username = username, let password = password {
      return
        try client
        .withServer(name: server)
        .withLogin(username: username, password: password)
    } else {
      throw TestConfigError.missingCredentials(
        "No authentication credentials found. Set STUDIO_TOKEN or "
          + "STUDIO_USERNAME and STUDIO_PASSWORD environment variables."
      )
    }
  }

  /// Create an authenticated client asynchronously.
  ///
  /// - Returns: Authenticated client instance.
  /// - Throws: TestConfigError if no credentials are available.
  static func getClientAsync() async throws -> Client {
    let client = try Client.withMemoryStorage()
    let serverClient = try client.withServer(name: server)

    if let token = token {
      return try serverClient.withToken(token: token)
    } else if let username = username, let password = password {
      return try await serverClient.withLoginAsync(
        username: username,
        password: password
      )
    } else {
      throw TestConfigError.missingCredentials(
        "No authentication credentials found. Set STUDIO_TOKEN or "
          + "STUDIO_USERNAME and STUDIO_PASSWORD environment variables."
      )
    }
  }
}
