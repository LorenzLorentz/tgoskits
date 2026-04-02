/* Hand-written contract probe: clock_gettime(2) with NULL timespec -> EFAULT. */
#include <errno.h>
#include <stdio.h>
#include <time.h>

int main(void)
{
	errno = 0;
	int r = clock_gettime(CLOCK_REALTIME, NULL);
	int e = errno;
	dprintf(1, "CASE clock_gettime.null_ts ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
