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
