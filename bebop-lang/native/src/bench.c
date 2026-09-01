/* Bebop benchmark — NTT convolution vs naive O(n^2), hypervector ops/s. */
#define _POSIX_C_SOURCE 200809L

#include "src/ntt.h"
#include "src/hyper.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

static double now_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec * 1000.0 + (double)ts.tv_nsec / 1e6;
}

static void bench_ntt(int n) {
    uint64_t *a = calloc((size_t)n, sizeof(uint64_t));
    uint64_t *b = calloc((size_t)n, sizeof(uint64_t));
    uint64_t *out = calloc((size_t)(2 * n - 1), sizeof(uint64_t));
    for (int i = 0; i < n; i++) {
        a[i] = (uint64_t)(i % 1000);
        b[i] = (uint64_t)((i * 7) % 1000);
    }
    double t0 = now_ms();
    for (int rep = 0; rep < 100; rep++) {
        ntt_convolve(a, (size_t)n, b, (size_t)n, out);
    }
    double t1 = now_ms();
    double per = (t1 - t0) / 100.0;

    /* naive O(n^2) — only for small n */
    double naive = -1.0;
    if (n <= 4096) {
        double s0 = now_ms();
        for (int rep = 0; rep < 100; rep++) {
            for (int i = 0; i < n; i++) {
                for (int j = 0; j < n; j++) {
                    out[i + j] = (out[i + j] + a[i] * b[j]) % BEBOP_NTT_MOD;
                }
            }
        }
        double s1 = now_ms();
        naive = (s1 - s0) / 100.0;
    }

    printf("NTT n=%d: Bebop=%.4f ms", n, per);
    if (naive >= 0) {
        printf("  naive=%.4f ms  speedup=%.1fx", naive, naive / per);
    }
    printf("\n");
    free(a);
    free(b);
    free(out);
}

static void bench_hyper(void) {
    Hypervector a = hv_code(1), b = hv_code(2);
    int nops = 1000000;
    double t0 = now_ms();
    uint64_t acc = 0;
    for (int i = 0; i < nops; i++) {
        Hypervector c = hv_bind(&a, &b);
        acc += hv_hamming(&a, &c);
    }
    double t1 = now_ms();
    printf("hypervector bind+hamming: %d ops in %.1f ms = %.1f Mops/s (acc=%llu)\n",
           nops, t1 - t0, (double)nops / (t1 - t0) / 1000.0, (unsigned long long)acc);
}

int main(void) {
    printf("=== Bebop benchmarks ===\n");
    bench_ntt(256);
    bench_ntt(1024);
    bench_ntt(4096);
    bench_ntt(16384);
    bench_hyper();
    return 0;
}
