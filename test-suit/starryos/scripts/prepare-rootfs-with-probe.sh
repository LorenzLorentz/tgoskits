#!/usr/bin/sh
# Usage: prepare-rootfs-with-probe.sh <probe_basename>
# Example: prepare-rootfs-with-probe.sh write_stdout
# Ensures base rootfs via ensure-starry-base-rootfs.sh, builds contract probes, copies base, injects /root/<probe_basename>.
set -eu
WS="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$WS"

probe="${1:?usage: $0 <probe_basename e.g. write_stdout or close_badfd>}"
TARGET_DIR="target/riscv64gc-unknown-none-elf"
BASE="$TARGET_DIR/rootfs-riscv64.img"
# 保持与早期文档一致：write_stdout 仍使用 rootfs-riscv64-probe.img
if [ "$probe" = write_stdout ]; then
  OUT="$TARGET_DIR/rootfs-riscv64-probe.img"
else
  OUT="$TARGET_DIR/rootfs-riscv64-probe-${probe}.img"
fi
PROBE_SRC="$WS/test-suit/starryos/probes/build-riscv64/$probe"

"$WS/test-suit/starryos/scripts/ensure-starry-base-rootfs.sh"

CC="${CC:-riscv64-linux-musl-gcc}"
export CC
"$WS/test-suit/starryos/scripts/build-probes.sh"

if [ ! -x "$PROBE_SRC" ]; then
  echo "Probe not built: $PROBE_SRC" >&2
  exit 1
fi

if ! command -v debugfs >/dev/null 2>&1; then
  echo "debugfs not found (install e2fsprogs)" >&2
  exit 1
fi

cp -f "$BASE" "$OUT"
HOST_ELF="$(realpath "$PROBE_SRC")"
guest_path="/root/$probe"
# shellcheck disable=SC2059
printf 'rm %s\nwrite %s %s\nchmod 0755 %s\nquit\n' "$guest_path" "$HOST_ELF" "$guest_path" "$guest_path" | debugfs -w "$OUT" 2>/dev/null || {
  printf 'write %s %s\nquit\n' "$HOST_ELF" "$guest_path" | debugfs -w "$OUT"
}

echo "Prepared $OUT (probe at $guest_path)"
echo "Run QEMU test (adjust testcase path to match):"
echo "  cargo xtask starry test qemu --target riscv64 \\"
echo "    --test-disk-image $OUT \\"
echo "    --shell-init-cmd test-suit/starryos/testcases/probe-${probe}-0 \\"
echo "    --timeout 120"
