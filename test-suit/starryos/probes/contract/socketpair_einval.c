#include <errno.h>
#include <stdio.h>
#include <sys/socket.h>
int main(void)
{
	int sv[2];
	errno = 0;
	int r = socketpair(AF_INET, -1, 0, sv);
	int e = errno;
	dprintf(1, "CASE socketpair.einval ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
