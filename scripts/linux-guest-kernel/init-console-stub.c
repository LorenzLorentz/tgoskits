/*
 * PID 1 for initramfs: open /dev/console on 0/1/2, fork+execve("/probe"), then poweroff.
 * Musl (and most libc) static ELFs fault as true PID 1 (tp/TLS unset); run the probe in a child.
 */
#define AT_FDCWD		(-100)
#define O_RDWR			0x00000002

#define __NR_openat		56
#define __NR_dup3		24
#define __NR_close		57
#define __NR_clone		220
#define __NR_execve		221
#define __NR_exit		93
#define __NR_wait4		260
#define __NR_reboot		142

#define SIGCHLD			17

#define LINUX_REBOOT_MAGIC1		0xfee1deadUL
#define LINUX_REBOOT_MAGIC2		672274793UL
#define LINUX_REBOOT_CMD_POWER_OFF	0x4321fedcUL

static inline long sc1(long n, long a)
{
	register long a7 __asm__("a7") = n;
	register long a0 __asm__("a0") = a;
	__asm__ volatile("scall" : "+r"(a0) : "r"(a7) : "memory");
	return a0;
}

static inline long sc3(long n, long a, long b, long c)
{
	register long a7 __asm__("a7") = n;
	register long a0 __asm__("a0") = a;
	register long a1 __asm__("a1") = b;
	register long a2 __asm__("a2") = c;
	__asm__ volatile("scall"
			 : "+r"(a0), "+r"(a1), "+r"(a2)
			 : "r"(a7)
			 : "memory");
	return a0;
}

static inline long sc4(long n, long a, long b, long c, long d)
{
	register long a7 __asm__("a7") = n;
	register long a0 __asm__("a0") = a;
	register long a1 __asm__("a1") = b;
	register long a2 __asm__("a2") = c;
	register long a3 __asm__("a3") = d;
	__asm__ volatile("scall"
			 : "+r"(a0), "+r"(a1), "+r"(a2), "+r"(a3)
			 : "r"(a7)
			 : "memory");
	return a0;
}

static inline long sc5(long n, long a, long b, long c, long d, long e)
{
	register long a7 __asm__("a7") = n;
	register long a0 __asm__("a0") = a;
	register long a1 __asm__("a1") = b;
	register long a2 __asm__("a2") = c;
	register long a3 __asm__("a3") = d;
	register long a4 __asm__("a4") = e;
	__asm__ volatile("scall"
			 : "+r"(a0), "+r"(a1), "+r"(a2), "+r"(a3), "+r"(a4)
			 : "r"(a7)
			 : "memory");
	return a0;
}

static void die(int code)
{
	sc1(__NR_exit, (long)(code & 255));
	for (;;)
		;
}

void _start(void)
{
	/*
	 * -nostdlib: no crt0 — gp must be initialized or strings/globals via gp are garbage
	 * (execve then fails; kernel reports init exit 127).
	 */
	__asm__ volatile(
		".option push\n\t"
		".option norelax\n\t"
		"lla gp, __global_pointer$\n\t"
		".option pop" ::: "gp", "memory");

	long fd;
	static char path_console[] = "/dev/console";
	static char path_probe[] = "/probe";
	char *argv[] = { path_probe, (char *)0 };
	char *envp[] = { (char *)0 };

	fd = sc4(__NR_openat, (long)AT_FDCWD, (long)path_console, (long)O_RDWR, 0);
	if (fd >= 0) {
		/* dup3(old, new, flags); RISC-V Linux has no dup2 syscall (23 is dup). */
		sc4(__NR_dup3, fd, 0, 0, 0);
		sc4(__NR_dup3, fd, 1, 0, 0);
		sc4(__NR_dup3, fd, 2, 0, 0);
		if (fd > 2)
			sc1(__NR_close, fd);
	}

	/*
	 * sys_clone(flags, newsp, parent_tid, tls, child_tid) — newsp=0 => fork-like.
	 */
	{
		long pid = sc5(__NR_clone, (long)SIGCHLD, 0, 0, 0, 0);

		if (pid == 0) {
			sc3(__NR_execve, (long)path_probe, (long)argv, (long)envp);
			die(127);
		}
		if (pid < 0)
			die(1);

		{
			int st = 0;

			sc4(__NR_wait4, pid, (long)&st, 0, 0);
		}

		/* Stop qemu cleanly after probe exits (init must not exit without this). */
		sc4(__NR_reboot, (long)LINUX_REBOOT_MAGIC1, (long)LINUX_REBOOT_MAGIC2,
		    (long)LINUX_REBOOT_CMD_POWER_OFF, 0);
		die(1);
	}
}
