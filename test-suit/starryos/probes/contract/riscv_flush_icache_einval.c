#include <errno.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	errno = 0;
	long r = syscall(SYS_riscv_flush_icache, (void *)0, (void *)0);
	int e = errno;
	dprintf(1, "CASE riscv_flush_icache.einval ret=%ld errno=%d note=handwritten\n", r, e);
	return 0;
}
