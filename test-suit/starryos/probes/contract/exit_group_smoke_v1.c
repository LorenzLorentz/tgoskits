#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>
int main(void)
{
	dprintf(1, "CASE exit_group.smoke_v1 ret=0 errno=0 note=handwritten\n");
	fflush(NULL);
	syscall(SYS_exit_group, 0);
	return 0;
}
