#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Copyright © 2025 Au-Zone Technologies. All Rights Reserved.
#
# Configures an instrumented build environment for cargo-llvm-cov, so that the
# Python bindings and the Rust tests contribute to one coverage report.
#
# Writes the environment to a file rather than exporting it globally. That is
# the whole point, and it is easy to get wrong: `cargo llvm-cov nextest` sets up
# its own instrumentation, and fails outright if it inherits an environment that
# already has one applied. So the file is sourced only by the steps that need it
# -- the maturin build and the Python tests -- and never by the Rust test step.
#
# The two overrides after show-env matter as much as show-env itself:
#
#   CARGO_TARGET_DIR   maturin builds the extension module into the same tree
#                      cargo-llvm-cov uses, so `report` can find the object file
#                      and attribute its counters. Note this moves the wheel to
#                      target/llvm-cov-target/wheels.
#   LLVM_PROFILE_FILE  profraw from the Python process lands where `report`
#                      looks. Without it the files are written somewhere `report`
#                      never scans, and the run reports success having collected
#                      nothing.
#
# Usage:
#   .github/scripts/setup-coverage-env.sh      # once, before any instrumented step
#   source /tmp/coverage-env.sh                # in each step that needs it
#
# Modelled on the equivalent in EdgeFirst/hal, which carries the same pattern
# through a more demanding case (coverage collected from on-target hardware runs
# and merged back for publication).

set -euo pipefail

# Start from a clean slate: stale profraw from an earlier run would be merged
# into this one and silently overstate coverage.
cargo llvm-cov clean --workspace

cargo llvm-cov show-env --export-prefix 2>&1 | grep '^export ' > /tmp/coverage-env.sh

# Single-quoted on purpose (shellcheck SC2016): these must reach the file
# unexpanded and resolve against CARGO_LLVM_COV_TARGET_DIR when it is sourced,
# which is a line show-env wrote above.
# shellcheck disable=SC2016
{
  echo 'export CARGO_TARGET_DIR="$CARGO_LLVM_COV_TARGET_DIR/llvm-cov-target"'
  echo 'export LLVM_PROFILE_FILE="$CARGO_LLVM_COV_TARGET_DIR/llvm-cov-target/client-%p-%16m.profraw"'
} >> /tmp/coverage-env.sh

echo "Coverage environment written to /tmp/coverage-env.sh"
cat /tmp/coverage-env.sh
