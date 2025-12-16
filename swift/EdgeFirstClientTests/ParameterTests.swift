// SPDX-License-Identifier: Apache-2.0
// Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

/// Tests for the Parameter enum.
///
/// These tests verify Parameter construction, pattern matching, equality,
/// and nested structures. Matches Python test patterns in test_parameter.py.

import XCTest

@testable import EdgeFirstClient

final class ParameterTests: XCTestCase {

  // MARK: - Constructor Tests

  /// Test Parameter.integer construction.
  func testIntegerConstruction() {
    let param = Parameter.integer(value: 42)

    switch param {
    case .integer(let value):
      XCTAssertEqual(value, 42)
    default:
      XCTFail("Expected integer parameter")
    }
  }

  /// Test Parameter.real construction.
  func testRealConstruction() {
    let param = Parameter.real(value: 3.14)

    switch param {
    case .real(let value):
      XCTAssertEqual(value, 3.14, accuracy: 0.0001)
    default:
      XCTFail("Expected real parameter")
    }
  }

  /// Test Parameter.boolean construction.
  func testBooleanConstruction() {
    let paramTrue = Parameter.boolean(value: true)
    let paramFalse = Parameter.boolean(value: false)

    switch paramTrue {
    case .boolean(let value):
      XCTAssertTrue(value)
    default:
      XCTFail("Expected boolean parameter")
    }

    switch paramFalse {
    case .boolean(let value):
      XCTAssertFalse(value)
    default:
      XCTFail("Expected boolean parameter")
    }
  }

  /// Test Parameter.string construction.
  func testStringConstruction() {
    let param = Parameter.string(value: "hello world")

    switch param {
    case .string(let value):
      XCTAssertEqual(value, "hello world")
    default:
      XCTFail("Expected string parameter")
    }
  }

  /// Test Parameter.array construction.
  func testArrayConstruction() {
    let param = Parameter.array(values: [
      .integer(value: 1),
      .real(value: 2.5),
      .boolean(value: true),
      .string(value: "hello"),
    ])

    switch param {
    case .array(let values):
      XCTAssertEqual(values.count, 4)
    default:
      XCTFail("Expected array parameter")
    }
  }

  /// Test Parameter.object construction.
  func testObjectConstruction() {
    let param = Parameter.object(entries: [
      "key1": .integer(value: 42),
      "key2": .real(value: 3.14),
      "key3": .boolean(value: true),
    ])

    switch param {
    case .object(let entries):
      XCTAssertEqual(entries.count, 3)
      XCTAssertNotNil(entries["key1"])
      XCTAssertNotNil(entries["key2"])
      XCTAssertNotNil(entries["key3"])
    default:
      XCTFail("Expected object parameter")
    }
  }

  // MARK: - Equality Tests

  /// Test Parameter.integer equality.
  func testIntegerEquality() {
    let param1 = Parameter.integer(value: 42)
    let param2 = Parameter.integer(value: 42)
    let param3 = Parameter.integer(value: 43)

    XCTAssertEqual(param1, param2)
    XCTAssertNotEqual(param1, param3)
  }

  /// Test Parameter.real equality.
  func testRealEquality() {
    let param1 = Parameter.real(value: 3.14)
    let param2 = Parameter.real(value: 3.14)
    let param3 = Parameter.real(value: 3.15)

    XCTAssertEqual(param1, param2)
    XCTAssertNotEqual(param1, param3)
  }

  /// Test Parameter.boolean equality.
  func testBooleanEquality() {
    let paramTrue1 = Parameter.boolean(value: true)
    let paramTrue2 = Parameter.boolean(value: true)
    let paramFalse = Parameter.boolean(value: false)

    XCTAssertEqual(paramTrue1, paramTrue2)
    XCTAssertNotEqual(paramTrue1, paramFalse)
  }

  /// Test Parameter.string equality.
  func testStringEquality() {
    let param1 = Parameter.string(value: "hello")
    let param2 = Parameter.string(value: "hello")
    let param3 = Parameter.string(value: "world")

    XCTAssertEqual(param1, param2)
    XCTAssertNotEqual(param1, param3)
  }

  /// Test different parameter types are not equal.
  func testDifferentTypesNotEqual() {
    let intParam = Parameter.integer(value: 42)
    let realParam = Parameter.real(value: 42.0)
    let strParam = Parameter.string(value: "42")

    XCTAssertNotEqual(intParam, realParam)
    XCTAssertNotEqual(intParam, strParam)
    XCTAssertNotEqual(realParam, strParam)
  }

  // MARK: - Hashability Tests

  /// Test Parameters can be used as dictionary values (via Hashable conformance).
  func testParameterHashability() {
    var paramSet: Set<Parameter> = []

    paramSet.insert(.integer(value: 1))
    paramSet.insert(.integer(value: 2))
    paramSet.insert(.integer(value: 1))  // Duplicate

    XCTAssertEqual(paramSet.count, 2)
  }

  // MARK: - Nested Structure Tests

  /// Test nested arrays preserve structure.
  func testNestedArrays() {
    let param = Parameter.array(values: [
      .array(values: [
        .integer(value: 1),
        .integer(value: 2),
      ]),
      .array(values: [
        .integer(value: 3),
        .integer(value: 4),
      ]),
    ])

    switch param {
    case .array(let values):
      XCTAssertEqual(values.count, 2)

      if case .array(let inner) = values[0] {
        XCTAssertEqual(inner.count, 2)
        if case .integer(let val) = inner[0] {
          XCTAssertEqual(val, 1)
        }
      } else {
        XCTFail("Expected nested array")
      }
    default:
      XCTFail("Expected array parameter")
    }
  }

  /// Test nested objects preserve structure.
  func testNestedObjects() {
    let param = Parameter.object(entries: [
      "config": .object(entries: [
        "timeout": .integer(value: 30),
        "retries": .integer(value: 3),
      ]),
      "data": .array(values: [
        .string(value: "a"),
        .string(value: "b"),
      ]),
    ])

    switch param {
    case .object(let entries):
      XCTAssertEqual(entries.count, 2)

      if case .object(let config) = entries["config"] {
        if case .integer(let timeout) = config["timeout"] {
          XCTAssertEqual(timeout, 30)
        } else {
          XCTFail("Expected timeout integer")
        }
      } else {
        XCTFail("Expected nested object")
      }
    default:
      XCTFail("Expected object parameter")
    }
  }

  /// Test complex nested structure with mixed types.
  func testComplexNestedStructure() {
    let param = Parameter.object(entries: [
      "version": .integer(value: 1),
      "settings": .object(entries: [
        "timeout": .real(value: 30.5),
        "retries": .integer(value: 3),
        "features": .array(values: [
          .string(value: "feature1"),
          .string(value: "feature2"),
        ]),
        "flags": .object(entries: [
          "debug": .boolean(value: true),
          "verbose": .boolean(value: false),
        ]),
      ]),
      "data": .array(values: [
        .integer(value: 1),
        .integer(value: 2),
        .array(values: [
          .integer(value: 3),
          .integer(value: 4),
        ]),
      ]),
    ])

    switch param {
    case .object(let entries):
      // Verify version
      if case .integer(let version) = entries["version"] {
        XCTAssertEqual(version, 1)
      } else {
        XCTFail("Expected version integer")
      }

      // Verify settings
      if case .object(let settings) = entries["settings"] {
        if case .real(let timeout) = settings["timeout"] {
          XCTAssertEqual(timeout, 30.5, accuracy: 0.001)
        }
        if case .integer(let retries) = settings["retries"] {
          XCTAssertEqual(retries, 3)
        }
        if case .array(let features) = settings["features"] {
          XCTAssertEqual(features.count, 2)
        }
        if case .object(let flags) = settings["flags"] {
          if case .boolean(let debug) = flags["debug"] {
            XCTAssertTrue(debug)
          }
          if case .boolean(let verbose) = flags["verbose"] {
            XCTAssertFalse(verbose)
          }
        }
      } else {
        XCTFail("Expected settings object")
      }

      // Verify data array with nested array
      if case .array(let data) = entries["data"] {
        XCTAssertEqual(data.count, 3)
        if case .array(let nested) = data[2] {
          XCTAssertEqual(nested.count, 2)
        }
      }
    default:
      XCTFail("Expected object parameter")
    }
  }

  // MARK: - Empty Collections Tests

  /// Test empty array.
  func testEmptyArray() {
    let param = Parameter.array(values: [])

    switch param {
    case .array(let values):
      XCTAssertTrue(values.isEmpty)
    default:
      XCTFail("Expected array parameter")
    }
  }

  /// Test empty object.
  func testEmptyObject() {
    let param = Parameter.object(entries: [:])

    switch param {
    case .object(let entries):
      XCTAssertTrue(entries.isEmpty)
    default:
      XCTFail("Expected object parameter")
    }
  }

  // MARK: - Type Checking Helper Functions

  /// Test type checking with switch statements.
  func testTypeChecking() {
    let params: [Parameter] = [
      .integer(value: 42),
      .real(value: 3.14),
      .boolean(value: true),
      .string(value: "test"),
      .array(values: []),
      .object(entries: [:]),
    ]

    for (index, param) in params.enumerated() {
      switch param {
      case .integer:
        XCTAssertEqual(index, 0, "integer should be at index 0")
      case .real:
        XCTAssertEqual(index, 1, "real should be at index 1")
      case .boolean:
        XCTAssertEqual(index, 2, "boolean should be at index 2")
      case .string:
        XCTAssertEqual(index, 3, "string should be at index 3")
      case .array:
        XCTAssertEqual(index, 4, "array should be at index 4")
      case .object:
        XCTAssertEqual(index, 5, "object should be at index 5")
      }
    }
  }

  // MARK: - Value Extraction Tests

  /// Test extracting integer value.
  func testExtractIntegerValue() {
    let param = Parameter.integer(value: 42)

    if case .integer(let value) = param {
      XCTAssertEqual(value, 42)
    } else {
      XCTFail("Failed to extract integer value")
    }
  }

  /// Test extracting real value.
  func testExtractRealValue() {
    let param = Parameter.real(value: 3.14159)

    if case .real(let value) = param {
      XCTAssertEqual(value, 3.14159, accuracy: 0.00001)
    } else {
      XCTFail("Failed to extract real value")
    }
  }

  /// Test extracting string value.
  func testExtractStringValue() {
    let param = Parameter.string(value: "hello world")

    if case .string(let value) = param {
      XCTAssertEqual(value, "hello world")
    } else {
      XCTFail("Failed to extract string value")
    }
  }

  /// Test extracting array values.
  func testExtractArrayValues() {
    let param = Parameter.array(values: [
      .integer(value: 10),
      .real(value: 20.5),
      .string(value: "thirty"),
    ])

    if case .array(let values) = param {
      XCTAssertEqual(values.count, 3)

      if case .integer(let first) = values[0] {
        XCTAssertEqual(first, 10)
      }
      if case .real(let second) = values[1] {
        XCTAssertEqual(second, 20.5, accuracy: 0.001)
      }
      if case .string(let third) = values[2] {
        XCTAssertEqual(third, "thirty")
      }
    } else {
      XCTFail("Failed to extract array values")
    }
  }

  /// Test extracting object entries.
  func testExtractObjectEntries() {
    let param = Parameter.object(entries: [
      "model": .string(value: "yolov5"),
      "detection": .boolean(value: true),
      "threshold": .real(value: 0.75),
    ])

    if case .object(let entries) = param {
      XCTAssertEqual(entries.count, 3)

      if case .string(let model) = entries["model"] {
        XCTAssertEqual(model, "yolov5")
      }
      if case .boolean(let detection) = entries["detection"] {
        XCTAssertTrue(detection)
      }
      if case .real(let threshold) = entries["threshold"] {
        XCTAssertEqual(threshold, 0.75, accuracy: 0.001)
      }
    } else {
      XCTFail("Failed to extract object entries")
    }
  }

  // MARK: - Real-World Usage Pattern Tests

  /// Test trainer params structure (common usage pattern).
  func testTrainerParamsStructure() {
    // Simulate trainer.model_params structure
    let trainerParams = Parameter.object(entries: [
      "model": .object(entries: [
        "detection": .boolean(value: true),
        "name": .string(value: "yolov5"),
        "threshold": .real(value: 0.75),
      ]),
      "epochs": .integer(value: 100),
    ])

    // Extract model params
    if case .object(let params) = trainerParams {
      if case .object(let model) = params["model"] {
        // Get detection flag
        if case .boolean(let detection) = model["detection"] {
          XCTAssertTrue(detection)
        }

        // Get model name
        if case .string(let name) = model["name"] {
          XCTAssertEqual(name, "yolov5")
        }

        // Get threshold
        if case .real(let threshold) = model["threshold"] {
          XCTAssertEqual(threshold, 0.75, accuracy: 0.001)
        }
      }

      // Get epochs
      if case .integer(let epochs) = params["epochs"] {
        XCTAssertEqual(epochs, 100)
      }
    }
  }

  /// Test metrics structure (common usage pattern).
  func testMetricsStructure() {
    let metrics = Parameter.object(entries: [
      "accuracy": .real(value: 0.95),
      "loss": .real(value: 0.05),
      "epoch": .integer(value: 50),
      "learning_rate": .real(value: 0.001),
    ])

    if case .object(let entries) = metrics {
      XCTAssertEqual(entries.count, 4)

      if case .real(let accuracy) = entries["accuracy"] {
        XCTAssertEqual(accuracy, 0.95, accuracy: 0.001)
      }
      if case .real(let loss) = entries["loss"] {
        XCTAssertEqual(loss, 0.05, accuracy: 0.001)
      }
      if case .integer(let epoch) = entries["epoch"] {
        XCTAssertEqual(epoch, 50)
      }
    }
  }
}
