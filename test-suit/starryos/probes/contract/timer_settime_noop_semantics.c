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
	memset(&its, 0, sizeof(its));
	errno = 0;
	int r = timer_settime(t, 0, &its, NULL);
	int e = errno;
	dprintf(1, "CASE timer_settime_noop.semantics ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
