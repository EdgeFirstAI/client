# Contributing to EdgeFirst Client

Thank you for your interest in contributing to EdgeFirst Client! This project is the official Client API and CLI for **[EdgeFirst Studio](https://edgefirst.studio)**, advancing 3D visual and 4D spatial perception AI capabilities.

**Using AI Coding Agents?** See [AGENTS.md](AGENTS.md) for a concise reference of project conventions, build commands, and pre-commit requirements optimized for AI assistants.

## Code of Conduct

By participating in this project, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md). Please be respectful and constructive in all interactions.

## Ways to Contribute

### For All Contributors

- **Code**: Features, bug fixes, performance improvements
- **Documentation**: Improvements, examples, tutorials
- **Testing**: Bug reports, test coverage, EdgeFirst Studio integration validation
- **Community**: Answer questions, write blog posts, share use cases

### EdgeFirst Studio Integration

When contributing, consider how changes might affect:
- **EdgeFirst Studio compatibility**: Maintain seamless integration with the platform
- **API versioning**: Ensure backward compatibility where possible
- **User workflows**: Common patterns users rely on
- **Documentation**: Keep Studio integration examples current

## Before You Start

1. Check existing [issues](https://github.com/EdgeFirstAI/client/issues) and [pull requests](https://github.com/EdgeFirstAI/client/pulls)
2. For significant changes, open an issue for discussion first
3. Review our [project roadmap](https://github.com/EdgeFirstAI/client/issues?q=is%3Aissue+is%3Aopen+label%3Aroadmap) to understand direction
4. Consider EdgeFirst Studio integration implications

## Development Setup

### Prerequisites

- **Rust** 1.90 or later
- **Python** 3.8 or later (for Python bindings)
- **Git**
- **EdgeFirst Studio account** (free tier available) for integration testing

### Clone and Build

```bash
git clone https://github.com/EdgeFirstAI/client.git
cd client
cargo build
```

### Install Python Dependencies

```bash
pip install -r requirements.txt
pip install maturin
maturin develop -m crates/edgefirst-client-py/Cargo.toml
```

## Contribution Process

### 1. Fork and Clone

Fork the repository and create a branch:

```bash
git checkout -b feature/my-feature
# or
git checkout -b fix/my-bugfix
```

**Branch Naming:**
- Use descriptive names: `feature/add-batch-upload`, `bugfix/fix-token-refresh`
- Use kebab-case (lowercase with hyphens)
- Keep descriptions concise but meaningful

### 2. Make Changes

Follow the coding standards below:

#### Rust Code

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use `cargo +nightly fmt` for formatting (project uses nightly for formatting features)
- Fix all `cargo clippy` warnings
- Add doc comments for public APIs with runnable examples
- Include examples in doc comments when helpful
- Write unit tests for new functionality

#### Python Code

- Follow [PEP 8](https://www.python.org/dev/peps/pep-0008/) strictly (79-character line limit)
- Use `autopep8` for automatic formatting
- Use type hints where possible
- Maintain `.pyi` type stubs in `crates/edgefirst-client-py/edgefirst_client.pyi`
- Add docstrings for public functions and classes
- Write unit tests for new functionality

### 3. Add Tests

**Rust Tests:**
- Add tests in the same file (`#[cfg(test)] mod tests`) or in `tests/` directory
- Test naming: `test_<function>_<scenario>` format

**Python Tests:**
- Add tests to `test*.py` files
- Use Python `unittest` framework
- Follow existing test patterns

**EdgeFirst Studio Integration Tests:**
- Add tests that validate Studio API compatibility
- Use test servers and test data conventions
- Requires credentials (see [Studio Integration Tests](#studio-integration-tests))

### 4. Run Tests

```bash
# Recommended: Run with coverage instrumentation
source <(cargo llvm-cov show-env --export-prefix --no-cfg-coverage)
cargo build --all-features --locked
maturin develop -m crates/edgefirst-client-py/Cargo.toml
python3 -m slipcover --xml --out coverage.xml -m xmlrunner discover -s . -p "test*.py" -o target/python

# Alternative: Quick test without coverage
cargo test --all-features --locked
cargo test --doc --locked
python -m unittest discover -s . -p "test*.py"
```

**Coverage Benefits**: VS Code users can install the [Coverage Gutters](https://marketplace.visualstudio.com/items?itemName=ryanluker.vscode-coverage-gutters) extension to see coverage inline while coding.

### 5. Format Your Code

```bash
# Rust (requires nightly)
cargo +nightly fmt --all

# Python
autopep8 --in-place --aggressive --aggressive *.py examples/*.py crates/edgefirst-client-py/edgefirst_client.pyi
```

### 6. Run Linting

```bash
cargo clippy --all-targets --all-features
```

### 7. Update Documentation

If your changes affect:
- **README.md**: User-facing features, installation, usage
- **CLI.md**: CLI commands, options, examples (update version/date on release)
- **CONTRIBUTING.md**: Development workflows, build processes
- **API doc comments**: Rust documentation with examples
- **`.pyi` stubs**: Python type hints in `crates/edgefirst-client-py/edgefirst_client.pyi`
- **CHANGELOG.md**: User-visible changes only (see below)

### 8. Update CHANGELOG

Update `CHANGELOG.md` for user-visible changes only:

✅ **Document:**
- New features
- API changes
- Behavior changes
- Bug fixes
- Breaking changes

❌ **Skip:**
- Internal refactoring
- Test updates
- Code cleanup

**Format:** Under `## [Unreleased]` use `### Added`, `### Changed`, `### Fixed`, `### Removed`

### 9. Commit Your Changes

Use clear, descriptive commit messages:

```bash
git commit -m "Add batch upload feature

- Implemented concurrent upload with semaphore
- Added progress tracking for batch operations
- Updated documentation with examples"
```

### 10. Push and Create Pull Request

```bash
git push origin feature/my-feature
```

Then create a Pull Request using the PR template.

## Pull Request Guidelines

### PR Title and Description

**Title**: Brief description of changes (no specific format required)

**Description Template:**
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

## EdgeFirst Studio Compatibility
- [ ] Tested against EdgeFirst Studio test server
- [ ] Backward compatibility maintained
- [ ] API version compatibility verified

## Checklist
- [ ] Code follows project conventions
- [ ] No secrets or credentials committed
- [ ] CHANGELOG.md updated (if user-facing changes)
- [ ] LICENSE policy compliance verified
```

### Code Review Process

1. Create PR via GitHub web interface
2. Link to related issues in description
3. Wait for CI/CD to complete successfully
4. Address reviewer feedback
5. Maintainer will merge once approved

## Coding Standards

### General Principles

- **Consistency**: Follow existing codebase patterns
- **Readability**: Optimize for comprehension
- **Simplicity**: Prefer straightforward solutions
- **Error Handling**: Validate inputs, provide actionable error messages
- **Performance**: Consider async patterns and memory efficiency

### Rust-Specific

- Use `cargo +nightly fmt --all` for formatting
- Run `cargo clippy --fix --allow-dirty --all-features --all-targets` before committing
- Imports grouped by crate: `imports_granularity = 'Crate'` (rustfmt.toml)
- Async-first design: All API calls are `async fn` using Tokio
- Feature gates: Use `#[cfg(feature = "polars")]` for conditional compilation

### Python-Specific

- Follow PEP 8 strictly (79-character line limit)
- Use `autopep8 --in-place --aggressive --aggressive` for formatting
- **Pylance type checking**: Code must be Pylance-clean
- Type narrowing patterns: `self.assertIsNotNone(x)` → `assert x is not None`
- Maintain `.pyi` stubs synchronized with implementation

### Git Commits

- Write clear, descriptive commit messages
- Reference issue numbers when applicable
- Keep commits focused and atomic
- Focus on "what" changed (implementation details belong in code/docs)

## Project Structure

```
client/
├── crates/
│   ├── edgefirst-client/       # Core Rust library
│   ├── edgefirst-cli/          # CLI application
│   └── edgefirst-client-py/    # Python bindings (PyO3)
├── .github/
│   └── workflows/              # GitHub Actions CI/CD
├── testdata/                   # Test data files
├── test/                       # Python test code
├── examples/                   # Python examples
├── Cargo.toml                  # Workspace manifest
├── README.md                   # Project overview
├── CONTRIBUTING.md             # This file
├── AGENTS.md                   # AI assistant guidelines
└── CLI.md                      # CLI man page documentation
```

## Testing

### Running Tests

**All Rust tests:**
```bash
cargo test --all-features --locked
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
cargo test --doc --locked
```

**Python tests:**
```bash
# Recommended: Run with slipcover (matches CI/CD behavior)
python3 -m slipcover --xml --out coverage.xml -m xmlrunner discover -s . -p "test*.py" -o target/python

# Alternative: Quick test without coverage
python -m unittest discover -s . -p "test*.py"
```

**Why use slipcover locally?**
- Matches CI/CD behavior exactly
- Catches syntax errors that unittest may miss
- Zero performance overhead
- Generates coverage.xml for local analysis
- Strict syntax validation prevents CI failures

### Test Coverage

Generate comprehensive coverage report:

```bash
# Set up coverage instrumentation environment
source <(cargo llvm-cov show-env --export-prefix --no-cfg-coverage)

# Rebuild with coverage instrumentation
cargo build --all-features --locked

# Build Python bindings with coverage
maturin develop -m crates/edgefirst-client-py/Cargo.toml

# Run Python tests (exercises instrumented Rust code)
python3 -m slipcover --xml --out coverage.xml -m xmlrunner discover -s . -p "test*.py" -o target/python

# Run CLI tests
cargo test --package edgefirst-cli --locked

# Generate combined coverage reports
cargo llvm-cov report --lcov --output-path lcov.info
cargo llvm-cov report  # Human-readable summary
```

**Performance impact**: Coverage instrumentation adds ~10% overhead (4-5 seconds for full test suite). **Recommended for local development** to catch coverage gaps early.

### SonarCloud Code Quality Analysis

The project uses SonarCloud for automated code quality and security analysis. The `sonar.py` script fetches current analysis results optimized for GitHub Copilot.

**Quick Start:**

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

3. **Fetch current findings:**
   ```bash
   # All open issues
   python3 sonar.py --branch main --output sonar-issues.json --verbose
   
   # Only critical/blocker issues
   python3 sonar.py --branch main --severity BLOCKER,CRITICAL -o critical.json
   ```

4. **Use with GitHub Copilot:**
   - Open `sonar-issues.json` in VS Code
   - Ask: `@workspace Review sonar-issues.json and help me fix the top 5 critical issues`

For advanced usage and CI/CD integration, see full documentation in [SonarCloud Code Quality Analysis](#sonarcloud-code-quality-analysis-1).

### Studio Integration Tests

The CLI test suite includes integration tests that interact with EdgeFirst Studio test servers. These tests validate server-side behavior and API compatibility.

**Test Infrastructure:**

**Test Servers:**
- `test`: https://test.edgefirst.studio (primary test environment)
- `stage`: https://stage.edgefirst.studio (staging environment)
- `saas`: https://edgefirst.studio (production environment)

**Test Data Conventions:**
- Test user: `testing` (common across all environments)
- Test project: `Unit Testing` (exists on all servers)
- Static dataset: `Deer` (for download, train, and validation operations)
- CRUD dataset: `Test Labels` (for create/update/delete operations)

**Environment Variables:**
- `STUDIO_SERVER`: Server environment name (`test`, `stage`, or `saas`)
- `STUDIO_USERNAME`: Username for authentication tests
- `STUDIO_PASSWORD`: Password for authentication tests

**For Contributors:**

> **Note**: Test credentials are **not publicly available**. Only project maintainers have access.

Contributors can run the full test suite through GitHub Actions CI/CD pipelines:
1. Fork the repository
2. Push changes to your fork
3. Create a pull request
4. The CI workflow will automatically run all tests using stored credentials

**For Local Development:**

Without Studio credentials, you can:
- Run all non-authenticated tests
- Develop and test CLI changes locally
- Rely on CI/CD for full integration testing

**For Maintainers:**

With Studio credentials:
```bash
export STUDIO_SERVER=test
export STUDIO_USERNAME=<username>
export STUDIO_PASSWORD=<password>
cargo test --all-features --locked
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

The CLI has comprehensive man-page style documentation in `CLI.md`:

```bash
# Build the man page (requires pandoc)
pandoc CLI.md --standalone --to man --output edgefirst-client.1

# View locally
man ./edgefirst-client.1
```

**When updating CLI commands:**

1. Add/update command documentation in `CLI.md` with syntax, options, and examples
2. Update the date in YAML front matter (e.g., `date: November 2025`)
3. Optionally rebuild to verify formatting
4. The man page (`.1` file) is auto-generated and git-ignored - don't commit it
5. The man page is automatically built and included in GitHub releases

**On release:** Update version in `CLI.md` YAML front matter:
```yaml
---
footer: edgefirst-client X.Y.Z  # <-- Update this
date: Month YYYY                # <-- Update this
---
```

## Versioning

This project follows [Semantic Versioning](https://semver.org/):

- **Stable releases**: `X.Y.Z` (e.g., `1.0.0`, `2.1.5`)
- **Release candidates**: `X.Y.ZrcN` (e.g., `1.0.0rc1`, `2.0.0rc2`)
- **Alpha releases**: `X.Y.ZaN` (e.g., `0.1.0a1`)
- **Beta releases**: `X.Y.ZbN` (e.g., `0.1.0b1`)

**Important**: Use format without separators (e.g., `1.0.0rc1`, not `1.0.0-rc.1`) for compatibility with both:
- Python's PEP 440 standard (PyPI)
- Rust's Cargo/SemVer standard (crates.io)

### Choosing the Version Number

**PATCH (X.Y.Z → X.Y.Z+1)** - Default for most releases:
- Bug fixes (no API changes)
- Performance improvements
- Internal refactoring
- New features (backward-compatible additions)
- Documentation updates
- Examples: `2.1.0 → 2.1.1`

**MINOR (X.Y.Z → X.Y+1.0)** - Required for breaking changes:
- API changes breaking backward compatibility
- Removing public functions/methods/types
- Changing function signatures
- Renaming public APIs
- Examples: `2.1.5 → 2.2.0`

**MAJOR (X.Y.Z → X+1.0.0)** - Reserved for maintainers:
- Major architectural changes
- Complete API rewrites
- Examples: `2.9.5 → 3.0.0`

**CHANGELOG Requirements:**
- **PATCH**: Document features or fixes in CHANGELOG
- **MINOR**: Document breaking changes with migration guidance
- **MAJOR**: Provide comprehensive migration guide

**Default**: When in doubt, use **PATCH** for backward-compatible changes and **MINOR** for breaking changes.

## Release Process

> **Note**: Releases are managed by maintainers using [cargo-release](https://github.com/crate-ci/cargo-release).

Contributors should focus on:
1. Updating CHANGELOG.md under `[Unreleased]` section
2. Updating CLI.md if CLI changes are included
3. Ensuring all tests pass
4. Verifying documentation is current

Maintainers handle the release process:
```bash
# 1. Update CHANGELOG.md with release notes
# 2. Update CLI.md version and date
# 3. Run cargo-release
cargo release patch --execute --no-confirm  # or: minor, major
# 4. Push to trigger CI/CD
git push && git push --tags
```

GitHub Actions automatically:
- Builds binaries
- Publishes to crates.io and PyPI
- Creates GitHub Release
- Generates man page as release artifact

## Getting Help

**For development questions:**
- Check this `CONTRIBUTING.md` for setup instructions
- Review [AGENTS.md](AGENTS.md) for project conventions
- Search [GitHub Issues](https://github.com/EdgeFirstAI/client/issues)
- Ask in [GitHub Discussions](https://github.com/orgs/EdgeFirstAI/discussions)

**For EdgeFirst Studio questions:**
- [EdgeFirst User Manual](https://doc.edgefirst.ai)
- [EdgeFirst Studio documentation](https://doc.edgefirst.ai/latest/)
- EdgeFirst Studio API reference

**For security concerns:**
- Use [GitHub Security Advisories](https://github.com/EdgeFirstAI/client/security/advisories)
- Email `support@au-zone.com` with subject "[SECURITY] EdgeFirst Client"
- See [SECURITY.md](SECURITY.md) for complete security policy

## Developer Certificate of Origin (DCO)

All contributors must sign off their commits to certify they have the right to submit the code under the project's open source license. This is done by adding a `Signed-off-by` line to commit messages.

### What is DCO?

The [Developer Certificate of Origin (DCO)](https://developercertificate.org/) is a lightweight way for contributors to certify that they wrote or otherwise have the right to submit the code they are contributing. By signing off commits, you certify the following:

```text
Developer Certificate of Origin
Version 1.1

Copyright (C) 2004, 2006 The Linux Foundation and its contributors.

Everyone is permitted to copy and distribute verbatim copies of this
license document, but changing it is not allowed.

Developer's Certificate of Origin 1.1

By making a contribution to this project, I certify that:

(a) The contribution was created in whole or in part by me and I
    have the right to submit it under the open source license
    indicated in the file; or

(b) The contribution is based upon previous work that, to the best
    of my knowledge, is covered under an appropriate open source
    license and I have the right under that license to submit that
    work with modifications, whether created in whole or in part
    by me, under the same open source license (unless I am
    permitted to submit under a different license), as indicated
    in the file; or

(c) The contribution was provided directly to me by some other
    person who certified (a), (b) or (c) and I have not modified
    it.

(d) I understand and agree that this project and the contribution
    are public and that a record of the contribution (including all
    personal information I submit with it, including my sign-off) is
    maintained indefinitely and may be redistributed consistent with
    this project or the open source license(s) involved.
```

### How to Sign Off Commits

Sign off your commits using the `--signoff` or `-s` flag:

```bash
git commit -s -m "Add new feature"
```

This automatically adds a line like this to your commit message:

```text
Signed-off-by: Your Name <your.email@example.com>
```

**Configure git with your real name and email:**

```bash
git config user.name "Your Name"
git config user.email "your.email@example.com"
```

### Signing Off Previous Commits

If you forgot to sign off your commits, you can amend them:

**For the last commit:**

```bash
git commit --amend --signoff
```

**For multiple commits:**

```bash
git rebase --signoff HEAD~N  # Where N is the number of commits
```

### DCO Enforcement

- **All commits** in a pull request **must be signed off**
- Pull requests with unsigned commits will fail automated checks
- You can check your commits with: `git log --show-signature`

**Note:** Signing off commits is **not the same** as GPG signing. DCO sign-off is a certification statement, while GPG signatures cryptographically verify commit authorship (GPG signing is optional but encouraged).

## License

By contributing, you agree that your contributions will be licensed under the Apache-2.0 License.

## Recognition

Contributors will be recognized in:
- Release notes
- Project contributors list
- GitHub contributor graphs

---

Thank you for contributing to EdgeFirst Client and helping advance 3D visual and 4D spatial perception AI!
