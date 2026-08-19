/* Bebop freestanding runtime (23B / #29): raw syscalls, no libc.
 *
 * The micro-entry `_start` initializes the arena + JIT table directly and
 * talks to the kernel through `svc #0` — no crt0, no stdio, no libc. This is
 * the substrate a freestanding Bebop binary (or embedded target) boots on.
 */
#ifndef BEBOP_SYSCALL_H
#define BEBOP_SYSCALL_H

#include <stddef.h>

/* raw aarch64 Linux syscalls (SVC). Returns the kernel's return value. */
long bp_syscall1(long n, long a);
long bp_syscall3(long n, long a, long b, long c);

/* write(2) via raw syscall (no libc). Returns bytes written or negative errno. */
long bp_write(int fd, const void *buf, size_t n);
/* exit(2) via raw syscall — does not return. */
void bp_exit(int code);
long bp_mmap(void *addr, size_t len, int prot, int flags, int fd, long off);
int  bp_open(const char *path, int flags, int mode);
long bp_read(int fd, void *buf, size_t n);
int  bp_close(int fd);
int  bp_nanosleep(unsigned sec, unsigned nsec);

int syscall_self_test(char *out, size_t cap);

#endif /* BEBOP_SYSCALL_H */
