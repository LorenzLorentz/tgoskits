/* GENERATED — ppoll — template contract_ppoll_zero_fds */
#define _GNU_SOURCE
#include <errno.h>
#include <poll.h>
#include <stdio.h>
#include <time.h>

int main(void) {
  struct pollfd fds[1];
  struct timespec ts = { 0, 0 };
  errno = 0;
  int r = ppoll(fds, 0, &ts, NULL);
  int e = errno;
  dprintf(1, "CASE ppoll.zero_fds_timeout0 ret=%d errno=%d note=generated-from-catalog\n", r, e);
  return 0;
}
