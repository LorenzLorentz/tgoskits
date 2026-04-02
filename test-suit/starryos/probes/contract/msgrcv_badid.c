#include <errno.h>
#include <stdio.h>
#include <sys/ipc.h>
#include <sys/msg.h>
int main(void)
{
	char buf[64];
	errno = 0;
	ssize_t rr = msgrcv(-1, buf, sizeof(buf), 0, IPC_NOWAIT);
	int e = errno;
	dprintf(1, "CASE msgrcv.badid ret=%d errno=%d note=handwritten\n", (int)rr, e);
	return 0;
}
