#!/usr/bin/sh
# Compare all CASE lines in a log (sorted set) to probes/expected/<probe>.cases
# Usage: diff-guest-cases.sh <probe_basename> <log_file>
set -eu
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PKG="$(cd "$SCRIPT_DIR/.." && pwd)"

probe="${1:?usage: $0 <probe_basename> <log_file>}"
logf="${2:?usage: $0 <probe_basename> <log_file>}"
exp="$PKG/probes/expected/${probe}.cases"
test -f "$exp" || {
  echo "Missing $exp" >&2
  exit 1
}
test -f "$logf" || {
  echo "Missing log file: $logf" >&2
  exit 1
}

t1="$(mktemp)"
t2="$(mktemp)"
trap 'rm -f "$t1" "$t2"' EXIT
"$SCRIPT_DIR/extract-case-lines.sh" "$logf" >"$t1"
sort -u "$exp" >"$t2"

if ! cmp -s "$t1" "$t2"; then
  echo "DIFF guest vs oracle ($probe) structured .cases:" >&2
  diff -u "$t2" "$t1" >&2 || true
  exit 1
fi
echo "OK: $probe matches oracle .cases"
