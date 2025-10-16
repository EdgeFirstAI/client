# GitHub Actions Workflow Architecture

This document provides a comprehensive overview of the GitHub Actions workflows for the EdgeFirst Client project.

## Workflow Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        GitHub Events                            │
└─────────────────────────────────────────────────────────────────┘
         │                    │                    │
         │ Push/PR            │ Manual             │ Tag (X.Y.Z)
         ▼                    ▼                    ▼
┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐
│   CI Workflow    │  │  Build Workflows │  │Release Workflow  │
│   (ci.yml)       │  │  (build.yml,     │  │  (release.yml)   │
│                  │  │   python.yml)    │  │                  │
└──────────────────┘  └──────────────────┘  └──────────────────┘
```

## Workflow Files

### 1. CI Workflow (`.github/workflows/ci.yml`)

**Purpose**: Continuous integration - code quality, testing, and coverage

**Triggers**:
- Push to `main` branch
- Pull requests to `main` branch

**Jobs**:

```
ci.yml
├── lint
│   ├── Check formatting (cargo fmt)
│   └── Run clippy linter
├── audit
│   └── Security audit (cargo audit)
├── test
│   ├── Run Rust tests with coverage
│   ├── Run Python tests with coverage
│   ├── Upload Rust coverage to Codecov
│   └── Upload Python coverage to Codecov
├── sonarcloud
│   ├── Download coverage reports
│   ├── Run SonarQube scan
│   └── Check quality gate (PR only)
├── doc-test
│   └── Run documentation tests
└── test-report
    ├── Run tests with nextest
    └── Publish test results
```

**Key Features**:
- Separate coverage tracking for Rust and Python
- Intelligent caching of cargo registry, index, and build artifacts
- Enhanced test reporting with cargo-nextest
- Automated security audits
- SonarCloud code quality analysis with quality gate for pull requests

**Secrets Required**:
- `CODECOV_TOKEN`: For uploading coverage reports
- `SONAR_TOKEN`: For SonarCloud analysis and quality gate checks
- `STUDIO_USERNAME`: For running Studio integration tests
- `STUDIO_PASSWORD`: For running Studio integration tests

**Studio Integration Tests**:

The CI workflow runs integration tests that authenticate and interact with EdgeFirst Studio test servers. These tests validate server-side behavior including:
- Authentication (login/logout with JWT token management)
- Dataset operations (download datasets and annotations)
- Project operations (list, create, read)
- Experiment and training workflows
- Artifact management

**Test Infrastructure:**
- Test servers: `test`, `stage`, and `saas` environments
- Test data conventions: Common `test` user, `Unit Testing` project, `Deer` and `Test Labels` datasets
- Server selection: `STUDIO_SERVER` environment variable (set to `test` in CI)
- Credentials: Stored as GitHub Secrets (not publicly available)

**For Contributors:**
Studio credentials are only available to project maintainers. Contributors can run the full test suite by creating pull requests, which trigger CI workflows with stored credentials. This ensures comprehensive testing while maintaining credential security.

**Artifacts Generated**:
- `coverage-reports`: lcov.info and coverage.xml files

---

### 2. Build CLI Workflow (`.github/workflows/build.yml`)

**Purpose**: Build CLI binaries for multiple platforms

**Triggers**:
- Push to `main` branch
- Pull requests to `main` branch
- Manual workflow dispatch

**Jobs**:

```
build.yml
├── build (matrix)
│   ├── Linux x86_64
│   ├── Linux aarch64
│   ├── macOS x86_64
│   ├── macOS aarch64
│   └── Windows x86_64
└── verify
    └── Download and list all artifacts
```

**Matrix Strategy**:
| OS | Target | Output |
|---|---|---|
| ubuntu-latest | x86_64-unknown-linux-gnu | edgefirst-client-linux-amd64 |
| ubuntu-latest | aarch64-unknown-linux-gnu | edgefirst-client-linux-arm64 |
| macos-latest | x86_64-apple-darwin | edgefirst-client-macos-amd64 |
| macos-latest | aarch64-apple-darwin | edgefirst-client-macos-arm64 |
| windows-latest | x86_64-pc-windows-msvc | edgefirst-client-windows-amd64.exe |

**Key Features**:
- Cross-compilation for Linux aarch64
- Intelligent caching for faster builds
- Per-platform artifacts uploaded
- Binaries are automatically stripped by cargo --release (no separate strip step needed)

**Artifacts Generated**:
- Individual binary artifacts for each platform

---

### 3. Python Wheels Workflow (`.github/workflows/python.yml`)

**Purpose**: Build Python wheels for multiple platforms

**Triggers**:
- Push to `main` branch
- Pull requests to `main` branch
- Manual workflow dispatch

**Jobs**:

```
python.yml
├── build-wheels (matrix)
│   ├── Linux x86_64 (manylinux2014)
│   ├── Linux aarch64 (manylinux2014)
│   ├── macOS x86_64
│   ├── macOS aarch64
│   └── Windows x86_64
└── test-wheels (matrix)
    ├── Test on Linux
    ├── Test on macOS
    └── Test on Windows
```

**Matrix Strategy**:
| OS | Target | Platform | Arch |
|---|---|---|---|
| ubuntu-latest | x86_64-unknown-linux-gnu | linux | x86_64 |
| ubuntu-latest | aarch64-unknown-linux-gnu | linux | aarch64 |
| macos-latest | x86_64-apple-darwin | macos | x86_64 |
| macos-latest | aarch64-apple-darwin | macos | aarch64 |
| windows-latest | x86_64-pc-windows-msvc | windows | x86_64 |

**Key Features**:
- Uses maturin with zig for cross-compilation
- manylinux2014 compatibility for broad Linux support
- Automated wheel testing on native platforms
- Individual platform-specific artifacts (no combined artifact)

**Secrets Required**:
- `STUDIO_USERNAME`: For testing wheels
- `STUDIO_PASSWORD`: For testing wheels

**Artifacts Generated**:
- `wheels-{platform}-{arch}`: Individual platform wheels

---

### 4. Release Workflow (`.github/workflows/release.yml`)

**Purpose**: Complete release automation with publishing

**Triggers**:
- Tags matching semantic versioning: 
  - Stable releases: `[0-9]+.[0-9]+.[0-9]+`
    - Examples: `1.0.0`, `2.1.3`, `0.5.0`
  - Release candidates: `[0-9]+.[0-9]+.[0-9]+rc[0-9]+`
    - Examples: `1.0.0rc1`, `2.1.0rc2`, `0.5.0rc1`

**Jobs**:

```
release.yml
├── create-release
│   ├── Extract version from tag
│   ├── Verify Cargo.toml version matches tag
│   └── Create GitHub release
├── generate-licenses
│   ├── Install cargo-license
│   ├── Generate TSV license file
│   ├── Convert TSV to Markdown table
│   └── Upload THIRD_PARTY.md to release
├── build-cli (matrix)
│   ├── Build CLI binaries (5 platforms)
│   ├── Create compressed archives
│   └── Upload to GitHub release
├── build-wheels (matrix)
│   ├── Build CLI and bundle with wheel
│   ├── Build wheels (5 platforms)
│   └── Upload as artifacts
├── publish-pypi
│   ├── Download all wheels
│   └── Publish to PyPI
├── publish-crates-io
│   ├── Verify Cargo.toml version matches tag
│   ├── Publish edgefirst-client (library crate)
│   └── Publish edgefirst-cli (CLI binary)
└── upload-wheels-to-release
    └── Upload all wheels to GitHub release
```

**Workflow Diagram**:

```
Tag Push (e.g., 1.0.0 or 1.0.0rc1)
         │
         ▼
  create-release ────┐
         │           │
         │           ├──► generate-licenses
         │           │       └──► Upload THIRD_PARTY.md to release
         │           │
         │           ▼
         ├──────► build-cli (5 platforms)
         │           │
         │           └──► Upload binaries to release
         │
         ├──────► build-wheels (5 platforms)
         │           │
         │           ├──► publish-pypi
         │           │       └──► PyPI
         │           │
         │           └──► upload-wheels-to-release
         │                   └──► GitHub Release
         │
         └──────► publish-crates-io
                     ├──► edgefirst-client → crates.io
                     └──► edgefirst-cli → crates.io
```

**Key Features**:
- Automatic version extraction from git tag
- **Verifies Cargo.toml version matches tag** (fails if mismatch)
- Parallel builds for all platforms
- Bundles CLI binary with Python wheel
- Intelligent build directory detection (handles both native and cross-compilation paths)
- Binaries are automatically stripped by cargo --release (no separate strip step needed)
- Publishes to three destinations:
  - crates.io (Rust crates)
  - PyPI (Python packages)
  - GitHub Releases (binaries + wheels)

**Important**: The version in `Cargo.toml` must be updated to match the git tag **before** creating the tag. The workflow will verify this and fail if they don't match. For release candidates, use the format `X.Y.ZrcN` (e.g., `1.0.0rc1`) with no separators.

**Secrets Required**:
- `CARGO_TOKEN`: For publishing to crates.io

**Note**: PyPI publishing uses **Trusted Publisher** authentication (OpenID Connect) and does not require an API token. The workflow uses the `pypi` environment with `id-token: write` permission for secure, token-less authentication.

**Artifacts Created**:
- **GitHub Release** with:
  - CLI binaries for 5 platforms (compressed)
  - Python wheels for 5 platforms
  - `THIRD_PARTY.md` - Third-party licenses as markdown table
  - Automatic release notes
- **crates.io** packages:
  - `edgefirst-client` - Rust library crate for EdgeFirst API
  - `edgefirst-cli` - CLI binary (installable via `cargo install edgefirst-cli`)
  - Note: `edgefirst-client-py` is NOT published to crates.io (Python bindings only, distributed via PyPI)
- **PyPI** package:
  - `edgefirst-client` - Python package with bundled CLI binary

---

## Caching Strategy

All workflows use a three-level caching strategy for Rust builds:

```yaml
cache:
  ├── cargo registry   (~/.cargo/registry)
  ├── cargo index      (~/.cargo/git)
  └── build artifacts  (target/)
```

**Cache Key Format**: `{os}-{component}-{lock-file-hash}`

**Benefits**:
- Faster builds (reuse of dependencies)
- Reduced network usage
- Improved reliability

---

## Version Format

The project uses version formats compatible with both Python (PEP 440) and Rust (Cargo/SemVer):

| Type | Format | Examples | PyPI Compatible | crates.io Compatible |
|------|--------|----------|-----------------|---------------------|
| Stable | `X.Y.Z` | `1.0.0`, `2.1.3` | ✅ | ✅ |
| Release Candidate | `X.Y.ZrcN` | `1.0.0rc1`, `2.0.0rc2` | ✅ | ✅ |
| Alpha | `X.Y.ZaN` | `0.1.0a1`, `1.0.0a2` | ✅ | ✅ |
| Beta | `X.Y.ZbN` | `0.1.0b1`, `1.0.0b2` | ✅ | ✅ |

**Important**: Do NOT use separators (dots or hyphens) in pre-release versions. Use `1.0.0rc1`, not `1.0.0-rc.1` or `1.0.0.rc.1`.

---

## Code Quality Analysis

### SonarCloud Integration

The CI workflow includes SonarCloud analysis for continuous code quality monitoring:

**Features**:
- Analyzes both Rust and Python code
- Tracks code coverage (from cargo-llvm-cov and slipcover)
- Detects code smells, bugs, and security vulnerabilities
- Quality gate enforcement for pull requests

**Configuration**:
- Project configuration: `sonar-project.properties`
- Organization: `edgefirstai`
- Project key: `EdgeFirstAI_client`

**Quality Gate**:
- Runs automatically on all pull requests
- Blocks merge if quality standards not met
- Timeout: 5 minutes
- Only fails on pull requests (not on main branch pushes)

**Metrics Tracked**:
- Code coverage (Rust and Python)
- Maintainability rating
- Reliability rating
- Security rating
- Code duplication
- Technical debt

---

## Testing Strategy

### Continuous Testing (CI)

```
Every Push/PR
     │
     ├──► cargo fmt check
     ├──► cargo clippy
     ├──► cargo audit
     ├──► cargo test (with coverage)
     ├──► cargo test --doc
     ├──► Python unittest (with coverage)
     ├──► cargo nextest (enhanced reporting)
     └──► SonarCloud analysis (with quality gate for PRs)
```

### Integration Testing (Python Wheels)

```
Wheel Build
     │
     └──► Test on native platform
           ├── Install wheel
           ├── Install test dependencies
           └── Run unittest
```

### Pre-Release Testing

Before creating a release tag, manually test:
1. Trigger build workflows
2. Download and test artifacts
3. Verify documentation builds

---

## Coverage Reporting

```
Test Execution
     │
     ├──► Rust Coverage (cargo llvm-cov)
     │       └──► lcov.info
     │
     └──► Python Coverage (slipcover)
             └──► coverage.xml
                  │
                  ├──► Upload to Codecov
                  │    ├── Flag: rust
                  │    └── Flag: python
                  │
                  └──► Upload to SonarCloud
                       └── Integrated with code analysis
```

**Codecov Configuration**: See `codecov.yml`

**SonarCloud Configuration**: See `sonar-project.properties`

---

## Release Process

### For Maintainers

> **Note**: This project uses [cargo-release](https://github.com/crate-ci/cargo-release) for automated version management and tagging. See [CONTRIBUTING.md](../CONTRIBUTING.md#release-process) for complete details.

**1. Install cargo-release** (if not already installed)
```bash
cargo install cargo-release
```

**2. Update CHANGELOG.md**
```bash
# Add release notes for the new version
# Edit CHANGELOG.md manually
```

**3. Run cargo-release**

For **stable releases**:
```bash
# Patch release (e.g., 2.2.2 → 2.2.3)
cargo release patch --execute --no-confirm

# Minor release (e.g., 2.2.3 → 2.3.0)
cargo release minor --execute --no-confirm

# Major release (e.g., 2.3.0 → 3.0.0)
cargo release major --execute --no-confirm
```

For **release candidates** (rarely used):
```bash
# Manually edit version to use rcN format (e.g., 2.3.0rc1)
sed -i '' 's/version = "2.2.2"/version = "2.3.0rc1"/' Cargo.toml
sed -i '' 's/edgefirst-client = { version = "2.2.2"/edgefirst-client = { version = "2.3.0rc1"/' Cargo.toml
cargo release 2.3.0rc1 --execute --no-confirm
```

**4. Push changes and tags**
```bash
git push && git push --tags
```

**Important**: 
- The workflow will fail if the tag version doesn't match the version in `Cargo.toml`
- cargo-release automatically updates all workspace crates and creates the git tag locally
- Tags use format `X.Y.Z` (no "v" prefix)

**5. Monitor Workflow**
- Go to Actions tab
- Watch "Release" workflow
- Verify all jobs complete successfully

**6. Verify Release**
- Check GitHub release page
- Verify crates.io publication
- Verify PyPI publication
- Test downloads

### What cargo-release Does

When you run `cargo release`, it automatically:
1. Updates workspace version in root `Cargo.toml`
2. Updates workspace dependency version for `edgefirst-client`
3. Updates all crate versions (inherited via `version.workspace = true`)
4. Updates `Cargo.lock`
5. Creates commit: "Release X.Y.Z Preparations"
6. Creates git tag: `X.Y.Z` (locally, not pushed)

Configuration is in `release.toml`:
- Only allows releases from `main` branch (safety)
- Uses tag format `X.Y.Z` without "v" prefix
- Disables automatic publishing (handled by CI)
- Disables automatic pushing (manual control for review)

### GitHub Actions Workflow

After pushing the tag, the workflow automatically:
1. Verifies Cargo.toml version matches the tag
2. Creates GitHub release
3. Builds CLI binaries (5 platforms)
4. Builds Python wheels (5 platforms)
5. Publishes to crates.io
6. Publishes to PyPI
7. Uploads all artifacts to GitHub release

**Note**: If the version verification fails (tag doesn't match Cargo.toml), the entire workflow will fail immediately.

---

## Environment Variables

### Common Environment Variables

```yaml
CARGO_TERM_COLOR: always    # Colored cargo output
RUST_BACKTRACE: 1           # Full backtraces on error
```

### Job-Specific Variables

**Testing Jobs**:
```yaml
STUDIO_SERVER: test         # Test environment
STUDIO_USERNAME: ${{ secrets.STUDIO_USERNAME }}
STUDIO_PASSWORD: ${{ secrets.STUDIO_PASSWORD }}
```

---

## Secrets Management

### Required Secrets

| Secret | Purpose | Required For |
|--------|---------|--------------|
| `CODECOV_TOKEN` | Upload coverage | CI workflow |
| `SONAR_TOKEN` | SonarCloud analysis and quality gate | CI workflow |
| `CARGO_TOKEN` | Publish to crates.io | Release workflow |
| `STUDIO_USERNAME` | Run tests | CI, Python workflows |
| `STUDIO_PASSWORD` | Run tests | CI, Python workflows |

### Optional Secrets

None currently required.

**Note**: PyPI publishing uses Trusted Publisher (OIDC) authentication and does not require an API token.

### Setting Secrets

1. Go to repository Settings
2. Navigate to Secrets and variables → Actions
3. Click "New repository secret"
4. Add each secret

---

## Monitoring and Debugging

### Viewing Workflow Runs

1. Go to Actions tab in repository
2. Select workflow from left sidebar
3. Click on specific run to see details

### Debugging Failed Jobs

1. Click on failed job
2. Expand failed step
3. Review logs
4. Check for:
   - Missing secrets
   - Compilation errors
   - Test failures
   - Network issues

### Re-running Workflows

- Click "Re-run jobs" button
- Select "Re-run failed jobs" or "Re-run all jobs"

---

## Best Practices

### For Contributors

1. **Always run locally first**:
   ```bash
   cargo fmt --all
   cargo clippy --all-targets
   cargo test
   ```

2. **Test on your platform**:
   - Build CLI
   - Build Python wheel
   - Run tests

3. **Keep PRs focused**:
   - One feature/fix per PR
   - Include tests
   - Update documentation

### For Maintainers

1. **Review workflow runs**:
   - Check all jobs pass
   - Review coverage reports
   - Check for security issues

2. **Test before releasing**:
   - Manual workflow dispatch
   - Verify artifacts
   - Test installations

3. **Monitor releases**:
   - Watch workflow completion
   - Verify publications
   - Test downloads

---

## Support

For questions about workflows:
- Review this document
- Check `CONTRIBUTING.md` for development guidelines
- Open an issue if problems persist
