// swift-tools-version:5.9
// The swift-tools-version declares the minimum version of Swift required to build this package.

import Foundation
import PackageDescription

// =============================================================================
// Configuration - Updated automatically by release workflow
// =============================================================================
let version = "2.6.4"
let checksum = "CHECKSUM_PLACEHOLDER"

// Toggle for local development vs release distribution
// Set USE_LOCAL_FRAMEWORK=true environment variable for local XCFramework
// Default (unset or false) uses remote URL for published releases
let useLocalFramework =
  ProcessInfo.processInfo.environment["USE_LOCAL_FRAMEWORK"] == "true"

// =============================================================================
// Package Definition
// =============================================================================
let package = Package(
  name: "EdgeFirstClient",
  platforms: [
    .iOS(.v13),
    .macOS(.v10_15),
  ],
  products: [
    .library(
      name: "EdgeFirstClient",
      targets: ["EdgeFirstClient"]
    )
  ],
  targets: [
    // Binary target: XCFramework containing the Rust FFI library
    // - Local: Used during development with locally-built XCFramework
    // - Remote: Used by consumers downloading from GitHub releases
    useLocalFramework
      ? .binaryTarget(
        name: "EdgeFirstClientFFI",
        path: "EdgeFirstClient.xcframework"
      )
      : .binaryTarget(
        name: "EdgeFirstClientFFI",
        url:
          "https://github.com/EdgeFirstAI/client/releases/download/v\(version)/EdgeFirstClient-\(version).xcframework.zip",
        checksum: checksum
      ),

    // Swift wrapper target containing UniFFI-generated bindings
    // Depends on the binary FFI target for the native Rust implementation
    .target(
      name: "EdgeFirstClient",
      dependencies: ["EdgeFirstClientFFI"],
      path: "swift",
      exclude: ["EdgeFirstClientTests"]
    ),

    // Test target for Swift SDK smoke tests
    // Requires credentials via STUDIO_TOKEN or STUDIO_USERNAME/STUDIO_PASSWORD
    .testTarget(
      name: "EdgeFirstClientTests",
      dependencies: ["EdgeFirstClient"],
      path: "swift/EdgeFirstClientTests"
    ),
  ]
)
