// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

/// Tests for StorageError enum.
///
/// These tests verify StorageError case construction, pattern matching,
/// equality, hashability, and error conformance.

import XCTest

@testable import EdgeFirstClient

final class StorageErrorTests: XCTestCase {

  // MARK: - Error Case Construction Tests

  /// Test NotAvailable construction.
  func testNotAvailableConstruction() {
    let error = StorageError.NotAvailable(message: "Config directory not found")

    if case .NotAvailable(let message) = error {
      XCTAssertEqual(message, "Config directory not found")
    } else {
      XCTFail("Expected NotAvailable")
    }
  }

  /// Test ReadError construction.
  func testReadErrorConstruction() {
    let error = StorageError.ReadError(message: "Failed to read token file")

    if case .ReadError(let message) = error {
      XCTAssertEqual(message, "Failed to read token file")
    } else {
      XCTFail("Expected ReadError")
    }
  }

  /// Test WriteError construction.
  func testWriteErrorConstruction() {
    let error = StorageError.WriteError(message: "Permission denied")

    if case .WriteError(let message) = error {
      XCTAssertEqual(message, "Permission denied")
    } else {
      XCTFail("Expected WriteError")
    }
  }

  /// Test ClearError construction.
  func testClearErrorConstruction() {
    let error = StorageError.ClearError(message: "Failed to delete token")

    if case .ClearError(let message) = error {
      XCTAssertEqual(message, "Failed to delete token")
    } else {
      XCTFail("Expected ClearError")
    }
  }

  // MARK: - Error Message Extraction Tests

  /// Test extracting message from NotAvailable.
  func testNotAvailableMessage() {
    let error = StorageError.NotAvailable(message: "Storage unavailable")

    switch error {
    case .NotAvailable(let message):
      XCTAssertEqual(message, "Storage unavailable")
    default:
      XCTFail("Expected NotAvailable")
    }
  }

  /// Test extracting message from ReadError.
  func testReadErrorMessage() {
    let error = StorageError.ReadError(message: "File corrupted")

    switch error {
    case .ReadError(let message):
      XCTAssertEqual(message, "File corrupted")
    default:
      XCTFail("Expected ReadError")
    }
  }

  /// Test error with empty message.
  func testErrorWithEmptyMessage() {
    let error = StorageError.WriteError(message: "")

    if case .WriteError(let message) = error {
      XCTAssertTrue(message.isEmpty)
    } else {
      XCTFail("Expected WriteError")
    }
  }

  /// Test error with special characters.
  func testErrorWithSpecialCharacters() {
    let error = StorageError.ClearError(message: "Path: /tmp/<test>&\"file\"")

    if case .ClearError(let message) = error {
      XCTAssertTrue(message.contains("<test>"))
      XCTAssertTrue(message.contains("&"))
    } else {
      XCTFail("Expected ClearError")
    }
  }

  /// Test error with unicode message.
  func testErrorWithUnicodeMessage() {
    let error = StorageError.NotAvailable(message: "存储不可用: 配置错误")

    if case .NotAvailable(let message) = error {
      XCTAssertEqual(message, "存储不可用: 配置错误")
    } else {
      XCTFail("Expected NotAvailable")
    }
  }

  // MARK: - Error Pattern Matching Tests

  /// Test pattern matching all error cases.
  func testErrorPatternMatching() {
    let errors: [StorageError] = [
      .NotAvailable(message: "not available"),
      .ReadError(message: "read error"),
      .WriteError(message: "write error"),
      .ClearError(message: "clear error"),
    ]

    var matchedCases: [String] = []

    for error in errors {
      switch error {
      case .NotAvailable:
        matchedCases.append("NotAvailable")
      case .ReadError:
        matchedCases.append("ReadError")
      case .WriteError:
        matchedCases.append("WriteError")
      case .ClearError:
        matchedCases.append("ClearError")
      }
    }

    XCTAssertEqual(matchedCases.count, 4)
    XCTAssertEqual(matchedCases[0], "NotAvailable")
    XCTAssertEqual(matchedCases[1], "ReadError")
    XCTAssertEqual(matchedCases[2], "WriteError")
    XCTAssertEqual(matchedCases[3], "ClearError")
  }

  /// Test StorageError is Swift Error.
  func testErrorIsSwiftError() {
    let error: Error = StorageError.NotAvailable(message: "test")

    XCTAssertTrue(error is StorageError)
  }

  /// Test error can be thrown and caught.
  func testErrorThrowAndCatch() {
    func throwingFunction() throws {
      throw StorageError.ReadError(message: "Cannot read token")
    }

    XCTAssertThrowsError(try throwingFunction()) { error in
      guard let storageError = error as? StorageError else {
        XCTFail("Expected StorageError")
        return
      }

      if case .ReadError(let message) = storageError {
        XCTAssertEqual(message, "Cannot read token")
      } else {
        XCTFail("Expected ReadError")
      }
    }
  }

  // MARK: - Error Equality Tests

  /// Test same error cases are equal.
  func testErrorEquality() {
    let error1 = StorageError.NotAvailable(message: "Storage unavailable")
    let error2 = StorageError.NotAvailable(message: "Storage unavailable")

    XCTAssertEqual(error1, error2)
  }

  /// Test different messages are not equal.
  func testErrorDifferentMessagesNotEqual() {
    let error1 = StorageError.ReadError(message: "Error 1")
    let error2 = StorageError.ReadError(message: "Error 2")

    XCTAssertNotEqual(error1, error2)
  }

  /// Test different error cases are not equal.
  func testErrorCasesDistinct() {
    let notAvailable = StorageError.NotAvailable(message: "error")
    let readError = StorageError.ReadError(message: "error")
    let writeError = StorageError.WriteError(message: "error")
    let clearError = StorageError.ClearError(message: "error")

    XCTAssertNotEqual(notAvailable, readError)
    XCTAssertNotEqual(notAvailable, writeError)
    XCTAssertNotEqual(notAvailable, clearError)
    XCTAssertNotEqual(readError, writeError)
    XCTAssertNotEqual(readError, clearError)
    XCTAssertNotEqual(writeError, clearError)
  }

  // MARK: - Error Hashability Tests

  /// Test errors can be used in sets.
  func testErrorHashability() {
    var errorSet: Set<StorageError> = []

    errorSet.insert(StorageError.NotAvailable(message: "error1"))
    errorSet.insert(StorageError.ReadError(message: "error2"))
    errorSet.insert(StorageError.NotAvailable(message: "error1"))  // Duplicate

    XCTAssertEqual(errorSet.count, 2)
  }

  /// Test errors can be used as dictionary keys.
  func testErrorAsDictionaryKey() {
    var errorCounts: [StorageError: Int] = [:]

    let notAvailable = StorageError.NotAvailable(message: "storage")
    let readError = StorageError.ReadError(message: "read")

    errorCounts[notAvailable] = 5
    errorCounts[readError] = 3

    XCTAssertEqual(errorCounts[notAvailable], 5)
    XCTAssertEqual(errorCounts[readError], 3)
  }

  // MARK: - Error in Async Context Tests

  /// Test error can be used in async throws.
  func testErrorInAsyncContext() async {
    func asyncThrowingFunction() async throws {
      throw StorageError.WriteError(message: "Async write failed")
    }

    do {
      try await asyncThrowingFunction()
      XCTFail("Should have thrown")
    } catch let error as StorageError {
      if case .WriteError(let message) = error {
        XCTAssertEqual(message, "Async write failed")
      } else {
        XCTFail("Expected WriteError")
      }
    } catch {
      XCTFail("Expected StorageError")
    }
  }

  // MARK: - Error Message Length Tests

  /// Test error with very long message.
  func testErrorWithLongMessage() {
    let longMessage = String(repeating: "x", count: 10000)
    let error = StorageError.NotAvailable(message: longMessage)

    if case .NotAvailable(let message) = error {
      XCTAssertEqual(message.count, 10000)
    } else {
      XCTFail("Expected NotAvailable")
    }
  }

  /// Test error with multiline message.
  func testErrorWithMultilineMessage() {
    let multilineMessage = """
      Storage error occurred.
      Path: /home/user/.config/edgefirst
      Reason: Permission denied
      """
    let error = StorageError.WriteError(message: multilineMessage)

    if case .WriteError(let message) = error {
      XCTAssertTrue(message.contains("Permission denied"))
      XCTAssertTrue(message.contains("Path:"))
    } else {
      XCTFail("Expected WriteError")
    }
  }
}
