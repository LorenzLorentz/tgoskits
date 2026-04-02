#include <errno.h>
#include <stdio.h>
#include <sys/mman.h>
int main(void)
{
	errno = 0;
	int r = mlock(NULL, 4096);
	int e = errno;
	dprintf(1, "CASE mlock.enomem ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
