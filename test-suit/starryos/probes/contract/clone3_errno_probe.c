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
