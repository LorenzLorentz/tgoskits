#include <errno.h>
#include <stdio.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	pid_t r = fork();
	int e = errno;
	if (r == 0) {
		_exit(0);
	}
	long out = (r > 0 && e == 0) ? 0L : (long)r;
	dprintf(1, "CASE fork.smoke_v1 ret=%ld errno=%d note=handwritten\n", out, e);
	return 0;
}
