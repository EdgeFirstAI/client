// SPDX-License-Identifier: Apache-2.0
// Copyright © 2026 Au-Zone Technologies. All Rights Reserved.

/// Tests for trainer/validator schema queries and session management.
///
/// Struct construction tests run offline; the schema query and
/// session-management tests require Studio credentials and are
/// skipped when none are configured.

import XCTest

@testable import EdgeFirstClient

final class SchemaTests: XCTestCase {

  /// SchemaField supports recursive nesting through children/options.
  func testSchemaFieldConstruction() {
    let epochs = SchemaField(
      name: "epochs",
      label: "Epochs",
      description: "Number of training epochs",
      required: true,
      default: .integer(value: 50),
      fieldType: .int,
      min: 1,
      max: 1000,
      step: 1,
      options: [],
      children: [],
      isDropdown: false,
      multiSelect: false,
      isMultiLine: false,
      hidden: false,
      numericOnly: false,
      enableTagsSelection: false,
      enableAnnotationSetSelection: false,
      values: nil
    )

    let group = SchemaField(
      name: "training",
      label: "Training",
      description: nil,
      required: false,
      default: nil,
      fieldType: .group,
      min: nil,
      max: nil,
      step: nil,
      options: [
        SchemaOption(name: .string(value: "adam"), label: "Adam", children: [])
      ],
      children: [epochs],
      isDropdown: false,
      multiSelect: false,
      isMultiLine: false,
      hidden: false,
      numericOnly: false,
      enableTagsSelection: false,
      enableAnnotationSetSelection: false,
      values: nil
    )

    XCTAssertEqual(group.children.count, 1)
    XCTAssertEqual(group.children[0].name, "epochs")
    XCTAssertEqual(group.fieldType, .group)
    XCTAssertEqual(group.options.count, 1)
  }

  /// StartTrainingRequest defaults resolve server-side/latest-tag.
  func testStartTrainingRequestConstruction() {
    let request = StartTrainingRequest(
      projectId: ProjectId(value: 1),
      name: "swift-test",
      experimentId: ExperimentId(value: 2),
      trainerType: "modelpack",
      datasetId: DatasetId(value: 3),
      annotationSetId: AnnotationSetId(value: 4),
      tagName: nil,
      trainGroup: nil,
      valGroup: nil,
      sessionName: nil,
      sessionDescription: nil,
      weightsSession: nil,
      params: ["epochs": .integer(value: 1)],
      isLocal: true,
      isKubernetes: false
    )

    XCTAssertNil(request.tagName, "nil tag selects the latest dataset tag")
    XCTAssertTrue(request.isLocal)
  }

  /// Trainer schema catalog and per-type schemas parse (live server).
  func testTrainerSchemas() throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try TestConfig.getClient()
    let schemas = try client.trainerSchemas()
    XCTAssertFalse(schemas.isEmpty, "Server should report trainer types")

    if let first = schemas.first {
      let type = first.schemaType.isEmpty ? first.name : first.schemaType
      let fields = try client.trainerSchema(schemaType: type)
      print("Schema \(type) has \(fields.count) top-level fields")
    }
  }

  /// Validator schema catalog parses (live server, async surface).
  func testValidatorSchemasAsync() async throws {
    try XCTSkipUnless(
      TestConfig.hasCredentials,
      "Skipping: No credentials available"
    )

    let client = try await TestConfig.getClientAsync()
    let schemas = try await client.validatorSchemasAsync()
    XCTAssertFalse(schemas.isEmpty, "Server should report validator schemas")
  }
}
