#!/usr/bin/sh
# Compare guest serial / log to Linux oracle.
# - If probes/expected/<probe>.cases exists: all ^CASE lines (sorted set) must match.
# - Else probes/expected/<probe>.line: first ^CASE line must match (legacy).
# Do not define both .cases and .line for the same probe.
#
# Usage:
#   ./verify-guest-log-oracle.sh <probe_basename> [log_file|-]
# Exit: 0 match, 1 mismatch, 2 no input / missing expected / both .line and .cases
set -eu
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PKG="$(cd "$SCRIPT_DIR/.." && pwd)"

probe="${1:?usage: $0 <probe_basename> [log_file|-]}"
shift

cases="$PKG/probes/expected/${probe}.cases"
linef="$PKG/probes/expected/${probe}.line"

if [ -f "$cases" ] && [ -f "$linef" ]; then
  echo "verify-guest-log-oracle: both .cases and .line exist for probe=$probe" >&2
  exit 2
fi

if [ "$#" -ge 1 ]; then
  log_arg="$1"
else
  log_arg="-"
fi

if [ ! -f "$cases" ] && [ ! -f "$linef" ]; then
  echo "verify-guest-log-oracle: missing expected for probe=$probe (.line or .cases)" >&2
  exit 2
fi

if [ -f "$cases" ]; then
  if [ "$log_arg" = "-" ]; then
    tmp="$(mktemp)"
    trap 'rm -f "$tmp"' EXIT
    cat >"$tmp"
    exec "$SCRIPT_DIR/diff-guest-cases.sh" "$probe" "$tmp"
  fi
  if [ ! -f "$log_arg" ]; then
    echo "verify-guest-log-oracle: 找不到日志文件: $log_arg" >&2
    exit 2
  fi
  exec "$SCRIPT_DIR/diff-guest-cases.sh" "$probe" "$log_arg"
fi

# --- single .line mode (first CASE line) ---
if [ "$log_arg" = "-" ]; then
  line="$("$SCRIPT_DIR/extract-case-line.sh")"
else
  if [ ! -f "$log_arg" ]; then
    echo "verify-guest-log-oracle: 找不到日志文件: $log_arg" >&2
    echo "" >&2
    echo "可选做法：" >&2
    echo "  1) 在仓库根目录先准备镜像（按需），再跑 QEMU 并 tee 保存输出，例如 write_stdout：" >&2
    echo "       cargo xtask starry rootfs --arch riscv64" >&2
    echo "       ./test-suit/starryos/scripts/prepare-rootfs-with-write_stdout-probe.sh" >&2
    echo "       cargo xtask starry test qemu --target riscv64 \\" >&2
    echo "         --test-disk-image target/riscv64gc-unknown-none-elf/rootfs-riscv64-probe.img \\" >&2
    echo "         --shell-init-cmd test-suit/starryos/testcases/probe-write_stdout-0 \\" >&2
    echo "         --timeout 120 \\" >&2
    echo "         2>&1 | tee serial.log" >&2
    echo "       $0 $probe serial.log" >&2
    echo "  2) 不传文件，从标准输入读入（粘贴整段串口文本后按 Ctrl+D 结束）：" >&2
    echo "       $0 $probe" >&2
    echo "       $0 $probe -" >&2
    exit 2
  fi
  line="$("$SCRIPT_DIR/extract-case-line.sh" "$log_arg")"
fi

if [ -z "$line" ]; then
  echo "verify-guest-log-oracle: no line matching ^CASE  (probe=$probe)" >&2
  exit 2
fi

exec "$SCRIPT_DIR/diff-guest-line.sh" "$probe" "$line"
