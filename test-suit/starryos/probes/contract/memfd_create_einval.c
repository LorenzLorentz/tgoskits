#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	int r = (int)syscall(SYS_memfd_create, "x", -1);
	int e = errno;
	dprintf(1, "CASE memfd_create.einval ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
