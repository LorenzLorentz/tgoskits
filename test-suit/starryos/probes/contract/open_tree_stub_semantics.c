#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = syscall(SYS_open_tree, -1, "/", -1);
	int e = errno;
	dprintf(1, "CASE open_tree_stub.semantics ret=%ld errno=%d note=handwritten\n", r, e);
	return 0;
}
