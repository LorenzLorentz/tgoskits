#include <errno.h>
#include <stdio.h>
#include <unistd.h>
int main(void)
{
	int fd[2];
	errno = 0;
	int r = pipe2(fd, -1);
	int e = errno;
	dprintf(1, "CASE pipe.linux_contract_p1 ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
