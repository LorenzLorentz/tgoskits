#!/usr/bin/sh
# Self-test: extract-case-lines + sorted set compare (no repo probe required).
set -eu
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
d="$(mktemp -d)"
t1=""
t2=""
trap 'rm -rf "$d"; rm -f "$t1" "$t2"' EXIT

# Unsorted .cases file (canonical form is sort -u at compare time)
printf '%s\n' "CASE z.second ret=2 errno=0 note=selftest" "CASE a.first ret=1 errno=0 note=selftest" >"$d/want.cases"
printf '%s\n' "noise" "CASE z.second ret=2 errno=0 note=selftest" "CASE a.first ret=1 errno=0 note=selftest" >"$d/log"

t1="$(mktemp)"
t2="$(mktemp)"
"$SCRIPT_DIR/extract-case-lines.sh" "$d/log" >"$t1"
sort -u "$d/want.cases" >"$t2"
if ! cmp -s "$t1" "$t2"; then
  echo "selftest-structured-cases: cmp failed" >&2
  diff -u "$t2" "$t1" >&2 || true
  exit 1
fi
rm -f "$t1" "$t2"
echo "OK: selftest-structured-cases (extract + sort -u set compare)"
