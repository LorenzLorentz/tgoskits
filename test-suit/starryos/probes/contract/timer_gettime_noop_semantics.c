#include <errno.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>
#include <time.h>
int main(void)
{
	timer_t t;
	memset(&t, 0, sizeof(t));
	struct itimerspec its;
	errno = 0;
	int r = timer_gettime(t, &its);
	int e = errno;
	dprintf(1, "CASE timer_gettime_noop.semantics ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
