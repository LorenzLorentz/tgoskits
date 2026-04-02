#!/usr/bin/env bash
# One-shot local verification for Linux guest oracle (qemu-system-riscv64 + initramfs probe).
#
# From repo root:
#   ./scripts/verify_linux_guest_oracle.sh
#   ./scripts/verify_linux_guest_oracle.sh -i /path/to/Image
#   ./scripts/verify_linux_guest_oracle.sh -a          # full guest track verify-oracle-all
#   ./scripts/verify_linux_guest_oracle.sh -r          # refresh expected/guest-alpine323/*.line
#
# Env (optional): CC, STARRY_LINUX_GUEST_CC, STARRY_LINUX_GUEST_IMAGE, LINUX_SRC,
#                 STARRY_LINUX_GUEST_SEARCH_ROOT (extra find root for Image), VERIFY_STRICT,
#                 QEMU_SYSTEM_RISCV64, PROBE_OUT
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BUILD_SH="$ROOT/test-suit/starryos/scripts/build-probes.sh"
RUN_GUEST="$ROOT/scripts/run_linux_guest_oracle.sh"
DIFF_SH="$ROOT/test-suit/starryos/scripts/run-diff-probes.sh"
REFRESH_SH="$ROOT/scripts/refresh_guest_oracle_expected.sh"
OUT="${PROBE_OUT:-$ROOT/test-suit/starryos/probes/build-riscv64}"
QEMU_SYS="${QEMU_SYSTEM_RISCV64:-qemu-system-riscv64}"

usage() {
	sed -n '2,12p' "$0" | sed 's/^# \{0,1\}//'
	exit "${1:-0}"
}

IMAGE=""
DO_ALL=0
DO_REFRESH=0
STRICT=0
SKIP_BUILD=0
PROBE="write_stdout"

while [ $# -gt 0 ]; do
	case "$1" in
	-h | --help) usage 0 ;;
	-i | --image)
		IMAGE="${2:?}"
		shift 2
		;;
	-a | --all) DO_ALL=1; shift ;;
	-r | --refresh) DO_REFRESH=1; shift ;;
	-S | --strict) STRICT=1; shift ;;
	--skip-build) SKIP_BUILD=1; shift ;;
	-p | --probe)
		PROBE="${2:?}"
		shift 2
		;;
	*)
		echo "unknown option: $1" >&2
		usage 1
		;;
	esac
done

resolve_kernel_image() {
	# -i / --image wins over STARRY_LINUX_GUEST_IMAGE for this invocation.
	if [ -n "$IMAGE" ] && [ -f "$IMAGE" ]; then
		printf '%s\n' "$IMAGE"
		return 0
	fi
	if [ -n "${STARRY_LINUX_GUEST_IMAGE:-}" ] && [ -f "$STARRY_LINUX_GUEST_IMAGE" ]; then
		printf '%s\n' "$STARRY_LINUX_GUEST_IMAGE"
		return 0
	fi
	if [ -n "${LINUX_SRC:-}" ] && [ -f "$LINUX_SRC/arch/riscv/boot/Image" ]; then
		printf '%s\n' "$LINUX_SRC/arch/riscv/boot/Image"
		return 0
	fi
	local f
	# Common local build locations (avoid scanning entire $HOME).
	f=$(find /tmp -path '*/arch/riscv/boot/Image' -type f 2>/dev/null | head -n1)
	if [ -n "$f" ]; then
		printf '%s\n' "$f"
		return 0
	fi
	# Optional: narrow search under a typical dev tree (set explicitly if needed).
	if [ -n "${STARRY_LINUX_GUEST_SEARCH_ROOT:-}" ] && [ -d "$STARRY_LINUX_GUEST_SEARCH_ROOT" ]; then
		f=$(find "$STARRY_LINUX_GUEST_SEARCH_ROOT" -path '*/arch/riscv/boot/Image' -type f 2>/dev/null | head -n1)
		if [ -n "$f" ]; then
			printf '%s\n' "$f"
			return 0
		fi
	fi
	if [ -d "${HOME:-}/thecodes/buildkernel" ]; then
		f=$(find "${HOME}/thecodes/buildkernel" -path '*/arch/riscv/boot/Image' -type f 2>/dev/null | head -n1)
		if [ -n "$f" ]; then
			printf '%s\n' "$f"
			return 0
		fi
	fi
	return 1
}

cd "$ROOT"

CC="${CC:-riscv64-linux-musl-gcc}"
export CC
export STARRY_LINUX_GUEST_CC="${STARRY_LINUX_GUEST_CC:-$CC}"

if ! command -v "$CC" >/dev/null 2>&1; then
	echo "verify_linux_guest_oracle: missing compiler: $CC (set CC=...)" >&2
	exit 1
fi
if ! command -v "$QEMU_SYS" >/dev/null 2>&1; then
	echo "verify_linux_guest_oracle: missing $QEMU_SYS" >&2
	exit 1
fi

KERN="$(resolve_kernel_image)" || {
	echo "verify_linux_guest_oracle: could not find riscv64 Image." >&2
	echo "  Set STARRY_LINUX_GUEST_IMAGE or LINUX_SRC, or pass: -i /path/to/arch/riscv/boot/Image" >&2
	exit 1
}
export STARRY_LINUX_GUEST_IMAGE="$KERN"
echo "verify_linux_guest_oracle: STARRY_LINUX_GUEST_IMAGE=$STARRY_LINUX_GUEST_IMAGE"

if [ "$SKIP_BUILD" -eq 0 ]; then
	sh "$BUILD_SH"
else
	mkdir -p "$OUT"
fi

test -x "$OUT/$PROBE" || {
	echo "verify_linux_guest_oracle: missing $OUT/$PROBE (build failed or wrong --probe?)" >&2
	exit 1
}

echo "=== smoke: guest qemu + CASE line ($PROBE) ==="
echo "hint: QEMU serial streams to stdout. loglevel=8 by default. Quiet: STARRY_LINUX_GUEST_QUIET=1" >&2
SMOKE_LOG="$(mktemp)"
trap 'rm -f "$SMOKE_LOG"' EXIT
set +e
bash "$RUN_GUEST" "$OUT/$PROBE" 2>&1 | tee "$SMOKE_LOG"
_guest_rc="${PIPESTATUS[0]}"
set -e
if [ "$_guest_rc" -ne 0 ]; then
	echo "verify_linux_guest_oracle: run_linux_guest_oracle exited $_guest_rc" >&2
	exit "$_guest_rc"
fi
SMOKE="$(tr -d '\r' < "$SMOKE_LOG" | grep -m1 '^CASE ' || true)"
if [ -z "$SMOKE" ]; then
	echo "verify_linux_guest_oracle: no '^CASE ' line in guest serial output." >&2
	exit 1
fi
echo "smoke CASE: $SMOKE"

export VERIFY_ORACLE_TRACK=guest-alpine323
if [ "$STRICT" -eq 1 ]; then
	export VERIFY_STRICT=1
else
	unset VERIFY_STRICT 2>/dev/null || true
fi

echo "=== verify-oracle: $PROBE ==="
set +e
sh "$DIFF_SH" verify-oracle "$PROBE"
v1=$?
set -e
if [ "$v1" -ne 0 ]; then
	exit "$v1"
fi

v2=0
if [ "$DO_ALL" -eq 1 ]; then
	echo "=== verify-oracle-all (guest-alpine323) ==="
	set +e
	sh "$DIFF_SH" verify-oracle-all
	v2=$?
	set -e
	if [ "$v2" -ne 0 ]; then
		exit "$v2"
	fi
fi

if [ "$DO_REFRESH" -eq 1 ]; then
	echo "=== refresh guest expected lines ==="
	bash "$REFRESH_SH"
fi

echo "verify_linux_guest_oracle: OK"
echo "--- summary ---"
echo "  1) build-probes: 已生成/更新 $OUT 下全部 contract 探针 ELF（CC … 为逐个编译）。"
echo "  2) smoke: qemu-system-riscv64 -machine virt -kernel <Image> -initrd <cpio>，initstub exec /probe，串口上的 '^CASE ' 即探针 oracle 行。"
echo "  3) verify-oracle: 与 test-suit/starryos/probes/expected/guest-alpine323/${PROBE}.line 逐行比对。"
echo "  paths: IMAGE=$STARRY_LINUX_GUEST_IMAGE"
echo "  probe: $OUT/$PROBE"
echo "  CASE:  $SMOKE"
echo "  verify-oracle($PROBE): exit $v1"
if [ "$DO_ALL" -eq 1 ]; then
	echo "  verify-oracle-all: exit $v2"
fi
