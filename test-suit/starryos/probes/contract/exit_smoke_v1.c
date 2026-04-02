#include <stdio.h>
#include <unistd.h>
int main(void)
{
	dprintf(1, "CASE exit.smoke_v1 ret=0 errno=0 note=handwritten\n");
	fflush(NULL);
	_exit(0);
}
