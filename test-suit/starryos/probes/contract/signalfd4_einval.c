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
