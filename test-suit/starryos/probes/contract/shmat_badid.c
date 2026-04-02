#include <errno.h>
#include <stdio.h>
#include <sys/ipc.h>
#include <sys/shm.h>
int main(void)
{
	errno = 0;
	void *p = shmat(-1, NULL, 0);
	long r = (long)(unsigned long)p;
	int e = errno;
	dprintf(1, "CASE shmat.badid ret=%ld errno=%d note=handwritten\n", r, e);
	return 0;
}
