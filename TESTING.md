# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.

# Testing Guide — Dataset Versioning

This guide describes how to test the dataset versioning feature introduced in
the `feature/DE-2439-dataset-versioning` branch. It covers CLI manual testing,
automated Python integration tests, and Rust unit/integration tests.

## Prerequisites

- Access to an EdgeFirst Studio test server (`test.edgefirst.studio`)
- Credentials for a test user with dataset write access
- A project named **Unit Testing** (required for automated tests)
- Rust toolchain (`stable`) and `cargo` installed
- Python venv with `maturin` and `edgefirst-client` wheel installed

## Environment Setup

All commands require the project's shell environment to be sourced first:

```bash
source env.sh
```

This sets `STUDIO_SERVER`, `STUDIO_USERNAME`, and `STUDIO_PASSWORD` from
your local configuration and activates the Python venv.

---

## 1. CLI Manual Testing

Build the CLI binary before testing:

```bash
cargo build --release -p edgefirst-cli
```

The binary is at `target/release/edgefirst-client`. The examples below use
`edgefirst-client` — replace with the full path if it is not on your `PATH`.

### 1.1 Setup: Create a Test Dataset

```bash
# List available projects to find a project ID
edgefirst-client projects

# Create a temporary test dataset (replace PROJECT_ID)
edgefirst-client create-dataset PROJECT_ID "Versioning Test" \
    --description "Manual versioning test"

# The command prints the new dataset ID — save it:
DATASET_ID=ds-xxxxxx

# Create an annotation set for the dataset
edgefirst-client create-annotation-set $DATASET_ID "Default"

# Upload a small sample to ensure the changelog has entries
# (the dataset must have at least one logged change before tagging)
edgefirst-client upload-dataset $DATASET_ID --images /path/to/images/
```

### 1.2 Tag Management

```bash
# Create a version tag at the current dataset state
edgefirst-client version tag create $DATASET_ID v1.0 -d "Initial version"

# Verify the tag was created
edgefirst-client version tag list $DATASET_ID

# Fetch details of the tag
edgefirst-client version tag get $DATASET_ID v1.0

# Create a second tag
edgefirst-client version tag create $DATASET_ID v1.1 -d "Second version"

# List should now show both tags
edgefirst-client version tag list $DATASET_ID

# Delete the draft tag
edgefirst-client version tag delete $DATASET_ID v1.1

# List should show only v1.0 again
edgefirst-client version tag list $DATASET_ID
```

### 1.3 Download at a Tagged Version

```bash
# Download dataset files from the tagged state (not current HEAD)
edgefirst-client download-dataset $DATASET_ID \
    --tag v1.0 --types image --output /tmp/tagged-data

# Verify files are present
ls /tmp/tagged-data/

# Download annotations at the same tagged state
ANNOTATION_SET_ID=as-yyyyyy   # from create-annotation-set output
edgefirst-client download-annotations $ANNOTATION_SET_ID /tmp/tagged-annotations.arrow \
    --tag v1.0 --types box2d
```

### 1.4 Changelog and Version Info

```bash
# Show the full changelog for the dataset
edgefirst-client version changelog $DATASET_ID

# Limit output to 10 entries
edgefirst-client version changelog $DATASET_ID --limit 10

# Filter to annotation changes only
edgefirst-client version changelog $DATASET_ID --types annotation

# Range query between two tags (both endpoints inclusive)
edgefirst-client version changelog $DATASET_ID --from v1.0 --to v1.0

# Range query using serial numbers
edgefirst-client version changelog $DATASET_ID --from 1 --to 10

# Show current serial number, all tags, and summary
edgefirst-client version current $DATASET_ID

# Show cached dataset metrics (image count, annotation counts, etc.)
edgefirst-client version summary $DATASET_ID
```

### 1.5 Restore a Dataset to a Tagged State

```bash
# Add more data to HEAD so there is something to undo
edgefirst-client upload-dataset $DATASET_ID --images /path/to/more-images/

# Verify HEAD now has more samples
edgefirst-client version summary $DATASET_ID

# Restore to the v1.0 tag — discards all changes after that serial
edgefirst-client version tag restore $DATASET_ID v1.0

# Confirm dataset is back to the state at v1.0
edgefirst-client version summary $DATASET_ID
edgefirst-client version current $DATASET_ID
```

### 1.6 Cleanup

```bash
# Delete the test tag (optional — dataset deletion cascades)
edgefirst-client version tag delete $DATASET_ID v1.0

# Delete the test dataset entirely
edgefirst-client delete-dataset $DATASET_ID
```

---

## 2. Automated Python Integration Tests

The Python integration tests in `test/test_versioning.py` cover:

- `VersionTagLifecycleTest` — create, list, get, delete tags
- `VersionTaggedDataFetchTest` — verify tagged fetch returns old state after HEAD modifications
- `VersionChangelogTest` — changelog entries are recorded and filterable
- `VersionTagRestoreTest` — restore returns the dataset to the tagged state

### 2.1 Build the Python Bindings

```bash
source env.sh
maturin develop -m crates/edgefirst-client-py/Cargo.toml
```

### 2.2 Run All Versioning Tests

```bash
venv/bin/python -m unittest test.test_versioning -v
```

Expected output:

```
test_tag_lifecycle (test.test_versioning.VersionTagLifecycleTest) ... ok
test_tagged_vs_head_data (test.test_versioning.VersionTaggedDataFetchTest) ... ok
test_changelog_entries (test.test_versioning.VersionChangelogTest) ... ok
test_restore_to_tag (test.test_versioning.VersionTagRestoreTest) ... ok

----------------------------------------------------------------------
Ran 4 tests in X.XXXs

OK
```

### 2.3 Run All Python Tests

```bash
venv/bin/python -m unittest discover -s test -p "test*.py" -v
```

### 2.4 Keep Test Datasets for Inspection

By default, tests delete the datasets they create. Set `SKIP_CLEANUP=1` to
retain them for manual inspection after a test run:

```bash
SKIP_CLEANUP=1 venv/bin/python -m unittest test.test_versioning -v
```

### 2.5 Python Coverage (CI-equivalent)

The CI pipeline uses `slipcover` with `xmlrunner`. To replicate the coverage
report locally:

```bash
source env.sh
maturin develop -m crates/edgefirst-client-py/Cargo.toml
venv/bin/python -m slipcover --xml --out coverage.xml \
    -m xmlrunner discover -s test -p "test*.py" -o target/python
```

---

## 3. Rust Tests

### 3.1 Unit and Library Tests

Run lib tests for the core client crate:

```bash
source env.sh
cargo test -p edgefirst-client --lib --all-features --locked
```

### 3.2 CLI Tests

```bash
cargo test -p edgefirst-cli --all-features --locked
```

### 3.3 All Tests (Single-Threaded to Avoid Conflicts)

Running all crates together can cause test-server conflicts. Use
`--test-threads=1` when running the full suite:

```bash
cargo test --all-features --locked -- --test-threads=1
```

### 3.4 Doc Tests

```bash
cargo test --doc --locked
```

---

## 4. Combined Rust + Python Coverage

To generate a combined coverage report (matches the CI/CD pipeline):

```bash
source env.sh

# Export llvm-cov environment variables
source <(cargo llvm-cov show-env --export-prefix --no-cfg-coverage)

# Build everything under coverage instrumentation
cargo build --all-features --locked
maturin develop -m crates/edgefirst-client-py/Cargo.toml

# Run Python tests with coverage
venv/bin/python -m slipcover --xml --out coverage.xml \
    -m xmlrunner discover -s test -p "test*.py" -o target/python

# Generate LCOV report
cargo llvm-cov report --lcov --output-path lcov.info
```

---

## 5. Makefile Shortcuts

The project Makefile provides convenient targets:

```bash
make test        # Run all tests (Rust + Python)
make rust-test   # Rust tests only
make py-test     # Python tests only
make build       # Build all crates
make pre-commit  # Format + lint + build (run before committing)
```

---

## 6. Troubleshooting

**"Cannot create tag: dataset has no changes yet"**
: The dataset must have at least one changelog entry before a tag can be
created. Upload at least one sample, then retry.

**"Tag 'x' already exists for this dataset"**
: Tag names are unique per dataset (case-sensitive). Delete the existing tag
or choose a different name.

**"Tag 'x' not found"**
: Verify the tag name and dataset ID. Tag names are case-sensitive (`v1.0`
and `V1.0` are different).

**Integration tests fail with authentication errors**
: Confirm `STUDIO_SERVER`, `STUDIO_USERNAME`, and `STUDIO_PASSWORD` are set
correctly after sourcing `env.sh`. External contributors can rely on GitHub
Actions CI to run integration tests automatically.

**Rust tests time out when running all together**
: Run lib and CLI tests separately (`-p edgefirst-client --lib` and
`-p edgefirst-cli`) or pass `-- --test-threads=1`.
