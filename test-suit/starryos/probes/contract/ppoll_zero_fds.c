/* Hand-written contract probe: ppoll(2) nfds=0, timeout zero -> immediate return. */
#define _GNU_SOURCE
#include <errno.h>
#include <poll.h>
#include <stdio.h>
#include <time.h>

int main(void)
{
	struct pollfd fds[1];
	struct timespec ts = { 0, 0 };

	errno = 0;
	/* nfds=0: Linux allows fds=NULL; some kernels reject NULL -> EFAULT — use a dummy slot. */
	int r = ppoll(fds, 0, &ts, NULL);
	int e = errno;
	dprintf(1, "CASE ppoll.zero_fds_timeout0 ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
