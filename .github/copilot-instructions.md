# EdgeFirst Client - AI Agent Instructions

## Project Overview

EdgeFirst Client is a **dual-language Rust+Python** REST API client for EdgeFirst Studio, an MLOps platform for 3D/4D spatial perception AI. The project uses PyO3 to expose Rust core functionality to Python users.

**Architecture**: Monorepo with Cargo workspace containing three crates:
- `crates/edgefirst-client/`: Core Rust library with async HTTP client (reqwest + Tokio)
- `crates/edgefirst-cli/`: CLI application using the core library
- `crates/edgefirst-client-py/`: Python bindings via PyO3/maturin

## Critical Build & Test Commands

### Local Development
```bash
# Build all crates
cargo build

# Run tests (requires Studio credentials - see CONTRIBUTING.md)
export STUDIO_SERVER=test
export STUDIO_USERNAME=<username>
export STUDIO_PASSWORD=<password>
cargo test

# Python bindings - build and install locally
pip install maturin
maturin develop -m crates/edgefirst-client-py/Cargo.toml

# Format code (required before commit)
cargo +nightly fmt --all

# Linting (fix all warnings)
cargo clippy --all-targets --all-features --locked
```

### Test Infrastructure
- **Studio Integration Tests**: Require authenticated access to EdgeFirst Studio test servers
- **Test Data**: Uses `Unit Testing` project, `Deer` dataset (read-only), `Test Labels` dataset (CRUD)
- **CI/CD**: Contributors without credentials can rely on GitHub Actions PR checks with stored secrets

### Cross-Platform Building
- **Linux builds**: Use `cargo-zigbuild` with `x86_64-unknown-linux-gnu.2.17` target for manylinux2014 compatibility
- **Python wheels**: Build with `maturin build --zig --compatibility manylinux2014` on Linux
- See `.github/workflows/build.yml` and `.github/workflows/python.yml` for platform-specific configurations

## Code Organization Patterns

### Client Architecture (crates/edgefirst-client/src/)
- `client.rs`: Main `Client` struct with JSON-RPC API methods, token management, multipart uploads
- `api.rs`: Type definitions for API requests/responses (Project, Dataset, TrainingSession, etc.)
- `dataset.rs`: Dataset operations, file downloads, annotation parsing
- `error.rs`: Custom error enum with manual `From` trait implementations for error conversions
- `lib.rs`: Public API surface and feature flags

**Key Design Patterns**:
- Async-first: All API calls are `async fn` using Tokio runtime
- Progress tracking: Use `tokio::sync::mpsc::Sender<Progress>` for long-running operations
- Concurrency limiting: Semaphore with `MAX_TASKS = 32` limits parallel uploads/downloads to prevent resource exhaustion
- Multipart upload: Files chunked at `PART_SIZE = 100MB` and uploaded via pre-signed S3 URLs from Studio API (`MAX_RETRIES = 10`)

### Python Bindings (crates/edgefirst-client-py/src/)
- Single `lib.rs` with PyO3 wrappers for all Rust types
- Use `tokio-wrap` to bridge async Rust → sync Python
- Maintain parallel type system with `From`/`TryFrom` conversions
- Export `.pyi` type stubs for IDE support

## Versioning & Release (Critical)

**Version Format**: `X.Y.Z` for stable, `X.Y.ZrcN` for release candidates (NO separators like `-rc.1`)
- **Why**: PyPI requires `rcN` format (PEP 440), Cargo accepts both, maturin doesn't convert
- **Workspace versioning**: Single version in root `Cargo.toml` inherited via `version.workspace = true`

**Release Process** (maintainers only):
```bash
# Update CHANGELOG.md first

# Stable release
cargo release patch --execute --no-confirm  # or: minor, major

# Release candidate (MANUAL version edit required)
sed -i '' 's/version = "2.2.2"/version = "2.3.0rc1"/' Cargo.toml
sed -i '' 's/edgefirst-client = { version = "2.2.2"/edgefirst-client = { version = "2.3.0rc1"/' Cargo.toml
cargo release 2.3.0rc1 --execute --no-confirm

# Push to trigger CI/CD
git push && git push --tags
```

See `CONTRIBUTING.md` (lines 280-340) and `release.toml` for full details.

## Dependency & Feature Management

**Workspace Dependencies**: All dependencies defined in root `Cargo.toml` `[workspace.dependencies]`
- TLS enforcement: `reqwest` with `rustls-tls` (no native-tls)
- Async runtime: `tokio` with `full` and `rt-multi-thread` features
- Optional Polars: Enable with `features = ["polars"]` for DataFrame support

**Feature Flags**:
- Default: `default = ["polars"]`
- Conditional compilation: Use `#[cfg(feature = "polars")]` for Polars-dependent code

## Testing Conventions

### Rust Tests
- Unit tests: In same file as implementation
- Integration tests: `crates/edgefirst-cli/tests/` and `crates/edgefirst-client/src/lib.rs`
- Test helpers: `get_test_data_dir()` creates `target/testdata/`
- Coverage: Use `cargo llvm-cov` with `--doctests` flag

### CI Workflows (GitHub Actions)
- `test.yml`: Lint, audit, test with coverage (Rust + Python), SonarCloud analysis
- `build.yml`: Cross-platform CLI binaries (Linux/macOS/Windows, x64/arm64)
- `python.yml`: Python wheels for multiple platforms via maturin
- `release.yml`: Triggered by version tags, publishes to crates.io/PyPI

**Coverage Collection** (see test.yml lines 113-122):
```bash
source <(cargo llvm-cov show-env --export-prefix --no-cfg-coverage --doctests)
cargo build --locked
cargo nextest run --locked --profile ci
cargo test --doc --locked
cargo llvm-cov report --doctests --lcov --output-path lcov.info
```

## API Client Patterns

### Authentication
- JWT token stored in OS-specific config directory as plaintext file named `token` (7-day expiry)
  - Linux: `~/.config/EdgeFirst Studio/token`
  - macOS: `~/Library/Application Support/ai.EdgeFirst.EdgeFirst Studio/token`
  - Windows: `%APPDATA%\EdgeFirst\EdgeFirst Studio\config\token`
- Auto-renewal via `verify_token()` → `renew_token()` flow
- Environment variable override: `STUDIO_TOKEN`

### JSON-RPC Requests
```rust
// Pattern from client.rs
let request = RpcRequest {
    id: 0,
    jsonrpc: "2.0".to_string(),
    method: "method_name".to_string(),
    params: Some(params_struct),
};
let response: RpcResponse<ResultType> = self.rpc(request).await?;
```

### Progress Callbacks
```rust
// Implement for downloads/uploads
let (tx, mut rx) = mpsc::channel(1);
tokio::spawn(async move {
    while let Some(progress) = rx.recv().await {
        println!("{}/{}", progress.current, progress.total);
    }
});
client.download_dataset(id, &["image"], path, Some(tx)).await?;
```

## Documentation Standards

- **Doc comments**: Required for all public APIs with examples
- **Code formatting**: Use `cargo fmt` (config in `rustfmt.toml`)
- **Example style**: Include `#[tokio::main]` for async examples, handle `Result<(), Error>`
- **Cross-language docs**: Mirror Rust examples in Python docstrings

## Common Pitfalls

1. **Async boundaries**: Python bindings use `tokio-wrap` - don't use bare `tokio::runtime::Handle::block_on`
2. **Version format**: Never use `-rc.1` format, always use `rc1` (no separators)
3. **Feature gates**: Remember `#[cfg(feature = "polars")]` when using Polars types
4. **Test credentials**: Integration tests need `STUDIO_USERNAME`/`STUDIO_PASSWORD` - use CI for full coverage
5. **Multipart uploads**: Files are split at `PART_SIZE = 100MB` - handle retries per part
6. **Import formatting**: Use `imports_granularity = 'Crate'` (rustfmt.toml) - imports grouped by crate

## Key Files to Reference

- `CONTRIBUTING.md`: Development setup, test infrastructure, release process
- `.github/WORKFLOW_ARCHITECTURE.md`: Detailed CI/CD documentation
- `Cargo.toml` (root): Workspace configuration, version management
- `crates/edgefirst-client/src/client.rs`: Core API implementation patterns
- `release.toml`: cargo-release configuration
