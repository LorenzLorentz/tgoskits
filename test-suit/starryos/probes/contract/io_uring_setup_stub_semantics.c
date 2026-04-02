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
