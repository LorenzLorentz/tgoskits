#include <errno.h>
#include <stdio.h>
#include <sys/ipc.h>
#include <sys/msg.h>
int main(void)
{
	errno = 0;
	int r = msgget(-1, 0);
	int e = errno;
	dprintf(1, "CASE msgget.einval ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
