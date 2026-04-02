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
