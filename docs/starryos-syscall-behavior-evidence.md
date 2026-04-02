# StarryOS syscall 行为证据（Linux oracle / guest 矩阵）

由 `scripts/render_starry_syscall_inventory.py --step 3` 生成。

- **matrix_probe**：矩阵 `contract_probe`；若仅有脚手架则显示 `(planned) …`（来自 `planned_contract_probe`，见 [docs/starryos-syscall-probe-rollout.yaml](docs/starryos-syscall-probe-rollout.yaml)）。
- **catalog_probes**：catalog `tests:` 中的 contract 文件名（不含路径）。
- **matrix_parity**：矩阵 `parity`（无行则为 —）。

全量 **Linux user oracle**：`VERIFY_STRICT=1 test-suit/starryos/scripts/run-diff-probes.sh verify-oracle-all`。
全量 **SMP2 guest vs oracle**：`test-suit/starryos/scripts/run-smp2-guest-matrix.sh`。

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
| `open` | `sys_open` | not_applicable | (planned) open_enoent | — |
| `openat` | `sys_openat` | partial | openat_badfd | openat_badfd, openat_enoent |
| `close` | `sys_close` | partial | close_badfd | close_badfd |
| `close_range` | `sys_close_range` | not_applicable | (planned) close_range_badfd | — |
| `dup` | `sys_dup` | partial | dup_badfd | dup_badfd |
| `dup2` | `sys_dup2` | not_applicable | (planned) dup2_badfd | — |
| `dup3` | `sys_dup3` | not_applicable | (planned) dup3_badfd | — |
| `fcntl` | `sys_fcntl` | partial | fcntl_badfd | fcntl_badfd |
| `flock` | `sys_flock` | not_applicable | (planned) flock_badfd | — |
| `read` | `sys_read` | partial | read_stdin_zero | read_stdin_zero |
| `readv` | `sys_readv` | not_applicable | (planned) readv_badfd | — |
| `write` | `sys_write` | partial | write_stdout | write_stdout |
| `writev` | `sys_writev` | not_applicable | (planned) writev_badfd | — |
| `lseek` | `sys_lseek` | partial | lseek_badfd | lseek_badfd |
| `truncate` | `sys_truncate` | not_applicable | (planned) truncate_enoent | — |
| `ftruncate` | `sys_ftruncate` | not_applicable | (planned) ftruncate_badfd | — |
| `fallocate` | `sys_fallocate` | not_applicable | (planned) fallocate_badfd | — |
| `fsync` | `sys_fsync` | not_applicable | (planned) fsync_badfd | — |
| `fdatasync` | `sys_fdatasync` | not_applicable | (planned) fdatasync_badfd | — |
| `fadvise64` | `sys_fadvise64` | not_applicable | (planned) fadvise64_badfd | — |
| `pread64` | `sys_pread64` | not_applicable | (planned) pread64_badfd | — |
| `pwrite64` | `sys_pwrite64` | not_applicable | (planned) pwrite64_badfd | — |
| `preadv` | `sys_preadv` | not_applicable | (planned) preadv_badfd | — |
| `pwritev` | `sys_pwritev` | not_applicable | (planned) pwritev_badfd | — |
| `preadv2` | `sys_preadv2` | not_applicable | (planned) preadv2_badfd | — |
| `pwritev2` | `sys_pwritev2` | not_applicable | (planned) pwritev2_badfd | — |
| `sendfile` | `sys_sendfile` | not_applicable | (planned) sendfile_badfd | — |
| `copy_file_range` | `sys_copy_file_range` | not_applicable | (planned) copy_file_range_badfd | — |
| `splice` | `sys_splice` | not_applicable | (planned) splice_badfd | — |
| `poll` | `sys_poll` | not_applicable | (planned) poll_linux_contract_p1 | — |
| `ppoll` | `sys_ppoll` | partial | ppoll_zero_fds | ppoll_zero_fds |
| `select` | `sys_select` | not_applicable | (planned) select_linux_contract_p1 | — |
| `pselect6` | `sys_pselect6` | not_applicable | (planned) pselect6_linux_contract_p1 | — |
| `epoll_create1` | `sys_epoll_create1` | not_applicable | (planned) epoll_create1_einval | — |
| `epoll_ctl` | `sys_epoll_ctl` | not_applicable | (planned) epoll_ctl_badfd | — |
| `epoll_pwait` | `sys_epoll_pwait` | not_applicable | (planned) epoll_pwait_badfd | — |
| `epoll_pwait2` | `sys_epoll_pwait2` | not_applicable | (planned) epoll_pwait2_badfd | — |
| `mount` | `sys_mount` | not_applicable | (planned) mount_enoent | — |
| `umount2` | `sys_umount2` | not_applicable | (planned) umount2_enoent | — |
| `pipe2` | `sys_pipe2` | partial | pipe2_nullfd | pipe2_nullfd |
| `pipe` | `sys_pipe2` | not_applicable | (planned) pipe_linux_contract_p1 | — |
| `eventfd2` | `sys_eventfd2` | not_applicable | (planned) eventfd2_einval | — |
| `pidfd_open` | `sys_pidfd_open` | not_applicable | (planned) pidfd_open_esrch | — |
| `pidfd_getfd` | `sys_pidfd_getfd` | not_applicable | (planned) pidfd_getfd_badfd | — |
| `pidfd_send_signal` | `sys_pidfd_send_signal` | not_applicable | (planned) pidfd_send_signal_badfd | — |
| `memfd_create` | `sys_memfd_create` | not_applicable | (planned) memfd_create_einval | — |
| `stat` | `sys_stat` | not_applicable | (planned) stat_enoent | — |
| `fstat` | `sys_fstat` | not_applicable | (planned) fstat_badfd | — |
| `lstat` | `sys_lstat` | not_applicable | (planned) lstat_enoent | — |
| `newfstatat` | `sys_fstatat` | not_applicable | (planned) newfstatat_enoent | — |
| `fstatat` | `sys_fstatat` | not_applicable | (planned) fstatat_enoent | — |
| `statx` | `sys_statx` | not_applicable | (planned) statx_enoent | — |
| `access` | `sys_access` | not_applicable | (planned) access_enoent | — |
| `faccessat` | `sys_faccessat2` | not_applicable | (planned) faccessat_enoent | — |
| `faccessat2` | `sys_faccessat2` | not_applicable | (planned) faccessat2_enoent | — |
| `statfs` | `sys_statfs` | not_applicable | (planned) statfs_linux_contract_p1 | — |
| `fstatfs` | `sys_fstatfs` | not_applicable | (planned) fstatfs_badfd | — |
| `brk` | `sys_brk` | not_applicable | (planned) brk_increment_smoke | — |
| `mmap` | `sys_mmap` | not_applicable | (planned) mmap_nonanon_badfd | — |
| `munmap` | `sys_munmap` | not_applicable | (planned) munmap_einval | — |
| `mprotect` | `sys_mprotect` | not_applicable | (planned) mprotect_einval | — |
| `mincore` | `sys_mincore` | not_applicable | (planned) mincore_efault | — |
| `mremap` | `sys_mremap` | not_applicable | (planned) mremap_einval | — |
| `madvise` | `sys_madvise` | not_applicable | (planned) madvise_einval | — |
| `msync` | `sys_msync` | not_applicable | (planned) msync_einval | — |
| `mlock` | `sys_mlock` | not_applicable | (planned) mlock_eagain | — |
| `mlock2` | `sys_mlock2` | not_applicable | (planned) mlock2_einval | — |
| `getpid` | `sys_getpid` | not_applicable | (planned) getpid_linux_contract_p1 | — |
| `getppid` | `sys_getppid` | not_applicable | (planned) getppid_linux_contract_p1 | — |
| `gettid` | `sys_gettid` | not_applicable | (planned) gettid_linux_contract_p1 | — |
| `getrusage` | `sys_getrusage` | not_applicable | (planned) getrusage_linux_contract_p1 | — |
| `sched_yield` | `sys_sched_yield` | not_applicable | (planned) sched_yield_linux_contract_p1 | — |
| `nanosleep` | `sys_nanosleep` | not_applicable | (planned) nanosleep_linux_contract_p1 | — |
| `clock_nanosleep` | `sys_clock_nanosleep` | not_applicable | (planned) clock_nanosleep_linux_contract_p1 | — |
| `sched_getaffinity` | `sys_sched_getaffinity` | not_applicable | (planned) sched_getaffinity_null_ptr_efault | — |
| `sched_setaffinity` | `sys_sched_setaffinity` | not_applicable | (planned) sched_setaffinity_null_ptr_efault | — |
| `sched_getscheduler` | `sys_sched_getscheduler` | not_applicable | (planned) sched_getscheduler_linux_contract_p1 | — |
| `sched_setscheduler` | `sys_sched_setscheduler` | not_applicable | (planned) sched_setscheduler_linux_contract_p1 | — |
| `sched_getparam` | `sys_sched_getparam` | not_applicable | (planned) sched_getparam_linux_contract_p1 | — |
| `getpriority` | `sys_getpriority` | not_applicable | (planned) getpriority_linux_contract_p1 | — |
| `execve` | `sys_execve` | partial | execve_enoent | execve_enoent |
| `set_tid_address` | `sys_set_tid_address` | not_applicable | (planned) set_tid_address_linux_contract_p1 | — |
| `arch_prctl` | `sys_arch_prctl` | not_applicable | (planned) arch_prctl_linux_contract_p1 | — |
| `prctl` | `sys_prctl` | not_applicable | (planned) prctl_linux_contract_p1 | — |
| `prlimit64` | `sys_prlimit64` | not_applicable | (planned) prlimit64_linux_contract_p1 | — |
| `capget` | `sys_capget` | not_applicable | (planned) capget_linux_contract_p1 | — |
| `capset` | `sys_capset` | not_applicable | (planned) capset_linux_contract_p1 | — |
| `umask` | `sys_umask` | not_applicable | (planned) umask_linux_contract_p1 | — |
| `setreuid` | `sys_setreuid` | not_applicable | (planned) setreuid_linux_contract_p1 | — |
| `setresuid` | `sys_setresuid` | not_applicable | (planned) setresuid_linux_contract_p1 | — |
| `setresgid` | `sys_setresgid` | not_applicable | (planned) setresgid_linux_contract_p1 | — |
| `get_mempolicy` | `sys_get_mempolicy` | not_applicable | (planned) get_mempolicy_linux_contract_p1 | — |
| `clone` | `sys_clone` | not_applicable | (planned) clone_errno_probe | — |
| `clone3` | `sys_clone3` | not_applicable | (planned) clone3_errno_probe | — |
| `fork` | `sys_fork` | not_applicable | (planned) fork_smoke_v1 | — |
| `exit` | `sys_exit` | not_applicable | (planned) exit_smoke_v1 | — |
| `exit_group` | `sys_exit_group` | not_applicable | (planned) exit_group_smoke_v1 | — |
| `wait4` | `sys_waitpid` | partial | wait4_echild | wait4_echild |
| `getsid` | `sys_getsid` | not_applicable | (planned) getsid_linux_contract_p1 | — |
| `setsid` | `sys_setsid` | not_applicable | (planned) setsid_linux_contract_p1 | — |
| `getpgid` | `sys_getpgid` | not_applicable | (planned) getpgid_linux_contract_p1 | — |
| `setpgid` | `sys_setpgid` | not_applicable | (planned) setpgid_linux_contract_p1 | — |
| `rt_sigprocmask` | `sys_rt_sigprocmask` | not_applicable | (planned) rt_sigprocmask_linux_contract_p1 | — |
| `rt_sigaction` | `sys_rt_sigaction` | not_applicable | (planned) rt_sigaction_linux_contract_p1 | — |
| `rt_sigpending` | `sys_rt_sigpending` | not_applicable | (planned) rt_sigpending_linux_contract_p1 | — |
| `rt_sigreturn` | `sys_rt_sigreturn` | not_applicable | (planned) rt_sigreturn_probe_tbd | — |
| `rt_sigtimedwait` | `sys_rt_sigtimedwait` | not_applicable | (planned) rt_sigtimedwait_probe_tbd | — |
| `rt_sigsuspend` | `sys_rt_sigsuspend` | not_applicable | (planned) rt_sigsuspend_probe_tbd | — |
| `kill` | `sys_kill` | not_applicable | (planned) kill_linux_contract_p1 | — |
| `tkill` | `sys_tkill` | not_applicable | (planned) tkill_linux_contract_p1 | — |
| `tgkill` | `sys_tgkill` | not_applicable | (planned) tgkill_linux_contract_p1 | — |
| `rt_sigqueueinfo` | `sys_rt_sigqueueinfo` | not_applicable | (planned) rt_sigqueueinfo_linux_contract_p1 | — |
| `rt_tgsigqueueinfo` | `sys_rt_tgsigqueueinfo` | not_applicable | (planned) rt_tgsigqueueinfo_linux_contract_p1 | — |
| `sigaltstack` | `sys_sigaltstack` | not_applicable | (planned) sigaltstack_linux_contract_p1 | — |
| `futex` | `sys_futex` | partial | futex_wake_nop | futex_wake_nop |
| `get_robust_list` | `sys_get_robust_list` | not_applicable | (planned) get_robust_list_linux_contract_p1 | — |
| `set_robust_list` | `sys_set_robust_list` | not_applicable | (planned) set_robust_list_linux_contract_p1 | — |
| `getuid` | `sys_getuid` | not_applicable | (planned) getuid_linux_contract_p1 | — |
| `geteuid` | `sys_geteuid` | not_applicable | (planned) geteuid_linux_contract_p1 | — |
| `getgid` | `sys_getgid` | not_applicable | (planned) getgid_linux_contract_p1 | — |
| `getegid` | `sys_getegid` | not_applicable | (planned) getegid_linux_contract_p1 | — |
| `setuid` | `sys_setuid` | not_applicable | (planned) setuid_linux_contract_p1 | — |
| `setgid` | `sys_setgid` | not_applicable | (planned) setgid_linux_contract_p1 | — |
| `getgroups` | `sys_getgroups` | not_applicable | (planned) getgroups_linux_contract_p1 | — |
| `setgroups` | `sys_setgroups` | not_applicable | (planned) setgroups_linux_contract_p1 | — |
| `uname` | `sys_uname` | not_applicable | (planned) uname_linux_contract_p1 | — |
| `sysinfo` | `sys_sysinfo` | not_applicable | (planned) sysinfo_linux_contract_p1 | — |
| `syslog` | `sys_syslog` | not_applicable | (planned) syslog_bad_type | — |
| `getrandom` | `sys_getrandom` | not_applicable | (planned) getrandom_linux_contract_p1 | — |
| `seccomp` | `sys_seccomp` | not_applicable | (planned) seccomp_einval | — |
| `riscv_flush_icache` | `sys_riscv_flush_icache` | not_applicable | (planned) riscv_flush_icache_einval | — |
| `membarrier` | `sys_membarrier` | not_applicable | (planned) membarrier_einval | — |
| `gettimeofday` | `sys_gettimeofday` | not_applicable | (planned) gettimeofday_null_ptr_efault | — |
| `times` | `sys_times` | not_applicable | (planned) times_linux_contract_p1 | — |
| `clock_gettime` | `sys_clock_gettime` | partial | clock_gettime_null_ts | clock_gettime_null_ts |
| `clock_getres` | `sys_clock_getres` | not_applicable | (planned) clock_getres_null_ptr_efault | — |
| `getitimer` | `sys_getitimer` | not_applicable | (planned) getitimer_null_ptr_efault | — |
| `setitimer` | `sys_setitimer` | not_applicable | (planned) setitimer_null_ptr_efault | — |
| `msgget` | `sys_msgget` | not_applicable | (planned) msgget_einval | — |
| `msgsnd` | `sys_msgsnd` | not_applicable | (planned) msgsnd_badid | — |
| `msgrcv` | `sys_msgrcv` | not_applicable | (planned) msgrcv_badid | — |
| `msgctl` | `sys_msgctl` | not_applicable | (planned) msgctl_badid | — |
| `shmget` | `sys_shmget` | not_applicable | (planned) shmget_einval | — |
| `shmat` | `sys_shmat` | not_applicable | (planned) shmat_badid | — |
| `shmctl` | `sys_shmctl` | not_applicable | (planned) shmctl_badid | — |
| `shmdt` | `sys_shmdt` | not_applicable | (planned) shmdt_einval | — |
| `socket` | `sys_socket` | not_applicable | (planned) socket_invalid_domain | — |
| `socketpair` | `sys_socketpair` | not_applicable | (planned) socketpair_einval | — |
| `bind` | `sys_bind` | not_applicable | (planned) bind_badfd | — |
| `connect` | `sys_connect` | not_applicable | (planned) connect_badfd | — |
| `getsockname` | `sys_getsockname` | not_applicable | (planned) getsockname_badfd | — |
| `getpeername` | `sys_getpeername` | not_applicable | (planned) getpeername_badfd | — |
| `listen` | `sys_listen` | not_applicable | (planned) listen_badfd | — |
| `accept` | `sys_accept` | not_applicable | (planned) accept_badfd | — |
| `accept4` | `sys_accept4` | not_applicable | (planned) accept4_badfd | — |
| `shutdown` | `sys_shutdown` | not_applicable | (planned) shutdown_badfd | — |
| `sendto` | `sys_sendto` | not_applicable | (planned) sendto_badfd | — |
| `recvfrom` | `sys_recvfrom` | not_applicable | (planned) recvfrom_badfd | — |
| `sendmsg` | `sys_sendmsg` | not_applicable | (planned) sendmsg_badfd | — |
| `recvmsg` | `sys_recvmsg` | not_applicable | (planned) recvmsg_badfd | — |
| `getsockopt` | `sys_getsockopt` | not_applicable | (planned) getsockopt_badfd | — |
| `setsockopt` | `sys_setsockopt` | not_applicable | (planned) setsockopt_badfd | — |
| `signalfd4` | `sys_signalfd4` | not_applicable | (planned) signalfd4_einval | — |
| `timerfd_create` | `sys_dummy_fd` | not_applicable | (planned) timerfd_create_stub_semantics | — |
| `fanotify_init` | `sys_dummy_fd` | not_applicable | (planned) fanotify_init_stub_semantics | — |
| `inotify_init1` | `sys_dummy_fd` | not_applicable | (planned) inotify_init1_stub_semantics | — |
| `userfaultfd` | `sys_dummy_fd` | not_applicable | (planned) userfaultfd_stub_semantics | — |
| `perf_event_open` | `sys_dummy_fd` | not_applicable | (planned) perf_event_open_stub_semantics | — |
| `io_uring_setup` | `sys_dummy_fd` | not_applicable | (planned) io_uring_setup_stub_semantics | — |
| `bpf` | `sys_dummy_fd` | not_applicable | (planned) bpf_stub_semantics | — |
| `fsopen` | `sys_dummy_fd` | not_applicable | (planned) fsopen_stub_semantics | — |
| `fspick` | `sys_dummy_fd` | not_applicable | (planned) fspick_stub_semantics | — |
| `open_tree` | `sys_dummy_fd` | not_applicable | (planned) open_tree_stub_semantics | — |
| `memfd_secret` | `sys_dummy_fd` | not_applicable | (planned) memfd_secret_stub_semantics | — |
| `timer_create` | `Ok(0)` | not_applicable | (planned) timer_create_noop_semantics | — |
| `timer_gettime` | `Ok(0)` | not_applicable | (planned) timer_gettime_noop_semantics | — |
| `timer_settime` | `Ok(0)` | not_applicable | (planned) timer_settime_noop_semantics | — |

## 兼容矩阵中有、但不在分发表 JSON 中的条目

| syscall | matrix_parity | matrix_probe | notes |
|---------|---------------|--------------|-------|
| `io_zero_rw` | partial | io_zero_rw | read stdin count=0 + write stdout len=0；仅 .cases，无 .line |
