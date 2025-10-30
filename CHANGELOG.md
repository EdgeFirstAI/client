# Changelog

All notable changes to EdgeFirst Client will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- `AGENTS.md`: Standardized AI coding agent instructions following agents.md specification
  - Project conventions, build commands, and pre-commit requirements
  - Succinct format optimized for AI assistants (GitHub Copilot, Cursor, Aider, etc.)
  - Referenced in README.md and CONTRIBUTING.md for discoverability
- Python bindings: `Parameter` class now implements Python magic methods for type conversions
  - `__int__()`: Convert Integer, Real, Boolean to Python int
  - `__float__()`: Convert Integer, Real, Boolean to Python float
  - `__bool__()`: Convert all Parameter types to Python bool
  - `__str__()` and `__repr__()`: String representations
  - `__eq__()`: Equality comparison with epsilon tolerance (1e-9) for numeric types
  - Enables natural Python usage: `float(param)`, `int(param)`, `param == 0.75`
- Test coverage: Added comprehensive ID type tests in `test/test_ids.py`
  - Validates string format for all 13 ID types (OrganizationID, ProjectID, DatasetID, etc.)
  - Verifies consistency between `.uid` property and `str(.id)` for 9 classes
- Test coverage: Added 27 unit tests to `error.rs` validating error wrapping behavior
  - Tests all wrapped error types (IoError, JsonError, HttpError, etc.) preserve inner error messages
  - Tests primitive-wrapped errors (RpcError, InvalidFileType, etc.) include original values
  - Tests simple errors (InvalidResponse, NotImplemented, etc.) display correct messages

### Changed
- CLI: Refactored Arrow annotation parsing to eliminate type complexity warning
  - `parse_annotations_from_arrow` now returns `Vec<Sample>` directly instead of intermediate HashMap
  - Merged `build_samples_from_map` logic into single function for cleaner architecture
  - Added 9 comprehensive test cases covering all code paths and edge cases
- **BREAKING**: `annotations_dataframe()` now returns `Result<DataFrame, Error>` instead of `DataFrame`
  - Polars operations (casting, DataFrame construction) now properly propagate errors
  - Callers must handle the Result with `?` or `.unwrap()` / `.expect()`
  - Improves robustness by eliminating panics in dataframe construction

### Deprecated
- Python bindings: All `.uid` properties are now deprecated
  - Affected classes: Project, Dataset, AnnotationSet, Experiment, TrainingSession, ValidationSession, Task, TaskInfo, Sample
  - Emits `DeprecationWarning` when accessed: "X.uid is deprecated and will be removed in a future version. Use str(X.id) instead."
  - Migration path: Replace `obj.uid` with `str(obj.id)`
  - Backward compatible: Properties still functional but will be removed in next major version

### Fixed
- Rust client: Updated samples.populate2 annotation serialization to match server schema
  - Emits annotations as a flat array with nested `box2d`/`box3d` geometry objects
  - Segmentation masks serialize as polygon arrays (`"mask": [[[x, y], ...]]`)
  - Backwards-compatible deserialization still accepts legacy map payloads
- Eliminated all `unwrap()` calls from library code (client.rs, dataset.rs, error.rs)
  - Download functions: Fixed file path and content-length handling
  - Multipart uploads: Fixed part key validation and ETag parsing
  - Dataset operations: Fixed file type and path parsing
  - All potential panic points now return proper errors
- Added `InvalidEtag` error variant for HTTP response validation
- Added `PolarsError` error variant (feature-gated) for dataframe operations
- Python tests: Float equality comparisons now use epsilon tolerance (fixes python:S1244)

## [2.3.1] - 2025-10-24

### Added
- `Sample` accessor functions for all fields in Rust API
  - `uuid()`, `sequence_uuid()`, `sequence_description()`, `frame_number()`
  - `image_name()`, `image_url()`, `width()`, `height()`, `date()`, `source()`
  - `location()`, `files()` - providing complete access to sample metadata
- Python bindings for all new `Sample` accessor properties
  - Complete property exposure matching Rust API
  - Setter methods: `set_group()`, `set_sequence_name()`, `set_frame_number()` for mutable fields
  - Updated type stubs in `edgefirst_client.pyi` with documentation
- `Client::create_dataset()` and `Client::delete_dataset()` methods in Rust API
  - Create new datasets with optional descriptions
  - Delete datasets by marking them as deleted
  - Python bindings with `description` defaulting to `None`
- `Client::create_annotation_set()` and `Client::delete_annotation_set()` methods in Rust API
  - Create new annotation sets for datasets with optional descriptions
  - Delete annotation sets by marking them as deleted
  - Python bindings with `description` defaulting to `None`
- CLI commands for dataset and annotation set management
  - `create-dataset` - Create new dataset in a project
  - `delete-dataset` - Delete dataset by ID
  - `create-annotation-set` - Create new annotation set for a dataset
  - `delete-annotation-set` - Delete annotation set by ID
- Comprehensive round-trip tests for dataset integrity verification
  - `test_deer_dataset_roundtrip()` in Rust library verifies download→upload data integrity
  - Equivalent Python test verifies byte-level image matching and annotation preservation
  - Tests create temporary datasets and annotation sets, then clean up after completion
  - Tests use random dataset names to prevent parallel execution conflicts
- CLI integration test for complete dataset and annotation set CRUD workflow
  - `test_dataset_crud()` - Comprehensive test covering create dataset → create annotation set → delete annotation set → delete dataset
  - Follows the complete lifecycle workflow with proper cleanup
  - Gracefully handles server API limitations (annotation set deletion not yet supported)
  - Uses `#[serial]` attribute to prevent race conditions
  - Uses timestamp-based unique names to avoid conflicts

### Changed
- `test_populate_samples` now creates and cleans up temporary datasets and annotation sets
  - Creates test dataset with random suffix to avoid conflicts
  - Creates annotation set for the new dataset
  - Uploads samples to new dataset instead of reusing existing dataset
  - Automatically deletes test dataset after verification
  - Both Rust and Python tests use this improved pattern
- `test_deer_dataset_roundtrip` now creates and cleans up temporary datasets
  - Creates test dataset with random suffix to avoid conflicts
  - Creates annotation set for the new dataset
  - Uploads subset of Deer dataset samples to new dataset
  - Verifies byte-level image matching and annotation preservation
  - Automatically deletes test dataset after verification
  - Both Rust and Python tests use this improved pattern

### Fixed
- `test_labels` test now uses random label names to avoid conflicts with parallel test execution
  - Previously tried to delete all labels which caused race conditions
  - Now creates/verifies/deletes a uniquely named test label

## [2.3.0] - 2025-10-23

### Added
- `Client::populate_samples()` method for importing samples with annotations
  - Automatically uploads local files to S3 using presigned URLs
  - Auto-generates UUIDs for samples if not provided (uuid crate v1.11.0)
  - Auto-extracts image dimensions using imagesize crate v0.13.0
  - Supports Box2d annotations with normalized coordinates (0.0-1.0 range)
  - Returns sample UUIDs and upload URLs for tracking
  - **Progress tracking**: Optional callback reports CUR/TOTAL as samples are uploaded
  - Parallel uploads with semaphore limiting (MAX_TASKS=32 concurrent uploads)
- Example `populate_with_circle.rs` demonstrating sample import with annotations
- **Python bindings** for `populate_samples()` API
  - `Client.populate_samples()` method with progress callback support
  - `Sample` and `Annotation` constructors for creating samples from Python
  - `SampleFile` class for specifying file types and paths
  - `SamplesPopulateResult` and `PresignedUrl` classes for tracking uploads
  - Setter methods: `Sample.set_image_name()`, `Sample.add_file()`, `Sample.add_annotation()`
  - Setter methods: `Annotation.set_label()`, `Annotation.set_object_id()`, `Annotation.set_box2d()`, etc.
  - Complete type stubs in `edgefirst_client.pyi` with documentation
  - Python test `test_populate_samples()` with 640x480 PNG and circle annotation
- **CLI `upload-dataset` command** for importing samples from EdgeFirst Dataset Format (Arrow)
  - **Flexible parameters**: All parameters except dataset ID are optional (must provide at least one of `--annotations` or `--images`)
  - **Auto-discovery**: Automatically finds images in folder/ZIP named after Arrow file or "dataset" if `--images` not specified
  - **Images-only mode**: Upload images without annotations by omitting `--annotations` and `--annotation-set-id`
  - **Warning system**: Warns if annotations provided without annotation_set_id (annotations will be skipped)
  - **Automatic batching**: Handles datasets larger than 500 samples by automatically batching uploads
  - Reads Arrow file with annotations following EdgeFirst Dataset Format schema
  - Handles samples without annotations (rows with name/group but null geometries)
  - Supports multiple annotations per sample (multiple rows with same name)
  - Supports multiple geometries per annotation (box2d/box3d/mask in same row)
  - Auto-generates object_id UUID when multiple geometries on same row without object_id
  - Progress bar with ETA for upload tracking
  - **Tested with 1646-sample Deer dataset** across all workflow modes

### Changed
- **BREAKING**: Simplified `Sample` and `Annotation` field types for better ergonomics
  - `Sample.files` changed from `Option<Vec<SampleFile>>` to `Vec<SampleFile>`
  - `Sample.annotations` changed from `Option<Vec<Annotation>>` to `Vec<Annotation>`
  - Empty vectors now use `#[serde(default, skip_serializing_if = "Vec::is_empty")]`
  - Eliminates verbose `Some(vec![...])` wrapping in user code
- Applied consistent Vec<T> serialization pattern across codebase
  - Added `skip_serializing_if = "Vec::is_empty"` to `SnapshotRestore.topics` and `.autolabel`
  - Added `skip_serializing_if = "Vec::is_empty"` to `TaskStages.stages`
  - Query parameters remain as `Option<Vec<T>>` where None vs Some([]) have different semantics
- Improved test coverage with comprehensive `test_populate_samples`
  - Generates 640x480 test image with red circle and bounding box annotation
  - Verifies byte-for-byte image upload/download matching
  - Uses image_name-based sample lookup (server doesn't return UUIDs)
  - Documents server limitations (width/height not returned in samples.list)

### Fixed
- Corrected field serialization names to match EdgeFirst Studio API
  - `Sample.location` now serializes as `"sensors"` (GPS/IMU data)
  - `Annotation.object_id` now serializes as `"object_reference"`
  - `Annotation.label` now serializes as `"label_name"`
  - Fields can still be deserialized from original names for backward compatibility
- Added GLIBC verification steps to CI/CD workflows to ensure manylinux2014 compatibility
  - Verifies CLI binaries require max GLIBC_2.17 after cargo zigbuild
  - Verifies Python extension modules (.so files) in wheels meet GLIBC requirements
  - Verifies bundled CLI binaries before packaging into Python wheels
  - Build fails with clear error if any binary violates manylinux2014 requirements
- Updated dependencies

### Added
- Automatic file upload in `populate_samples()` - detects local files and uploads to presigned S3 URLs
- Automatic UUID generation for samples in `populate_samples()` using UUIDv4
- Example `populate_with_circle.rs` demonstrating bbox annotations with auto-generated image and UUID
- Example `populate_with_annotations.rs` demonstrating location (sensors) usage with populate API
- Added `cargo-license` tool to Docker build image
- Auto-generate `THIRD_PARTY` file listing all third-party dependencies and their licenses
- Added `uuid` crate dependency (v1.11.0) with v4 and serde features

### Changed (License)
- Updated project license to Apache-2.0

---

## [2.1.0] - 2025-10-08

### Added
- Comprehensive API documentation throughout the codebase
- CLI testing support with unit tests
- Python coverage reporting (coverage.xml)
- Coverage reporting integration in CI/CD pipelines

### Changed
- Replaced generic `ID` type with strongly typed ID classes for better type safety
- Removed `server()` API; replaced with `url()` function that doesn't require a valid token
- `client::with_token` now returns self instead of error when given an empty token
- Reorganized CI/CD parallel builds based on dependencies
- Moved Python examples to dedicated examples folder

### Fixed
- Handle missing `project_id` in `TaskInfo`
- Correctly handle `STUDIO_USERNAME`/`STUDIO_PASSWORD` or `STUDIO_TOKEN` from CLI
- Updated tests to use `client.url` instead of `client.server`

---

## [2.0.8] - 2025-09-30

### Fixed
- The `annotations` function now returns empty annotations for images containing no annotations (instead of error)

### Changed
- Clippy fixes and code quality improvements
- Added `.gitignore` rule for HTML files generated by code coverage tools

---

## [2.0.7] - 2025-09-24

### Added
- Extended task filtering capabilities
- Improved `Box3d` API consistency

---

## [2.0.6] - 2025-09-23

### Added
- Label index support
- Updated dependencies

---

## [2.0.5] - 2025-09-23

### Changed
- Dependency updates

---

## [2.0.4] - 2025-09-18

### Changed
- Cleaned up logging output
- Enabled automatic token renewal when using the CLI

### Updated
- Rust toolchain updated to 1.89.0

---

## [2.0.3] - 2025-09-10

### Fixed
- Corrected `edgefirst_client.data` location to be under `crates/edgefirst-client-py`

---

## [2.0.2] - 2025-09-10

### Fixed
- Arrow dataframes corrected to use YOLO box2d format (center coordinates) by default
- Fixed splitting name and frame for arrow dataframe export

---

## [2.0.1] - 2025-09-09

### Changed
- Renamed binary to `edgefirst-client`

### Fixed
- Fixed ID usage in downloads
- Renamed `validation_session_id` parameter to `session_id`

---

## [2.0.0] - 2025-09-09

### Breaking Changes
- Major refactoring of authentication mechanism:
  - Rust client now uses factory-like pattern with `with_token()`, `with_server()`, `with_login()` extensions
  - Base `new()` method is now barebones; authentication methods add fields to Client
  - Python parameter ordering adjusted
  - Added parameter to disable loading tokens from disk
- Introduced strongly-typed `ID` type for object identifiers (replacing generic strings)
- `TrainerSession` renamed to `TrainingSession`
- Major Python client interface refactoring

### Added
- COCO label index/name support with tests
- Derive traits for `Label` type
- File upload and download APIs for sessions
- Metrics API for `TrainingSession` and `ValidationSession`
- Validation session support
- Task API support with filtering
- Label management support for datasets (CLI and API)
- Annotations and annotations_dataframe methods to Python client
- `Project` and `Dataset` Python classes with full method implementations
- Upload and download checkpoint functionality
- Upload artifact with automatic server path configuration
- Token parsing and renewal functions
- Organization query support
- Model parameters for `TrainingSession`
- Nested parameters support in `ValidationSession`

### Changed
- Refactored `download_artifact` API to allow default parameters for local path
- Updated `set_stages` API to use array of tuples instead of HashMap (preserves ordering)
- Workspace restructured with split crates
- Save images with correct extension for given image file format

### Fixed
- Login function now uses `rpc_without_auth`
- Python tests now passing

---

## [1.3.6] - 2025-05-28

### Added
- Default Python parameters for `restore_snapshot_sync` API

---

## [1.3.5] - 2025-05-28

### Fixed
- Raise error from `upload_multipart` instead of unwrapping on progress feedback

---

## [1.3.4] - 2025-05-02

### Fixed
- Fixed issue with `annotations_sync` function from Python

---

## [1.3.3] - 2025-04-30

### Added
- Progress feedback to annotations download (improves UX for slow operations)

### Changed
- Updated Cargo dependencies
- Added `.gitignore` rule for `*.arrow` files

### Fixed
- Fixed annotations feedback progress when multiple annotation types are requested

---

## [1.3.1] - 2025-04-16

### Fixed
- When combining annotations, use `object_id+image_id` as the key to ensure uniqueness across frames

### Changed
- Use `*.edgefirst.studio` URL instead of `*.dveml.com`

---

## [1.3.0] - 2025-04-10

- The annotations function will provide box2d using center-points (yolo format)
    - Studio uses top-left (coco format) internally but we document center-points.
- The annotations_list function is now private.
- Trainer renamed to Experiement to match the EdgeFirst Studio terminology in the UI.
    - Previously Trainer was used which is the terminology used by the REST API.
    - Functions `trainers`, `trainer`, `find_trainers` renamed to `experiements`, `experiment`, `find_experiments`.
- The `find_item` functions have been renamed `find_items` to emphasize that multiple matches can be returned.
- Functions returning objects instead of object_id have been added throughout the API.
    - For example Experiment can return the parent Project directly instead of project_id.
    - As well Experiement can return the list of TrainingSessions directly instead of having to use the Experiement ID when calling the `Client::training_sessions`.
- TrainingDataset has been renamed DatasetParams and the `TrainingSession::dataset()` renamed `TrainingSession::dataset_params()`.
