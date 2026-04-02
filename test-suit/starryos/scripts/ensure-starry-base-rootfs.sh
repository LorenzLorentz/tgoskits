#!/usr/bin/sh
# Ensure StarryOS riscv64 base disk image exists for probe workflows.
#
# If missing, runs: cargo xtask starry rootfs --arch riscv64 (may download; needs network once).
# Env:
#   STARRY_REFRESH_ROOTFS=1  — always run rootfs command (refresh / re-download path in xtask)
#   SKIP_STARRY_ROOTFS_FETCH=1 — do not invoke cargo; fail if base image still missing (offline/CI)
set -eu
WS="$(cd "$(dirname "$0")/../../.." && pwd)"
cd "$WS"

BASE="$WS/target/riscv64gc-unknown-none-elf/rootfs-riscv64.img"

need_fetch=0
if [ ! -f "$BASE" ]; then
  need_fetch=1
elif [ "${STARRY_REFRESH_ROOTFS:-0}" = 1 ]; then
  need_fetch=1
fi

if [ "$need_fetch" -eq 1 ]; then
  if [ "${SKIP_STARRY_ROOTFS_FETCH:-0}" = 1 ]; then
    echo "Missing $BASE and SKIP_STARRY_ROOTFS_FETCH=1 — not running cargo xtask." >&2
    exit 2
  fi
  echo "== ensure base rootfs: cargo xtask starry rootfs --arch riscv64 (may download) =="
  cargo xtask starry rootfs --arch riscv64
fi

if [ ! -f "$BASE" ]; then
  echo "Missing $BASE after preparation — check network, disk space, and xtask logs." >&2
  exit 2
fi
