#include <errno.h>
#include <stdio.h>
#include <sys/socket.h>
int main(void)
{
	errno = 0;
	int r = socket(12345, SOCK_STREAM, 0);
	int e = errno;
	dprintf(1, "CASE socket.invalid_domain ret=%d errno=%d note=handwritten\n", r, e);
	return 0;
}
