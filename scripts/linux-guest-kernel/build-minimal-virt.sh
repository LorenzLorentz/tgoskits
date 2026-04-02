#!/usr/bin/env bash
# Build a small riscv64 Linux Image for qemu-system-riscv64 -machine virt
# (virtio-blk, virtio-net, 8250 serial, SMP, initrd). Not a full distro kernel.
#
# Prerequisites: host build deps (flex bison bc openssl libssl-dev libelf-dev),
# and CROSS_COMPILE riscv64 gcc (e.g. riscv64-linux-musl-gcc or riscv64-linux-gnu-gcc).
#
# Usage:
#   export LINUX_SRC=/path/to/linux-6.18.x   # or let script use LINUX_SRC default
#   export CROSS_COMPILE=riscv64-linux-musl-   # trailing hyphen; compiler on PATH
#   ./scripts/linux-guest-kernel/build-minimal-virt.sh
# Output: $LINUX_SRC/arch/riscv/boot/Image
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
FRAG="$ROOT/scripts/linux-guest-kernel/minimal-riscv64-virt.fragment"
LINUX_SRC="${LINUX_SRC:-}"

if [ -z "$LINUX_SRC" ] || [ ! -f "$LINUX_SRC/Makefile" ]; then
  echo "Set LINUX_SRC to a Linux source tree (e.g. linux-6.18.20)." >&2
  echo "Example: LINUX_SRC=/tmp/linux-6.18.20 $0" >&2
  exit 1
fi
if [ ! -f "$FRAG" ]; then
  echo "Missing fragment: $FRAG" >&2
  exit 1
fi
if [ -z "${CROSS_COMPILE:-}" ]; then
  if command -v riscv64-linux-musl-gcc >/dev/null 2>&1; then
    CROSS_COMPILE=riscv64-linux-musl-
  elif command -v riscv64-linux-gnu-gcc >/dev/null 2>&1; then
    CROSS_COMPILE=riscv64-linux-gnu-
  else
    echo "Set CROSS_COMPILE (e.g. riscv64-linux-musl-) and ensure gcc is on PATH." >&2
    exit 1
  fi
  export CROSS_COMPILE
fi

cd "$LINUX_SRC"
make ARCH=riscv CROSS_COMPILE="$CROSS_COMPILE" tinyconfig
./scripts/kconfig/merge_config.sh -m .config "$FRAG"
make ARCH=riscv CROSS_COMPILE="$CROSS_COMPILE" olddefconfig
make ARCH=riscv CROSS_COMPILE="$CROSS_COMPILE" Image -j"$(nproc)"

IMG="$LINUX_SRC/arch/riscv/boot/Image"
ls -la "$IMG"
file "$IMG"
echo "OK: $IMG (qemu-system-riscv64 -machine virt -kernel $IMG ...)"
