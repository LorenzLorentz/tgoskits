#!/bin/sh
# Anti-fallback verification for the `profile` eBPF demo (D3).
#
# `profile` hangs a kprobe on the kernel's central syscall dispatcher
# (`handle_syscall`), reads the syscall number out of the saved user register
# frame via bpf_probe_read, and counts it in a BPF HashMap. The userspace
# loader ranks that histogram and prints it. This script asserts the USER side
# observed a real, well-shaped syscall-frequency profile:
#
#   * a clear hot syscall (top-1 share >= 20%) — proves the probe actually
#     fires on a hot path, not once;
#   * >= 3 distinct syscall numbers — proves it is a real histogram, not a
#     single mis-wired key;
#   * a large absolute sample count — proves it is not the "> 0" weak assertion
#     the workflow blacklist forbids.
#
# It does NOT read trace_pipe / dmesg: the whole data path is kprobe -> rbpf
# VM -> HashMap -> bpf(BPF_MAP_GET_NEXT_KEY/LOOKUP) on the user side. A broken
# kprobe or map path cannot pass this silently.
#
# Prereqs in qemu: /usr/bin/profile (built by
# `cargo xtask starry user-ebpf build --program profile`) installed/executable,
# and /proc/kallsyms populated (two-pass kernel build embeds the symbol table).
set -u

BIN=/usr/bin/profile

[ -x "$BIN" ] || { echo "PROFILE_FAIL: $BIN not found or not executable"; exit 1; }

# Resolve the *mangled* dispatcher symbol from kallsyms. The kernel resolves
# the kprobe target through this table, so the name must match exactly.
SYM=$(grep -m1 'handle_syscall' /proc/kallsyms | awk '{print $3}')
if [ -z "$SYM" ]; then
    echo "PROFILE_FAIL: handle_syscall not found in /proc/kallsyms"
    exit 1
fi
echo "profile: target symbol = $SYM"

# ---- one profiling run -------------------------------------------------------
# $1 = output file. Attaches, drives a syscall-heavy workload with a dominant
# syscall (dd does one read()+one write() per byte), then SIGTERM-dumps.
run_profile() {
    out="$1"
    "$BIN" "$SYM" >"$out" 2>"$out.err" &
    pid=$!

    # Wait for the attach line so the workload is counted.
    sleep 2

    # Dominant hot syscalls: dd bs=1 => 20000 read() + 20000 write().
    dd if=/dev/zero of=/dev/null bs=1 count=20000 2>/dev/null
    # A little extra syscall variety so the histogram is clearly multi-key.
    ls / >/dev/null 2>&1
    cat /proc/kallsyms >/dev/null 2>&1

    sleep 1
    kill -TERM "$pid" 2>/dev/null
    wait "$pid" 2>/dev/null
}

OUT=/tmp/profile.out
run_profile "$OUT"

if [ ! -s "$OUT" ]; then
    echo "PROFILE_FAIL: no profiler output"
    [ -s "$OUT.err" ] && cat "$OUT.err"
    exit 1
fi

SUMMARY=$(grep -m1 '^PROFILE_END' "$OUT")
if [ -z "$SUMMARY" ]; then
    echo "PROFILE_FAIL: no PROFILE_END summary line"
    cat "$OUT"
    exit 1
fi
echo "profile: $SUMMARY"

# Parse "PROFILE_END total=.. distinct=.. top1_sysno=.. top1_count=.. top1_pct=.."
TOTAL=$(echo "$SUMMARY" | sed -n 's/.*total=\([0-9]*\).*/\1/p')
DISTINCT=$(echo "$SUMMARY" | sed -n 's/.*distinct=\([0-9]*\).*/\1/p')
TOP1=$(echo "$SUMMARY" | sed -n 's/.*top1_count=\([0-9]*\).*/\1/p')
: "${TOTAL:=0}" "${DISTINCT:=0}" "${TOP1:=0}"

# Strong absolute count (not "> 0"): 20000 dd iterations alone dwarf this.
if [ "$TOTAL" -lt 1000 ]; then
    echo "PROFILE_FAIL: only $TOTAL samples (<1000); probe not firing on hot path"
    exit 1
fi

# Real histogram, not one mis-wired key.
if [ "$DISTINCT" -lt 3 ]; then
    echo "PROFILE_FAIL: only $DISTINCT distinct syscalls (<3); probe wired wrong?"
    exit 1
fi

# Clear hot spot: top1 >= 20% of total, i.e. top1*5 >= total (integer-safe).
if [ $((TOP1 * 5)) -lt "$TOTAL" ]; then
    echo "PROFILE_FAIL: top1=$TOP1 of $TOTAL (<20%); no hot syscall — flat/garbage?"
    exit 1
fi

# ---- detach / re-attach leak check ------------------------------------------
# Repeatedly attach+detach the kprobe (each detach rewrites kernel text via
# write_kernel_text). A leaked probe or a botched unregister would panic the
# kernel (caught by the panic fail_regex) or stop producing samples.
i=1
while [ "$i" -le 3 ]; do
    R=/tmp/profile.re.$i
    run_profile "$R"
    RT=$(grep -m1 '^PROFILE_END' "$R" | sed -n 's/.*total=\([0-9]*\).*/\1/p')
    : "${RT:=0}"
    if [ "$RT" -lt 1000 ]; then
        echo "PROFILE_FAIL: re-attach #$i produced only $RT samples — detach leaked/broke probe"
        exit 1
    fi
    echo "profile: re-attach #$i ok ($RT samples)"
    i=$((i + 1))
done

echo "PROFILE_PASS: total=$TOTAL distinct=$DISTINCT top1=$TOP1 (>=20%); 3 re-attach cycles clean"
exit 0
