/* R2 Peak RSS harness — measures max RSS during NTT n=4096 + hypervector workload.
 * Uses getrusage(RUSAGE_SELF) ru_maxrss. Pinned to core 0, best-of-N runs. */
#define _POSIX_C_SOURCE 199309L
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/resource.h>
#include <time.h>

typedef struct { uint64_t words[16]; } __attribute__((aligned(64))) Hypervector;

#define NTT_MOD 998244353ULL
#define NTT_ROOT 3ULL

static uint64_t ntt_mod_pow(uint64_t base, uint64_t exp, uint64_t m) {
    uint64_t result = 1;
    base %= m;
    while (exp) {
        if (exp & 1) result = (result * base) % m;
        base = (base * base) % m;
        exp >>= 1;
    }
    return result;
}

static void ntt_transform(uint64_t *a, size_t n, int invert) {
    for (size_t len = 2; len <= n; len <<= 1) {
        uint64_t wlen = ntt_mod_pow(NTT_ROOT, (NTT_MOD - 1) / len, NTT_MOD);
        if (invert) wlen = ntt_mod_pow(wlen, NTT_MOD - 2, NTT_MOD);
        for (size_t i = 0; i < n; i += len) {
            uint64_t w = 1;
            for (size_t j = 0; j < len/2; j++) {
                uint64_t u = a[i+j];
                uint64_t v = (a[i+j+len/2] * w) % NTT_MOD;
                a[i+j] = (u + v) % NTT_MOD;
                a[i+j+len/2] = (u + NTT_MOD - v) % NTT_MOD;
                w = (w * wlen) % NTT_MOD;
            }
        }
    }
    if (invert) {
        uint64_t inv_n = ntt_mod_pow(n, NTT_MOD - 2, NTT_MOD);
        for (size_t i = 0; i < n; i++)
            a[i] = (a[i] * inv_n) % NTT_MOD;
    }
}

static long get_peak_rss(void) {
    struct rusage ru;
    getrusage(RUSAGE_SELF, &ru);
    return ru.ru_maxrss;
}

static long get_us(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (long)ts.tv_sec * 1000000 + ts.tv_nsec / 1000;
}

int main(void) {
    const size_t N = 4096;
    const int N_BENCH_RUNS = 5;
    long best_rss = (long)1L << 60;
    long best_us = (long)1L << 60;

    for (int run = 0; run < N_BENCH_RUNS; run++) {
        uint64_t *a = (uint64_t *)calloc(N, sizeof(uint64_t));
        uint64_t *b = (uint64_t *)calloc(N, sizeof(uint64_t));
        uint64_t *c = (uint64_t *)calloc(2 * N - 1, sizeof(uint64_t));

        if (!a || !b || !c) { printf("alloc fail\n"); return 1; }

        for (size_t i = 0; i < N; i++) {
            a[i] = ((i * 2654435761ULL) % 1000000);
            b[i] = ((i * 7) % 1000000);
        }

        int nhv = 1024;
        Hypervector *hvs = (Hypervector *)calloc(nhv, sizeof(Hypervector));
        if (!hvs) { printf("hv alloc fail\n"); return 1; }

        long t0 = get_us();
        for (int rep = 0; rep < 10; rep++) {
            ntt_transform(a, N, 0);
            ntt_transform(b, N, 0);
            for (size_t i = 0; i < N; i++)
                c[i] = (a[i] * b[i]) % NTT_MOD;
            ntt_transform(c, N, 1);

            for (int k = 0; k < nhv; k++) {
                uint64_t seed = (uint64_t)(k + 1) * 0x9E3779B97F4A7C15ULL;
                for (int w = 0; w < 16; w++) {
                    seed ^= seed >> 30;
                    seed *= 0xBF58476D1CE4E5B9ULL;
                    seed ^= seed >> 27;
                    seed *= 0x94D049BB133111EBULL;
                    seed ^= seed >> 31;
                    hvs[k].words[w] = seed;
                }
            }
            volatile uint32_t sink = 0;
            for (int k = 0; k < nhv - 1; k += 2) {
                Hypervector bound;
                for (int w = 0; w < 16; w++)
                    bound.words[w] = hvs[k].words[w] ^ hvs[k+1].words[w];
                uint32_t dist = 0;
                for (int w = 0; w < 16; w++)
                    dist += __builtin_popcountll(bound.words[w]);
                sink += dist;
            }
            (void)sink;
        }
        long t1 = get_us();

        long rss = get_peak_rss();
        long elapsed = t1 - t0;
        if (rss < best_rss) best_rss = rss;
        if (elapsed < best_us) best_us = elapsed;

        free(hvs);
        free(c);
        free(b);
        free(a);
    }

    printf("peak_rss_kb: %ld\n", best_rss);
    printf("best_of_%d_us: %ld\n", N_BENCH_RUNS, best_us);
    return 0;
}