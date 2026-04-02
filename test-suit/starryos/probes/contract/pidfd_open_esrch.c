#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	int r = (int)syscall(SYS_pidfd_open, 999999999, 0);
	int e = errno;
	dprintf(1, "CASE pidfd_open.esrch ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
