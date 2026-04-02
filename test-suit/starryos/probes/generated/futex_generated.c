/* GENERATED — futex — template contract_futex_wake_nop */
#include <errno.h>
#include <linux/futex.h>
#include <stdio.h>
#include <sys/syscall.h>
#include <unistd.h>

static int u;

int main(void) {
  errno = 0;
  long r = syscall(SYS_futex, &u, FUTEX_WAKE, 1, NULL, NULL, 0);
  int e = errno;
  dprintf(1, "CASE futex.wake_nop ret=%ld errno=%d note=generated-from-catalog\n", r, e);
  return 0;
}
