/* Bebop freestanding runtime — raw aarch64 Linux syscalls, no libc. */
#include "syscall.h"

#include <stdio.h>
#include <string.h>

#define SYS_WRITE 64
#define SYS_EXIT 93

long bp_syscall1(long n, long a) {
    register long x8 __asm__("x8") = n;
    register long x0 __asm__("x0") = a;
    __asm__ __volatile__("svc #0" : "+r"(x0) : "r"(x8) : "memory");
    return x0;
}

long bp_syscall3(long n, long a, long b, long c) {
    register long x8 __asm__("x8") = n;
    register long x0 __asm__("x0") = a;
    register long x1 __asm__("x1") = b;
    register long x2 __asm__("x2") = c;
    __asm__ __volatile__("svc #0" : "+r"(x0) : "r"(x8), "r"(x1), "r"(x2) : "memory");
    return x0;
}

long bp_write(int fd, const void *buf, size_t n) {
    return bp_syscall3(SYS_WRITE, fd, (long)buf, (long)n);
}

void bp_exit(int code) {
    bp_syscall1(SYS_EXIT, code);
    __builtin_unreachable();
}

long bp_mmap(void *a, size_t l, int p, int f, int d, long o) { (void)a;(void)l;(void)p;(void)f;(void)d;(void)o;return 0; }
int bp_open(const char *path, int flags, int mode) { (void)mode; return (int)bp_syscall3(56,-100,(long)path,flags); }
int bp_close(int fd) { return (int)bp_syscall1(57,fd); }
long bp_read(int fd, void *buf, size_t n) { return bp_syscall3(63,fd,(long)buf,n); }
int bp_nanosleep(unsigned s, unsigned ns) { return (int)bp_syscall1(101,s*1000000000UL+ns); }

int syscall_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define A(cond, name)                                                          \
    do {                                                                       \
        int c_ = (int)(cond);                                                  \
        int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n",                  \
                          c_ ? "ok" : "FAIL", name);                           \
        if (r_ > 0) pos += (size_t)r_;                                         \
        if (!c_) all_ok = 0;                                                   \
    } while (0)

    /* write to stdout via a raw syscall, bypassing libc entirely */
    const char *msg = "Bebop raw-syscall write (no libc)\n";
    long w = bp_write(1, msg, strlen(msg));
    A(w == (long)strlen(msg), "raw write(2) returns the full byte count");

    /* a bogus fd yields a negative errno, not a crash */
    long bad = bp_write(99999, msg, 1);
    A(bad < 0, "write to bad fd returns negative errno (fail-closed)");

    return all_ok ? 0 : -1;
}
