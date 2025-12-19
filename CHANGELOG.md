# Changelog

All notable changes to EdgeFirst Client will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- **Client() now uses correct server from stored token** - Fixed issue where `Client()` would always connect to production server even when token was issued for a different server (test, stage, etc.). The client now extracts and uses the server from the JWT token payload.

## [2.7.2] - 2025-12-17

### Changed

- **Optimized release workflow** - Automated Package.swift update PRs now skip CI (saves ~30 minutes per release)
- **Simplified PR creation** - Use default GITHUB_TOKEN with repository setting instead of PAT

## [2.7.1] - 2025-12-17

### Fixed

- **Release workflow Swift package update** - Changed from direct push to creating pull request to respect branch protection rules

## [2.7.0] - 2025-12-17

### Added

- **Swift Package Manager (SPM) support for iOS/macOS SDK**
  - Distributed as `edgefirst-client-swift-{version}.zip` containing source files
  - Fully compatible with Xcode projects and SPM-based workflows
  - Enables seamless integration with modern Swift development tools

- **Comprehensive Swift SDK test coverage**
  - Swift tests now run on all PR changes via `swift-tests.yml` workflow
  - Test coverage reporting integrated with SonarCloud
  - Expanded test scenarios: `createClientWithStorage`, dataset operations, error handling
  - Coverage collection and reporting for iOS/macOS SDK

### Changed

- **Swift test execution on pull requests** - Swift SDK tests automatically run on all PRs to catch regressions early

### Fixed

- **Swift coverage collection and reporting** - Fixed Swift code coverage generation for SonarCloud integration
- **CI/CD workflow reliability** - Improved cross-workflow artifact handling and test execution

## [2.6.4] - 2025-12-11

### Changed

- **Clarified server selection priority for `--server` and `with_server()`**

  Server selection now follows a clear priority order:

  1. **Token's server** (highest) - JWT tokens encode their server; if a valid token exists, its server is used
  2. **`--server` / `with_server()`** - Used when logging in or when no token is available
  3. **Default "saas"** - Production server (`https://edgefirst.studio`) when no token and no server specified

  **Behavior changes:**
  - `login` command ignores existing tokens and uses `--server` (or defaults to saas)
  - Other commands warn if `--server` conflicts with token's server: the token's server takes priority
  - Username/password authentication honors `--server` (obtains new token for that server)

### Added

- **`Client.server` property** - Returns the server name for the current client (e.g., "saas", "test", "stage"). Extracts the name from the client's URL regardless of how the server was selected.

### Fixed

- **CLI binary now bundled in Python wheel** - The `edgefirst-client` CLI binary is now properly included in the Python wheel and installed to the PATH when using `pip install edgefirst-client`. Previously, only the Python library was included.

## [2.6.2] - 2025-12-11

### Changed

- **CI/CD workflow optimization**
  - Refactored release workflow to download pre-built artifacts instead of rebuilding
  - Removed redundant tag triggers from `build.yml`, `mobile.yml`, and `sbom.yml`
  - Release workflow now waits for CI workflows to complete on the commit, then downloads artifacts
  - Reduces CI time and ensures release artifacts are identical to what was tested
  - Uses `lewagon/wait-on-check-action` to wait for dependent workflows
  - Uses `dawidd6/action-download-artifact` to download cross-workflow artifacts

## [2.6.1] - 2025-12-11

### Fixed

- **CI/CD release workflow fixes**
  - Fixed duplicate GitHub release bug: release workflow jobs now correctly use `v` prefix for tag names
  - Fixed CHANGELOG link in release notes to use correct `v{version}` tag URL
  - Prevents creation of separate releases for `v2.6.0` and `2.6.0` tags

- **Mobile SDK naming consistency**
  - Renamed SDK zip files for consistency: `edgefirst-android-sdk` → `edgefirst-client-android`
  - Renamed SDK zip files for consistency: `edgefirst-swift-sdk` → `edgefirst-client-swift`
  - Renamed Swift header file: `EdgeFirstClientFFI.h` → `EdgeFirstClient.h`
  - Updated all documentation references in README.md, ANDROID.md, APPLE.md

## [2.6.0] - 2025-12-11

### Added

- **Mobile SDKs for Android and iOS/macOS**

  New UniFFI-based mobile bindings enable native app development with EdgeFirst Client:

  **Kotlin (Android):**
  - Full API coverage with coroutine-based async methods
  - JNI native libraries for arm64-v8a, armeabi-v7a, x86_64
  - `TokenStorage` callback interface for custom storage (EncryptedSharedPreferences)
  - All data types: `Sample`, `Annotation`, `Mask`, `Project`, `Dataset`, etc.
  - See [ANDROID.md](ANDROID.md) for complete documentation

  **Swift (iOS/macOS):**
  - Full API coverage with async/await pattern
  - XCFramework supporting iOS, iOS Simulator, and macOS
  - `TokenStorage` protocol for custom storage (Keychain Services)
  - Swift Package Manager compatible distribution
  - See [APPLE.md](APPLE.md) for complete documentation

  **CI/CD Integration:**
  - New `.github/workflows/mobile.yml` workflow
  - Automated builds for Android NDK and Apple platforms
  - Automated binding generation and SDK packaging
  - Release artifacts: `edgefirst-client-android-{version}.zip`, `edgefirst-client-swift-{version}.zip`

- **Token Storage Abstraction for Platform Portability**

  New trait-based token storage delegate API enables custom storage backends for
  different platforms (iOS Keychain, Android EncryptedSharedPreferences, etc.):

  **Rust API:**
  - `TokenStorage` trait with `store()`, `load()`, `clear()` methods
  - `FileTokenStorage` - file-based persistence (default on desktop)
  - `MemoryTokenStorage` - in-memory storage (no persistence)
  - `StorageError` - dedicated error type for storage operations

  **Python API:**
  - `FileTokenStorage` class with `store()`, `load()`, `clear()`, `path` property
  - `MemoryTokenStorage` class with `store()`, `load()`, `clear()`
  - Custom Python storage via duck typing (any object with the 3 methods)

  **Client Builder Methods (both Rust and Python):**
  - `with_server(name)` - configure server instance ("test", "stage", "saas", etc.)
  - `with_storage(storage)` - configure custom token storage backend
  - `with_memory_storage()` - use in-memory storage (no persistence)
  - `with_no_storage()` - disable token storage entirely
  - `with_login(username, password)` - authenticate with credentials (builder-style)
  - `with_token(token)` - authenticate with existing token (builder-style)
  - `logout()` - clear stored token from memory and storage

  **Use Cases:**
  - Desktop: Use default `FileTokenStorage` for persistent login sessions
  - iOS: Implement `TokenStorage` using Keychain Services
  - Android: Implement `TokenStorage` using EncryptedSharedPreferences
  - Testing: Use `MemoryTokenStorage` for isolated test environments
  - Serverless: Use `with_no_storage()` when tokens are managed externally

- **New `create_snapshot_from_dataset` API method**
  - Python: `client.create_snapshot_from_dataset(dataset_id, description, annotation_set_id=None)`
  - Rust: `client.create_snapshot_from_dataset(dataset_id, description, annotation_set_id).await`
  - Creates server-side snapshots from existing datasets
  - Returns `SnapshotFromDatasetResult` with snapshot ID and task ID for progress monitoring
  - Automatically selects default "annotations" set if not specified

- **New `format` module for EdgeFirst Dataset Format utilities**
  - `resolve_arrow_files()` - Read Arrow files and extract sample references
  - `resolve_files_with_container()` - Match Arrow references against actual files
  - `validate_dataset_structure()` - Validate dataset directory structure
  - `generate_arrow_from_folder()` - Create Arrow manifest from image folders
  - Exposed as `edgefirst_client::format` in Rust API

- **Comprehensive snapshot workflow tests**
  - `test_snapshot_restore` - Validates restore preserves group assignments
  - `test_create_snapshot_from_dataset` - Validates export preserves data
  - `test_server_rejects_inconsistent_group_snapshot` - Tests server validation

- **Additional `filter_and_sort_by_name` tests**
  - Tests ensure exact match determinism for name searches
  - Prevents flaky tests from similarly-named resources

- **Python bindings: New ergonomic shortcut methods**
  
  Objects now include shortcut methods that internally delegate to the client,
  eliminating the need to call `client.method(object.id, ...)` patterns:

  **Dataset:**
  - `dataset.download(groups, types, output, ...)` → shortcut for `client.download_dataset(dataset.id, ...)`
  - `dataset.annotation_sets()` → shortcut for `client.annotation_sets(dataset.id)`
  - `dataset.samples(...)` → shortcut for `client.samples(dataset.id, ...)`
  - `dataset.samples_count(...)` → shortcut for `client.samples_count(dataset.id, ...)`

  **AnnotationSet:**
  - `annotation_set.annotations(groups, types, ...)` → shortcut for `client.annotations(annotation_set.id, ...)`

  **Experiment:**
  - `experiment.training_sessions(name)` → shortcut for `client.training_sessions(experiment.id, name)`

  **Snapshot:**
  - `snapshot.download(output)` → shortcut for `client.download_snapshot(snapshot.id, output)`

  **Client:**
  - `client.download_sample(sample, file_type)` → downloads a single sample's file data

  **Sample:**
  - `sample.download(file_type)` → now uses embedded client reference (new ergonomic API)

### Changed

- **README.md documentation improvements**
  - Added comprehensive `--detect-sequences` documentation with behavior table
  - Clarified supported file types: depth maps (16-bit PNG), radar cubes (16-bit PNG)
  - Updated annotation support section: `create-snapshot` supports annotated Arrow files
  - Clarified AGTG `--autolabel` only works with MCAP snapshots

### Deprecated

- **Python Client Constructor Parameters**

  The `Client()` constructor parameters are deprecated in favor of builder methods.
  Old API still works but emits `DeprecationWarning`. Planned removal: v3.0.0.

  - `Client(server="test")` → `Client().with_server("test")`
  - `Client(use_token_file=False)` → `Client().with_memory_storage()`
  - `Client(username="...", password="...")` → `Client().with_login("...", "...")`
  - `Client(token="...")` → `Client().with_token("...")`

- **Python bindings: Comprehensive ergonomic API with embedded client references**
  
  The following classes now store an internal client reference, enabling cleaner
  method calls without passing `client` explicitly. Old API still works but emits
  `DeprecationWarning`. Planned removal: v3.0.0.

  **Project:**
  - `project.datasets(client)` → `project.datasets()`
  - `project.experiments(client)` → `project.experiments()`
  - `project.validation_sessions(client)` → `project.validation_sessions()`

  **Dataset:**
  - `dataset.labels(client)` → `dataset.labels()`
  - `dataset.add_label(client, name)` → `dataset.add_label(name)`
  - `dataset.remove_label(client, name)` → `dataset.remove_label(name)`

  **TrainingSession:**
  - `session.metrics(client)` → `session.metrics()`
  - `session.set_metrics(client, metrics)` → `session.set_metrics(metrics)`
  - `session.artifacts(client)` → `session.artifacts()`
  - `session.upload(client, files)` → `session.upload(files)`
  - `session.download(client, filename)` → `session.download(filename)`
  - `session.download_artifact(client, filename)` → `session.download_artifact(filename)`
  - `session.upload_artifact(client, filename)` → `session.upload_artifact(filename)`
  - `session.download_checkpoint(client, filename)` → `session.download_checkpoint(filename)`
  - `session.upload_checkpoint(client, filename)` → `session.upload_checkpoint(filename)`

  **ValidationSession:**
  - `session.metrics(client)` → `session.metrics()`
  - `session.set_metrics(client, metrics)` → `session.set_metrics(metrics)`
  - `session.artifacts(client)` → `session.artifacts()`
  - `session.upload(client, files)` → `session.upload(files)`

  **Leaf objects (many instances, no embedded client):**
  - `Label.remove(client)`, `Label.set_name(client, name)`, `Label.set_index(client, index)` - deprecated, use `client.remove_label()` / `client.update_label()`

  **Sample (now has embedded client):**
  - `sample.download(client)` → `sample.download()` - client parameter deprecated
  - For bulk downloads, use `dataset.download()` or `client.download_dataset()` which is significantly faster

- **Rust: Convenience methods on data objects**
  - `Dataset::labels(&self, client)` → use `client.labels(dataset.id())`
  - `Dataset::add_label(&self, client, name)` → use `client.add_label(dataset.id(), name)`
  - `Dataset::remove_label(&self, client, name)` → use `client.remove_label(label.id())`
  - `Label::remove(&self, client)` → use `client.remove_label(label.id())`
  - Methods marked `#[deprecated]` - emit compile-time warnings
  - Planned removal: v3.0.0

### Fixed

- **Python bindings: Added `SnapshotFromDatasetResult` class**
  - Complete type stub in `.pyi` file with docstrings
  - Properties: `id` (SnapshotID), `task_id` (TaskID | None)

### Migration Guide: Token Storage API

The new builder-style API for Client configuration provides better platform
portability and cleaner code. The old constructor parameters still work but
emit deprecation warnings.

**Python Migration:**

```python
# OLD: Constructor parameters (deprecated, emits DeprecationWarning)
client = Client(server="test")
client = Client(username="user", password="pass", server="test")
client = Client(token="eyJ...", use_token_file=False)

# NEW: Builder methods (recommended)
client = Client().with_server("test")
client = Client().with_server("test").with_login("user", "pass")
client = Client().with_memory_storage().with_token("eyJ...")

# Custom storage for mobile platforms
class KeychainStorage:
    def store(self, token): ...
    def load(self): return self._token
    def clear(self): ...

client = Client().with_storage(KeychainStorage()).with_server("test")
```

**Rust Migration:**

```rust
// OLD: Direct token file handling
let client = Client::new()?;
client.login("user", "pass").await?;

// NEW: Builder pattern with explicit storage
let client = Client::new()?
    .with_server("test")?
    .with_login("user", "pass")
    .await?;

// Custom storage (implement TokenStorage trait)
let storage = Arc::new(MySecureStorage::new());
let client = Client::new()?
    .with_storage(storage)
    .with_server("test")?;
```

### Migration Guide: Ergonomic Python API

Objects obtained from `Client` methods now store an internal client reference,
enabling cleaner method calls without passing `client` explicitly. The old API
with `client` parameter is deprecated but still works.

**Before (deprecated):**

```python
# OLD: Passing client to every method (emits DeprecationWarning)
project = client.project("p-123")
datasets = project.datasets(client)

dataset = client.dataset("ds-123")
labels = dataset.labels(client)
dataset.add_label(client, "person")

session = client.training_session("t-456")
session.upload(client, files)
session.set_metrics(client, {"accuracy": 0.95})
```

**After (recommended):**

```python
# NEW: Clean API - objects store client reference internally
project = client.project("p-123")
datasets = project.datasets()          # No client needed!

dataset = client.dataset("ds-123")
labels = dataset.labels()
dataset.add_label("person")            # Clean and simple

session = client.training_session("t-456")
session.upload(files)                  # Intuitive
session.set_metrics({"accuracy": 0.95})

# Alternative: Use Client methods directly (also valid)
labels = client.labels(dataset.id)
client.add_label(dataset.id, "person")
```

**Rust users:** Use `client.method(id)` pattern instead of `dataset.method(&client)`:

```rust
// Deprecated (emits compile warning)
let labels = dataset.labels(&client).await?;

// Recommended
let labels = client.labels(dataset.id()).await?;
```

**Why this change?**

1. **Pythonic**: Objects that can perform operations should do so directly
2. **Ergonomic**: Less boilerplate, cleaner code
3. **Consistent**: Rust keeps explicit `client` passing (idiomatic), Python gets OOP style

## [2.5.2] - 2025-12-01

### Fixed

- **Release workflow SBOM generation reliability**
  - SBOM workflow (`sbom.yml`) now triggers on version tags in addition to main branch pushes
  - Release workflow waits for SBOM workflow to complete before downloading artifact
  - Downloads SBOM from exact tag commit SHA ensuring release integrity
  - Removes scancode-toolkit dependency from release workflow (uses pre-built artifact)
  - Uses `lewagon/wait-on-check-action` to synchronize workflow execution

## [2.5.1] - 2025-11-29

### Fixed

- **Improved search result ordering for name-filtered API queries**
  - All list APIs that filter by name now return results sorted by match quality
  - Exact matches (case-sensitive) appear first, followed by case-insensitive exact matches
  - Shorter names (more specific matches) are prioritized over longer names
  - Affects: `projects()`, `datasets()`, `snapshots()`, `experiments()`, `training_sessions()`, and `tasks()`
  - Example: Searching for "Deer" now returns "Deer" before "Deer Roundtrip 20251129"
  - Resolves test flakiness caused by stale datasets matching search patterns

- **Documentation clarification for flattened dataset downloads**
  - Clarified behavior of automatic filename prefixing in `flatten` mode
  - Improved documentation for `Client::download_dataset()` Rust API
  - Updated Python type hints for `download_dataset()` method

## [2.5.0] - 2025-11-27

### Added

- **Test coverage for examples/download.py**
  - Added `test/test_examples.py` with integration test for download example
  - Refactored `download.py` to export `download_dataset_yolo()` function for testability
  - Example now properly covered by slipcover during CI/CD test runs
  - Improves overall test coverage metrics for SonarCloud quality gate
- **Download dataset with flattened directory structure (`--flatten` option)**
  - New `--flatten` flag for `download-dataset` CLI command to download all files into a single directory
  - Smart filename prefixing: automatically adds `{sequence_name}_{frame}_` prefix to avoid conflicts
  - Only prefixes files when necessary (checks if sequence prefix already exists)
  - Python API: `client.download_dataset(..., flatten=False)` parameter (non-breaking, defaults to False)
  - Helper function `Client::build_filename()` for intelligent prefix handling
  - Comprehensive documentation in CLI.md with directory structure examples

### Breaking Changes

- **BREAKING**: Rust API: Added `flatten: bool` parameter to `Client::download_dataset()`. This changes the function signature and will break existing Rust client code that does not specify the new parameter.
- **Upload dataset with automatic flattened structure detection**
  - `upload-dataset` now automatically detects both nested and flattened directory structures
  - Enhanced `parse_annotations_from_arrow()` to work with both organizational patterns
  - Enhanced `extract_sequence_name()` to parse sequence info from filename prefixes (flattened) or subdirectories (nested)
  - Recursive directory walking in `build_image_index()` handles both structures transparently
  - Arrow file metadata (`name` and `frame` columns) is authoritative source for sequence information
  - No manual configuration needed - works automatically for both upload and download
- **DATASET_FORMAT.md enhancements**
  - Documented nested vs. flattened directory structure patterns
  - Added client implementation guidelines for upload/download operations
  - Included file naming conventions for both organizational patterns
  - Added examples showing directory structure transformations

### Fixed

- **Graceful handling of corrupted or expired authentication tokens**
  - `with_token_path()` now catches token validation errors and automatically removes corrupted token files
  - CLI displays helpful error message when token renewal fails, directing users to login again
  - Invalid tokens are cleaned up automatically, preventing "stuck" authentication states
  - Users can now successfully run `login` or `logout` commands even with corrupted token files

## [2.4.3] - 2025-11-18

### Added

- **Python bindings: Pythonic dict/list-like API for `Parameter` class**
  - `.get(key, default=None)`: Dict-like method for Object parameters (recommended API)
  - `.keys()`, `.values()`, `.items()`: Dict-like iteration methods for Object parameters

### Changed

- **Python bindings: `__str__()` for String parameters now returns plain string values**
  - `str(Parameter.string("hello"))` now returns `"hello"` instead of `"String(hello)"`
  - `__repr__()` still returns descriptive format `"String(hello)"` for debugging
  - Eliminates need for manual string parsing: `modelname.removeprefix("String(").removesuffix(")")`
- **Reduced log verbosity for retry configuration**
  - Retry configuration details now logged at `debug` level instead of `info`
  - Eliminates unnecessary INFO messages during normal operation

### Notes

- **Python bindings: Parameter API limitations due to PyO3**
  - Bracket indexing (`param["key"]` or `param[0]`) is not supported directly on Parameter objects
  - `len()` and `in` operators not supported directly on Parameter objects
  - **Workarounds**: Use `.get(key)` for Objects, `.as_array()` for Arrays, `len(param.keys())` for length
  - This is a PyO3 framework limitation with enum variants wrapping collections
  - The `.get()` API is Pythonic and widely used (e.g., `os.environ.get('KEY')`)
- **Python bindings: Pre-existing conversion methods work with new API**
  - `.as_array()`: Convert Array parameters to native Python lists (enables indexing `arr[0]`)
  - `.as_object()`: Convert Object parameters to native Python dicts (enables indexing `obj["key"]`)
  - These methods existed before this release and complement the new `.get()` API

## [2.4.2] - 2025-11-17

### Added

- **URL-based retry classification** for intelligent error handling
  - Classifies requests into two categories: StudioApi vs FileIO
  - **StudioApi** (`*.edgefirst.studio/api`): Fast-fail on auth errors (401/403), retry server errors
  - **FileIO** (S3, CloudFront): Aggressive retry on all transient errors (408, 409, 423, 429, 5xx)
  - Optimized for high-concurrency file operations (100+ parallel S3 uploads/downloads)
  - Prevents unnecessary retries on authentication failures while maximizing robustness for file transfers
  - See `crates/edgefirst-client/src/retry.rs` for detailed documentation

### Fixed

- **JWT token parsing now correctly extracts server name from `server` field instead of `database` field**
  - Fixes authentication failures when using tokens from SaaS server
  - `Client::with_token()` now properly identifies target EdgeFirst Studio instance
  - Critical fix for token-based authentication across all server environments
- Retry mechanism now functions correctly for all HTTP requests
  - Removed per-request timeout override that was preventing reqwest retry policy from activating
  - Reduced default timeout from 120s to 30s for faster failure detection (configurable via `EDGEFIRST_TIMEOUT`)
  - Reduced default retries from 5 to 3 for faster test execution (configurable via `EDGEFIRST_MAX_RETRIES`)
  - Improved retry classification: connection errors and timeouts now properly trigger retries
  - Enhanced logging: retry configuration now displays at client initialization
- Internal: Marked `test_dataset_roundtrip` as `#[ignore]` - requires `EDGEFIRST_TIMEOUT=120` due to 1600+ sample upload
- Internal: Added `Sleep` command for diagnostic testing (not for production use)

### Changed

- HTTP timeout defaults changed to improve test performance
  - Default timeout: 120s → 30s (override with `EDGEFIRST_TIMEOUT=<seconds>`)
  - Default max retries: 5 → 3 (override with `EDGEFIRST_MAX_RETRIES=<count>`)
  - Worst-case timeout: 600s → 90s (30s × 3 retries)
  - **For bulk file operations**: Set `EDGEFIRST_MAX_RETRIES=10` for better S3 resilience
  - For bulk operations with large datasets, set `EDGEFIRST_TIMEOUT=120` before running tests

## [2.4.1] - 2025-11-06

### Fixed

- Internal: CLI tests now run serially to prevent concurrent Studio server access issues
  - Added `#[serial]` attribute to `test_download_annotations`, `test_download_artifact`, and `test_upload_artifact`
  - Prevents race conditions and server overload during integration testing
- Internal: GitHub Actions workflow fixes for improved CI/CD reliability
  - Updated GitHub Actions workflows for improved reliability
  - Added comprehensive trace-level logging to retry mechanisms in client.rs
  - Enhanced download() with URL tracking, attempt counters, and error classification
  - Enhanced try_rpc_request() with method name logging, socket error details, and HTTP response capture
  - Added error type classification (is_timeout, is_connect, is_request)

### Changed

- Internal: Updated GitHub Actions image references for CI/CD pipeline

## [2.4.0] - 2025-11-06

### Fixed

- Annotation object tracking IDs now correctly preserved through upload/download cycle
  - Fixed `Annotation.object_id` field serialization: now sends `object_reference` (server consumes and responds with this field name)
  - Added `object_id` as alias for forward compatibility
- Sample group assignments now correctly preserved through upload/download cycle
  - Fixed `Sample` struct serialization: now sends `group` field (was incorrectly sending `group_name`)
  - Server expects `group` for uploads, returns `group_name` in queries - client now handles both correctly
  - Workaround for server bug: batching logic groups by (sequence_uuid, group) tuple to prevent server from assigning all samples in batch to first sample's group
  - Test comparison now uses (name, frame) tuple as sample key (previously used only name, causing false mismatches)
- DataFrame frame column now correctly uses integer type (u32) instead of string
  - `extract_annotation_name()` and `extract_annotation_name_from_sample()` now parse frame numbers from image_name and return `Option<u32>`
  - Server's frame_number is now preferred when available; filename parsing used as fallback for legacy data
  - Reduces Arrow file size by ~5-10% (integers more efficient than strings)

### Added

- Mask polygon conversion helpers for public API
  - Added `unflatten_polygon_coordinates()` function: reconstructs nested polygon structure from flat coordinates with NaN separators
  - Enables CLI to properly convert Arrow file mask data back to Studio's expected nested format during upload
  - Public API available for applications that need to parse EdgeFirst Arrow files
- Comprehensive `DATASET_FORMAT.md` specification (v2.1.0)
  - Consolidated all dataset format documentation into single comprehensive document
  - Added 5 Mermaid diagrams for visual clarity (dataset architecture, format relationships, coordinate systems, format deviations, conversion flow)
  - Documented JSON vs DataFrame format differences explicitly (Box2D: left/top vs cx/cy, Mask: nested lists vs NaN-separated flat lists, Sample metadata: JSON-only)
  - Clarified Box2D coordinate system: JSON uses legacy Studio API format (left/top), DataFrame uses ML-standard center-point format (YOLO)
  - Enhanced mask format documentation: JSON stores nested polygon lists, DataFrame uses flattened coordinates with NaN separators (Polars limitation)
  - Documented sample metadata fields (width, height, GPS, IMU, degradation) as JSON-only (not in DataFrame)
  - Complete directory structure examples for sequence-based, image-based, and mixed datasets
  - Sensor data formats: Camera (JPEG/PNG with EXIF), Radar (PCD + data cube PNG), LiDAR (PCD)
  - Conversion guidelines with code examples for bidirectional JSON ↔ DataFrame transformation
  - Best practices for format selection, dataset organization, and annotation quality
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
- New DataFrame API: `samples_dataframe()` function with complete 2025.10 schema support
  - 13-column DataFrame (name, frame, object_reference, label, label_index, group, mask, box2d, box3d, size, location, pose, degradation)
  - Takes `&[Sample]` input for access to all sample metadata (GPS, IMU, degradation)
  - Optional columns populated from sample sensor data when available
  - Rust: `edgefirst_client::samples_dataframe(&[Sample]) -> Result<DataFrame, Error>`
  - Python: `Client.samples_dataframe(dataset_id, annotation_set_id, groups, annotation_types, progress) -> DataFrame`
  - CLI: `download` command now generates 13-column Arrow files automatically
- Sample struct: Added `degradation: Option<String>` field for image quality metadata

### Changed

- Documentation consolidation: Removed redundant files
  - Removed: `JSON_FORMAT_SPECIFICATION.md`, `DATASET_STRUCTURE.md`, `DATASET_DOCS_REVIEW.md`, `JSON_NESTED_ANALYSIS.md`, `JSON_OPTIONAL_FIELDS.md`, `FIELD_RENAMING_SUMMARY.md`
  - All content merged into comprehensive `DATASET_FORMAT.md`
- CLI: Refactored Arrow annotation parsing to eliminate type complexity warning
  - `parse_annotations_from_arrow` now returns `Vec<Sample>` directly instead of intermediate HashMap
  - Merged `build_samples_from_map` logic into single function for cleaner architecture
  - Added 9 comprehensive test cases covering all code paths and edge cases
- DataFrame column names: Fixed incorrect naming to match 2025.10 specification
  - "label_name" → "label" (categorical annotation class name)
  - "group_name" → "group" (categorical dataset split: train/val/test)
  - Affects `annotations_dataframe()` (deprecated) for backward compatibility

### Deprecated

- `annotations_dataframe()` function - use `samples_dataframe()` instead
  - Rust: `edgefirst_client::annotations_dataframe(&[Annotation])`
  - Python: `Client.annotations_dataframe(annotation_set_id, groups, annotation_types, progress)`
  - Reason: Cannot access sample metadata (width, height, GPS, IMU, degradation) from Annotation struct
  - Migration: Use `samples_dataframe()` with same parameters plus `dataset_id`
  - Removal: Planned for v1.0.0 (several minor versions away)
  - Still works: No breaking changes, will emit deprecation warnings

### Migration Guide

#### Rust API

```rust
// OLD (deprecated):
let annotations = client.annotations(annotation_set_id, &groups, &types, Some(tx)).await?;
let df = edgefirst_client::annotations_dataframe(&annotations)?;

// NEW:
let annotation_set = client.annotation_set(annotation_set_id).await?;
let dataset_id = annotation_set.dataset_id();
let df = client.samples_dataframe(dataset_id, Some(annotation_set_id), &groups, &types, Some(tx)).await?;
```

#### Python API

```python
# OLD (deprecated):
df = client.annotations_dataframe(annotation_set_id, ["train"], [], None)

# NEW:
df = client.samples_dataframe(dataset_id, annotation_set_id, ["train"], [], None)
```

#### CLI (Automatic)

The CLI `download` command automatically uses the new API - no user changes required.

#### DataFrame Schema Changes

- Column count: 9 columns → 13 columns
- New optional columns: `size` (Array<UInt32, 2>), `location` (Array<Float32, 2>), `pose` (Array<Float32, 3>), `degradation` (String)
- Column names: `label_name` → `label`, `group_name` → `group`
- Existing columns unchanged: name, frame, object_reference, label_index, mask, box2d, box3d

### Changed

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
- Auto-generate SBOM (Software Bill of Materials) in CycloneDX format listing all third-party dependencies and their licenses
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
