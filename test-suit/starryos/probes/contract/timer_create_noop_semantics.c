#include <errno.h>
#include <signal.h>
#include <stdio.h>
#include <string.h>
#include <time.h>
int main(void)
{
	timer_t t;
	errno = 0;
	int r = timer_create(CLOCK_REALTIME, NULL, &t);
	int e = errno;
	dprintf(1, "CASE timer_create_noop.semantics ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
