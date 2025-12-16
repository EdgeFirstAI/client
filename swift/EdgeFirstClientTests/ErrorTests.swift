// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

/// Tests for ClientError enum.
///
/// These tests verify ClientError case construction, pattern matching,
/// equality, hashability, and LocalizedError conformance.

import XCTest

@testable import EdgeFirstClient

final class ErrorTests: XCTestCase {

  // MARK: - Error Case Construction Tests

  /// Test AuthenticationError construction.
  func testAuthenticationErrorConstruction() {
    let error = ClientError.AuthenticationError(message: "Token expired")

    if case .AuthenticationError(let message) = error {
      XCTAssertEqual(message, "Token expired")
    } else {
      XCTFail("Expected AuthenticationError")
    }
  }

  /// Test NetworkError construction.
  func testNetworkErrorConstruction() {
    let error = ClientError.NetworkError(message: "Connection timeout")

    if case .NetworkError(let message) = error {
      XCTAssertEqual(message, "Connection timeout")
    } else {
      XCTFail("Expected NetworkError")
    }
  }

  /// Test InvalidParameters construction.
  func testInvalidParametersConstruction() {
    let error = ClientError.InvalidParameters(message: "Missing required field: name")

    if case .InvalidParameters(let message) = error {
      XCTAssertEqual(message, "Missing required field: name")
    } else {
      XCTFail("Expected InvalidParameters")
    }
  }

  /// Test NotFound construction.
  func testNotFoundConstruction() {
    let error = ClientError.NotFound(message: "Dataset not found: ds-123")

    if case .NotFound(let message) = error {
      XCTAssertEqual(message, "Dataset not found: ds-123")
    } else {
      XCTFail("Expected NotFound")
    }
  }

  /// Test StorageError construction.
  func testStorageErrorConstruction() {
    let error = ClientError.StorageError(message: "Failed to write token file")

    if case .StorageError(let message) = error {
      XCTAssertEqual(message, "Failed to write token file")
    } else {
      XCTFail("Expected StorageError")
    }
  }

  /// Test InternalError construction.
  func testInternalErrorConstruction() {
    let error = ClientError.InternalError(message: "Unexpected server response")

    if case .InternalError(let message) = error {
      XCTAssertEqual(message, "Unexpected server response")
    } else {
      XCTFail("Expected InternalError")
    }
  }

  // MARK: - Error Message Extraction Tests

  /// Test extracting message from AuthenticationError.
  func testAuthenticationErrorMessage() {
    let error = ClientError.AuthenticationError(message: "Invalid credentials")

    switch error {
    case .AuthenticationError(let message):
      XCTAssertEqual(message, "Invalid credentials")
    default:
      XCTFail("Expected AuthenticationError")
    }
  }

  /// Test extracting message from NetworkError.
  func testNetworkErrorMessage() {
    let error = ClientError.NetworkError(message: "DNS resolution failed")

    switch error {
    case .NetworkError(let message):
      XCTAssertEqual(message, "DNS resolution failed")
    default:
      XCTFail("Expected NetworkError")
    }
  }

  /// Test extracting message with empty string.
  func testErrorWithEmptyMessage() {
    let error = ClientError.InternalError(message: "")

    if case .InternalError(let message) = error {
      XCTAssertTrue(message.isEmpty)
    } else {
      XCTFail("Expected InternalError")
    }
  }

  /// Test extracting message with special characters.
  func testErrorWithSpecialCharacters() {
    let error = ClientError.InvalidParameters(message: "Field 'name' cannot contain: <>&\"")

    if case .InvalidParameters(let message) = error {
      XCTAssertTrue(message.contains("<>&\""))
    } else {
      XCTFail("Expected InvalidParameters")
    }
  }

  /// Test extracting message with unicode.
  func testErrorWithUnicodeMessage() {
    let error = ClientError.NotFound(message: "项目未找到: 测试项目")

    if case .NotFound(let message) = error {
      XCTAssertEqual(message, "项目未找到: 测试项目")
    } else {
      XCTFail("Expected NotFound")
    }
  }

  // MARK: - Error Pattern Matching Tests

  /// Test pattern matching all error cases.
  func testErrorPatternMatching() {
    let errors: [ClientError] = [
      .AuthenticationError(message: "auth"),
      .NetworkError(message: "network"),
      .InvalidParameters(message: "params"),
      .NotFound(message: "not found"),
      .StorageError(message: "storage"),
      .InternalError(message: "internal"),
    ]

    var matchedCases: [String] = []

    for error in errors {
      switch error {
      case .AuthenticationError:
        matchedCases.append("AuthenticationError")
      case .NetworkError:
        matchedCases.append("NetworkError")
      case .InvalidParameters:
        matchedCases.append("InvalidParameters")
      case .NotFound:
        matchedCases.append("NotFound")
      case .StorageError:
        matchedCases.append("StorageError")
      case .InternalError:
        matchedCases.append("InternalError")
      }
    }

    XCTAssertEqual(matchedCases.count, 6)
    XCTAssertEqual(matchedCases[0], "AuthenticationError")
    XCTAssertEqual(matchedCases[1], "NetworkError")
    XCTAssertEqual(matchedCases[2], "InvalidParameters")
    XCTAssertEqual(matchedCases[3], "NotFound")
    XCTAssertEqual(matchedCases[4], "StorageError")
    XCTAssertEqual(matchedCases[5], "InternalError")
  }

  /// Test ClientError is Swift Error.
  func testErrorIsSwiftError() {
    let error: Error = ClientError.AuthenticationError(message: "test")

    XCTAssertTrue(error is ClientError)
  }

  /// Test error can be thrown and caught.
  func testErrorThrowAndCatch() {
    func throwingFunction() throws {
      throw ClientError.NotFound(message: "Resource not found")
    }

    XCTAssertThrowsError(try throwingFunction()) { error in
      guard let clientError = error as? ClientError else {
        XCTFail("Expected ClientError")
        return
      }

      if case .NotFound(let message) = clientError {
        XCTAssertEqual(message, "Resource not found")
      } else {
        XCTFail("Expected NotFound error")
      }
    }
  }

  // MARK: - Error Equality Tests

  /// Test same error cases are equal.
  func testErrorEquality() {
    let error1 = ClientError.AuthenticationError(message: "Token expired")
    let error2 = ClientError.AuthenticationError(message: "Token expired")

    XCTAssertEqual(error1, error2)
  }

  /// Test different messages are not equal.
  func testErrorDifferentMessagesNotEqual() {
    let error1 = ClientError.AuthenticationError(message: "Token expired")
    let error2 = ClientError.AuthenticationError(message: "Invalid token")

    XCTAssertNotEqual(error1, error2)
  }

  /// Test different error cases are not equal.
  func testErrorCasesDistinct() {
    let authError = ClientError.AuthenticationError(message: "error")
    let networkError = ClientError.NetworkError(message: "error")
    let paramsError = ClientError.InvalidParameters(message: "error")
    let notFoundError = ClientError.NotFound(message: "error")
    let storageError = ClientError.StorageError(message: "error")
    let internalError = ClientError.InternalError(message: "error")

    // All should be distinct from each other
    XCTAssertNotEqual(authError, networkError)
    XCTAssertNotEqual(authError, paramsError)
    XCTAssertNotEqual(authError, notFoundError)
    XCTAssertNotEqual(authError, storageError)
    XCTAssertNotEqual(authError, internalError)
    XCTAssertNotEqual(networkError, paramsError)
    XCTAssertNotEqual(notFoundError, storageError)
  }

  // MARK: - Error Hashability Tests

  /// Test errors can be used in sets.
  func testErrorHashability() {
    var errorSet: Set<ClientError> = []

    errorSet.insert(ClientError.AuthenticationError(message: "error1"))
    errorSet.insert(ClientError.NetworkError(message: "error2"))
    errorSet.insert(ClientError.AuthenticationError(message: "error1"))  // Duplicate

    XCTAssertEqual(errorSet.count, 2)
  }

  /// Test errors can be used as dictionary keys.
  func testErrorAsDictionaryKey() {
    var errorCounts: [ClientError: Int] = [:]

    let authError = ClientError.AuthenticationError(message: "auth")
    let networkError = ClientError.NetworkError(message: "network")

    errorCounts[authError] = 5
    errorCounts[networkError] = 3

    XCTAssertEqual(errorCounts[authError], 5)
    XCTAssertEqual(errorCounts[networkError], 3)
  }

  // MARK: - LocalizedError Conformance Tests

  /// Test errorDescription is not nil.
  func testErrorLocalizedDescription() {
    let error = ClientError.NotFound(message: "Dataset not found")

    // LocalizedError provides errorDescription
    XCTAssertNotNil(error.errorDescription)
  }

  /// Test errorDescription contains error information.
  func testErrorDescriptionContainsMessage() {
    let error = ClientError.AuthenticationError(message: "Token invalid")
    let description = error.errorDescription ?? ""

    // The description should contain the error type or message
    XCTAssertFalse(description.isEmpty)
  }

  /// Test all error cases have descriptions.
  func testAllErrorCasesHaveDescriptions() {
    let errors: [ClientError] = [
      .AuthenticationError(message: "auth"),
      .NetworkError(message: "network"),
      .InvalidParameters(message: "params"),
      .NotFound(message: "not found"),
      .StorageError(message: "storage"),
      .InternalError(message: "internal"),
    ]

    for error in errors {
      XCTAssertNotNil(error.errorDescription, "Missing description for \(error)")
    }
  }

  // MARK: - Error in Async Context Tests

  /// Test error can be used in async throws.
  func testErrorInAsyncContext() async {
    func asyncThrowingFunction() async throws {
      throw ClientError.NetworkError(message: "Connection lost")
    }

    do {
      try await asyncThrowingFunction()
      XCTFail("Should have thrown")
    } catch let error as ClientError {
      if case .NetworkError(let message) = error {
        XCTAssertEqual(message, "Connection lost")
      } else {
        XCTFail("Expected NetworkError")
      }
    } catch {
      XCTFail("Expected ClientError")
    }
  }

  // MARK: - Error Message Length Tests

  /// Test error with very long message.
  func testErrorWithLongMessage() {
    let longMessage = String(repeating: "a", count: 10000)
    let error = ClientError.InternalError(message: longMessage)

    if case .InternalError(let message) = error {
      XCTAssertEqual(message.count, 10000)
    } else {
      XCTFail("Expected InternalError")
    }
  }

  /// Test error with multiline message.
  func testErrorWithMultilineMessage() {
    let multilineMessage = """
      Error occurred at line 42.
      Stack trace:
        - function1()
        - function2()
        - main()
      """
    let error = ClientError.InternalError(message: multilineMessage)

    if case .InternalError(let message) = error {
      XCTAssertTrue(message.contains("line 42"))
      XCTAssertTrue(message.contains("Stack trace"))
    } else {
      XCTFail("Expected InternalError")
    }
  }
}
