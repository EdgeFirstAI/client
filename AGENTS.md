# AGENTS.md - AI Assistant Development Guidelines

This document provides instructions for AI coding assistants (GitHub Copilot, Cursor, Claude Code, etc.) working on the EdgeFirst Client project. These guidelines ensure consistent code quality, proper workflow adherence, and maintainable contributions.

**Version:** 2.0
**Last Updated:** November 2025
**Project:** EdgeFirst Client - Client API and CLI for EdgeFirst Studio

---

## Table of Contents

1. [Overview](#overview)
2. [⚠️ Critical Rules](#️-critical-rules)
3. [Git Workflow](#git-workflow)
4. [Code Quality Standards](#code-quality-standards)
5. [Testing Requirements](#testing-requirements)
6. [Documentation Expectations](#documentation-expectations)
7. [License Policy](#license-policy)
8. [Security Practices](#security-practices)
9. [Project-Specific Guidelines](#project-specific-guidelines)

---

## Overview

**EdgeFirst Client** is the official Client API and CLI for [EdgeFirst Studio](https://edgefirst.studio), the MLOps platform for 3D visual and 4D spatial perception AI. This dual-language (Rust + Python) library enables programmatic access to EdgeFirst Studio's capabilities for dataset management, model training, validation, and deployment.

### Project Context

EdgeFirst Client serves as the **bridge between developers and EdgeFirst Studio**, providing:

- Direct integration with EdgeFirst Studio's REST API
- Automation for CI/CD pipelines and custom workflows
- Production-grade reliability (used internally by EdgeFirst Studio's training and validation services)
- Cross-platform support (Rust library + Python bindings + CLI)

When contributing to EdgeFirst Client, AI assistants should prioritize:

- **EdgeFirst Studio integration**: Maintain seamless compatibility with the platform
- **Code quality**: Maintainability, readability, and adherence to established patterns
- **Testing**: Comprehensive coverage with unit, integration, and Studio integration tests
- **Documentation**: Clear explanations for APIs and workflows
- **License compliance**: Strict adherence to Apache-2.0 and approved dependencies

---

## ⚠️ Critical Rules

### #1: NEVER Use cd Commands

```bash
# ✅ Modern tools work from root
cargo build --release
venv/bin/pytest tests/

# ❌ AI loses context
cd build && cmake ..  # Where are we now?
```

### #1.5: NEVER Hide Command Output

```bash
# ✅ User sees full output
cargo test --all-features --locked

# ✅ If you must save logs, use tee (preserves live output)
cargo test --all-features --locked 2>&1 | tee test.log

# ❌ User can't see what's happening
cargo test --all-features --locked | tail -20
cargo build 2>&1 | head -50
```

**Why:** Users need to see full output for better experience and debugging. Hiding output with `head`/`tail` prevents users from understanding what's happening.

### #2: ALWAYS Use Python venv

```bash
# ✅ Direct invocation (no activation needed)
venv/bin/python script.py
venv/bin/pytest tests/

# ❌ System Python pollution
python script.py  # Which Python?
```

**requirements.txt - Semver ranges:**

```txt
# ✅ Allow patches/minors, block breaking changes
numpy>=1.21.0,<2.0.0

# ❌ Exact pins block security patches
numpy==1.21.0
```

### #3: DCO Sign-Off Required

All commits must include Developer Certificate of Origin sign-off:

```bash
git commit -s -m "Brief description"
```

This adds `Signed-off-by: Your Name <your.email@example.com>` to the commit message.

---

## Git Workflow

### Branch Naming Convention

**For External Contributors (Recommended):**

```bash
feature/description-of-feature
bugfix/description-of-bug
hotfix/critical-issue-description
```

**For Internal Au-Zone Developers:**
Use JIRA-integrated format: `<type>/<PROJECTKEY-###>[-optional-description]`

**Branch Types:**

- `feature/` - New features and enhancements
- `bugfix/` - Non-critical bug fixes
- `hotfix/` - Critical issues requiring immediate fix

**Examples:**

```bash
# External contributors (GitHub issue-based)
feature/add-batch-upload
bugfix/fix-token-refresh
hotfix/security-patch

# Internal Au-Zone (JIRA-based)
feature/STUDIO-123-add-authentication
bugfix/STUDIO-456-fix-memory-leak
```

**Rules:**

- Branch from `main` for all work
- Use kebab-case for descriptions (lowercase with hyphens)
- Keep descriptions concise but meaningful

### Commit Message Format

**Required format**:

```
Short descriptive header

- Bullet 1: what changed
- Bullet 2: what changed
- Bullet 3: what changed

Signed-off-by: Your Name <your.email@example.com>
[Fixes #123 (only if user provides issue reference)]
```

**Guidelines**:

- Keep it succinct: Focus on **what** changed, not detailed **why** or **how**
- One bullet per major change area
- Avoid implementation details (those belong in docs/comments/issues)
- Scannable and actionable
- **MUST include DCO sign-off** (use `git commit -s`)

**Examples of Good Commits:**

```bash
Add JWT authentication to user API

- Implemented token validation middleware
- Added login/logout endpoints
- Updated documentation with auth examples

Fix memory leak in dataset download

- Released buffers properly after multipart completion
- Added test for large file downloads
- Verified with valgrind and cargo-leak

Optimize S3 upload performance

- Increased concurrent uploads to 32
- Implemented retry logic with exponential backoff
- Reduced memory footprint by 40%
```

### Pull Request Process

**Requirements:**

- All CI/CD checks must pass
- PR title: Brief description of changes (no specific format required for external contributors)
- PR description should link to relevant issues

**PR Description Template:**

```markdown
## Related Issues
Fixes #123
Related to #456

## Changes
Brief summary of what changed and why

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests pass
- [ ] Manual testing completed
- [ ] Documentation updated

## Checklist
- [ ] Code follows project conventions
- [ ] No secrets or credentials committed
- [ ] CHANGELOG.md updated (if user-facing changes)
- [ ] LICENSE policy compliance verified
```

**Process:**

1. Create PR via GitHub web interface
2. Link to related issues in description
3. Wait for CI/CD to complete successfully
4. Address reviewer feedback through additional commits
5. Maintainer will merge once approved

---

## Code Quality Standards

### General Principles

- **Consistency**: Follow existing codebase patterns and conventions
- **Readability**: Code is read more often than written - optimize for comprehension
- **Simplicity**: Prefer simple, straightforward solutions over clever ones
- **Error Handling**: Validate inputs, provide actionable error messages
- **Performance**: Consider async patterns and memory efficiency

### Language-Specific Standards

**Rust:**

- Use `cargo +nightly fmt` for formatting (project uses nightly for formatting features)
- Run `cargo clippy --all-features --all-targets` and fix all warnings
- Follow Rust API guidelines
- Add doc comments for public APIs with runnable examples
- Imports grouped by crate: `imports_granularity = 'Crate'` (rustfmt.toml)
- Async-first design: All API calls are `async fn` using Tokio

**Python:**

- Follow PEP 8 strictly (79-character line limit)
- Use `ruff format` for formatting and `ruff check --fix` for linting
- Maintain `.pyi` type stubs in `crates/edgefirst-client-py/edgefirst_client.pyi`
- **Pylance type checking**: Code must be Pylance-clean (VS Code's Python language server)
  - All `.pyi` stubs must have complete type annotations
  - Use type narrowing patterns: `self.assertIsNotNone(x)` → `assert x is not None`
  - Prefer specific assertions: `assertGreater(len(x), 0)`

### Code Quality Tools

Before submitting code, verify:

- [ ] Code follows project style guidelines (check `.editorconfig`, `rustfmt.toml`)
- [ ] No commented-out code or debug statements
- [ ] Error handling is comprehensive with useful messages
- [ ] Complex logic has explanatory comments
- [ ] Public APIs have documentation
- [ ] No hardcoded values that should be configuration
- [ ] Resource cleanup (memory, file handles, connections) is proper
- [ ] No obvious security vulnerabilities

---

## Testing Requirements

### Coverage Standards

- **Minimum coverage**: 80% (enforced by CI/CD)
- **Critical paths**: 90%+ coverage for core functionality
- **Edge cases**: Explicit tests for boundary conditions
- **Error paths**: Validate error handling and recovery

### Test Types

**Unit Tests (Rust):**

- Test individual functions/methods in isolation
- Co-located in `#[cfg(test)] mod tests` at end of implementation files
- Mock external dependencies
- Fast execution

**Integration Tests (Rust):**

- Separate `tests/` directory at crate root
- Test API workflows end-to-end
- Use real EdgeFirst Studio test servers (requires credentials)

**Python Tests:**

- Framework: Python `unittest`
- Test files: `test*.py` in repository root
- **Recommended**: Use `slipcover` to match CI/CD behavior
- Fixtures: Standard unittest patterns

**Studio Integration Tests:**

- Require authenticated access to EdgeFirst Studio test servers
- Test data:
  - Test server: `test.edgefirst.studio`
  - Test user: `testing`
  - Test project: `Unit Testing`
  - Static dataset: `Deer`
- Environment variables:
  - `STUDIO_SERVER=test` (or `stage`, `saas`)
  - `STUDIO_USERNAME=<username>`
  - `STUDIO_PASSWORD=<password>`

### Running Tests

```bash
# Rust tests - IMPORTANT: Run lib tests separately or use single thread to avoid conflicts
cargo test -p edgefirst-client --lib --all-features --locked
cargo test -p edgefirst-cli --all-features --locked
# OR run with single thread to avoid timeouts:
cargo test --all-features --locked -- --test-threads=1

# Doc tests
cargo test --doc --locked

# Python tests (recommended: use slipcover)
maturin develop -m crates/edgefirst-client-py/Cargo.toml
python3 -m slipcover --xml --out coverage.xml -m xmlrunner discover -s . -p "test*.py" -o target/python

# Comprehensive coverage (Rust + Python combined)
source <(cargo llvm-cov show-env --export-prefix --no-cfg-coverage)
cargo build --all-features --locked
maturin develop -m crates/edgefirst-client-py/Cargo.toml
python3 -m slipcover --xml --out coverage.xml -m xmlrunner discover -s . -p "test*.py" -o target/python
cargo llvm-cov report --lcov --output-path lcov.info
```

**CRITICAL Testing Rules:**

- **NEVER use `tail` when running commands** - Users need to see full output for better experience. Use `| tee logfile.txt` if logs need to be captured.
- **Run lib and CLI tests separately** - Running all tests together causes conflicts and timeouts. Use `-p edgefirst-client --lib` and `-p edgefirst-cli` separately, or `-- --test-threads=1`.

**Note**: Integration tests require EdgeFirst Studio credentials. External contributors can rely on CI/CD to run these tests automatically via GitHub Actions.

---

## Documentation Expectations

### Code Documentation

**When to document:**

- Public APIs, functions, and classes (ALWAYS)
- Complex algorithms or non-obvious logic
- EdgeFirst Studio integration patterns
- Error conditions and edge cases

**Rust documentation style:**

```rust
/// Downloads a dataset from EdgeFirst Studio with optional progress tracking.
///
/// # Arguments
///
/// * `dataset_id` - The unique identifier of the dataset
/// * `types` - Data types to download (e.g., ["image", "lidar"])
/// * `output_path` - Local directory to save downloaded files
/// * `progress_tx` - Optional channel for progress updates
///
/// # Returns
///
/// Returns `Ok(())` on success or an error if download fails.
///
/// # Example
///
/// ```rust
/// # use edgefirst_client::{Client, DatasetID};
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = Client::new()?.with_token_path(None)?;
/// let dataset_id = DatasetID::from("ds-123");
/// client.download_dataset(dataset_id, &["image"], "./data", None).await?;
/// # Ok(())
/// # }
/// ```
pub async fn download_dataset(&self, dataset_id: DatasetID, ...) -> Result<()> {
    // implementation
}
```

**Python documentation style:**

```python
def download_dataset(self, dataset_id: str, types: List[str], output_path: str) -> None:
    """
    Downloads a dataset from EdgeFirst Studio.

    Args:
        dataset_id: The unique identifier of the dataset (e.g., "ds-123")
        types: Data types to download (e.g., ["image", "lidar"])
        output_path: Local directory to save downloaded files

    Raises:
        ValueError: If dataset_id is invalid
        RuntimeError: If download fails

    Example:
        >>> client = Client().with_token_path(None)
        >>> client.download_dataset("ds-123", ["image"], "./data")
    """
```

### Project Documentation

**Essential files:**

- `README.md` - Project overview, quick start, EdgeFirst Studio integration
- `CONTRIBUTING.md` - Development setup, contribution process
- `CODE_OF_CONDUCT.md` - Community standards
- `SECURITY.md` - Vulnerability reporting process
- `LICENSE` - Apache-2.0 license text
- `AGENTS.md` - AI assistant guidelines (this file)
- `CLI.md` - Comprehensive CLI command reference

### Documentation Updates

When modifying code, update corresponding documentation:

- README.md if user-facing behavior changes
- CLI.md if CLI commands/options change (update version/date on release)
- API docs if function signatures or semantics change
- CHANGELOG.md for all user-visible changes
- `.pyi` stubs for Python type hints

---

## License Policy

**CRITICAL**: EdgeFirst Client is licensed under Apache-2.0 and has strict dependency requirements.

### Allowed Licenses

✅ **Permissive licenses (APPROVED)**:

- MIT
- Apache-2.0
- BSD-2-Clause, BSD-3-Clause
- ISC
- 0BSD
- Unlicense

### Review Required

⚠️ **Weak copyleft (REQUIRES LEGAL REVIEW)**:

- MPL-2.0 (Mozilla Public License)
- LGPL-2.1-or-later, LGPL-3.0-or-later (if dynamically linked)

### Strictly Disallowed

❌ **NEVER USE THESE LICENSES**:

- GPL (any version)
- AGPL (any version)
- Creative Commons with NC (Non-Commercial) or ND (No Derivatives)
- SSPL (Server Side Public License)
- BSL (Business Source License, before conversion)

### Verification Process

**Before adding dependencies:**

1. Check license compatibility with Apache-2.0
2. Verify no GPL/AGPL in dependency tree
3. Document third-party licenses appropriately

**CI/CD will automatically:**

- Validate license compatibility
- Block PR merges if violations detected

---

## Security Practices

### Secure Coding Guidelines

**Credential Handling:**

- Never hardcode credentials or API keys
- Use environment variables or secure token storage
- Session tokens stored in OS-specific config directories
- Never log credentials or tokens

**Network Security:**

- All communications over HTTPS/TLS (enforced)
- Uses `rustls-tls` backend for TLS
- Connects only to `*.edgefirst.studio` domains

**Data Protection:**

- Encrypt sensitive data in transit (HTTPS)
- Session tokens are time-limited
- Proper error handling without exposing sensitive details

### Vulnerability Reporting

For security issues:

- **GitHub Security Advisories**: [Report a vulnerability](https://github.com/EdgeFirstAI/client/security/advisories)
- **Email**: support@au-zone.com with subject "[SECURITY] EdgeFirst Client"
- **Do not**: Open public GitHub issues for security vulnerabilities

---

## Project-Specific Guidelines

### Technology Stack

- **Languages**: Rust 1.90+ (nightly for formatting), Python 3.8+
- **Architecture**: Cargo workspace monorepo with 3 crates:
  - `crates/edgefirst-client/`: Core Rust library
  - `crates/edgefirst-cli/`: CLI application
  - `crates/edgefirst-client-py/`: Python bindings via PyO3
- **Key dependencies**: Tokio (async runtime), reqwest (HTTP with rustls-tls), serde (JSON), PyO3 (Python bindings)
- **TLS**: reqwest with `rustls-tls` only (no native-tls)
- **Target platforms**: Linux, macOS, Windows (x86_64, ARM64)

### Architecture Patterns

**Authentication:**

- JWT token cached in OS-specific config directory (7-day expiry)
- Auto-renewal via `verify_token()` → `renew_token()` flow
- Override with `STUDIO_TOKEN` environment variable

**JSON-RPC Pattern:**

```rust
let request = RpcRequest {
    id: 0,
    jsonrpc: "2.0".to_string(),
    method: "method_name".to_string(),
    params: Some(params_struct),
};
let response: RpcResponse<ResultType> = self.rpc(request).await?;
```

**Progress Tracking:**

```rust
let (tx, mut rx) = mpsc::channel(1);
tokio::spawn(async move {
    while let Some(progress) = rx.recv().await {
        println!("{}/{}", progress.current, progress.total);
    }
});
client.download_dataset(id, &["image"], path, Some(tx)).await?;
```

**Async Design:**

- All API calls are `async fn` using Tokio runtime
- Concurrency limiting: Semaphore with `MAX_TASKS = 32`
- Multipart upload: Files chunked at `PART_SIZE = 100MB` with pre-signed S3 URLs

### Build and Deployment

```bash
# Build all crates
cargo build --all-features --locked

# Build CLI binary
cargo build --release -p edgefirst-cli

# Build Python bindings
maturin develop -m crates/edgefirst-client-py/Cargo.toml

# Build Python wheel
maturin build --release -m crates/edgefirst-client-py/Cargo.toml

# Format code (nightly required)
cargo +nightly fmt --all

# Lint and auto-fix
cargo clippy --fix --allow-dirty --all-features --all-targets

# Python formatting
ruff format *.py examples/*.py crates/edgefirst-client-py/edgefirst_client.pyi
ruff check --fix *.py examples/*.py crates/edgefirst-client-py/edgefirst_client.pyi
```

### Testing Conventions

**Rust:**

- Unit tests: Co-located in `#[cfg(test)] mod tests` at end of implementation files
- Integration tests: Separate `tests/` directory at project root
- Test naming: `test_<function>_<scenario>` format
- Run with: `cargo test --all-features --locked`

**Python:**

- Framework: Python `unittest`
- Test files: `test*.py` in repository root
- Run with slipcover (recommended): `python3 -m slipcover --xml --out coverage.xml -m xmlrunner discover -s . -p "test*.py" -o target/python`
- Alternative: `python -m unittest discover -s . -p "test*.py"`

### Versioning & Release

**Version format**: `X.Y.Z` for stable, `X.Y.ZrcN` for release candidates (NO separators like `-rc.1`)

- **Why**: PyPI requires `rcN` format (PEP 440), maturin doesn't convert
- **Workspace versioning**: Single version in root `Cargo.toml` via `version.workspace = true`
- **Git tags**: Use `vX.Y.Z` format (e.g., `v2.5.0`)

**Semantic versioning**:

- **PATCH** (X.Y.Z+1): Bug fixes, backward-compatible additions
- **MINOR** (X.Y+1.0): New features (backwards compatible)
- **MAJOR** (X+1.0.0): Breaking API changes

**For detailed release management procedures**, refer to [~/Documents/SPS/10-release-management.md](https://github.com/au-zone/sps) for authoritative guidance on version management, git workflow, and release processes.

### Release Process (SPS-Compliant)

**Pre-release checklist:**

1. ✅ All changes committed and tests passing
2. ✅ CHANGELOG.md updated: Move `[Unreleased]` section to versioned release with date
3. ✅ All documentation current (README.md, API docs, .pyi stubs)
4. ✅ Version numbers synchronized across all files

**Release steps (manual - per SPS guidelines):**

```bash
# 1. Ensure working directory is clean
git status  # Should show no uncommitted changes

# 2. Update version in Cargo.toml (workspace and dependencies)
# Change version = "X.Y.Z" to version = "X.Y.Z+1"

# 3. Update CLI.md header with new version and date
# footer: edgefirst-client X.Y.Z+1
# date: YYYY-MM-DD

# 4. Update CHANGELOG.md: Move [Unreleased] to [X.Y.Z+1] - YYYY-MM-DD

# 5. Commit all changes with DCO sign-off
git add Cargo.toml CHANGELOG.md CLI.md
git commit -s -m "chore: prepare vX.Y.Z+1 release

- Bump version from X.Y.Z to X.Y.Z+1
- Update CHANGELOG.md with release date
- Update CLI.md with new version and date"

# 6. Create annotated tag with vX.Y.Z format
git tag -a vX.Y.Z+1 -m "Release vX.Y.Z+1"

# 7. Push to trigger GitHub Actions release workflow
git push origin main --tags
```

**GitHub Actions (automatic on tag push):**

- Verifies version in tag matches Cargo.toml
- Builds binaries for all platforms
- Publishes to crates.io and PyPI
- Creates GitHub Release with artifacts
- Generates man page and SBOM

### Pre-Commit Requirements

**MUST complete before committing** (in order):

1. **Update documentation** if changes affect:
   - `README.md`: User-facing features, installation, usage
   - `CLI.md`: CLI commands, options (NOTE: version/date auto-updated by cargo-release)
   - `CONTRIBUTING.md`: Development workflows
   - API doc comments: Rust documentation with examples
   - `.pyi` stubs: Python type hints

2. **Update CHANGELOG.md** for user-visible changes only

3. **Run code quality checks**:

   ```bash
   cargo +nightly fmt --all
   cargo clippy --fix --allow-dirty --all-features --all-targets
   ruff format *.py examples/*.py crates/edgefirst-client-py/edgefirst_client.pyi
   ruff check --fix *.py examples/*.py crates/edgefirst-client-py/edgefirst_client.pyi
   ```

4. **Verify build succeeds** - **MUST BUILD WITHOUT ERRORS**:

   ```bash
   cargo build --all-features --locked  # MUST succeed
   cargo clippy --all-features --all-targets --locked  # MUST pass
   ```

5. **Run tests** (if credentials available) - **ALL TESTS MUST PASS**:

   ```bash
   cargo test --all-features --locked
   cargo test --doc --locked
   maturin develop -m crates/edgefirst-client-py/Cargo.toml
   python3 -m slipcover --xml --out coverage.xml -m xmlrunner discover -s . -p "test*.py" -o target/python
   ```

6. **Check dependency licenses** (if dependencies changed):

   ```bash
   make sbom
   make check-license
   ```

7. **Sign commits with DCO**:

   ```bash
   git commit -s -m "Your commit message"
   ```

### Common Pitfalls

1. **Async boundaries**: Python bindings use `tokio-wrap` - don't use bare `tokio::runtime::Handle::block_on`
2. **Version format**: Never use `-rc.1`, always use `rc1` (no separators)
3. **Feature gates**: Remember `#[cfg(feature = "polars")]` when using Polars types
4. **Test credentials**: Integration tests need credentials - rely on CI if unavailable
5. **Import formatting**: Use `imports_granularity = 'Crate'` (rustfmt.toml)
6. **Nightly Rust**: Formatting requires nightly (`cargo +nightly fmt`)
7. **Pylance type narrowing**: After `self.assertIsNotNone(x)`, add `assert x is not None`

### EdgeFirst Studio Integration

When making changes that affect EdgeFirst Studio integration:

- **Test against EdgeFirst Studio test server**: Verify API compatibility
- **Check API version compatibility**: Ensure changes work with current Studio version
- **Document Studio-specific behavior**: Note any platform-specific requirements
- **Consider backward compatibility**: Studio users may be on different versions
- **Update integration examples**: Ensure documentation reflects current Studio API

---

## Working with AI Assistants

### For GitHub Copilot / Cursor

These tools provide inline suggestions. Ensure:

- Suggestions match project conventions (run linters after accepting)
- Complex logic has explanatory comments
- Generated tests have meaningful assertions
- Security best practices are followed (no hardcoded credentials)

### For Claude Code / Chat-Based Assistants

When working with conversational AI:

1. **Provide context**: Share relevant files, error messages, and requirements
2. **Verify outputs**: Review generated code critically before committing
3. **Iterate**: Refine solutions through follow-up questions
4. **Document decisions**: Capture architectural choices and tradeoffs
5. **Test thoroughly**: AI-generated code needs human verification
6. **Studio context**: Explain EdgeFirst Studio integration requirements

### Common AI Assistant Pitfalls

- **Hallucinated APIs**: Verify EdgeFirst Studio API endpoints exist
- **Outdated patterns**: Check if suggestions match current Rust/Python best practices
- **Over-engineering**: Prefer simple solutions over complex ones
- **Missing edge cases**: Explicitly test boundary conditions
- **License violations**: AI may suggest code with incompatible licenses
- **Async patterns**: Ensure proper Tokio usage in Rust code

---

## Getting Help

**For development questions:**

- Check `CONTRIBUTING.md` for setup instructions
- Review existing code for patterns and conventions
- Search [GitHub Issues](https://github.com/EdgeFirstAI/client/issues)
- Ask in [GitHub Discussions](https://github.com/orgs/EdgeFirstAI/discussions)

**For security concerns:**

- Use [GitHub Security Advisories](https://github.com/EdgeFirstAI/client/security/advisories)
- Email `support@au-zone.com` with subject "[SECURITY] EdgeFirst Client"
- Do not disclose vulnerabilities publicly

**For EdgeFirst Studio questions:**

- [EdgeFirst User Manual](https://doc.edgefirst.ai)
- EdgeFirst Studio documentation and API reference

---

## Document Maintenance

**Project maintainers should:**

- Keep project-specific guidelines current
- Update examples when APIs change
- Review and update after major EdgeFirst Studio API changes
- Maintain alignment with EdgeFirst Studio features

**This document version**: 1.1 (November 2025)
**Organization**: Au-Zone Technologies
**License**: Apache-2.0

---

*This document helps AI assistants contribute effectively to EdgeFirst Client while maintaining quality, security, and seamless integration with EdgeFirst Studio.*
