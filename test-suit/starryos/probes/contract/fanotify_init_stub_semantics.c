#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = syscall(SYS_fanotify_init, -1u, 0);
	int e = errno;
	dprintf(1, "CASE fanotify_init_stub.semantics ret=%ld errno=%d note=handwritten\n", r, e);
	return 0;
}
