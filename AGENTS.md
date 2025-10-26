# AGENTS.md

EdgeFirst Client - Dual-language Rust+Python REST API client for EdgeFirst Studio MLOps platform.

## Project Overview

**Architecture**: Cargo workspace monorepo with three tightly-coupled crates:
- `crates/edgefirst-client/`: Core Rust library (async reqwest + Tokio)
- `crates/edgefirst-cli/`: CLI application
- `crates/edgefirst-client-py/`: Python bindings via PyO3/maturin

**Key Technologies**: Rust (nightly for formatting), Python 3.8+, PyO3, Tokio, reqwest with rustls-tls

## Development Environment

### Required Tools
- Rust 1.90+ (nightly toolchain for formatting)
- Python 3.8+
- `cargo-nextest`, `cargo-llvm-cov`, `maturin`, `ruff`

### Environment Setup for Testing

Before running tests, verify environment variables are set:
```bash
# Check for required credentials
if [ -z "$STUDIO_SERVER" ] || [ -z "$STUDIO_USERNAME" ] || [ -z "$STUDIO_PASSWORD" ]; then
    # Look for env.sh and source it
    if [ -f ./env.sh ]; then
        source ./env.sh
    else
        echo "Missing required environment: STUDIO_SERVER, STUDIO_USERNAME, STUDIO_PASSWORD"
        exit 1
    fi
fi
```

**Required environment variables**:
- `STUDIO_SERVER=test` (or `stage`, `saas`)
- `STUDIO_USERNAME=<username>`
- `STUDIO_PASSWORD=<password>`

**Python virtualenv**: Ensure virtualenv (venv/.venv) is activated before running maturin or Python tests.

## Build & Test Commands

```bash
# Build all crates
cargo build --all-features --locked

# Run Rust tests (requires credentials)
cargo test --all-features --locked
cargo test --doc --locked

# Build Python bindings
maturin develop -m crates/edgefirst-client-py/Cargo.toml

# Run Python tests (RECOMMENDED: use slipcover to match CI/CD behavior)
python3 -m slipcover --xml --out coverage.xml -m xmlrunner discover -s . -p "test*.py" -o target/python

# Alternative: Run Python tests without coverage (for quick checks)
python -m unittest discover -s . -p "test*.py"

# Format code (nightly required)
cargo +nightly fmt --all

# Lint and auto-fix
cargo clippy --fix --allow-dirty --all-features --all-targets

# Sort dependencies
cargo sort --workspace

# Check for unused dependencies (may have false positives - verify before removing)
cargo shear

# Python formatting (PEP-8 standard: 79-character lines)
autopep8 --in-place --aggressive --aggressive *.py examples/*.py crates/edgefirst-client-py/edgefirst_client.pyi
```

## Code Quality Standards

### Rust
- Follow standard Rust conventions (rustfmt.toml configures project-specific settings)
- All public APIs require doc comments with runnable examples
- Doc tests must pass: `cargo test --doc`
- Use `#[cfg(feature = "polars")]` for Polars-dependent code
- Imports grouped by crate: `imports_granularity = 'Crate'` (rustfmt.toml)
- Async-first design: All API calls are `async fn` using Tokio

### Python
- Follow PEP-8 strictly (79-character line limit)
- Use autopep8 for automatic formatting and compliance fixes
- Mirror Rust examples in Python docstrings
- Maintain `.pyi` type stubs for IDE support in `crates/edgefirst-client-py/edgefirst_client.pyi`

### Testing
- **Target**: SonarCloud clean, full test coverage
- Unit tests: In same file as implementation (Rust) or test files (Python)
- Integration tests: `crates/edgefirst-cli/tests/` and `crates/edgefirst-client/src/lib.rs`
- Test data: Uses EdgeFirst Studio test server with "Unit Testing" project
- **Coverage**: Use `cargo llvm-cov` with `--doctests` flag
- **Python testing**: Use `slipcover` (recommended) to match CI/CD behavior - catches syntax errors that standard unittest may miss
- **Optional**: Run `python3 sonar.py --branch main -o sonar-issues.json` for local SonarCloud analysis

### Test Execution Strategy
- **Default: Run with coverage instrumentation** - Low overhead (~10%), provides robust results and coverage visibility
- **Prefer slipcover for Python tests** - matches CI/CD exactly, strict syntax validation
- Run tests when credentials are available (especially if `env.sh` exists)
- If credentials unavailable, rely on CI/CD to run tests (PRs blocked on test failures)
- CI/CD workflows have stored secrets for full test suite
- **VS Code users**: Install Coverage Gutters extension to see coverage inline while coding

### Full Coverage Instrumentation (Recommended for Local Development)
Standard workflow matching CI/CD (~10% performance overhead):
```bash
source <(cargo llvm-cov show-env --export-prefix --no-cfg-coverage --doctests)
cargo build --all-features --locked
maturin develop -m crates/edgefirst-client-py/Cargo.toml
python3 -m slipcover --xml --out coverage.xml -m xmlrunner discover -s . -p "test*.py" -o target/python
cargo llvm-cov report --doctests --lcov --output-path lcov.info
```
**Performance**: Adds ~4-5 seconds to test suite. Benefits: catch coverage gaps early, see what code paths need tests.

## Pre-Commit Requirements

**MUST complete before committing** (in order):

1. **Update documentation** if changes affect:
   - `README.md`: User-facing features, installation, usage
   - `CLI.md`: CLI commands, options, examples (update version/date on release)
   - `CONTRIBUTING.md`: Development workflows, build processes
   - `.github/WORKFLOW_ARCHITECTURE.md`: CI/CD changes (must align with `.github/workflows/*.yml`)
   - API doc comments: Rust documentation with examples
   - `.pyi` stubs: Python type hints in `crates/edgefirst-client-py/edgefirst_client.pyi`

2. **Update CHANGELOG.md** for user-visible changes only:
   - ✅ Document: New features, API changes, behavior changes, bug fixes, breaking changes
   - ❌ Skip: Internal refactoring, test updates, code cleanup
   - Format: Under `## [Unreleased]` use `### Added`, `### Changed`, `### Fixed`, `### Removed`

3. **Run code quality checks**:
   ```bash
   cargo shear                                                    # Check unused deps (verify before removing)
   cargo sort --workspace                                         # Sort dependencies
   cargo +nightly fmt --all                                       # Format Rust code
   cargo clippy --fix --allow-dirty --all-features --all-targets # Fix lints
   autopep8 --in-place --aggressive --aggressive *.py examples/*.py crates/edgefirst-client-py/edgefirst_client.pyi # Format Python (PEP-8)
   ```

4. **Run tests** (if credentials available):
   ```bash
   cargo test --all-features --locked                             # Rust tests
   cargo test --doc --locked                                      # Doc tests
   maturin develop -m crates/edgefirst-client-py/Cargo.toml      # Build Python bindings
   python3 -m slipcover --xml --out coverage.xml -m xmlrunner discover -s . -p "test*.py" -o target/python # Python tests (recommended - matches CI/CD)
   ```

5. **Verify no temporary .md files** are staged (e.g., `CHANGES.md`, `UPDATES.md`):
   - Temporary documentation for explaining changes is okay during development
   - **MUST ask user before committing** any new .md file not already tracked in git
   - User decides if temporary docs add long-term value or create clutter

6. **Audit workflow documentation** if applicable:
   - Quick check: Do changes affect `.github/workflows/*.yml`?
   - If yes: Verify `.github/WORKFLOW_ARCHITECTURE.md` accurately describes workflow structure
   - Full audit only when workflow files are modified

## Commit Message Format

**Required format**:
```
<Short descriptive header>

- Bullet 1: what changed
- Bullet 2: what changed
- Bullet 3: what changed
[- Fixes #123 (only if user provides issue reference)]
```

**Guidelines**:
- Keep it succinct: Focus on **what** changed, not detailed **why** or **how**
- One bullet per major change area
- Avoid implementation details (those belong in docs/comments/issues)
- Scannable and actionable

## Versioning & Release

**Version format**: `X.Y.Z` for stable, `X.Y.ZrcN` for release candidates (NO separators like `-rc.1`)
- **Why**: PyPI requires `rcN` format (PEP 440), maturin doesn't convert
- **Workspace versioning**: Single version in root `Cargo.toml` via `version.workspace = true`

**Semantic versioning**:
- **PATCH** (X.Y.Z+1): Bug fixes, performance improvements, backward-compatible additions (default)
- **MINOR** (X.Y+1.0): Breaking API changes
- **MAJOR** (X+1.0.0): Major architectural changes (maintainer decision only)

**Release process** (maintainers):
```bash
# 1. Update CHANGELOG.md with release notes under [Unreleased]
# 2. Update CLI.md version and date in YAML front matter
# 3. Release
cargo release patch --execute --no-confirm  # or: minor, major
# 4. Push tags to trigger CI/CD
git push && git push --tags
```

See `CONTRIBUTING.md` and `release.toml` for full release details.

## Code Organization

### Core Library (`crates/edgefirst-client/src/`)
- `client.rs`: Main `Client` struct, JSON-RPC methods, token management, multipart uploads
- `api.rs`: Type definitions (Project, Dataset, TrainingSession, etc.)
- `dataset.rs`: Dataset operations, downloads, annotation parsing
- `error.rs`: Custom error enum with manual `From` trait implementations
- `lib.rs`: Public API surface, feature flags

### CLI (`crates/edgefirst-cli/`)
- Commands documented in `CLI.md`
- Help text must match documentation
- Man page auto-generated during releases (do not commit `.1` file)

### Python Bindings (`crates/edgefirst-client-py/src/`)
- Single `lib.rs` with PyO3 wrappers
- `tokio-wrap` bridges async Rust → sync Python
- Parallel type system with `From`/`TryFrom` conversions
- Export `.pyi` type stubs for IDE support

## Dependencies & Features

**Workspace dependencies**: All deps defined in root `Cargo.toml` `[workspace.dependencies]`
- TLS: `reqwest` with `rustls-tls` only (no native-tls)
- Async: `tokio` with `full` and `rt-multi-thread` features
- Optional: `polars` feature for DataFrame support (enabled by default)

**Feature flags**:
- Default: `default = ["polars"]`
- Use `#[cfg(feature = "polars")]` for conditional compilation

## Key Design Patterns

### Authentication
- JWT token cached in OS-specific config directory (7-day expiry):
  - Linux: `~/.config/EdgeFirst Studio/token`
  - macOS: `~/Library/Application Support/ai.EdgeFirst.EdgeFirst Studio/token`
  - Windows: `%APPDATA%\EdgeFirst\EdgeFirst Studio\config\token`
- Auto-renewal via `verify_token()` → `renew_token()` flow
- Override with `STUDIO_TOKEN` environment variable

### JSON-RPC Pattern
```rust
let request = RpcRequest {
    id: 0,
    jsonrpc: "2.0".to_string(),
    method: "method_name".to_string(),
    params: Some(params_struct),
};
let response: RpcResponse<ResultType> = self.rpc(request).await?;
```

### Progress Tracking
```rust
let (tx, mut rx) = mpsc::channel(1);
tokio::spawn(async move {
    while let Some(progress) = rx.recv().await {
        println!("{}/{}", progress.current, progress.total);
    }
});
client.download_dataset(id, &["image"], path, Some(tx)).await?;
```

### Async Design
- All API calls are `async fn` using Tokio runtime
- Concurrency limiting: Semaphore with `MAX_TASKS = 32` prevents resource exhaustion
- Multipart upload: Files chunked at `PART_SIZE = 100MB` with pre-signed S3 URLs (`MAX_RETRIES = 10`)

## Common Pitfalls

1. **Async boundaries**: Python bindings use `tokio-wrap` - don't use bare `tokio::runtime::Handle::block_on`
2. **Version format**: Never use `-rc.1`, always use `rc1` (no separators)
3. **Feature gates**: Remember `#[cfg(feature = "polars")]` when using Polars types
4. **Test credentials**: Integration tests need credentials - missing vars causes test failures (rely on CI if unavailable)
5. **Import formatting**: Use `imports_granularity = 'Crate'` (rustfmt.toml) - imports grouped by crate
6. **Nightly Rust**: Formatting requires nightly (`cargo +nightly fmt`), configured in `rustfmt.toml`

## Key Reference Files

- `CONTRIBUTING.md`: Development setup, test infrastructure, full release process
- `.github/WORKFLOW_ARCHITECTURE.md`: Detailed CI/CD documentation
- `Cargo.toml` (root): Workspace configuration, version management
- `release.toml`: cargo-release configuration
- `CLI.md`: Complete CLI command documentation
- `CHANGELOG.md`: User-visible changes (update on commit)
