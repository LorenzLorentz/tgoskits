#!/usr/bin/sh
set -eu
PKG="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${PROBE_OUT:-$PKG/probes/build-riscv64}"
QEMU_RV64="${QEMU_RV64:-qemu-riscv64}"

usage() {
  echo "Usage: $0 {build|oracle|verify-oracle|verify-oracle-all|help}" >&2
  echo "  build                 run build-probes.sh" >&2
  echo "  oracle [name]         run \$OUT/<name> under \$QEMU_RV64 (default: write_stdout)" >&2
  echo "  verify-oracle [name]  diff vs probes/expected/<name>.line or .cases" >&2
  echo "  verify-oracle-all     every expected *.line / *.cases (unique probe basenames)" >&2
  echo "Env: VERIFY_STRICT=1  treat missing \$QEMU_RV64 as failure (exit 2)" >&2
  exit 1
}

verify_one() {
  p="$1"
  cases="$PKG/probes/expected/${p}.cases"
  linef="$PKG/probes/expected/${p}.line"
  if [ -f "$cases" ] && [ -f "$linef" ]; then
    echo "verify-oracle: both .cases and .line for probe $p" >&2
    return 1
  fi
  if [ ! -f "$cases" ] && [ ! -f "$linef" ]; then
    echo "Missing expected for probe $p (.line or .cases)" >&2
    return 1
  fi
  test -x "$OUT/$p" || { echo "Missing $OUT/$p — run: $0 build" >&2; return 1; }
  if ! command -v "$QEMU_RV64" >/dev/null 2>&1; then
    if [ "${VERIFY_STRICT:-0}" = 1 ]; then
      echo "STRICT: missing $QEMU_RV64 (set VERIFY_STRICT=0 to allow SKIP)" >&2
      return 2
    fi
    echo "SKIP: $QEMU_RV64 not installed" >&2
    return 0
  fi
  if [ -f "$cases" ]; then
    t1="$(mktemp)"
    t2="$(mktemp)"
    "$QEMU_RV64" "$OUT/$p" 2>/dev/null | tr -d '\r' | "$PKG/scripts/extract-case-lines.sh" >"$t1"
    sort -u "$cases" >"$t2"
    if ! cmp -s "$t1" "$t2"; then
      echo "DIFF oracle $p (.cases):" >&2
      diff -u "$t2" "$t1" >&2 || true
      rm -f "$t1" "$t2"
      return 1
    fi
    rm -f "$t1" "$t2"
    echo "verify-oracle OK: $p (structured .cases)"
    return 0
  fi
  got="$("$QEMU_RV64" "$OUT/$p" 2>/dev/null | tr -d '\r' | grep -m1 '^CASE ' || true)"
  want="$(cat "$linef")"
  if [ "$got" != "$want" ]; then
    echo "DIFF oracle $p:" >&2
    echo "  want: $want" >&2
    echo "  got:  $got" >&2
    return 1
  fi
  echo "verify-oracle OK: $p -> $want"
  return 0
}

cmd="${1:-help}"
case "$cmd" in
  build)
    exec "$PKG/scripts/build-probes.sh"
    ;;
  oracle)
    p="${2:-write_stdout}"
    test -x "$OUT/$p" || { echo "Missing $OUT/$p — run: $0 build" >&2; exit 1; }
    if ! command -v "$QEMU_RV64" >/dev/null 2>&1; then
      echo "Missing $QEMU_RV64 (install qemu-user / qemu-system user package)" >&2
      exit 1
    fi
    "$QEMU_RV64" "$OUT/$p"
    ;;
  verify-oracle)
    p="${2:-write_stdout}"
    set +e
    verify_one "$p"
    rc=$?
    set -e
    exit "$rc"
    ;;
  verify-oracle-all)
    failed=0
    strict_fail=0
    any=0
    donef="$(mktemp)"
    : >"$donef"
    trap 'rm -f "$donef"' EXIT
    for exp in "$PKG/probes/expected/"*.line "$PKG/probes/expected/"*.cases; do
      [ -f "$exp" ] || continue
      any=1
      b=$(basename "$exp")
      case "$b" in
        *.line) base="${b%.line}" ;;
        *.cases) base="${b%.cases}" ;;
        *) continue ;;
      esac
      if grep -qxF "$base" "$donef"; then
        continue
      fi
      echo "$base" >>"$donef"
      set +e
      verify_one "$base"
      rc=$?
      set -e
      if [ "$rc" -eq 2 ]; then
        strict_fail=1
        failed=1
      elif [ "$rc" -ne 0 ]; then
        failed=1
      fi
    done
    if [ "$any" -eq 0 ]; then
      echo "No probes/expected/*.line or *.cases files" >&2
      exit 1
    fi
    if [ "$strict_fail" -eq 1 ]; then
      exit 2
    fi
    exit "$failed"
    ;;
  help|*)
    usage
    ;;
esac
