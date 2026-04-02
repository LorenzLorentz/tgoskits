#!/usr/bin/sh
set -eu
# Package root: test-suit/starryos
PKG="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${PROBE_OUT:-$PKG/probes/build-riscv64}"
CC="${CC:-riscv64-linux-musl-gcc}"

mkdir -p "$OUT"
if ! command -v "$CC" >/dev/null 2>&1; then
  echo "Missing cross compiler: $CC" >&2
  echo "Install riscv64-linux-musl-gcc (see probes/README.md)" >&2
  exit 1
fi

for src in "$PKG/probes/contract/"*.c; do
  [ -f "$src" ] || continue
  base="$(basename "$src" .c)"
  echo "CC $base"
  # musl riscv64 GCC still links rcrt1.o (static PIE crt) unless you pass the driver flag -no-pie;
  # rcrt1 + ET_EXEC breaks _start_c (a1=0 → crash under real Linux); crt1.o is required for guest oracle.
  "$CC" -static -no-pie -O2 -fno-stack-protector -fno-pie -Wl,-no-pie -o "$OUT/$base" "$src"
done
echo "Built probes -> $OUT"
