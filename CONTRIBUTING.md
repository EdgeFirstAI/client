# Contributing to EdgeFirst Client

Thank you for your interest in contributing to EdgeFirst Client! This document provides guidelines and instructions for contributing.

**Using AI Coding Agents?** See [AGENTS.md](AGENTS.md) for a concise reference of project conventions, build commands, and pre-commit requirements optimized for AI assistants.

## Code of Conduct

By participating in this project, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md). Please be respectful and constructive in all interactions.

## How to Contribute

### Reporting Bugs

Before creating a bug report:
1. Check the [existing issues](https://github.com/EdgeFirstAI/client/issues) to avoid duplicates
2. Gather relevant information (version, OS, error messages, etc.)

Use the bug report template when creating an issue.

### Suggesting Features

Feature requests are welcome! Please:
1. Check if the feature has already been suggested
2. Provide clear use cases
3. Explain how it benefits users

Use the feature request template when creating an issue.

### Contributing Code

#### Development Setup

**Prerequisites:**
- Rust 1.90 or later
- Python 3.8 or later
- Git

**Clone and build:**

```bash
git clone https://github.com/EdgeFirstAI/client.git
cd client
cargo build
```

**Install Python dependencies:**

```bash
pip install -r requirements.txt
pip install maturin
maturin develop -m crates/edgefirst-client-py/Cargo.toml
```

#### Making Changes

1. **Fork the repository** and create a branch from `main`:
   ```bash
   git checkout -b feature/my-feature
   # or
   git checkout -b fix/my-bugfix
   ```

2. **Make your changes** following the coding standards below

3. **Add tests** for your changes:
   - Rust: Add tests in the same file or in `tests/` directory
   - Python: Add tests to test files

4. **Run tests** to ensure everything works:
   ```bash
   # Recommended: Run with coverage instrumentation (adds ~10% overhead, provides robust results)
   source <(cargo llvm-cov show-env --export-prefix --no-cfg-coverage --doctests)
   cargo build --all-features --locked
   maturin develop -m crates/edgefirst-client-py/Cargo.toml
   python3 -m slipcover --xml --out coverage.xml -m xmlrunner discover -s . -p "test*.py" -o target/python
   
   # Alternative: Quick test without coverage
   cargo test --all-features --locked
   cargo test --doc --locked
   python -m unittest discover -s . -p "test*.py"
   ```
   
   **Coverage Benefits**: VS Code users can install the [Coverage Gutters](https://marketplace.visualstudio.com/items?itemName=ryanluker.vscode-coverage-gutters) extension to see directly in the editor which parts of code are covered or not, helpful when making changes to understand if you're modifying something without a direct unit test.

5. **Format your code:**
   ```bash
   cargo fmt --all
   ```

6. **Run clippy:**
   ```bash
   cargo clippy --all-targets --all-features
   ```

7. **Update documentation** if needed:
   - Add/update doc comments in code
   - Update README.md for significant changes
   - Update CHANGELOG.md

8. **Commit your changes** with a clear, descriptive message:
   ```bash
   git commit -m "Add new feature X
   
   - Detailed description of changes
   - Related to #123"
   ```

9. **Push to your fork:**
   ```bash
   git push origin feature/my-feature
   ```

10. **Create a Pull Request** using the PR template

#### Code Review Process

- Maintainers will review your PR
- Address any feedback or requested changes
- Once approved, a maintainer will merge your PR

## Coding Standards

### Rust Code

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo fmt` for formatting
- Fix all `cargo clippy` warnings
- Add doc comments for public APIs
- Include examples in doc comments when helpful
- Write unit tests for new functionality

### Python Code

- Follow [PEP 8](https://www.python.org/dev/peps/pep-0008/) strictly (79-character line limit)
- Use `autopep8` for automatic formatting and PEP-8 compliance
- Use type hints where possible
- Add docstrings for public functions and classes
- Write unit tests for new functionality

### Documentation

- Use clear, concise language
- Include code examples
- Update CHANGELOG.md for user-facing changes
- Keep README.md up to date

### Git Commits

- Write clear, descriptive commit messages
- Reference issue numbers when applicable
- Keep commits focused and atomic
- Explain the "why" behind changes in commit messages

## Project Structure

```
client/
├── crates/
│   ├── edgefirst-client/       # Core Rust library
│   ├── edgefirst-cli/          # CLI application
│   └── edgefirst-client-py/    # Python bindings
├── .github/
│   └── workflows/              # GitHub Actions
├── testdata/                   # Test data files
├── Cargo.toml                  # Workspace manifest
└── README.md
```

## Testing

### Running Tests

**All tests:**
```bash
cargo test
```

**Specific test:**
```bash
cargo test test_name
```

**With output:**
```bash
cargo test -- --nocapture
```

**Doc tests:**
```bash
cargo test --doc
```

**Python tests:**
```bash
# Recommended: Run with slipcover to match CI/CD behavior (zero overhead, strict syntax validation)
python3 -m slipcover --xml --out coverage.xml -m xmlrunner discover -s . -p "test*.py" -o target/python

# Alternative: Quick test without coverage
python -m unittest
```

**Why use slipcover locally?**
- Matches CI/CD behavior exactly (catches syntax errors that unittest may miss)
- Zero performance overhead (same execution time as unittest)
- Generates coverage.xml for local analysis
- Strict syntax validation prevents CI failures

### Test Coverage

Generate Rust coverage report:
```bash
cargo install cargo-llvm-cov
cargo llvm-cov --html
```

For comprehensive coverage testing (matches CI/CD environment):
```bash
# Set up coverage instrumentation environment
source <(cargo llvm-cov show-env --export-prefix --no-cfg-coverage --doctests)

# Rebuild with coverage instrumentation
cargo build --all-features --locked

# Build Python bindings with coverage
maturin develop -m crates/edgefirst-client-py/Cargo.toml

# Run Python tests with coverage-instrumented Rust
python3 -m slipcover --xml --out coverage.xml -m xmlrunner discover -s . -p "test*.py" -o target/python

# Generate combined coverage report
cargo llvm-cov report --doctests --lcov --output-path lcov.info
```

**Performance impact:** Coverage instrumentation adds ~10% overhead (4-5 seconds for full test suite). **Recommended for local development** to catch coverage gaps early. VS Code users should install the Coverage Gutters extension for inline coverage visibility.

### SonarCloud Code Quality Analysis

The project uses SonarCloud for automated code quality and security analysis. The `sonar.py` script fetches current analysis results in a format optimized for GitHub Copilot to help you fix identified issues directly in your IDE.

**Features:**
- ✅ Fetch fresh analysis results with staleness detection
- ✅ Rich metadata optimized for GitHub Copilot interpretation
- ✅ Filter issues by severity, type, and status
- ✅ Comprehensive rule descriptions and remediation guidance
- ✅ Support for both branches and pull requests

#### Quick Start

1. **Create a SonarCloud token:**
   - Visit [SonarCloud](https://sonarcloud.io/)
   - Navigate to: **Account → Security → Generate Tokens**
   - Create a new token with read permissions

2. **Set up environment variables:**
   ```bash
   export SONAR_TOKEN=your_token_here
   export SONAR_ORG=edgefirstai
   export SONAR_PROJECT=EdgeFirstAI_client
   ```

3. **Fetch current findings (optimized for Copilot):**
   ```bash
   # All open issues
   python3 sonar.py --branch main --output sonar-issues.json --verbose
   
   # Only critical/blocker issues
   python3 sonar.py --branch main --severity BLOCKER,CRITICAL -o critical.json
   
   # Only bugs and vulnerabilities
   python3 sonar.py --branch main --type BUG,VULNERABILITY -o security.json
   ```

4. **Use with GitHub Copilot:**
   - Open `sonar-issues.json` in VS Code
   - Ask: `@workspace Review sonar-issues.json and help me fix the top 5 critical issues`
   - For specific files: `@workspace Show me all issues in src/client.rs from sonar-issues.json and suggest fixes`

#### Advanced Usage

**Pull Request Analysis:**
```bash
python3 sonar.py --pull-request 123 --output pr-issues.json -v
```

**Staleness Detection:**
```bash
# Warn if analysis is older than 6 hours
python3 sonar.py --branch main --max-age-hours 6 -o sonar-issues.json -v
```

**Include Resolved Issues (for historical analysis):**
```bash
python3 sonar.py --branch main --include-resolved -o all-issues.json
```

**CI/CD Integration Example:**
```yaml
- name: Fetch SonarCloud Issues
  env:
    SONAR_TOKEN: ${{ secrets.SONAR_TOKEN }}
    SONAR_ORG: edgefirstai
    SONAR_PROJECT: EdgeFirstAI_client
  run: |
    python3 sonar.py --branch ${{ github.ref_name }} \
      --severity BLOCKER,CRITICAL \
      --output sonar-issues.json --verbose
```

#### Output Format

The script generates JSON optimized for Copilot with:
- **file**: Relative path from project root
- **line/endLine**: Precise line numbers
- **severity**: BLOCKER, CRITICAL, MAJOR, MINOR, INFO
- **type**: BUG, VULNERABILITY, CODE_SMELL, SECURITY_HOTSPOT
- **message**: Human-readable description
- **context.ruleDescription**: Full HTML remediation guidance

#### Troubleshooting

**Authentication Error (401):**
```bash
# Verify token is valid
curl -H "Authorization: Bearer $SONAR_TOKEN" \
  https://sonarcloud.io/api/authentication/validate
```

**Stale Analysis Warning:**
```
⚠️  WARNING: Analysis is 25.3 hours old (threshold: 24h)
```
Solution: Push a new commit or wait for scheduled SonarCloud analysis.

**No Issues Found:**
Check branch name is correct and analysis exists for that branch using `--verbose`.

**Rate Limiting (429):**
Wait a few minutes before retrying. SonarCloud limits API requests.

#### Tips for Best Results

1. **Run fresh analyses regularly** - Don't rely on stale data
2. **Start with high-severity issues** - Use `--severity BLOCKER,CRITICAL`
3. **Focus on specific types** - Use `--type BUG,VULNERABILITY` for security
4. **Use verbose mode** - Add `-v` to see detailed progress
5. **Iterate quickly** - Fix issues, commit, wait for new analysis, repeat

**Note:** The `sonar-issues.json` file is gitignored and should not be committed.

### Studio Integration Tests

The CLI test suite includes integration tests that interact with EdgeFirst Studio test servers. These tests require authenticated access to validate server-side behavior.

#### Test Infrastructure

**Test Servers:**
- `test`: https://test.edgefirst.studio (primary test environment)
- `stage`: https://stage.edgefirst.studio (staging environment)
- `saas`: https://edgefirst.studio (production environment)

**Test Data Conventions:**
- Test user: `testing` (common across all environments)
- Test project: `Unit Testing` (exists on all servers)
- Static dataset: `Deer` (for download, train, and validation operations)
- CRUD dataset: `Test Labels` (for create/update/delete operations)

#### Running Studio Tests

**Environment Variables:**
- `STUDIO_SERVER`: Server environment name (`test`, `stage`, or `saas`)
- `STUDIO_USERNAME`: Username for authentication tests
- `STUDIO_PASSWORD`: Password for authentication tests

**Credential Access:**
> **Note**: Test credentials are **not publicly available**. Only project maintainers have access to these credentials.

**For Contributors:**
Contributors can run the full test suite (including Studio integration tests) through GitHub Actions CI/CD pipelines:
1. Fork the repository
2. Push changes to your fork
3. Create a pull request
4. The CI workflow will automatically run all tests using stored credentials

**For Local Development:**
Without Studio credentials, you can:
- Run all non-authenticated tests: Most of the test suite
- Develop and test CLI changes locally
- Rely on CI/CD for full integration testing

**For Maintainers:**
With Studio credentials, set environment variables before running tests:
```bash
export STUDIO_SERVER=test
export STUDIO_USERNAME=<username>
export STUDIO_PASSWORD=<password>
cargo test
```

## Building

### CLI Binary

```bash
cargo build --release -p edgefirst-cli
```

### Python Wheel

```bash
maturin build --release -m crates/edgefirst-client-py/Cargo.toml
```

## Documentation

### Generate Rust Docs

```bash
cargo doc --open
```

### API Documentation

Rust documentation is automatically published to [docs.rs](https://docs.rs/edgefirst-client) on release.

### CLI Man Page Documentation

The CLI has comprehensive man-page style documentation in `CLI.md` that can be converted to a Unix man page:

```bash
# Build the man page (requires pandoc)
pandoc CLI.md --standalone --to man --output edgefirst-client.1

# View locally
man ./edgefirst-client.1
```

**When updating CLI commands:** If you add, modify, or remove CLI commands, update `CLI.md` to reflect the changes:

1. Add/update command documentation with syntax, options, arguments, and examples
2. Update the date in the YAML front matter (e.g., `date: October 2025`)
3. Optionally rebuild the man page to verify formatting: `pandoc CLI.md --standalone --to man --output edgefirst-client.1`
4. The man page (`.1` file) is auto-generated and git-ignored - don't commit it
5. The man page is automatically built and included in GitHub releases

**On release:** Update the version in `CLI.md` YAML front matter:
```yaml
---
title: EDGEFIRST-CLIENT
section: 1
header: EdgeFirst Client Manual
footer: edgefirst-client X.Y.Z  # <-- Update this version
date: Month YYYY                # <-- Update this date
---
```

## Versioning

This project follows [Semantic Versioning](https://semver.org/) (SemVer) with the following format:

- **Stable releases**: `X.Y.Z` (e.g., `1.0.0`, `2.1.5`)
- **Release candidates**: `X.Y.ZrcN` (e.g., `1.0.0rc1`, `2.0.0rc2`)
- **Alpha releases**: `X.Y.ZaN` (e.g., `0.1.0a1`, `1.0.0a2`)
- **Beta releases**: `X.Y.ZbN` (e.g., `0.1.0b1`, `1.0.0b2`)

**Important**: Use the format without separators (e.g., `1.0.0rc1`, not `1.0.0-rc.1`) to ensure compatibility with both:
- Python's PEP 440 standard (for PyPI publishing)
- Rust's Cargo/SemVer standard (for crates.io publishing)

### Choosing the Version Number

When preparing a release, select the appropriate version bump based on changes since the last release:

**PATCH (X.Y.Z → X.Y.Z+1)** - Default for most releases
- Bug fixes that don't change the API
- Performance improvements
- Internal refactoring (no API changes)
- New features that don't change existing APIs (backward-compatible additions)
- Documentation updates
- Examples: `2.1.0 → 2.1.1`, `2.1.1 → 2.1.2`

**MINOR (X.Y.Z → X.Y+1.0)** - Required for breaking changes
- API changes that break backward compatibility
- Removing public functions, methods, or types
- Changing function signatures (parameters, return types)
- Renaming public APIs
- Changing behavior in ways that existing code depends on
- Examples: `2.1.5 → 2.2.0`, `2.2.0 → 2.3.0`

**MAJOR (X.Y.Z → X+1.0.0)** - Reserved for maintainers
- Major architectural changes
- Complete API rewrites
- Reserved for maintainer decision only
- Examples: `2.9.5 → 3.0.0`, `1.5.2 → 2.0.0`

**CHANGELOG Requirements:**
- **PATCH releases**: Document new features or bug fixes in CHANGELOG under `### Added`, `### Fixed`, or `### Changed`
- **MINOR releases**: Document breaking changes in CHANGELOG under `### Changed` with clear migration guidance
- **MAJOR releases**: Provide comprehensive migration guide

**Default**: When in doubt, use **PATCH** for backward-compatible changes and **MINOR** for breaking changes.

## Release Process

Releases are managed by maintainers using [cargo-release](https://github.com/crate-ci/cargo-release):

```bash
# 1. Update CHANGELOG.md with release notes under [Unreleased]

# 2. Update CLI.md version and date in YAML front matter
#    footer: edgefirst-client X.Y.Z
#    date: Month YYYY

# 3. Run cargo-release to bump versions and create tag
cargo release patch --execute --no-confirm    # or: minor, major

# 4. Push to trigger CI/CD
git push && git push --tags
```

GitHub Actions will automatically build binaries, publish to crates.io and PyPI, create a GitHub Release, and generate the man page as a release artifact.

**Version Format**: Use `X.Y.Z` for stable releases, `X.Y.ZrcN` for release candidates (without separators for PyPI/Cargo compatibility).

### Release Candidates

> [!WARNING]
> **Extra Manual Steps Required**: Release candidates require additional manual version editing because:
> - PyPI requires `X.Y.ZrcN` format (PEP 440)
> - cargo-release uses `X.Y.Z-rc.N` format (SemVer)
> - Maturin does NOT automatically convert between these formats
> - You must use `X.Y.ZrcN` in Cargo.toml for dual compatibility

For release candidates, manually specify the version since cargo-release uses `-rc.1` format instead of `rc1`:

```bash
# Manually update version in Cargo.toml to use rcN format (e.g., 2.3.0rc1)
sed -i '' 's/version = "2.2.2"/version = "2.3.0rc1"/' Cargo.toml
sed -i '' 's/edgefirst-client = { version = "2.2.2"/edgefirst-client = { version = "2.3.0rc1"/' Cargo.toml

# Then use cargo release with explicit version
cargo release 2.3.0rc1 --execute --no-confirm

# Push
git push && git push --tags
```

### What cargo-release Does

The project uses workspace dependencies in `Cargo.toml`, so cargo-release automatically:
- Updates workspace version in root `Cargo.toml`
- Updates workspace dependency version for `edgefirst-client`
- Updates all crate versions (inherited via `version.workspace = true`)
- Updates `Cargo.lock`
- Creates commit: "Release X.Y.Z Preparations"
- Creates git tag: `X.Y.Z` (locally, not pushed)

### Configuration

The `release.toml` file configures cargo-release:
- Only allows releases from `main` branch (safety)
- Uses tag format `X.Y.Z` without "v" prefix (matches existing tags)
- Disables automatic publishing (handled by CI)
- Disables automatic pushing (manual control for review)

See [cargo-release reference](https://github.com/crate-ci/cargo-release/blob/master/docs/reference.md) for more details.

## Getting Help

- Open an [issue](https://github.com/EdgeFirstAI/client/issues) for questions
- Check existing documentation
- Review closed issues and PRs

## License

By contributing, you agree that your contributions will be licensed under the Apache-2.0 License.

## Recognition

Contributors will be recognized in release notes and the project's contributors list.

Thank you for contributing to EdgeFirst Client!
