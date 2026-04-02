#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = syscall(SYS_recvmsg, -1, (void *)0, 0);
	int e = errno;
	dprintf(1, "CASE recvmsg.badfd ret=%ld errno=%d note=handwritten\n", r, e);
	return 0;
}
