#!/usr/bin/env bash
# Run a static riscv64 probe ELF as PID 1 in an initramfs under qemu-system-riscv64.
# Requires: STARRY_LINUX_GUEST_IMAGE (riscv64 Linux Image or vmlinuz), timeout(1), qemu-system-riscv64.
# Initramfs: /init is a tiny stub that opens /dev/console and execs /probe (the real ELF), so dprintf(1) works.
# Prefer cpio(1) + gzip; if cpio is missing, falls back to scripts/pack_probe_initrd.py (python3).
#
# Usage: STARRY_LINUX_GUEST_IMAGE=/path/to/Image scripts/run_linux_guest_oracle.sh /path/to/probe_elf
# Env: QEMU_SYSTEM_RISCV64 (default qemu-system-riscv64), STARRY_LINUX_GUEST_TIMEOUT sec (default 90),
#      STARRY_LINUX_GUEST_APPEND (extra kernel cmdline, optional),
#      STARRY_LINUX_GUEST_CC (riscv64 cross gcc for init stub; else CROSS_COMPILE+gcc or PATH heuristic),
#      STARRY_LINUX_GUEST_QUIET=1 (use quiet+loglevel=3; default is verbose boot on ttyS0 for debugging)
#
# Serial output is streamed to stdout as QEMU prints it (do not wrap qemu in $(...) — that hides all
# output until exit/timeout and looks like a “frozen” run with no kernel logs).
set -euo pipefail

ELF="${1:?usage: $0 <static-riscv64-probe-elf>}"
KERNEL="${STARRY_LINUX_GUEST_IMAGE:?STARRY_LINUX_GUEST_IMAGE is not set (see docs/starryos-linux-guest-oracle-pin.md)}"
QEMU="${QEMU_SYSTEM_RISCV64:-qemu-system-riscv64}"
TIMEOUT="${STARRY_LINUX_GUEST_TIMEOUT:-90}"
EXTRA_APPEND="${STARRY_LINUX_GUEST_APPEND:-}"
# Default: no "quiet", loglevel=8 so printk boot messages appear on serial (oracle debugging).
# quiet+loglevel=3 hides almost all kernel output — easy to mistake for "kernel silent".
if [ "${STARRY_LINUX_GUEST_QUIET:-0}" = 1 ]; then
  KERN_APPEND_BASE="console=ttyS0 earlycon=sbi quiet loglevel=3 rdinit=/init"
else
  KERN_APPEND_BASE="console=ttyS0 earlycon=sbi loglevel=8 rdinit=/init"
fi

if [ ! -f "$ELF" ]; then
  echo "run_linux_guest_oracle: not a file: $ELF" >&2
  exit 1
fi
if [ ! -f "$KERNEL" ]; then
  echo "run_linux_guest_oracle: kernel not found: $KERNEL" >&2
  exit 1
fi
if ! command -v "$QEMU" >/dev/null 2>&1; then
  echo "run_linux_guest_oracle: missing $QEMU" >&2
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
STUB_SRC="${SCRIPT_DIR}/linux-guest-kernel/init-console-stub.c"
if [ ! -f "$STUB_SRC" ]; then
  echo "run_linux_guest_oracle: missing stub source: $STUB_SRC" >&2
  exit 1
fi

pick_guest_cc() {
  if [ -n "${STARRY_LINUX_GUEST_CC:-}" ]; then
    echo "$STARRY_LINUX_GUEST_CC"
    return
  fi
  if [ -n "${CROSS_COMPILE:-}" ] && command -v "${CROSS_COMPILE}gcc" >/dev/null 2>&1; then
    echo "${CROSS_COMPILE}gcc"
    return
  fi
  for c in riscv64-linux-musl-gcc riscv64-linux-gnu-gcc riscv64-unknown-linux-gnu-gcc; do
    if command -v "$c" >/dev/null 2>&1; then
      echo "$c"
      return
    fi
  done
  echo ""
}

CC_BIN="$(pick_guest_cc)"
if [ -z "$CC_BIN" ]; then
  echo "run_linux_guest_oracle: no riscv64 cross-gcc found; set STARRY_LINUX_GUEST_CC or CROSS_COMPILE" >&2
  exit 1
fi

td="$(mktemp -d)"
trap 'rm -rf "$td"' EXIT

# Freestanding stub (no libc): libc init + TLS can SIGSEGV when this ELF is PID 1.
"$CC_BIN" -static -nostdlib -ffreestanding -O2 -fno-stack-protector -fno-pie \
  -Wl,-no-pie -Wl,-e,_start \
  -o "$td/init" "$STUB_SRC"
cp "$ELF" "$td/probe"
chmod +x "$td/probe"

if command -v cpio >/dev/null 2>&1; then
  (cd "$td" && printf '%s\n' init probe | cpio -o -H newc 2>/dev/null | gzip -9 >"$td/initrd.gz")
else
  python3 "$SCRIPT_DIR/pack_probe_initrd.py" "$td/init" "$ELF" "$td/initrd.gz"
fi

append="$KERN_APPEND_BASE"
if [ -n "$EXTRA_APPEND" ]; then
  append="$append $EXTRA_APPEND"
fi

# --foreground: keep QEMU in the caller's foreground process group.
# Without it, timeout creates a new process group; QEMU then becomes a "background" job
# relative to the terminal and gets SIGTTOU/SIGTTIN → stopped (state T) → zero output.
set +e
timeout --foreground "$TIMEOUT" "$QEMU" \
  -machine virt \
  -cpu rv64 \
  -smp 1 \
  -m 256M \
  -nographic \
  -kernel "$KERNEL" \
  -initrd "$td/initrd.gz" \
  -append "$append" 2>&1
rc=$?
set -e

if [ "$rc" -eq 124 ]; then
  echo "run_linux_guest_oracle: qemu timed out after ${TIMEOUT}s" >&2
  exit 124
fi
# Guest probe may exit non-zero; still print output for CASE extraction.
exit 0
