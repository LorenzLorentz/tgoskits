"""EXTRA_C-style sources for rollout rows still on probe_pattern=special (stage A remainder)."""

from __future__ import annotations

_EXTRA: dict[str, str] = {
    "clone_errno_probe": r"""
#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = syscall(SYS_clone, -1L, NULL, NULL, NULL);
	int e = errno;
	dprintf(1, "CASE clone.errno_probe ret=%ld errno=%d note=handwritten\n", r, e);
	return 0;
}
""",
    "clone3_errno_probe": r"""
#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = syscall(SYS_clone3, NULL, 0);
	int e = errno;
	dprintf(1, "CASE clone3.errno_probe ret=%ld errno=%d note=handwritten\n", r, e);
	return 0;
}
""",
    "exit_smoke_v1": r"""
#include <stdio.h>
#include <unistd.h>
int main(void)
{
	dprintf(1, "CASE exit.smoke_v1 ret=0 errno=0 note=handwritten\n");
	fflush(NULL);
	_exit(0);
}
""",
    "exit_group_smoke_v1": r"""
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	dprintf(1, "CASE exit_group.smoke_v1 ret=0 errno=0 note=handwritten\n");
	fflush(NULL);
	syscall(SYS_exit_group, 0);
	return 0;
}
""",
    "fork_smoke_v1": r"""
#include <errno.h>
#include <stdio.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	pid_t r = fork();
	int e = errno;
	if (r == 0) {
		_exit(0);
	}
	long out = (r > 0 && e == 0) ? 0L : (long)r;
	dprintf(1, "CASE fork.smoke_v1 ret=%ld errno=%d note=handwritten\n", out, e);
	return 0;
}
""",
    "rt_sigtimedwait_probe_tbd": r"""
#include <errno.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>
#include <time.h>
#ifndef SIGUSR1
#define SIGUSR1 10
#endif
int main(void)
{
	sigset_t set;
	sigemptyset(&set);
	sigaddset(&set, SIGUSR1);
	sigprocmask(SIG_BLOCK, &set, NULL);
	siginfo_t info;
	struct timespec ts = {0, 0};
	errno = 0;
	int r = sigtimedwait(&set, &info, &ts);
	int e = errno;
	dprintf(1, "CASE rt_sigtimedwait.probe_tbd ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
""",
    "signalfd4_einval": r"""
#include <errno.h>
#include <signal.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	sigset_t m;
	sigemptyset(&m);
	errno = 0;
	long r = syscall(SYS_signalfd4, -1, &m, (unsigned)-1);
	int e = errno;
	dprintf(1, "CASE signalfd4.einval ret=%ld errno=%d note=handwritten\n", r, e);
	return 0;
}
""",
    "riscv_flush_icache_einval": r"""
#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = syscall(SYS_riscv_flush_icache, (void *)0, (void *)0);
	int e = errno;
	dprintf(1, "CASE riscv_flush_icache.einval ret=%ld errno=%d note=handwritten\n", r, e);
	return 0;
}
""",
    "seccomp_einval": r"""
#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = syscall(SYS_seccomp, -1, 0, NULL);
	int e = errno;
	dprintf(1, "CASE seccomp.einval ret=%ld errno=%d note=handwritten\n", r, e);
	return 0;
}
""",
    "syslog_bad_type": r"""
#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = syscall(SYS_syslog, -1, NULL, 0);
	int e = errno;
	dprintf(1, "CASE syslog.bad_type ret=%ld errno=%d note=handwritten\n", r, e);
	return 0;
}
""",
    "membarrier_einval": r"""
#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = syscall(SYS_membarrier, -1, 0, 0);
	int e = errno;
	dprintf(1, "CASE membarrier.einval ret=%ld errno=%d note=handwritten\n", r, e);
	return 0;
}
""",
    "msgctl_badid": r"""
#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <sys/ipc.h>
#include <sys/msg.h>
int main(void)
{
	struct msqid_ds ds;
	memset(&ds, 0, sizeof(ds));
	errno = 0;
	int r = msgctl(-1, IPC_STAT, &ds);
	int e = errno;
	dprintf(1, "CASE msgctl.badid ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
""",
    "msgget_einval": r"""
#include <errno.h>
#include <stdio.h>
#include <sys/ipc.h>
#include <sys/msg.h>
int main(void)
{
	errno = 0;
	int r = msgget(-1, 0);
	int e = errno;
	dprintf(1, "CASE msgget.einval ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
""",
    "msgrcv_badid": r"""
#include <errno.h>
#include <stdio.h>
#include <sys/ipc.h>
#include <sys/msg.h>
int main(void)
{
	char buf[64];
	errno = 0;
	ssize_t rr = msgrcv(-1, buf, sizeof(buf), 0, IPC_NOWAIT);
	int e = errno;
	dprintf(1, "CASE msgrcv.badid ret=%d errno=%d note=handwritten\n", (int)rr, e);
	return 0;
}
""",
    "msgsnd_badid": r"""
#include <errno.h>
#include <stdio.h>
#include <sys/ipc.h>
#include <sys/msg.h>
int main(void)
{
	char buf[1] = {0};
	errno = 0;
	int r = msgsnd(-1, buf, 1, IPC_NOWAIT);
	int e = errno;
	dprintf(1, "CASE msgsnd.badid ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
""",
    "shmat_badid": r"""
#include <errno.h>
#include <stdio.h>
#include <sys/ipc.h>
#include <sys/shm.h>
int main(void)
{
	errno = 0;
	void *p = shmat(-1, NULL, 0);
	long r = (long)(unsigned long)p;
	int e = errno;
	dprintf(1, "CASE shmat.badid ret=%ld errno=%d note=handwritten\n", r, e);
	return 0;
}
""",
    "shmctl_badid": r"""
#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <sys/ipc.h>
#include <sys/shm.h>
int main(void)
{
	struct shmid_ds sds;
	memset(&sds, 0, sizeof(sds));
	errno = 0;
	int r = shmctl(-1, IPC_STAT, &sds);
	int e = errno;
	dprintf(1, "CASE shmctl.badid ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
""",
    "shmdt_einval": r"""
#include <errno.h>
#include <stdio.h>
#include <sys/shm.h>
int main(void)
{
	errno = 0;
	int r = shmdt((void *)0x10000);
	int e = errno;
	dprintf(1, "CASE shmdt.einval ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
""",
    "shmget_einval": r"""
#include <errno.h>
#include <stdio.h>
#include <sys/ipc.h>
#include <sys/shm.h>
int main(void)
{
	errno = 0;
	int r = shmget(IPC_PRIVATE, (size_t)-1, 0);
	int e = errno;
	dprintf(1, "CASE shmget.einval ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
""",
    "bpf_stub_semantics": r"""
#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = syscall(SYS_bpf, 0, NULL, 0);
	int e = errno;
	dprintf(1, "CASE bpf_stub.semantics ret=%ld errno=%d note=handwritten\n", r, e);
	return 0;
}
""",
    "fanotify_init_stub_semantics": r"""
#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = syscall(SYS_fanotify_init, -1u, 0);
	int e = errno;
	dprintf(1, "CASE fanotify_init_stub.semantics ret=%ld errno=%d note=handwritten\n", r, e);
	return 0;
}
""",
    "fsopen_stub_semantics": r"""
#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = syscall(SYS_fsopen, "ext4", 0);
	int e = errno;
	dprintf(1, "CASE fsopen_stub.semantics ret=%ld errno=%d note=handwritten\n", r, e);
	return 0;
}
""",
    "fspick_stub_semantics": r"""
#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = syscall(SYS_fspick, -1, ".", 0);
	int e = errno;
	dprintf(1, "CASE fspick_stub.semantics ret=%ld errno=%d note=handwritten\n", r, e);
	return 0;
}
""",
    "inotify_init1_stub_semantics": r"""
#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = syscall(SYS_inotify_init1, -1);
	int e = errno;
	dprintf(1, "CASE inotify_init1_stub.semantics ret=%ld errno=%d note=handwritten\n", r, e);
	return 0;
}
""",
    "io_uring_setup_stub_semantics": r"""
#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = syscall(SYS_io_uring_setup, 1, NULL);
	int e = errno;
	dprintf(1, "CASE io_uring_setup_stub.semantics ret=%ld errno=%d note=handwritten\n", r, e);
	return 0;
}
""",
    "memfd_secret_stub_semantics": r"""
#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
#ifndef SYS_memfd_secret
#define SYS_memfd_secret 447
#endif
int main(void)
{
	errno = 0;
	long r = syscall(SYS_memfd_secret, "", 0);
	int e = errno;
	dprintf(1, "CASE memfd_secret_stub.semantics ret=%ld errno=%d note=handwritten\n", r, e);
	return 0;
}
""",
    "open_tree_stub_semantics": r"""
#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = syscall(SYS_open_tree, -1, "/", -1);
	int e = errno;
	dprintf(1, "CASE open_tree_stub.semantics ret=%ld errno=%d note=handwritten\n", r, e);
	return 0;
}
""",
    "perf_event_open_stub_semantics": r"""
#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = syscall(SYS_perf_event_open, NULL, 0, -1, -1, 0);
	int e = errno;
	dprintf(1, "CASE perf_event_open_stub.semantics ret=%ld errno=%d note=handwritten\n", r, e);
	return 0;
}
""",
    "timer_create_noop_semantics": r"""
#include <errno.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>
#include <time.h>
int main(void)
{
	timer_t t;
	errno = 0;
	int r = timer_create(CLOCK_REALTIME, NULL, &t);
	int e = errno;
	dprintf(1, "CASE timer_create_noop.semantics ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
""",
    "timer_gettime_noop_semantics": r"""
#include <errno.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>
#include <time.h>
int main(void)
{
	timer_t t;
	memset(&t, 0, sizeof(t));
	struct itimerspec its;
	errno = 0;
	int r = timer_gettime(t, &its);
	int e = errno;
	dprintf(1, "CASE timer_gettime_noop.semantics ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
""",
    "timer_settime_noop_semantics": r"""
#include <errno.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>
#include <time.h>
int main(void)
{
	timer_t t;
	memset(&t, 0, sizeof(t));
	struct itimerspec its;
	memset(&its, 0, sizeof(its));
	errno = 0;
	int r = timer_settime(t, 0, &its, NULL);
	int e = errno;
	dprintf(1, "CASE timer_settime_noop.semantics ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
""",
    "timerfd_create_stub_semantics": r"""
#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = syscall(SYS_timerfd_create, -1, 0);
	int e = errno;
	dprintf(1, "CASE timerfd_create_stub.semantics ret=%ld errno=%d note=handwritten\n", r, e);
	return 0;
}
""",
    "userfaultfd_stub_semantics": r"""
#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = syscall(SYS_userfaultfd, -1);
	int e = errno;
	dprintf(1, "CASE userfaultfd_stub.semantics ret=%ld errno=%d note=handwritten\n", r, e);
	return 0;
}
""",
}

EXTRA_C_REMAINING: dict[str, str] = {k: v.strip() + "\n" for k, v in _EXTRA.items()}
