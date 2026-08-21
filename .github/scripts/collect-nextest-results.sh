#!/usr/bin/env bash
#
# Collect test results from a `cargo nextest` run.
#
# nextest already writes a correct JUnit report when run under `--profile ci`
# (see the `junit` key in .config/nextest.toml). Earlier revisions of the
# workflow ignored that file and rebuilt a JUnit document by grepping stdout
# for libtest's `test NAME ... ok` / `test result:` lines. nextest does not
# emit either format -- it prints `PASS [ 0.393s] (83/94) ...` and a trailing
# `Summary [...]` line -- so those greps never matched and every report was
# published as `tests="0" failures="0"`, for green runs and red runs alike.
# The job still went red (the gate reads the process exit code, not these
# counts), which is precisely why it went unnoticed: the signal was right and
# only the diagnostics were empty.
#
# So: copy the report nextest already produced, and derive the summary counts
# from it rather than from console output.
#
# Usage:
#   collect-nextest-results.sh <suite-label> <log-file> <dest-junit> <exit-code>
#
# Writes passed/failed/ignored/total to $GITHUB_OUTPUT.

set -euo pipefail

SUITE_LABEL="$1"
LOG_FILE="$2"
DEST_JUNIT="$3"
TEST_EXIT_CODE="${4:-0}"

# Path is fixed by the profile name passed to `cargo nextest --profile ci`.
NEXTEST_JUNIT="target/nextest/ci/junit.xml"

mkdir -p "$(dirname "$DEST_JUNIT")"

if [ -f "$NEXTEST_JUNIT" ]; then
  cp "$NEXTEST_JUNIT" "$DEST_JUNIT"
else
  # No report means nextest never got as far as running tests -- a compile
  # error, a panic in a build script, a cancelled job. Emit a report that says
  # so. Writing an empty-but-valid document here would reproduce the exact bug
  # this script exists to fix: a red run that reports zero failures.
  echo "WARNING: $NEXTEST_JUNIT not found; nextest did not complete a run." >&2
  cat > "$DEST_JUNIT" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<testsuites name="$SUITE_LABEL" tests="1" failures="0" errors="1">
  <testsuite name="$SUITE_LABEL" tests="1" failures="0" errors="1">
    <testcase name="nextest-run" classname="$SUITE_LABEL">
      <error message="nextest produced no JUnit report (exit code $TEST_EXIT_CODE)">The test run did not complete. See the job log for the underlying build or runner failure.</error>
    </testcase>
  </testsuite>
</testsuites>
EOF
fi

# `tests` counts only what actually ran; filtered-out and #[ignore]d tests are
# absent from the document entirely, so the skipped count comes from the
# console summary below.
read -r TOTAL FAILED < <(
  python3 - "$DEST_JUNIT" <<'PY'
import sys
import xml.etree.ElementTree as ET

try:
    root = ET.parse(sys.argv[1]).getroot()
except (ET.ParseError, OSError) as exc:
    print(f"failed to parse JUnit report: {exc}", file=sys.stderr)
    # Report one error rather than a clean zero, for the reason above.
    print("1 1")
    sys.exit(0)

tests = int(root.get("tests", 0))
failures = int(root.get("failures", 0)) + int(root.get("errors", 0))
print(f"{tests} {failures}")
PY
)

PASSED=$((TOTAL - FAILED))

# CARGO_TERM_COLOR=always means the log carries SGR escapes, so strip them
# before matching. The summary reads e.g.
#   Summary [ 688.766s] 94 tests run: 90 passed (5 slow), 4 failed, 4 skipped
# and the "N skipped" segment is the only place the ignored count appears.
IGNORED=0
if [ -f "$LOG_FILE" ]; then
  IGNORED=$(
    sed -e 's/\x1b\[[0-9;]*m//g' "$LOG_FILE" \
      | grep -oE '[0-9]+ skipped' \
      | tail -1 \
      | grep -oE '[0-9]+' \
      || echo 0
  )
  IGNORED=${IGNORED:-0}
fi

{
  echo "passed=$PASSED"
  echo "failed=$FAILED"
  echo "ignored=$IGNORED"
  echo "total=$TOTAL"
} >> "$GITHUB_OUTPUT"

echo "$SUITE_LABEL: $PASSED passed, $FAILED failed, $IGNORED skipped ($TOTAL run)"
