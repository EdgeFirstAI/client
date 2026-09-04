# GitHub Actions Workflow Architecture

This document provides a comprehensive overview of the GitHub Actions workflows for the EdgeFirst Client project.

## Workflow Overview

```
 Push / PR to main                      Tag vX.Y.Z[rcN]        Manual
        │                                      │                  │
        ├──► test.yml    lint ─┐               │                  ├──► studio.yml
        │                test ─┴─► sonarcloud  │                  │
        │                                      │                  └──► (any workflow,
        ├──► build.yml   build-cli                                     workflow_dispatch)
        │                    └─► build-wheels ─► verify
        │                                      │
        └──► sbom.yml    sbom-compliance       │
                                               ▼
                                          release.yml
                                               │
                              create-release ──┴── wait-for-ci
                                                        │  resolves the build.yml and
                                                        │  sbom.yml *run ids* for this
                                                        │  commit, then fans out:
                                    ┌───────────────────┼───────────────────┐
                                    ▼                   ▼                   ▼
                              upload-cli          upload-wheels        upload-sbom
                              upload-manpage      publish-pypi         publish-crates-io
```

Release never rebuilds. It waits for the `build.yml` and `sbom.yml` runs that
were triggered by the push to `main`, then downloads their artifacts by run id,
so what ships is byte-identical to what was tested.

A separate `tag-release.yml` creates the tag when a `release/*` PR merges.

## Workflow Files

### 1. Test Workflow (`.github/workflows/test.yml`)

**Purpose**: Code quality checks, testing, coverage, and security audit

**Triggers**:

- Push to `main` branch
- Pull requests to `main` branch
- Manual workflow dispatch

**Jobs**:

```
test.yml
├── lint                         (ubuntu-24.04-xlarge)
│   ├── Audit workflows for script injection
│   ├── Check formatting (cargo fmt --all --check)
│   ├── Clippy (--all-targets --all-features, JSON)
│   └── Upload artifact: clippy-report-linux
├── test                         (ubuntu-24.04-xlarge)
│   ├── Rust doc tests (before instrumentation)
│   ├── cargo llvm-cov show-env  ──► instruments everything below
│   ├── cargo nextest (JUnit via the `ci` profile)
│   ├── maturin build + Python unittest under slipcover
│   ├── Upload artifact: coverage-reports (lcov.info, coverage.xml)
│   └── Publish merged Rust + Python test results
└── sonarcloud                   [needs: lint, test]
    ├── Download coverage-reports and clippy-report-*
    ├── Scan with -Dsonar.rust.clippy.reportPaths=…
    └── Check quality gate (PR only)
```

**Key Features**:

- **Lint split from test**: formatting and clippy report in about a minute instead of behind the whole suite. `cargo fmt` runs first because it needs no build.
- **Rust and Python share one job**: `cargo llvm-cov show-env` is exported into the environment before the maturin build, so the extension module is compiled instrumented and its profraw feeds the same `lcov.info` as the Rust tests. Splitting them would mean a second instrumented build of the workspace for no coverage gain.
- **Clippy is imported by SonarCloud, not re-run**: the scan used to compile the workspace with clippy a second time and report a different set of lints than CI enforces. The `lint` job emits `--message-format=json` and the scan consumes it.
- **No dependency-audit job**: SonarCloud covers dependency advisories. `make security-audit` remains for local pre-commit use.
- **Enhanced test reporting**: cargo-nextest with JUnit XML output, merged with the Python results into one check.

**Secrets Required**:

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

- `coverage-reports`: `lcov.info` and `coverage.xml`
- `clippy-report-linux`: clippy diagnostics in JSON, consumed by the `sonarcloud` job

---

### 2. Build Workflow (`.github/workflows/build.yml`)

**Purpose**: Build CLI binaries and Python wheels for multiple platforms (unified workflow)

**Triggers**:

- Push to `main` branch
- Pull requests to `main` branch
- Manual workflow dispatch

**Jobs**:

```
build.yml
├── build-cli (matrix)
│   ├── Linux x86_64
│   ├── Linux aarch64
│   ├── macOS x86_64
│   ├── macOS aarch64
│   └── Windows x86_64
├── build-wheels (matrix) [needs: build-cli]
│   ├── Linux x86_64 (manylinux2014)
│   ├── Linux aarch64 (manylinux2014)
│   ├── macOS x86_64
│   ├── macOS aarch64
│   └── Windows x86_64
└── verify [needs: build-cli, build-wheels]
    └── Download and verify all artifacts
```

**Matrix Strategy** (same for CLI and wheels):

| Runner | `platform` | Target | CLI Output | Wheel Platform |
|---|---|---|---|---|
| ubuntu-24.04 | linux | x86_64-unknown-linux-gnu | edgefirst-client-linux-amd64 | wheels-linux-x86_64 |
| ubuntu-24.04 | linux | aarch64-unknown-linux-gnu | edgefirst-client-linux-arm64 | wheels-linux-aarch64 |
| macos-14 | macos | x86_64-apple-darwin | edgefirst-client-macos-amd64 | wheels-macos-x86_64 |
| macos-14 | macos | aarch64-apple-darwin | edgefirst-client-macos-arm64 | wheels-macos-aarch64 |
| windows-2025 | windows | x86_64-pc-windows-msvc | edgefirst-client-windows-amd64.exe | wheels-windows-x86_64 |

Steps branch on the `platform` field, never on the runner label. Keying on
`matrix.os == 'ubuntu-latest'` meant that repinning a runner would silently skip
the `cargo-zigbuild` step and the glibc gate below and fall through to a plain
`cargo build` — publishing a binary against the runner's much newer glibc with a
green check. `platform` describes what is being built, so a runner bump cannot
disable a build step.

**Linux aarch64 is cross-compiled on x86_64**, deliberately. `cargo-zigbuild`
targets `…-gnu.2.17` to hold the manylinux2014 glibc floor, and an `objdump`
gate fails the build if any symbol requires more. Moving these lanes to native
ARM runners would raise the floor to the runner's glibc and break older
distributions.

**Key Features**:

- **Wheels build after CLI**: each wheel bundles its platform's CLI binary, so the dependency is real. Actions cannot express per-matrix-entry `needs`, so the whole wheel matrix waits on the whole CLI matrix; dropping the edge would mean building every CLI twice.
- **Shared cache**: Both CLI and wheels use same cache key per target (`{target}-build`)
- **Cross-compilation**: Uses `cargo-zigbuild` with zig for Linux targets (manylinux2014 compatibility)
- **Explicit --target flag**: All builds use `--target` for consistent `target/<triple>/release/` paths
- **maturin with zig**: Python wheels use maturin[zig] for cross-platform wheel builds
- **Swatinem/rust-cache**: Intelligent caching with incremental compilation state preservation
- **`verify` asserts, not counts**: it checks the expected number of binaries and wheels arrived and fails otherwise. It previously piped `wc -l` into the job summary and always exited 0, so a matrix entry that produced nothing still showed green.

**Artifacts Generated**:

- CLI: Individual binary artifacts per platform (e.g., `edgefirst-client-linux-amd64`)
- Wheels: Platform-specific wheel artifacts (e.g., `wheels-linux-x86_64`)

---

### 3. Release Workflow (`.github/workflows/release.yml`)

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
├── create-release                       Verify Cargo.toml version == tag
├── wait-for-ci      [needs: create-release]
│   ├── Wait for build.yml  ──► outputs build-run-id
│   └── Wait for sbom.yml   ──► outputs sbom-run-id
├── upload-sbom      [needs: wait-for-ci]   sbom.json  ──► release
├── generate-manpage [needs: create-release] pandoc CLI.md ──► release
├── upload-cli       [needs: wait-for-ci]   5 binaries ──► release
├── upload-wheels    [needs: wait-for-ci]   5 wheels   ──► release
├── publish-pypi     [needs: wait-for-ci]   wheels     ──► PyPI (OIDC)
└── publish-crates-io[needs: create-release] cargo publish --workspace
```

`wait-for-ci` runs `.github/scripts/wait-for-run.sh`, which polls
`gh run list` for the workflow's run on this exact commit and returns its
run id. Downstream jobs pass that id to `actions/download-artifact`, which
is what lets them reach artifacts from a different workflow run.

This replaced `lewagon/wait-on-check-action`, which matched on
human-readable **check names**. That coupling was silent and brittle:
renaming or deleting a job left the release blocking on a check that would
never appear, surfacing only as a timeout. Matching on the workflow file
also yields the run id the download step needs, which the check-name
approach could not provide.

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

- `CARGO_REGISTRY_TOKEN`: For publishing to crates.io

**Note**: PyPI publishing uses **Trusted Publisher** authentication (OpenID Connect) and does not require an API token. The workflow uses the `pypi` environment with `id-token: write` permission for secure, token-less authentication.

**Artifacts Created**:

- **GitHub Release** with:
  - CLI binaries for 5 platforms (compressed)
  - Python wheels for 5 platforms
  - `sbom.json` - Software Bill of Materials in CycloneDX format
  - Automatic release notes
- **crates.io** packages:
  - `edgefirst-client` - Rust library crate for EdgeFirst API
  - `edgefirst-cli` - CLI binary (installable via `cargo install edgefirst-cli`)
  - Note: `edgefirst-client-py` is NOT published to crates.io (Python bindings only, distributed via PyPI)
- **PyPI** package:
  - `edgefirst-client` - Python package with bundled CLI binary

---

## Caching Strategy

All workflows use the **Swatinem/rust-cache** action for intelligent, incremental Rust build caching:

```yaml
cache:
  ├── cargo registry   (~/.cargo/registry)
  ├── cargo index      (~/.cargo/git)
  ├── build artifacts  (target/)
  └── incremental compilation state
```

**Cache Key Strategy**: Each workflow/job has a unique cache key to avoid conflicts:

- **Lint**: `lint` (clippy and rustfmt only; no instrumentation)
- **Test**: `test` (nextest and llvm-cov; kept separate so an uninstrumented lint build is never reused by the coverage build)
- **Build (CLI + Python)**: `{target}-build` (per-target architecture, shared between CLI and Python wheel builds)

**Key Features**:

- **Incremental compilation**: Preserves compiler state for faster rebuilds
- **Target-specific caching**: Separate caches for different architectures prevent conflicts
- **Profile-aware**: Different caches for different use cases (lint/test vs release builds)
- **Automatic cleanup**: Old cache entries are pruned automatically
- **Shared cache optimization**: CLI and Python builds use same cache, maximizing incremental compilation benefits

**Benefits**:

- **10x faster builds**: Typical build time reduced from ~10 minutes to ~1 minute on cache hit
- **Sequential build optimization**: Python wheels run after CLI builds, reusing compiled dependencies
- **Reduced CI costs**: Less compute time means lower GitHub Actions usage
- **Better reliability**: Less network dependency, fewer transient failures

**Design Decisions**:

- Lint is split from test so a formatting or clippy failure reports in about a minute, and so its clippy report can be imported by SonarCloud
- Merged Python wheels into build workflow to share cache with CLI builds (serial execution for incremental benefits)
- No dependency-audit job: SonarCloud covers advisories, and `make security-audit` covers the local pre-commit case
- Direct triggers (push/PR) instead of workflow_run for simpler pipeline and parallel execution
- No separate dependency pre-warming workflow as Swatinem/rust-cache handles this efficiently

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
- Clippy findings are **imported** from the `lint` job's JSON report via a
  `-Dsonar.rust.clippy.reportPaths` scan argument. That property is set from the
  workflow rather than the properties file because the artifact path is only
  known at run time. The scan fails loudly if no report is found: passing an
  empty list would make Sonar silently report zero clippy issues.

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
     ├──► cargo clippy (JSON, exported to SonarCloud)
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
                  └──► artifact: coverage-reports
                           │
                           └──► sonarcloud job
                                └── Integrated with code analysis
```

Both languages land in a single `lcov.info` because the Python extension module
is built under the same llvm-cov instrumentation as the Rust tests.

**SonarCloud Configuration**: See `sonar-project.properties`

---

## Release Process

### For Maintainers

See [CONTRIBUTING.md](../CONTRIBUTING.md#release-process) for the release
checklist. The authoritative path is a reviewed release pull request:

1. Create `release/X.Y.Z` from current `main`.
2. Update the workspace and internal dependency versions in `Cargo.toml`,
   regenerate `Cargo.lock`, promote the changelog's `[Unreleased]` entries to a
   dated `[X.Y.Z]` section, and update the `CLI.md` footer/date.
3. Run `make pre-release`, commit with DCO, push the branch, and open
   `Release X.Y.Z` against `main`.
4. Merge only after required review and checks. Do not create or push a tag
   manually.
5. `tag-release.yml` validates the branch name and creates annotated tag
   `vX.Y.Z` on the merge commit.
6. The tag triggers `release.yml`, which waits for `build.yml` and `sbom.yml`
   artifacts from that same commit, creates the GitHub Release, uploads CLI
   binaries/wheels/SBOM/man page, and publishes to PyPI and crates.io.

The release publisher accepts `vX.Y.ZrcN`, but `tag-release.yml` currently
validates stable `release/X.Y.Z` branches only. Release candidates therefore
require an explicit workflow update before using this automated path.

After publication, verify the Release workflow, GitHub assets, PyPI package,
and crates.io crate. A version mismatch between `Cargo.toml`, the release
branch, and the generated tag fails the workflow.

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
| `SONAR_TOKEN` | SonarCloud analysis and quality gate | Test workflow |
| `CARGO_REGISTRY_TOKEN` | Publish to crates.io | Release workflow |
| `STUDIO_USERNAME` | Run tests | Test workflow |
| `STUDIO_PASSWORD` | Run tests | Test workflow |

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
