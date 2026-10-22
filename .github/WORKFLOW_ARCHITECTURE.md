# GitHub Actions Workflow Architecture

This document provides a comprehensive overview of the GitHub Actions workflows for the EdgeFirst Client project.

## Workflow Overview

```
 Push / PR to main           Push release/X.Y.Z        Release PR merged     Tag vX.Y.Z
        │                           │                          │                  │
        ├──► test.yml               ▼                          ▼                  ▼
        │                      release.yml               tag-release.yml     publish.yml
        ├──► build.yml ◄─────── artifacts (called)       requires a green    resolves the
        │                       sbom (called)            release.yml run     release.yml run,
        └──► sbom.yml  ◄─────── verify (shared)          for the PR head,    binds it by tree,
                                manpage                  then creates the    publishes it
                                                         annotated tag
```

**A tag publishes; it never builds.** Every file a release ships is built by `release.yml` on the release branch, so a broken build is a red release PR rather than a failed deploy after the tag already exists. `release.yml` calls the same `build.yml` and `sbom.yml` that every pull request runs, so what ships is what was tested. Manual dispatch is available for `studio.yml` and for a publish rehearsal.

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

### 3. Release Chain (`release.yml` → `tag-release.yml` → `publish.yml`)

The chain follows SPS 10 / SPS 11 and uses the shared workflows in `EdgeFirstAI/.github`, pinned by SHA.

```
release.yml        push to release/*.*.*       BUILD every artifact
├── verify         release-rust.yml (shared): version vs branch, workspace
│                  versions, CHANGELOG section, cargo package --locked
│                  for edgefirst-client and edgefirst-cli
├── artifacts      build.yml (retention 30): 5 CLI archives, 5 wheels
├── sbom           sbom.yml  (retention 30): sbom.json under the license policy
└── manpage        pandoc CLI.md → edgefirst-client.1

tag-release.yml    release PR merged           TAG
└── tag            tag-release.yml (shared, require-build: release.yml)

publish.yml        push of v*.*.* tag          PUBLISH, builds nothing
├── publish        publish-rust.yml (shared)
│   ├── resolve        latest green release.yml run on release/X.Y.Z,
│   │                  tree(tag) == tree(build), artifacts not expired
│   ├── crates         cargo publish --no-verify (crates.io trusted publishing)
│   └── github-release CHANGELOG notes + every artifact attached
└── publish-pypi   stage-pypi (shared) + pypa/gh-action-pypi-publish
```

**Binding by tree.** The release PR merges with a new commit SHA, but when the release branch is up to date with `main` the merge commit has the same tree as the branch head that was built. `publish.yml` requires that equality, so keep the release branch up to date with `main` before merging. A mismatch fails the publish; it never triggers a rebuild.

**Rehearsal.** `gh workflow run publish.yml --ref release/X.Y.Z -f tag=vX.Y.Z` resolves, verifies and stages everything against the green release branch and uploads nothing. Run it after any change to `release.yml` or `publish.yml`, before merging that release PR. It cannot detect a stale Trusted Publisher, because it skips the upload.

**Trusted Publishers** match on the workflow filename and environment:

| Registry | Project | Workflow | Environment |
|---|---|---|---|
| PyPI | `edgefirst-client` | `publish.yml` | `pypi` |
| crates.io | `edgefirst-client`, `edgefirst-cli` | `publish.yml` | `crates-io` |

**What ships**:

- **GitHub Release**: `edgefirst-client-<platform>-X.Y.Z.tar.gz` / `.zip` for 5 platforms, 5 wheels, `sbom.json`, `edgefirst-client.1`, and the packaged `.crate` files, with notes from the `CHANGELOG.md` section
- **crates.io**: `edgefirst-client` (library) and `edgefirst-cli` (`cargo install edgefirst-cli`). `edgefirst-client-py` is `publish = false`.
- **PyPI**: `edgefirst-client`, with the CLI bundled in each wheel

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
| Release Candidate | `X.Y.Z-rcN` (Cargo, branch, tag) | `1.0.0-rc1` | ✅ as `1.0.0rc1` | ✅ |

Release candidates use the `release/X.Y.Z-rcN` branch and `vX.Y.Z-rcN` tag. maturin writes the PEP 440 spelling (`X.Y.ZrcN`) into the wheel. If the manifests carry the base version `X.Y.Z` instead, `publish.yml` creates a GitHub pre-release and skips crates.io and PyPI, so the final release can still claim that version.

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
3. Run `make pre-release`, commit with DCO, push the branch, and open `Release X.Y.Z` against `main`. The push runs `release.yml`, which builds every artifact the release will ship. A fixup push rebuilds.
4. Optionally rehearse the publish against the green branch: `gh workflow run publish.yml --ref release/X.Y.Z -f tag=vX.Y.Z`. This is required after any change to `release.yml` or `publish.yml`.
5. Bring the branch up to date with `main` if it has moved, then merge after required review and checks. Do not create or push a tag manually.
6. `tag-release.yml` creates annotated tag `vX.Y.Z` on the merge commit, but only if `release.yml` is green for the merged release-branch head.
7. The tag triggers `publish.yml`, which publishes the release build's artifacts to crates.io, PyPI and the GitHub Release. It builds nothing.

Do not delete the release branch until `publish.yml` has finished, because the publish resolves the build by branch name. A failed publish is fixed forward with the next PATCH release; tags are never moved or recreated.

After publication, verify the Publish workflow, GitHub assets, PyPI package, and crates.io crates.

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
| `RELEASE_TAG_TOKEN` | Create the release tag so it triggers `publish.yml` (organization secret) | Tag Release workflow |
| `STUDIO_USERNAME` | Run tests | Test workflow |
| `STUDIO_PASSWORD` | Run tests | Test workflow |

### Optional Secrets

None currently required.

**Note**: PyPI and crates.io publishing use Trusted Publisher (OIDC) authentication and need no API token.

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
