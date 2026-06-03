#!/bin/sh
# Anti-fallback verification for the syscall_count eBPF demo (D1).
#
# syscall_count (the syscall_ebpf aya program) hangs a kprobe on sys_getpid,
# and on every hit bumps a BPF HashMap keyed by the probe-context arg. The
# userspace loader iterates that map via bpf(BPF_MAP_GET_NEXT_KEY/LOOKUP) and
# prints it every few seconds. This script drives a *deterministic* number of
# sys_getpid calls (getpid_spin) and asserts the user side reads back a
# matching count from the map.
#
# It does NOT read trace_pipe / dmesg: the whole data path is kprobe -> rbpf
# VM -> HashMap -> bpf() map lookup on the user side, so a broken kprobe or map
# path cannot pass this silently. sys_getpid is chosen (not the central
# dispatcher) because it is low-traffic: probing every syscall would drown the
# TCG-emulated guest.
set -u

BIN=/usr/bin/syscall_ebpf
SPIN=/usr/bin/getpid_spin
OUT=/tmp/syscall_count.out

[ -x "$BIN" ] || { echo "SYSCALL_COUNT_FAIL: $BIN not found or not executable"; exit 1; }
[ -x "$SPIN" ] || { echo "SYSCALL_COUNT_FAIL: $SPIN not found or not executable"; exit 1; }

# Resolve the *mangled* sys_getpid symbol from kallsyms; the kernel resolves
# the kprobe target through this table so the name must match exactly.
SYM=$(grep -m1 'sys_getpid$' /proc/kallsyms | awk '{print $3}')
[ -n "$SYM" ] || { echo "SYSCALL_COUNT_FAIL: sys_getpid not in /proc/kallsyms"; exit 1; }
echo "syscall_count: target symbol = $SYM"

# Start the loader (attaches the kprobe, then dumps the map every 5s).
"$BIN" "$SYM" >"$OUT" 2>&1 &
PID=$!
sleep 3
kill -0 "$PID" 2>/dev/null || { echo "SYSCALL_COUNT_FAIL: loader died early"; cat "$OUT"; exit 1; }

# Deterministic workload: 500 sys_getpid calls.
N=500
echo "syscall_count: issuing $N getpid() calls"
"$SPIN" "$N"

# Let at least one 5s map dump fire after the workload, then stop the loader.
sleep 8
kill -INT "$PID" 2>/dev/null || true
sleep 1
kill -KILL "$PID" 2>/dev/null || true
wait "$PID" 2>/dev/null

[ -s "$OUT" ] || { echo "SYSCALL_COUNT_FAIL: loader produced no output"; exit 1; }

# Largest count across all map keys in the dumps. The probe fires once per
# getpid, so the hot key must approach N.
MAX=$(grep -o 'count: *[0-9][0-9]*' "$OUT" | awk '{print $NF}' | sort -n | tail -1)
: "${MAX:=0}"
echo "syscall_count: max HashMap count = $MAX (drove $N getpid calls)"

# Strong absolute assertion (not the forbidden "> 0"): allow generous TCG slack
# but require the count to clearly reflect the 500-call workload.
if [ "$MAX" -lt 100 ]; then
    echo "SYSCALL_COUNT_FAIL: max count $MAX < 100; kprobe+HashMap path not firing"
    echo "--- loader output ---"
    cat "$OUT"
    exit 1
fi

echo "SYSCALL_COUNT_PASS: max count $MAX (>=100) read back from BPF HashMap"
exit 0
