#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <sys/ipc.h>
#include <sys/msg.h>
int main(void)
{
	struct msqid_ds ds;
	memset(&ds, 0, sizeof(ds));
	errno = 0;
	int r = msgctl(-1, IPC_STAT, &ds);
	int e = errno;
	dprintf(1, "CASE msgctl.badid ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
