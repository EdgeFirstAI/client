#!/usr/bin/env bash
#
# Reports which JSON-RPC methods the client calls without a wiremock test, and
# fails when a new one appears.
#
# Why this exists: an unmocked RPC method is only exercised by the Studio
# integration suite, which runs nightly against a live server. A break there
# surfaces hours after the merge that caused it, against a commit range rather
# than a pull request. Mocking the wire path moves that signal into the PR lane,
# where it costs seconds.
#
# The check is a ratchet, not a gate on the current total. The baseline lists
# methods already unmocked when this was introduced; the build fails only if a
# method that is NOT in the baseline turns up unmocked. Adding a client call
# therefore obliges you to mock it. Removing an entry from the baseline as you
# add its mock is what makes the number fall.
#
# Usage:
#   .github/scripts/rpc-mock-coverage.sh                     # ratchet check (CI)
#   .github/scripts/rpc-mock-coverage.sh --list              # print gaps, exit 0
#   .github/scripts/rpc-mock-coverage.sh --dve-database DIR  # cross-review vs server
#
# The --dve-database cross-review is deliberately NOT part of CI: that repository
# is private and absent from the runner. Run it locally when adding an API, to
# confirm the method you are calling is one the server actually registers.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
BASELINE="$ROOT/.github/rpc-mock-baseline.txt"
SRC="$ROOT/crates/edgefirst-client/src"
TESTS="$ROOT/crates/edgefirst-client/tests"

# Indent a multi-line list for display. shellcheck suggests
# ${var//search/replace} over sed (SC2001), but parameter expansion has no line
# anchor, so prefixing every line that way needs a read loop -- more code, and
# it mangles trailing whitespace. One sed, suppressed once, in one place.
indent() {
  # shellcheck disable=SC2001
  sed 's/^/  /'
}

MODE="check"
DVE_DIR=""
while [ $# -gt 0 ]; do
  case "$1" in
    --list) MODE="list"; shift ;;
    --dve-database) DVE_DIR="${2:-}"; shift 2 ;;
    # Print the header block: every leading comment line, stopping at the first
    # line that is not one. Derived rather than a fixed range, so editing the
    # header cannot silently leak code into the help output.
    -h|--help) sed -n '2,/^[^#]/p' "$0" | grep '^#' | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

# Wrappers whose call sites pass a literal method name. This list is maintained
# by hand -- nothing derives it -- so assert_wrapper_list_is_current() below
# cross-checks it against the source rather than trusting it.
RPC_FNS='rpc|rpc_bulk|rpc_without_auth|rpc_download|post_multipart'

# Functions that also take a method name but never receive it as a literal:
# rpc_with_http is the shared implementation the wrappers above delegate to, and
# process_rpc_response only needs the name to label an error. Listing them keeps
# the cross-check quiet about functions that are correctly absent from RPC_FNS.
NON_WRAPPER_FNS='rpc_with_http|process_rpc_response'

# Methods the client calls. Joined with the following line first, because many
# call sites wrap after the opening paren, and tolerant of a turbofish
# (`.rpc::<(), Vec<Project>>("project.list", ...)`) between the name and the arg.
client_methods() {
  grep -rhA1 -E "\.($RPC_FNS)" "$SRC" \
    | tr '\n' ' ' \
    | grep -oE "\.($RPC_FNS)(::<[^\"]*>)?\([[:space:]]*\"[^\"]+\"" \
    | grep -oE '"[^"]+"$' | tr -d '"' | sort -u
}

# Methods with at least one wiremock test. Covers the three ways a mock names
# its method: the rpc_method_body helper, an inline body_partial_json, and the
# query-string form used by multipart uploads.
mocked_methods() {
  {
    grep -rhoE 'rpc_method_body\("[^"]+"' "$TESTS" | grep -oE '"[^"]+"' | tr -d '"'
    grep -rhoE '"method":[[:space:]]*"[^"]+"' "$TESTS" | grep -oE '"[^"]+"$' | tr -d '"'
    grep -rhoE 'query_param\("method",[[:space:]]*"[^"]+"' "$TESTS" | grep -oE '"[^"]+"$' | tr -d '"'
    # Filtered to things shaped like a method name. The file's header comment
    # documents the wire format with a `"method": "<name>"` example, which the
    # pattern above would otherwise collect as a mocked method called "<name>".
  } 2>/dev/null | grep -E '^[a-z][a-z_0-9]*(\.[a-z][a-z_0-9]*)*$' | sort -u
}

# Cross-check RPC_FNS against the source, in both directions.
#
# This exists because RPC_FNS is hand-maintained and the rest of the script
# cannot detect its own blind spot: a wrapper missing from the list is invisible
# to client_methods() AND to assert_extraction_is_complete(), since both filter
# through the same regex. Counting call sites cannot reveal a wrapper the count
# never looked for. So this looks at function *definitions* instead, which the
# list does not filter.
#
# It earns its keep: the list shipped with `rpc_with_retry` in it, a function
# that does not exist, while `rpc_with_http` did exist and was absent.
assert_wrapper_list_is_current() {
  local defined unknown missing
  # Functions taking a method name. Newlines collapsed first because these
  # signatures wrap across several lines.
  defined=$(tr '\n' ' ' < "$SRC/client.rs" \
    | grep -oE 'async fn [a-z_0-9]+[^{]{0,200}method: *(String|&str)' \
    | grep -oE 'async fn [a-z_0-9]+' | awk '{print $3}' | sort -u)

  unknown=$(echo "$defined" | grep -vE "^($RPC_FNS|$NON_WRAPPER_FNS)$" || true)
  if [ -n "$unknown" ]; then
    echo "::warning::rpc-mock-coverage: these take a method name but are in neither RPC_FNS nor NON_WRAPPER_FNS:"
    echo "$unknown" | indent
    echo "::warning::If any is called with a literal method name, its calls are invisible to this check. Add it to RPC_FNS."
  fi

  # The other direction: a name in RPC_FNS that no longer exists silently
  # narrows the regex and looks like thoroughness.
  missing=$(echo "$RPC_FNS" | tr '|' '\n' | while read -r fn; do
    grep -qE "async fn $fn[<(]" "$SRC/client.rs" || echo "$fn"
  done)
  if [ -n "$missing" ]; then
    echo "::warning::rpc-mock-coverage: RPC_FNS names functions that do not exist: $(echo "$missing" | tr '\n' ' ')"
  fi
}

# Guard the extractor itself. If call sites exist that the regex cannot resolve
# to a literal, the gap list is silently short and this check is worthless --
# so say so loudly rather than report a clean run.
#
# Note this only covers wrappers already in RPC_FNS; one missing from the list
# is caught by assert_wrapper_list_is_current() above, not here.
assert_extraction_is_complete() {
  local call_sites literals
  # Count occurrences, not matching lines: the literal pass joins everything
  # onto one line, so `grep -c` there would always answer 1.
  call_sites=$(grep -rhoE "\.($RPC_FNS)(::<[^\"]*>)?\(" "$SRC" | wc -l | tr -d ' ')
  literals=$(grep -rhA1 -E "\.($RPC_FNS)" "$SRC" | tr '\n' ' ' \
    | grep -oE "\.($RPC_FNS)(::<[^\"]*>)?\([[:space:]]*\"[^\"]+\"" | wc -l | tr -d ' ')
  if [ "$literals" -lt "$call_sites" ]; then
    echo "::warning::rpc-mock-coverage: $call_sites RPC call sites but only $literals resolved to a literal method name."
    echo "::warning::A call built from a variable cannot be checked. Either inline the method name or extend this script."
  fi
}

CLIENT=$(client_methods)
MOCKED=$(mocked_methods)
GAPS=$(comm -23 <(echo "$CLIENT") <(echo "$MOCKED"))

TOTAL=$(echo "$CLIENT" | grep -c . || true)
COVERED=$(comm -12 <(echo "$CLIENT") <(echo "$MOCKED") | grep -c . || true)
MISSING=$(echo "$GAPS" | grep -c . || true)

assert_wrapper_list_is_current
assert_extraction_is_complete

if [ "$MODE" = "list" ] || [ -n "$DVE_DIR" ]; then
  echo "RPC methods called by the client : $TOTAL"
  echo "  with a wiremock test           : $COVERED"
  echo "  without                        : $MISSING"
  echo
  if [ "$MISSING" -gt 0 ]; then
    echo "Unmocked:"
    echo "$GAPS" | indent
    echo
  fi
fi

# Cross-review against the server. Catches the two drifts the mock check cannot:
# a client calling something the server does not register, and server APIs the
# client has never wired up.
if [ -n "$DVE_DIR" ]; then
  if [ ! -d "$DVE_DIR/api" ]; then
    echo "error: $DVE_DIR does not look like a dve-database checkout (no api/)" >&2
    exit 2
  fi
  SERVER=$(grep -rhoE 'EndPoint:[[:space:]]+"[^"]+"' "$DVE_DIR/api" \
    | grep -oE '"[^"]+"' | tr -d '"' | sort -u)

  echo "Server endpoints registered      : $(echo "$SERVER" | grep -c . || true)"
  echo

  UNKNOWN=$(comm -23 <(echo "$CLIENT") <(echo "$SERVER"))
  if [ -n "$UNKNOWN" ]; then
    echo "Client calls NOT registered by the server (drift, or renamed server-side):"
    echo "$UNKNOWN" | indent
    echo
  fi

  UNUSED=$(comm -13 <(echo "$CLIENT") <(echo "$SERVER"))
  echo "Server endpoints the client does not call: $(echo "$UNUSED" | grep -c . || true)"
  echo "  (informational -- most are UI or internal, not gaps)"
  exit 0
fi

[ "$MODE" = "list" ] && exit 0

# Ratchet.
if [ ! -f "$BASELINE" ]; then
  echo "error: missing baseline at $BASELINE" >&2
  exit 2
fi
KNOWN=$(grep -vE '^\s*(#|$)' "$BASELINE" | sort -u)

NEW=$(comm -23 <(echo "$GAPS") <(echo "$KNOWN"))
if [ -n "$NEW" ]; then
  echo "::error::New RPC methods are called without a wiremock test."
  echo
  echo "$NEW" | indent
  echo
  echo "Add a test to crates/edgefirst-client/tests/wiremock_coverage.rs covering"
  echo "the success path and at least one error path for each. Cross-check the"
  echo "server's shape first:"
  echo
  echo "  .github/scripts/rpc-mock-coverage.sh --dve-database ~/Software/Studio/dve-database"
  echo
  echo "Deliberately shipping without one? Add it to $BASELINE with a reason."
  exit 1
fi

# Stale entries mean somebody added a mock without claiming the credit. Not a
# failure, but left unpruned the baseline stops shrinking and the ratchet slips.
STALE=$(comm -13 <(echo "$GAPS") <(echo "$KNOWN"))
if [ -n "$STALE" ]; then
  echo "::notice::These baseline entries are now mocked and can be removed from $BASELINE:"
  echo "$STALE" | indent
fi

echo "rpc-mock-coverage: $COVERED/$TOTAL client RPC methods mocked, $MISSING known gaps, 0 new."
