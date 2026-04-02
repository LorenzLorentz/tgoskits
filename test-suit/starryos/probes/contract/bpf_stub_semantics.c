#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = syscall(SYS_bpf, 0, NULL, 0);
	int e = errno;
	dprintf(1, "CASE bpf_stub.semantics ret=%ld errno=%d note=handwritten\n", r, e);
	return 0;
}
