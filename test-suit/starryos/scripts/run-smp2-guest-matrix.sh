#!/usr/bin/sh
# For each contract probe (or one named probe): SMP2 QEMU + verify-guest-log-oracle.
# Ensures base rootfs exists first (runs cargo xtask starry rootfs when missing).
#
# Prereq: cargo xtask, riscv64-linux-musl-gcc, e2fsprogs, qemu-system-riscv64; network on first fetch.
#
# Usage:
#   ./run-smp2-guest-matrix.sh              # all probes from list-contract-probes.sh
#   ./run-smp2-guest-matrix.sh write_stdout # single probe
# Env:
#   LOGDIR=/path   # default: ${TMPDIR:-/tmp}/starry-smp2-matrix
#   STARRY_REFRESH_ROOTFS=1 — refresh base rootfs before matrix
#   SKIP_STARRY_ROOTFS_FETCH=1 — fail if base img missing (no auto cargo xtask)
set -eu
WS="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$WS"

LOGDIR="${LOGDIR:-${TMPDIR:-/tmp}/starry-smp2-matrix}"
mkdir -p "$LOGDIR"

"$WS/test-suit/starryos/scripts/ensure-starry-base-rootfs.sh"

failed=0
if [ "$#" -ge 1 ]; then
  probes="$1"
else
  probes="$("$WS/test-suit/starryos/scripts/list-contract-probes.sh" | tr '\n' ' ')"
fi

for probe in $probes; do
  [ -n "$probe" ] || continue
  echo "========== SMP2 + guest oracle: $probe =========="
  log="$LOGDIR/smp2-${probe}.log"
  if ! "$WS/test-suit/starryos/scripts/run-starry-probe-qemu-smp2.sh" "$probe" >"$log" 2>&1; then
    echo "FAILED: QEMU starry test ($probe) — see $log" >&2
    failed=1
    continue
  fi
  if ! "$WS/test-suit/starryos/scripts/verify-guest-log-oracle.sh" "$probe" "$log"; then
    echo "FAILED: guest CASE vs oracle ($probe) — see $log" >&2
    failed=1
  fi
done

if [ "$failed" -eq 0 ]; then
  echo "OK: all listed probes passed SMP2 + guest-oracle (logs under $LOGDIR)"
fi
exit "$failed"
