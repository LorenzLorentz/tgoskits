# StarryOS syscall 行为证据（Linux oracle / guest 矩阵）

由 `scripts/render_starry_syscall_inventory.py --step 3` 生成。

- **matrix_probe**：矩阵 `contract_probe`；若仅有脚手架则显示 `(planned) …`（来自 `planned_contract_probe`，见 [docs/starryos-syscall-probe-rollout.yaml](docs/starryos-syscall-probe-rollout.yaml)）。
- **catalog_probes**：catalog `tests:` 中的 contract 文件名（不含路径）。
- **matrix_parity**：矩阵 `parity`（无行则为 —）。

全量 **Linux user oracle**：`VERIFY_STRICT=1 test-suit/starryos/scripts/run-diff-probes.sh verify-oracle-all`。
全量 **SMP2 guest vs oracle**：`test-suit/starryos/scripts/run-smp2-guest-matrix.sh`。

**轨 B（Linux guest oracle，真内核）**：锚点见 [starryos-linux-guest-oracle-pin.md](starryos-linux-guest-oracle-pin.md)。需本机 `riscv64` `Image` 与交叉 `gcc`（探针 `build-probes.sh` 已带 `-no-pie`）。金线在 `test-suit/starryos/probes/expected/guest-alpine323/*.line`。一键：`./scripts/verify_linux_guest_oracle.sh -i /path/to/Image`（可加 `-a` 全量比对）；重写金线：`STARRY_LINUX_GUEST_IMAGE=... CC=riscv64-...-gcc scripts/refresh_guest_oracle_expected.sh`。CI 可选全量：`starryos-linux-guest-oracle` workflow 勾选 **run_full_guest_verify**。与轨 A（`qemu-riscv64` user）偏差时，以 guest 输出为 **轨 B 叙事** 参考。

**分发表条目数**: 210

| syscall | handler | matrix_parity | matrix_probe | catalog_probes |
|---------|---------|---------------|--------------|----------------|
| `ioctl` | `sys_ioctl` | partial | ioctl_badfd | ioctl_badfd |
| `chdir` | `sys_chdir` | partial | chdir_enoent | chdir_enoent |
| `fchdir` | `sys_fchdir` | partial | fchdir_badfd | fchdir_badfd |
| `chroot` | `sys_chroot` | partial | chroot_enoent | chroot_enoent |
| `mkdir` | `sys_mkdir` | partial | mkdir_enoent | mkdir_enoent |
| `mkdirat` | `sys_mkdirat` | partial | mkdirat_enoent | mkdirat_enoent |
| `getdents64` | `sys_getdents64` | partial | getdents64_badfd | getdents64_badfd |
| `link` | `sys_link` | partial | link_enoent | link_enoent |
| `linkat` | `sys_linkat` | partial | linkat_enoent | linkat_enoent |
| `rmdir` | `sys_rmdir` | partial | rmdir_enoent | rmdir_enoent |
| `unlink` | `sys_unlink` | partial | unlink_enoent | unlink_enoent |
| `unlinkat` | `sys_unlinkat` | partial | unlinkat_enoent | unlinkat_enoent |
| `getcwd` | `sys_getcwd` | partial | getcwd_size0 | getcwd_size0 |
| `symlink` | `sys_symlink` | partial | symlink_enoent | symlink_enoent |
| `symlinkat` | `sys_symlinkat` | partial | symlinkat_enoent | symlinkat_enoent |
| `rename` | `sys_rename` | partial | rename_enoent | rename_enoent |
| `renameat` | `sys_renameat` | partial | renameat_enoent | renameat_enoent |
| `renameat2` | `sys_renameat2` | partial | renameat2_enoent | renameat2_enoent |
| `sync` | `sys_sync` | partial | sync_void_smoke | sync_void_smoke |
| `syncfs` | `sys_syncfs` | partial | syncfs_badfd | syncfs_badfd |
| `chown` | `sys_chown` | partial | chown_enoent | chown_enoent |
| `lchown` | `sys_lchown` | partial | lchown_enoent | lchown_enoent |
| `fchown` | `sys_fchown` | partial | fchown_badfd | fchown_badfd |
| `fchownat` | `sys_fchownat` | partial | fchownat_enoent | fchownat_enoent |
| `chmod` | `sys_chmod` | partial | chmod_enoent | chmod_enoent |
| `fchmod` | `sys_fchmod` | partial | fchmod_badfd | fchmod_badfd |
| `fchmodat` | `sys_fchmodat` | partial | fchmodat_enoent | fchmodat_enoent |
| `fchmodat2` | `sys_fchmodat` | partial | fchmodat2_enoent | fchmodat2_enoent |
| `readlink` | `sys_readlink` | partial | readlink_enoent | readlink_enoent |
| `readlinkat` | `sys_readlinkat` | partial | readlinkat_enoent | readlinkat_enoent |
| `utime` | `sys_utime` | partial | utime_enoent | utime_enoent |
| `utimes` | `sys_utimes` | partial | utimes_enoent | utimes_enoent |
| `utimensat` | `sys_utimensat` | partial | utimensat_enoent | utimensat_enoent |
| `open` | `sys_open` | partial | open_enoent | open_enoent |
| `openat` | `sys_openat` | partial | openat_badfd | openat_badfd, openat_enoent |
| `close` | `sys_close` | partial | close_badfd | close_badfd |
| `close_range` | `sys_close_range` | partial | close_range_badfd | close_range_badfd |
| `dup` | `sys_dup` | partial | dup_badfd | dup_badfd |
| `dup2` | `sys_dup2` | partial | dup2_badfd | dup2_badfd |
| `dup3` | `sys_dup3` | partial | dup3_badfd | dup3_badfd |
| `fcntl` | `sys_fcntl` | partial | fcntl_badfd | fcntl_badfd |
| `flock` | `sys_flock` | partial | flock_badfd | flock_badfd |
| `read` | `sys_read` | partial | read_stdin_zero | read_stdin_zero |
| `readv` | `sys_readv` | partial | readv_badfd | readv_badfd |
| `write` | `sys_write` | partial | write_stdout | write_stdout |
| `writev` | `sys_writev` | partial | writev_badfd | writev_badfd |
| `lseek` | `sys_lseek` | partial | lseek_badfd | lseek_badfd |
| `truncate` | `sys_truncate` | partial | truncate_enoent | truncate_enoent |
| `ftruncate` | `sys_ftruncate` | partial | ftruncate_badfd | ftruncate_badfd |
| `fallocate` | `sys_fallocate` | partial | fallocate_badfd | fallocate_badfd |
| `fsync` | `sys_fsync` | partial | fsync_badfd | fsync_badfd |
| `fdatasync` | `sys_fdatasync` | partial | fdatasync_badfd | fdatasync_badfd |
| `fadvise64` | `sys_fadvise64` | partial | fadvise64_badfd | fadvise64_badfd |
| `pread64` | `sys_pread64` | partial | pread64_badfd | pread64_badfd |
| `pwrite64` | `sys_pwrite64` | partial | pwrite64_badfd | pwrite64_badfd |
| `preadv` | `sys_preadv` | partial | preadv_badfd | preadv_badfd |
| `pwritev` | `sys_pwritev` | partial | pwritev_badfd | pwritev_badfd |
| `preadv2` | `sys_preadv2` | partial | preadv2_badfd | preadv2_badfd |
| `pwritev2` | `sys_pwritev2` | partial | pwritev2_badfd | pwritev2_badfd |
| `sendfile` | `sys_sendfile` | partial | sendfile_badfd | sendfile_badfd |
| `copy_file_range` | `sys_copy_file_range` | partial | copy_file_range_badfd | copy_file_range_badfd |
| `splice` | `sys_splice` | partial | splice_badfd | splice_badfd |
| `poll` | `sys_poll` | partial | poll_linux_contract_p1 | poll_linux_contract_p1 |
| `ppoll` | `sys_ppoll` | partial | ppoll_zero_fds | ppoll_zero_fds |
| `select` | `sys_select` | partial | select_linux_contract_p1 | select_linux_contract_p1 |
| `pselect6` | `sys_pselect6` | partial | pselect6_linux_contract_p1 | pselect6_linux_contract_p1 |
| `epoll_create1` | `sys_epoll_create1` | partial | epoll_create1_einval | epoll_create1_einval |
| `epoll_ctl` | `sys_epoll_ctl` | partial | epoll_ctl_badfd | epoll_ctl_badfd |
| `epoll_pwait` | `sys_epoll_pwait` | partial | epoll_pwait_badfd | epoll_pwait_badfd |
| `epoll_pwait2` | `sys_epoll_pwait2` | partial | epoll_pwait2_badfd | epoll_pwait2_badfd |
| `mount` | `sys_mount` | partial | mount_enoent | mount_enoent |
| `umount2` | `sys_umount2` | partial | umount2_enoent | umount2_enoent |
| `pipe2` | `sys_pipe2` | partial | pipe2_nullfd | pipe2_nullfd |
| `pipe` | `sys_pipe2` | partial | pipe_linux_contract_p1 | pipe_linux_contract_p1 |
| `eventfd2` | `sys_eventfd2` | partial | eventfd2_einval | eventfd2_einval |
| `pidfd_open` | `sys_pidfd_open` | partial | pidfd_open_esrch | pidfd_open_esrch |
| `pidfd_getfd` | `sys_pidfd_getfd` | partial | pidfd_getfd_badfd | pidfd_getfd_badfd |
| `pidfd_send_signal` | `sys_pidfd_send_signal` | partial | pidfd_send_signal_badfd | pidfd_send_signal_badfd |
| `memfd_create` | `sys_memfd_create` | partial | memfd_create_einval | memfd_create_einval |
| `stat` | `sys_stat` | partial | stat_enoent | stat_enoent |
| `fstat` | `sys_fstat` | partial | fstat_badfd | fstat_badfd |
| `lstat` | `sys_lstat` | partial | lstat_enoent | lstat_enoent |
| `newfstatat` | `sys_fstatat` | partial | newfstatat_enoent | newfstatat_enoent |
| `fstatat` | `sys_fstatat` | partial | fstatat_enoent | fstatat_enoent |
| `statx` | `sys_statx` | partial | statx_enoent | statx_enoent |
| `access` | `sys_access` | partial | access_enoent | access_enoent |
| `faccessat` | `sys_faccessat2` | partial | faccessat_enoent | faccessat_enoent |
| `faccessat2` | `sys_faccessat2` | partial | faccessat2_enoent | faccessat2_enoent |
| `statfs` | `sys_statfs` | partial | statfs_linux_contract_p1 | statfs_linux_contract_p1 |
| `fstatfs` | `sys_fstatfs` | partial | fstatfs_badfd | fstatfs_badfd |
| `brk` | `sys_brk` | partial | brk_increment_smoke | brk_increment_smoke |
| `mmap` | `sys_mmap` | partial | mmap_nonanon_badfd | mmap_nonanon_badfd |
| `munmap` | `sys_munmap` | partial | munmap_einval | munmap_einval |
| `mprotect` | `sys_mprotect` | partial | mprotect_einval | mprotect_einval |
| `mincore` | `sys_mincore` | partial | mincore_efault | mincore_efault |
| `mremap` | `sys_mremap` | partial | mremap_einval | mremap_einval |
| `madvise` | `sys_madvise` | partial | madvise_einval | madvise_einval |
| `msync` | `sys_msync` | partial | msync_einval | msync_einval |
| `mlock` | `sys_mlock` | partial | mlock_enomem | mlock_enomem |
| `mlock2` | `sys_mlock2` | partial | mlock2_einval | mlock2_einval |
| `getpid` | `sys_getpid` | partial | getpid_linux_contract_p1 | getpid_linux_contract_p1 |
| `getppid` | `sys_getppid` | partial | getppid_linux_contract_p1 | getppid_linux_contract_p1 |
| `gettid` | `sys_gettid` | partial | gettid_linux_contract_p1 | gettid_linux_contract_p1 |
| `getrusage` | `sys_getrusage` | partial | getrusage_linux_contract_p1 | getrusage_linux_contract_p1 |
| `sched_yield` | `sys_sched_yield` | partial | sched_yield_linux_contract_p1 | sched_yield_linux_contract_p1 |
| `nanosleep` | `sys_nanosleep` | partial | nanosleep_linux_contract_p1 | nanosleep_linux_contract_p1 |
| `clock_nanosleep` | `sys_clock_nanosleep` | partial | clock_nanosleep_linux_contract_p1 | clock_nanosleep_linux_contract_p1 |
| `sched_getaffinity` | `sys_sched_getaffinity` | partial | sched_getaffinity_null_ptr_efault | sched_getaffinity_null_ptr_efault |
| `sched_setaffinity` | `sys_sched_setaffinity` | partial | sched_setaffinity_null_ptr_efault | sched_setaffinity_null_ptr_efault |
| `sched_getscheduler` | `sys_sched_getscheduler` | partial | sched_getscheduler_linux_contract_p1 | sched_getscheduler_linux_contract_p1 |
| `sched_setscheduler` | `sys_sched_setscheduler` | partial | sched_setscheduler_linux_contract_p1 | sched_setscheduler_linux_contract_p1 |
| `sched_getparam` | `sys_sched_getparam` | partial | sched_getparam_linux_contract_p1 | sched_getparam_linux_contract_p1 |
| `getpriority` | `sys_getpriority` | partial | getpriority_linux_contract_p1 | getpriority_linux_contract_p1 |
| `execve` | `sys_execve` | partial | execve_enoent | execve_enoent |
| `set_tid_address` | `sys_set_tid_address` | partial | set_tid_address_linux_contract_p1 | set_tid_address_linux_contract_p1 |
| `arch_prctl` | `sys_arch_prctl` | not_applicable | (planned) arch_prctl_linux_contract_p1 | — |
| `prctl` | `sys_prctl` | partial | prctl_linux_contract_p1 | prctl_linux_contract_p1 |
| `prlimit64` | `sys_prlimit64` | partial | prlimit64_linux_contract_p1 | prlimit64_linux_contract_p1 |
| `capget` | `sys_capget` | partial | capget_linux_contract_p1 | capget_linux_contract_p1 |
| `capset` | `sys_capset` | partial | capset_linux_contract_p1 | capset_linux_contract_p1 |
| `umask` | `sys_umask` | partial | umask_linux_contract_p1 | umask_linux_contract_p1 |
| `setreuid` | `sys_setreuid` | partial | setreuid_linux_contract_p1 | setreuid_linux_contract_p1 |
| `setresuid` | `sys_setresuid` | partial | setresuid_linux_contract_p1 | setresuid_linux_contract_p1 |
| `setresgid` | `sys_setresgid` | partial | setresgid_linux_contract_p1 | setresgid_linux_contract_p1 |
| `get_mempolicy` | `sys_get_mempolicy` | partial | get_mempolicy_linux_contract_p1 | get_mempolicy_linux_contract_p1 |
| `clone` | `sys_clone` | partial | clone_errno_probe | clone_errno_probe |
| `clone3` | `sys_clone3` | partial | clone3_errno_probe | clone3_errno_probe |
| `fork` | `sys_fork` | partial | fork_smoke_v1 | fork_smoke_v1 |
| `exit` | `sys_exit` | partial | exit_smoke_v1 | exit_smoke_v1 |
| `exit_group` | `sys_exit_group` | partial | exit_group_smoke_v1 | exit_group_smoke_v1 |
| `wait4` | `sys_waitpid` | partial | wait4_echild | wait4_echild |
| `getsid` | `sys_getsid` | partial | getsid_linux_contract_p1 | getsid_linux_contract_p1 |
| `setsid` | `sys_setsid` | partial | setsid_linux_contract_p1 | setsid_linux_contract_p1 |
| `getpgid` | `sys_getpgid` | partial | getpgid_linux_contract_p1 | getpgid_linux_contract_p1 |
| `setpgid` | `sys_setpgid` | partial | setpgid_linux_contract_p1 | setpgid_linux_contract_p1 |
| `rt_sigprocmask` | `sys_rt_sigprocmask` | partial | rt_sigprocmask_linux_contract_p1 | rt_sigprocmask_linux_contract_p1 |
| `rt_sigaction` | `sys_rt_sigaction` | partial | rt_sigaction_linux_contract_p1 | rt_sigaction_linux_contract_p1 |
| `rt_sigpending` | `sys_rt_sigpending` | partial | rt_sigpending_linux_contract_p1 | rt_sigpending_linux_contract_p1 |
| `rt_sigreturn` | `sys_rt_sigreturn` | not_applicable | (planned) rt_sigreturn_probe_tbd | — |
| `rt_sigtimedwait` | `sys_rt_sigtimedwait` | partial | rt_sigtimedwait_probe_tbd | rt_sigtimedwait_probe_tbd |
| `rt_sigsuspend` | `sys_rt_sigsuspend` | not_applicable | (planned) rt_sigsuspend_probe_tbd | — |
| `kill` | `sys_kill` | partial | kill_linux_contract_p1 | kill_linux_contract_p1 |
| `tkill` | `sys_tkill` | partial | tkill_linux_contract_p1 | tkill_linux_contract_p1 |
| `tgkill` | `sys_tgkill` | partial | tgkill_linux_contract_p1 | tgkill_linux_contract_p1 |
| `rt_sigqueueinfo` | `sys_rt_sigqueueinfo` | partial | rt_sigqueueinfo_linux_contract_p1 | rt_sigqueueinfo_linux_contract_p1 |
| `rt_tgsigqueueinfo` | `sys_rt_tgsigqueueinfo` | partial | rt_tgsigqueueinfo_linux_contract_p1 | rt_tgsigqueueinfo_linux_contract_p1 |
| `sigaltstack` | `sys_sigaltstack` | partial | sigaltstack_linux_contract_p1 | sigaltstack_linux_contract_p1 |
| `futex` | `sys_futex` | partial | futex_wake_nop | futex_wake_nop |
| `get_robust_list` | `sys_get_robust_list` | partial | get_robust_list_linux_contract_p1 | get_robust_list_linux_contract_p1 |
| `set_robust_list` | `sys_set_robust_list` | partial | set_robust_list_linux_contract_p1 | set_robust_list_linux_contract_p1 |
| `getuid` | `sys_getuid` | partial | getuid_linux_contract_p1 | getuid_linux_contract_p1 |
| `geteuid` | `sys_geteuid` | partial | geteuid_linux_contract_p1 | geteuid_linux_contract_p1 |
| `getgid` | `sys_getgid` | partial | getgid_linux_contract_p1 | getgid_linux_contract_p1 |
| `getegid` | `sys_getegid` | partial | getegid_linux_contract_p1 | getegid_linux_contract_p1 |
| `setuid` | `sys_setuid` | partial | setuid_linux_contract_p1 | setuid_linux_contract_p1 |
| `setgid` | `sys_setgid` | partial | setgid_linux_contract_p1 | setgid_linux_contract_p1 |
| `getgroups` | `sys_getgroups` | partial | getgroups_linux_contract_p1 | getgroups_linux_contract_p1 |
| `setgroups` | `sys_setgroups` | partial | setgroups_linux_contract_p1 | setgroups_linux_contract_p1 |
| `uname` | `sys_uname` | partial | uname_linux_contract_p1 | uname_linux_contract_p1 |
| `sysinfo` | `sys_sysinfo` | partial | sysinfo_linux_contract_p1 | sysinfo_linux_contract_p1 |
| `syslog` | `sys_syslog` | partial | syslog_bad_type | syslog_bad_type |
| `getrandom` | `sys_getrandom` | partial | getrandom_linux_contract_p1 | getrandom_linux_contract_p1 |
| `seccomp` | `sys_seccomp` | partial | seccomp_einval | seccomp_einval |
| `riscv_flush_icache` | `sys_riscv_flush_icache` | partial | riscv_flush_icache_einval | riscv_flush_icache_einval |
| `membarrier` | `sys_membarrier` | partial | membarrier_einval | membarrier_einval |
| `gettimeofday` | `sys_gettimeofday` | partial | gettimeofday_null_ptr_efault | gettimeofday_null_ptr_efault |
| `times` | `sys_times` | partial | times_linux_contract_p1 | times_linux_contract_p1 |
| `clock_gettime` | `sys_clock_gettime` | partial | clock_gettime_null_ts | clock_gettime_null_ts |
| `clock_getres` | `sys_clock_getres` | partial | clock_getres_null_ptr_efault | clock_getres_null_ptr_efault |
| `getitimer` | `sys_getitimer` | partial | getitimer_null_ptr_efault | getitimer_null_ptr_efault |
| `setitimer` | `sys_setitimer` | partial | setitimer_null_ptr_efault | setitimer_null_ptr_efault |
| `msgget` | `sys_msgget` | partial | msgget_einval | msgget_einval |
| `msgsnd` | `sys_msgsnd` | partial | msgsnd_badid | msgsnd_badid |
| `msgrcv` | `sys_msgrcv` | partial | msgrcv_badid | msgrcv_badid |
| `msgctl` | `sys_msgctl` | partial | msgctl_badid | msgctl_badid |
| `shmget` | `sys_shmget` | partial | shmget_einval | shmget_einval |
| `shmat` | `sys_shmat` | partial | shmat_badid | shmat_badid |
| `shmctl` | `sys_shmctl` | partial | shmctl_badid | shmctl_badid |
| `shmdt` | `sys_shmdt` | partial | shmdt_einval | shmdt_einval |
| `socket` | `sys_socket` | partial | socket_invalid_domain | socket_invalid_domain |
| `socketpair` | `sys_socketpair` | partial | socketpair_einval | socketpair_einval |
| `bind` | `sys_bind` | partial | bind_badfd | bind_badfd |
| `connect` | `sys_connect` | partial | connect_badfd | connect_badfd |
| `getsockname` | `sys_getsockname` | partial | getsockname_badfd | getsockname_badfd |
| `getpeername` | `sys_getpeername` | partial | getpeername_badfd | getpeername_badfd |
| `listen` | `sys_listen` | partial | listen_badfd | listen_badfd |
| `accept` | `sys_accept` | partial | accept_badfd | accept_badfd |
| `accept4` | `sys_accept4` | partial | accept4_badfd | accept4_badfd |
| `shutdown` | `sys_shutdown` | partial | shutdown_badfd | shutdown_badfd |
| `sendto` | `sys_sendto` | partial | sendto_badfd | sendto_badfd |
| `recvfrom` | `sys_recvfrom` | partial | recvfrom_badfd | recvfrom_badfd |
| `sendmsg` | `sys_sendmsg` | partial | sendmsg_badfd | sendmsg_badfd |
| `recvmsg` | `sys_recvmsg` | partial | recvmsg_badfd | recvmsg_badfd |
| `getsockopt` | `sys_getsockopt` | partial | getsockopt_badfd | getsockopt_badfd |
| `setsockopt` | `sys_setsockopt` | partial | setsockopt_badfd | setsockopt_badfd |
| `signalfd4` | `sys_signalfd4` | partial | signalfd4_einval | signalfd4_einval |
| `timerfd_create` | `sys_dummy_fd` | partial | timerfd_create_stub_semantics | timerfd_create_stub_semantics |
| `fanotify_init` | `sys_dummy_fd` | partial | fanotify_init_stub_semantics | fanotify_init_stub_semantics |
| `inotify_init1` | `sys_dummy_fd` | partial | inotify_init1_stub_semantics | inotify_init1_stub_semantics |
| `userfaultfd` | `sys_dummy_fd` | partial | userfaultfd_stub_semantics | userfaultfd_stub_semantics |
| `perf_event_open` | `sys_dummy_fd` | partial | perf_event_open_stub_semantics | perf_event_open_stub_semantics |
| `io_uring_setup` | `sys_dummy_fd` | partial | io_uring_setup_stub_semantics | io_uring_setup_stub_semantics |
| `bpf` | `sys_dummy_fd` | partial | bpf_stub_semantics | bpf_stub_semantics |
| `fsopen` | `sys_dummy_fd` | partial | fsopen_stub_semantics | fsopen_stub_semantics |
| `fspick` | `sys_dummy_fd` | partial | fspick_stub_semantics | fspick_stub_semantics |
| `open_tree` | `sys_dummy_fd` | partial | open_tree_stub_semantics | open_tree_stub_semantics |
| `memfd_secret` | `sys_dummy_fd` | partial | memfd_secret_stub_semantics | memfd_secret_stub_semantics |
| `timer_create` | `Ok(0)` | partial | timer_create_noop_semantics | timer_create_noop_semantics |
| `timer_gettime` | `Ok(0)` | partial | timer_gettime_noop_semantics | timer_gettime_noop_semantics |
| `timer_settime` | `Ok(0)` | partial | timer_settime_noop_semantics | timer_settime_noop_semantics |

## 兼容矩阵中有、但不在分发表 JSON 中的条目

| syscall | matrix_parity | matrix_probe | notes |
|---------|---------------|--------------|-------|
| `io_zero_rw` | partial | io_zero_rw | read stdin count=0 + write stdout len=0；仅 .cases，无 .line |
