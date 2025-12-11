// swift-tools-version:5.9
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

// =============================================================================
// Configuration - Updated automatically by release workflow
// =============================================================================
let version = "2.6.4"
let checksum = "CHECKSUM_PLACEHOLDER"

// Toggle for local development vs release distribution
// Set to true when developing locally with a built XCFramework
// Set to false (default) for published releases
let useLocalFramework = false

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
    // Binary target: switches between local path and remote URL
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
    // Note: The swift/ directory is managed by GitHub Actions and updated on each release
    .target(
      name: "EdgeFirstClient",
      dependencies: ["EdgeFirstClientFFI"],
      path: "swift"
    ),
  ]
)
