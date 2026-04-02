#include <errno.h>
#include <stdio.h>
#include <sys/shm.h>
int main(void)
{
	errno = 0;
	int r = shmdt((void *)0x10000);
	int e = errno;
	dprintf(1, "CASE shmdt.einval ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
