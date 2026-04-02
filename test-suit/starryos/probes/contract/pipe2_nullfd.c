/* Hand-written contract probe: pipe2(2) with NULL pipefd -> EFAULT. */
#include <errno.h>
#include <stdio.h>
#include <unistd.h>

int main(void)
{
	errno = 0;
	int r = pipe2(NULL, 0);
	int e = errno;
	dprintf(1, "CASE pipe2.null_pipefd ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
