#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <sys/ipc.h>
#include <sys/shm.h>
int main(void)
{
	struct shmid_ds sds;
	memset(&sds, 0, sizeof(sds));
	errno = 0;
	int r = shmctl(-1, IPC_STAT, &sds);
	int e = errno;
	dprintf(1, "CASE shmctl.badid ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
