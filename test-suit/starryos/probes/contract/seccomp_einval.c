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
