// Deterministic getpid(2) driver for the syscall_count eBPF demo (D1).
//
// Issues a fixed number of raw sys_getpid calls so the kprobe-backed BPF
// HashMap accumulates an assertable count. Uses syscall(SYS_getpid) directly
// (not getpid(3)) so glibc/musl PID caching cannot elide the kernel entry the
// kprobe is attached to.
#define _GNU_SOURCE
#include <stdlib.h>
#include <sys/syscall.h>
#include <unistd.h>

int main(int argc, char **argv) {
    long n = (argc > 1) ? atol(argv[1]) : 500;
    for (long i = 0; i < n; i++) {
        syscall(SYS_getpid);
    }
    return 0;
}
