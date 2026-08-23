/* Reference kernels, compiled gcc -O2. Identical algorithms to kernels/*.bp
 * and rust/src/main.rs. Prints per-run wall times in ns for aggregation. */
#define _POSIX_C_SOURCE 200809L
#include <stdio.h>
#include <stdint.h>
#include <time.h>
#include <stdlib.h>

static uint64_t now_ns(void) {
    struct timespec ts; clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint64_t)ts.tv_sec * 1000000000ull + (uint64_t)ts.tv_nsec;
}

static long long k1(void) {
    static volatile long long nseed = 1000000;
    long long n = nseed;
    static volatile long long vs = 0;
    vs = 0;
    long long i = n;
    while (i > 0) { vs = vs + i; i -= 1; }
    return vs;
}
static long long fib(long long n) {
    if (n < 2) return n;
    return fib(n - 1) + fib(n - 2);
}
static long long k2(void) { return fib(25); }
static long long k3(void) {
    static volatile long long nseed = 300;
    long long n = nseed;
    static volatile long long va = 0;
    va = 0;
    long long x = n;
    while (x > 0) {
        long long y = n;
        while (y > 0) { va = va + x * 2 + y * 3; y -= 1; }
        x -= 1;
    }
    return va;
}
static long long k4(void) {
    long long v = 1, i = 2000000;
    while (i > 0) { v = (v + i * 7) * 3 - 11; i -= 1; }
    return v;
}

int main(int argc, char **argv) {
    if (argc < 3) { fprintf(stderr, "usage: %s <kernel> <iters>\n", argv[0]); return 2; }
    int which = atoi(argv[1]), iters = atoi(argv[2]);
    long long (*fn)(void) = which == 1 ? k1 : which == 2 ? k2 : which == 3 ? k3 : k4;
    static uint64_t runs[4096];
    if (iters > 4096) iters = 4096;
    volatile long long warm = fn(); (void)warm;
    for (int i = 0; i < iters; i++) {
        uint64_t t0 = now_ns();
        volatile long long r = fn(); (void)r;
        runs[i] = now_ns() - t0;
    }
    printf("result=%lld\n", fn());
    printf("ns");
    for (int i = 0; i < iters; i++) printf(" %llu", (unsigned long long)runs[i]);
    printf("\n");
    return 0;
}
