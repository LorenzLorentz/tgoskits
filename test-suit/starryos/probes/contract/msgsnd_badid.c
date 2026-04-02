#include <errno.h>
#include <stdio.h>
#include <sys/ipc.h>
#include <sys/msg.h>
int main(void)
{
	char buf[1] = {0};
	errno = 0;
	int r = msgsnd(-1, buf, 1, IPC_NOWAIT);
	int e = errno;
	dprintf(1, "CASE msgsnd.badid ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
