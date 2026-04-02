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
