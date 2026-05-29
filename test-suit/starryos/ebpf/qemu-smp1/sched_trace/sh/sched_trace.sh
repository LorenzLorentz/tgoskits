#!/bin/sh
# Anti-fallback verification for the sched_trace eBPF demo (D2).
#
# It asserts that the USER side observes real sched:sched_switch records that
# came back through the perf ring buffer (bpf_perf_event_output -> mmap'd
# PerfEventArray). It deliberately does NOT inspect trace_pipe text or dmesg:
# those paths do not exercise the ringbuf and would let a broken perf mmap
# pass silently (see docs/ebpf-followup/perf-ringbuf-audit.md).
#
# Prereqs inside qemu: /usr/bin/sched_trace (built by
# `cargo xtask starry user-ebpf build --program sched_trace`) installed and
# executable. No debugfs/tracefs mount is required — raw-tracepoint attach
# goes through bpf(BPF_RAW_TRACEPOINT_OPEN), not the tracefs control files.
set -u

BIN=/usr/bin/sched_trace
OUT=/tmp/sched_trace.out
ERR=/tmp/sched_trace.err

[ -x "$BIN" ] || { echo "SCHED_TRACE_FAIL: $BIN not found or not executable"; exit 1; }

# Start the tracer. Its stdout is the user-side ringbuf stream, one line per
# captured sched_switch ("prev=<tid> next=<tid> state=<n> ts=<ns>").
"$BIN" >"$OUT" 2>"$ERR" &
PID=$!

# Let it load the program, open the per-cpu perf buffer(s), and attach.
sleep 2

# Force scheduler churn: two CPU-bound loops that keep preempting each other
# and the tracer, generating many context switches over the window.
sh -c 'while :; do :; done' &
A=$!
sh -c 'while :; do :; done' &
B=$!
sleep 3
kill "$A" "$B" 2>/dev/null
wait "$A" "$B" 2>/dev/null

# Drain the tail, then stop the tracer (SIGTERM; lines are already flushed).
sleep 1
kill "$PID" 2>/dev/null
wait "$PID" 2>/dev/null

if [ ! -s "$OUT" ]; then
    echo "SCHED_TRACE_FAIL: no ringbuf output — perf mmap path may be broken"
    [ -s "$ERR" ] && cat "$ERR"
    exit 1
fi

N=$(grep -c 'prev=' "$OUT")
echo "sched_trace: captured $N sched_switch records"

# Strong count assertion (not just > 0): a broken or never-firing hook would
# produce far fewer than this over 3s of churn.
if [ "$N" -lt 100 ]; then
    echo "SCHED_TRACE_FAIL: only $N records (<100); hook not firing or samples lost"
    exit 1
fi

# Both ends of every switch must be present.
grep -q 'prev=' "$OUT" || { echo "SCHED_TRACE_FAIL: no prev= field"; exit 1; }
grep -q 'next=' "$OUT" || { echo "SCHED_TRACE_FAIL: no next= field"; exit 1; }

# A correct sched_switch stream involves more than one task; a single distinct
# next tid would mean the probe is wired to the wrong place.
DISTINCT=$(sed -n 's/.*next=\([0-9][0-9]*\).*/\1/p' "$OUT" | sort -u | wc -l)
if [ "$DISTINCT" -lt 2 ]; then
    echo "SCHED_TRACE_FAIL: only $DISTINCT distinct next tid — probe wired wrong?"
    exit 1
fi

echo "SCHED_TRACE_PASS: $N records, $DISTINCT distinct next tids"
exit 0
