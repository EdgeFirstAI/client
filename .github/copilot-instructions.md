# EdgeFirst Client - AI Agent Instructions

## Project Overview

EdgeFirst Client is a **dual-language Rust+Python** REST API client for EdgeFirst Studio, an MLOps platform for 3D/4D spatial perception AI. The project uses PyO3 to expose Rust core functionality to Python users.

**Architecture**: Monorepo with Cargo workspace containing three crates:
- `crates/edgefirst-client/`: Core Rust library with async HTTP client (reqwest + Tokio)
- `crates/edgefirst-cli/`: CLI application using the core library
- `crates/edgefirst-client-py/`: Python bindings via PyO3/maturin

## AI Agent Workflow Guidelines

### 1. Self-Review Before Committing

**ALWAYS** perform a comprehensive self-review of changes before proposing them, considering:

- **GitHub Actions Workflows** (`.github/workflows/*.yml`):
  - Verify workflow syntax is valid
  - Check that job dependencies (`needs:`) are correct
  - Ensure matrix configurations match actual platforms
  - Validate cache keys and artifact names are consistent
  - Confirm secrets and environment variables are correctly referenced

- **Unit Tests** (Rust + Python):
  - Run `cargo test` to verify Rust tests pass
  - Run Python tests with `python -m unittest discover`
  - Check that new code has corresponding test coverage
  - Verify test data fixtures are still valid

- **Documentation Tests** (Rust):
  - Run `cargo test --doc` to verify doc examples compile and run
  - Ensure code examples in documentation are up-to-date with API changes

- **Python Bindings**:
  - Verify changes to Rust API are reflected in `crates/edgefirst-client-py/src/lib.rs`
  - Update `.pyi` type stub file (`crates/edgefirst-client-py/edgefirst_client.pyi`) with type signatures
  - Test that `maturin develop` builds successfully
  - Validate Python examples in docstrings still work

- **Command-Line Application**:
  - Verify CLI commands still parse correctly
  - Check that help text (`--help`) is accurate
  - Test that example commands from documentation work
  - Ensure error messages are user-friendly

- **Documentation Consistency**:
  - Update `CONTRIBUTING.md` if development workflows change
  - Update `.github/WORKFLOW_ARCHITECTURE.md` if CI/CD changes
  - Update `README.md` if user-facing features change
  - Verify API documentation in doc comments matches implementation
  - Check that version references are consistent across files

**Review Checklist** (mental check before committing):
- [ ] Does this change affect workflows? → Verified workflow files are correct
- [ ] Does this change APIs? → Updated tests, Python bindings, and `.pyi` stubs
- [ ] Does this affect CLI? → Tested command parsing and help text
- [ ] Does this require doc updates? → Verified all relevant docs are updated
- [ ] Does this break existing examples? → Updated examples to match changes

### 2. Environment Setup for Testing

Before running any shell commands that involve testing:

**ALWAYS check that the terminal environment is properly configured:**

```bash
# Check if virtualenv is activated
if [ -z "$VIRTUAL_ENV" ]; then
    echo "⚠️  Virtual environment not activated"
    # ASK USER to activate it
fi

# Check if Studio credentials are set
if [ -z "$STUDIO_SERVER" ] || [ -z "$STUDIO_USERNAME" ] || [ -z "$STUDIO_PASSWORD" ]; then
    echo "⚠️  Studio credentials not configured"
    # ASK USER to set them
fi
```

**Required environment variables for full test suite**:
- `STUDIO_SERVER=test` (or `stage`, `saas`)
- `STUDIO_USERNAME=<your-username>`
- `STUDIO_PASSWORD=<your-password>`

**If environment variables are missing**:
- **DO NOT** attempt to run tests that require Studio authentication
- **ASK the user** to manually set them in the current terminal session:
  ```bash
  export STUDIO_SERVER=test
  export STUDIO_USERNAME=<username>
  export STUDIO_PASSWORD=<password>
  ```
- Explain that these are needed for integration tests that interact with EdgeFirst Studio servers
- Offer to run only unit tests that don't require credentials as an alternative

**Python virtualenv activation**:
- If not activated, **ASK the user** to activate it:
  ```bash
  source venv/bin/activate  # or wherever their venv is located
  ```
- Explain that this ensures Python packages are installed in the project environment
- Note that `maturin develop` needs to install into the active virtualenv

### 3. Temporary Documentation Files

**NEVER commit temporary documentation files** generated during development or to explain large changes.

Examples of temporary files to avoid committing:
- `CHANGES.md`, `UPDATES.md`, `MODIFICATIONS.md`
- `CACHING_IMPROVEMENTS.md`, `WORKFLOW_CHANGES.md`
- Any `.md` files not already tracked in the repository
- Throwaway analysis or planning documents

**Process**:
1. Generate temporary documentation if needed to explain complex changes
2. Share the content with the user for review
3. **ASK the user** if they want to keep the document before staging it
4. If user says no, delete the temporary file
5. Only commit temporary docs if user explicitly requests it

**Rationale**: The user will decide on a case-by-case basis whether temporary documentation adds long-term value or creates clutter. Maintain a clean repository history by default.

### 4. Pre-Commit Housekeeping

**ALWAYS** perform these housekeeping steps before committing changes:

#### Step 1: Verify Documentation is Up-to-Date
```bash
# Check that all relevant documentation reflects your changes:
# - README.md (user-facing features, installation, usage)
# - CONTRIBUTING.md (development workflows, build processes)
# - .github/WORKFLOW_ARCHITECTURE.md (CI/CD changes)
# - API documentation in doc comments (Rust)
# - Python docstrings and .pyi type stubs
# - CLI help text (--help output)
```

**What to check**:
- Does the change affect user-facing features? → Update README.md
- Does the change affect developer workflows? → Update CONTRIBUTING.md
- Does the change affect CI/CD? → Update WORKFLOW_ARCHITECTURE.md
- Does the change affect APIs? → Update doc comments and .pyi stubs
- Does the change affect CLI? → Update help text, examples, and CLI.md

#### CLI Man Page Documentation

If you add, modify, or remove CLI commands, update `CLI.md`:

1. **Add/update command documentation** with syntax, options, arguments, and examples
2. **Optionally rebuild the man page** to verify formatting is correct:
   ```bash
   pandoc CLI.md --standalone --to man --output edgefirst-client.1
   man ./edgefirst-client.1
   ```
3. **Do NOT commit the generated `.1` file** (it's auto-generated and git-ignored)
4. **Man page is built automatically** during GitHub releases

**On release:** Update version and date in `CLI.md` YAML front matter:
```yaml
---
title: EDGEFIRST-CLIENT
section: 1
header: EdgeFirst Client Manual
footer: edgefirst-client X.Y.Z  # <-- Update this version
date: Month YYYY                # <-- Update this date
---
```

#### Step 2: Update CHANGELOG.md

**Always update CHANGELOG.md** with user-visible changes:

```markdown
## [Unreleased]

### Added
- New features or APIs that users can utilize

### Changed
- Modifications to existing behavior that users will notice
- Performance improvements (e.g., "Improved upload speed by 3x")

### Fixed
- Bug fixes that affect user experience

### Removed
- Deprecated or removed features
```

**Guidelines**:
- ✅ **DO document**: New features, API changes, behavior changes, performance improvements, bug fixes, breaking changes
- ❌ **DO NOT document**: Internal refactoring, code cleanup, test updates (unless they enable new test scenarios for users)
- ✅ **User perspective**: Write from the perspective of someone using the library or CLI
- ✅ **Be specific**: Include function names, CLI commands, or specific behaviors changed
- ✅ **Link to issues**: Reference issue numbers if applicable (e.g., "Fixed #123")

**Examples of good changelog entries**:
```markdown
### Added
- `Client::download_with_resume()` method for resumable dataset downloads
- `--parallel` flag to CLI for concurrent uploads (3x faster)

### Changed
- `Dataset::annotations()` now returns `Result<Vec<Annotation>>` instead of `Vec<Annotation>` for better error handling
- Improved multipart upload performance by 40% through connection pooling

### Fixed
- Fixed authentication token refresh failing after 6 days (#234)
- CLI no longer crashes when dataset name contains special characters
```

**Examples of what NOT to document**:
```markdown
### Changed
- Refactored internal error handling (internal detail)
- Updated test fixtures (not user-visible)
- Reorganized module structure (internal detail unless it affects imports)
```

#### Step 3: Format Code with Nightly Rust
```bash
cargo +nightly fmt --all
```

**Why nightly**: Project uses nightly-specific formatting features configured in `rustfmt.toml`

**What this does**:
- Formats all Rust code according to project style
- Ensures consistent formatting across all crates
- Required before commit (CI will fail if not formatted)

#### Step 4: Auto-Fix Clippy Warnings
```bash
cargo clippy --fix --allow-dirty --all-features --all-targets
```

**What this does**:
- Automatically fixes lints that have safe automatic fixes
- `--allow-dirty`: Allows fixing uncommitted changes
- `--all-features`: Checks code with all feature flags enabled (including optional `polars`)
- `--all-targets`: Checks lib, bins, tests, examples, benches

**Important**: Review the changes made by `--fix` to ensure they're correct

#### Step 5: Run Full Test Suite

**Prerequisite**: Verify environment is properly configured (see "Environment Setup for Testing" above)

```bash
# 1. Run Rust unit tests with coverage
cargo test --all-features --locked

# 2. Run Rust documentation tests
cargo test --doc --locked

# 3. Build and test Python bindings
maturin develop -m crates/edgefirst-client-py/Cargo.toml
python -m unittest discover -s . -p "test*.py"
```

**If any tests fail**:
- ❌ **DO NOT** commit the changes
- Fix the failing tests first
- Re-run the full test suite
- Only commit when all tests pass

**If environment variables are not set**:
- **ASK the user** to set them (see "Environment Setup for Testing")
- Alternatively, rely on CI to run integration tests (only if running local unit tests that don't require credentials)

#### Step 6: Review SonarCloud Findings (Optional)

**Download Latest Code Quality Findings:**

To assist with identifying and fixing code quality issues, you can download the latest SonarCloud findings using the improved `sonar.py` script:

1. **Set SonarCloud token** (requires user to create token):
   - User must create token at: [SonarCloud Security Settings](https://sonarcloud.io/)
   - User sets environment variables:
     ```bash
     export SONAR_TOKEN=their_token_here
     export SONAR_ORG=edgefirstai
     export SONAR_PROJECT=EdgeFirstAI_client
     ```

2. **Download findings (optimized for Copilot)**:
   ```bash
   python3 sonar.py --branch main --output sonar-issues.json --verbose
   ```

3. **Use with GitHub Copilot**:
   - The `sonar-issues.json` file contains structured issue data optimized for Copilot
   - Ask: `@workspace Review sonar-issues.json and help me fix the top critical issues`
   - Copilot can read file paths, line numbers, rule descriptions, and suggest fixes
   - The file is gitignored and should not be committed

4. **Advanced filtering**:
   ```bash
   # Only critical/blocker issues
   python3 sonar.py --branch main --severity BLOCKER,CRITICAL -o critical-issues.json
   
   # Only bugs and vulnerabilities
   python3 sonar.py --branch main --type BUG,VULNERABILITY -o security-issues.json
   ```

**Use Cases:**
- Understanding SonarCloud issues reported in CI
- Prioritizing code quality improvements with Copilot assistance
- Systematically addressing technical debt
- Planning refactoring work with AI-assisted fixes

**See Also:** [SONAR_INTEGRATION.md](../SONAR_INTEGRATION.md) for complete documentation.

**Note**: This is optional - CI/CD automatically runs SonarCloud analysis on all PRs.

#### Pre-Commit Checklist Summary

Before committing, verify:
- [ ] All relevant documentation updated
- [ ] CHANGELOG.md updated with user-visible changes
- [ ] Code formatted with `cargo +nightly fmt --all`
- [ ] Clippy warnings fixed with `cargo clippy --fix --allow-dirty --all-features --all-targets`
- [ ] All Rust tests pass: `cargo test --all-features --locked`
- [ ] All Rust doc tests pass: `cargo test --doc --locked`
- [ ] Python bindings build: `maturin develop`
- [ ] All Python tests pass: `python -m unittest discover`
- [ ] No temporary documentation files included (unless user explicitly requested)

### 5. Commit Message Format

**ALWAYS use this concise format for commit messages:**

```
<Short descriptive title>

- Bullet point 1: what changed
- Bullet point 2: what changed
- Bullet point 3: what changed
[- Issue reference if provided by user: "Fixes #123" or "Closes PROJ-456"]
```

**Guidelines**:
- ✅ **Keep it concise**: Focus on **what** changed, not detailed **why** or **how**
- ✅ **Use bullet points**: One per major change area
- ✅ **Avoid implementation details**: Those belong in documentation, code comments, or issue trackers
- ✅ **Wait for user direction**: Only add issue references (GitHub #, JIRA tickets) if user explicitly provides them
- ✅ **Make it scannable**: Each line should be clear and actionable

**Example**:
```
Optimize CI/CD workflows and add AI agent guidelines

- Replace manual caching with Swatinem/rust-cache for faster builds
- Merge lint/test jobs and Python wheels into unified workflows
- Remove deps.yml and python.yml (consolidated into build.yml)
- Update workflow documentation to reflect current structure
- Add comprehensive AI agent development guidelines to copilot-instructions.md
- Fix clippy warning in CLI tests
```

**NOT this** (too detailed):
```
Optimize CI/CD workflows and add AI agent guidelines

## Workflow Optimizations
### Caching Improvements
- Replace manual actions/cache with Swatinem/rust-cache for intelligent 
  incremental compilation because it provides better support for...
[hundreds of lines of implementation details and reasoning]
```

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
- See `.github/workflows/build.yml` for platform-specific configurations (includes both CLI and Python wheels)

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

### Choosing the Version Number

When preparing a release, select the appropriate version bump based on changes since the last release:

**PATCH (X.Y.Z → X.Y.Z+1)** - Default for most releases
- Bug fixes that don't change the API
- Performance improvements
- Internal refactoring (no API changes)
- New features that don't change existing APIs (backward-compatible additions)
- Documentation updates
- Examples: `2.1.0 → 2.1.1`, `2.1.1 → 2.1.2`
- **Use**: `cargo release patch`

**MINOR (X.Y.Z → X.Y+1.0)** - Required for breaking changes
- API changes that break backward compatibility
- Removing public functions, methods, or types
- Changing function signatures (parameters, return types)
- Renaming public APIs
- Changing behavior in ways that existing code depends on
- Examples: `2.1.5 → 2.2.0`, `2.2.0 → 2.3.0`
- **Use**: `cargo release minor`

**MAJOR (X.Y.Z → X+1.0.0)** - Reserved for maintainers
- Major architectural changes
- Complete API rewrites
- Reserved for maintainer decision only
- Examples: `2.9.5 → 3.0.0`, `1.5.2 → 2.0.0`
- **Use**: `cargo release major`

**CHANGELOG Requirements:**
- **PATCH releases**: Document new features or bug fixes in CHANGELOG under `### Added`, `### Fixed`, or `### Changed`
- **MINOR releases**: Document breaking changes in CHANGELOG under `### Changed` with clear migration guidance
- **MAJOR releases**: Provide comprehensive migration guide

**Default**: When in doubt, use **PATCH** for backward-compatible changes and **MINOR** for breaking changes.

**Release Process** (maintainers only):
```bash
# 1. Update CHANGELOG.md with release notes under [Unreleased]

# 2. Update CLI.md version and date in YAML front matter
#    footer: edgefirst-client X.Y.Z
#    date: Month YYYY

# 3. Stable release (choose: patch, minor, or major)
cargo release patch --execute --no-confirm  # or: minor, major

# 4. Release candidate (MANUAL version edit required)
sed -i '' 's/version = "2.2.2"/version = "2.3.0rc1"/' Cargo.toml
sed -i '' 's/edgefirst-client = { version = "2.2.2"/edgefirst-client = { version = "2.3.0rc1"/' Cargo.toml
cargo release 2.3.0rc1 --execute --no-confirm

# 5. Push to trigger CI/CD
git push && git push --tags
```

See `CONTRIBUTING.md` for full release details and `release.toml` for configuration.

**Note**: The man page is automatically generated and included as a release artifact in GitHub releases.

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
- `build.yml`: Cross-platform CLI binaries + Python wheels (Linux/macOS/Windows, x64/arm64) with serial execution
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
