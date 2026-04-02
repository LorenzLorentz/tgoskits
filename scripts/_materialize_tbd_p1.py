"""Minimal Linux-oracle C probes for matrix rows still marked probe_pattern=tbd_errno.

riscv64-linux-musl + qemu-riscv64 user-mode. arch_prctl has no riscv syscall number in musl — omitted (None)."""

from __future__ import annotations


def _case_tag(probe: str) -> str:
    i = probe.rfind("_")
    if i <= 0:
        return probe
    return probe[:i] + "." + probe[i + 1 :]


# Full translation units; $CASE replaced by e.g. getpid.linux_contract_p1
_TABLE: dict[str, str] = {
    "capget": r"""#include <errno.h>
#include <linux/capability.h>
#include <stdio.h>
#include <sys/syscall.h>
int main(void)
{
	struct __user_cap_header_struct h = {_LINUX_CAPABILITY_VERSION_3, 0};
	struct __user_cap_data_struct d[2];
	errno = 0;
	long r = syscall(SYS_capget, &h, d);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "capset": r"""#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
int main(void)
{
	errno = 0;
	long r = syscall(SYS_capset, (void *)0, (void *)0);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "get_mempolicy": r"""#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
int main(void)
{
	errno = 0;
	long r = syscall(SYS_get_mempolicy, (void *)0, 0, (void *)0, 0, 0);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "prctl": r"""#include <errno.h>
#include <stdio.h>
#include <sys/prctl.h>
int main(void)
{
	errno = 0;
	long r = prctl(-1, 0, 0, 0, 0);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "prlimit64": r"""#include <errno.h>
#include <stdio.h>
#include <sys/resource.h>
int main(void)
{
	struct rlimit rl;
	errno = 0;
	long r = prlimit64(0, RLIMIT_NOFILE, NULL, &rl);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "set_tid_address": r"""#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = (long)syscall(SYS_set_tid_address, NULL);
	int e = errno;
	long out = (r != -1L && e == 0) ? 0L : r;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", out, e);
	return 0;
}
""",
    "setresgid": r"""#include <errno.h>
#include <stdio.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = (long)setresgid((gid_t)-1, (gid_t)-1, (gid_t)-1);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "setresuid": r"""#include <errno.h>
#include <stdio.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = (long)setresuid((uid_t)-1, (uid_t)-1, (uid_t)-1);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "setreuid": r"""#include <errno.h>
#include <stdio.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = (long)setreuid((uid_t)-1, (uid_t)-1);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "umask": r"""#include <errno.h>
#include <stdio.h>
#include <sys/stat.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = (long)umask(0022);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "getpgid": r"""#include <errno.h>
#include <stdio.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	pid_t p = getpgid(0);
	int e = errno;
	long r = (p > 0 && e == 0) ? 0L : (long)p;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "getsid": r"""#include <errno.h>
#include <stdio.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	pid_t p = getsid(0);
	int e = errno;
	long r = (p > 0 && e == 0) ? 0L : (long)p;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "setpgid": r"""#include <errno.h>
#include <stdio.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = (long)setpgid(0, 0);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "setsid": r"""#include <errno.h>
#include <stdio.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	pid_t p = setsid();
	int e = errno;
	long r = (p > 0 && e == 0) ? 0L : (long)p;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "clock_nanosleep": r"""#include <errno.h>
#include <stdio.h>
#include <time.h>
int main(void)
{
	struct timespec rq = {0, 0}, rm;
	errno = 0;
	long r = clock_nanosleep(CLOCK_REALTIME, 0, &rq, &rm);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "getpriority": r"""#include <errno.h>
#include <stdio.h>
#include <sys/resource.h>
int main(void)
{
	errno = 0;
	long r = (long)getpriority(PRIO_PROCESS, 0);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "nanosleep": r"""#include <errno.h>
#include <stdio.h>
#include <time.h>
int main(void)
{
	struct timespec ts = {0, 0};
	errno = 0;
	long r = nanosleep(&ts, NULL);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "sched_getparam": r"""#include <errno.h>
#include <sched.h>
#include <stdio.h>
int main(void)
{
	struct sched_param sp;
	errno = 0;
	long r = sched_getparam(0, &sp);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "sched_getscheduler": r"""#include <errno.h>
#include <sched.h>
#include <stdio.h>
int main(void)
{
	errno = 0;
	long r = (long)sched_getscheduler(0);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "sched_setscheduler": r"""#include <errno.h>
#include <sched.h>
#include <stdio.h>
int main(void)
{
	struct sched_param sp = {0};
	errno = 0;
	long r = sched_setscheduler(0, -1, &sp);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "sched_yield": r"""#include <errno.h>
#include <sched.h>
#include <stdio.h>
int main(void)
{
	errno = 0;
	long r = (long)sched_yield();
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "getpid": r"""#include <errno.h>
#include <stdio.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	pid_t p = getpid();
	int e = errno;
	long r = (p > 0 && e == 0) ? 0L : (long)p;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "getppid": r"""#include <errno.h>
#include <stdio.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	pid_t p = getppid();
	int e = errno;
	long r = (p > 0 && e == 0) ? 0L : (long)p;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "getrusage": r"""#include <errno.h>
#include <stdio.h>
#include <sys/resource.h>
int main(void)
{
	struct rusage ru;
	errno = 0;
	long r = getrusage(RUSAGE_SELF, &ru);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "gettid": r"""#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long p = syscall(SYS_gettid);
	int e = errno;
	long r = (p > 0 && e == 0) ? 0L : p;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "get_robust_list": r"""#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	void *head_ptr = NULL;
	size_t len = 0;
	errno = 0;
	long r = syscall(SYS_get_robust_list, 0, &head_ptr, &len);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "kill": r"""#include <errno.h>
#include <signal.h>
#include <stdio.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = (long)kill(0, 0);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "rt_sigaction": r"""#include <errno.h>
#include <signal.h>
#include <stdio.h>
#ifndef SIGUSR1
#define SIGUSR1 10
#endif
int main(void)
{
	struct sigaction old;
	errno = 0;
	long r = sigaction(SIGUSR1, NULL, &old);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "rt_sigpending": r"""#include <errno.h>
#include <signal.h>
#include <stdio.h>
int main(void)
{
	sigset_t set;
	errno = 0;
	long r = sigpending(&set);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "rt_sigprocmask": r"""#include <errno.h>
#include <signal.h>
#include <stdio.h>
int main(void)
{
	sigset_t empty;
	sigemptyset(&empty);
	errno = 0;
	long r = sigprocmask(SIG_BLOCK, &empty, NULL);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "rt_sigqueueinfo": r"""#include <errno.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>
#include <sys/syscall.h>
#ifndef SIGUSR1
#define SIGUSR1 10
#endif
int main(void)
{
	siginfo_t info;
	memset(&info, 0, sizeof(info));
	errno = 0;
	long r = syscall(SYS_rt_sigqueueinfo, -1, SIGUSR1, &info);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "rt_tgsigqueueinfo": r"""#include <errno.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>
#include <sys/syscall.h>
#ifndef SIGUSR1
#define SIGUSR1 10
#endif
int main(void)
{
	siginfo_t info;
	memset(&info, 0, sizeof(info));
	errno = 0;
	long r = syscall(SYS_rt_tgsigqueueinfo, -1, -1, SIGUSR1, &info);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "set_robust_list": r"""#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
int main(void)
{
	errno = 0;
	long r = syscall(SYS_set_robust_list, (void *)0, 0);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "sigaltstack": r"""#include <errno.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>
int main(void)
{
	stack_t ss, oss;
	memset(&ss, 0, sizeof(ss));
	ss.ss_flags = SS_DISABLE;
	errno = 0;
	long r = sigaltstack(&ss, &oss);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "tgkill": r"""#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = syscall(SYS_tgkill, getpid(), getpid(), 0);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "tkill": r"""#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = syscall(SYS_tkill, getpid(), 0);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "getegid": r"""#include <errno.h>
#include <stdio.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = (long)getegid();
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "geteuid": r"""#include <errno.h>
#include <stdio.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = (long)geteuid();
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "getgid": r"""#include <errno.h>
#include <stdio.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = (long)getgid();
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "getuid": r"""#include <errno.h>
#include <stdio.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = (long)getuid();
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "getgroups": r"""#include <errno.h>
#include <stdio.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = getgroups(0, NULL);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "getrandom": r"""#include <errno.h>
#include <stdio.h>
#include <sys/random.h>
int main(void)
{
	char b[16];
	errno = 0;
	long r = getrandom(b, sizeof(b), 0);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "setgid": r"""#include <errno.h>
#include <stdio.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = (long)setgid((gid_t)-1);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "setgroups": r"""#include <errno.h>
#include <stdio.h>
#include <unistd.h>
int main(void)
{
	gid_t g = 0;
	errno = 0;
	long r = (long)setgroups(-1, &g);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "setuid": r"""#include <errno.h>
#include <stdio.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = (long)setuid((uid_t)-1);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "sysinfo": r"""#include <errno.h>
#include <stdio.h>
#include <sys/sysinfo.h>
int main(void)
{
	struct sysinfo si;
	errno = 0;
	long r = sysinfo(&si);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "uname": r"""#include <errno.h>
#include <stdio.h>
#include <sys/utsname.h>
int main(void)
{
	struct utsname u;
	errno = 0;
	long r = uname(&u);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
    "times": r"""#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = syscall(SYS_times, (void *)1);
	int e = errno;
	dprintf(1, "CASE $CASE ret=%ld errno=%d note=tbd_p1\n", r, e);
	return 0;
}
""",
}


def tbd_p1_source(syscall: str, probe: str) -> str | None:
    tpl = _TABLE.get(syscall)
    if not tpl:
        return None
    ct = _case_tag(probe)
    return ("/* Generated by materialize (tbd_p1) */\n" + tpl.replace("$CASE", ct)).rstrip() + "\n"
