#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	int r = (int)syscall(SYS_eventfd2, 0, -1);
	int e = errno;
	dprintf(1, "CASE eventfd2.einval ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
