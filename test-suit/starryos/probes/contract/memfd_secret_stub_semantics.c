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
