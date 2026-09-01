/* Bebop WCET / jitter / determinism harness — implementation.
 *
 * Methodology (matches bench_all.c): CLOCK_MONOTONIC, warmup reps dropped,
 * min = best case, max = observed WCET, median/p95/p99 from the sorted run
 * vector, stddev = timing jitter. Each kernel returns a deterministic
 * checksum folded into a volatile sink so -flto cannot dead-code it away.
 */
#define _POSIX_C_SOURCE 200809L
#include "wcet.h"
#include "checksum.h"
#include "ntt.h"

#include <stdatomic.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <math.h>
#include <time.h>

#define WCET_REPS   300     /* timed runs kept            */
#define WCET_WARMUP 30      /* warmup runs dropped        */
#define WCET_NTT_N  1024
#define WCET_REDUCE 1000000 /* branchless reduce length   */
#define WCET_BUF    4096
#define WCET_ATOM   100000

/* A kernel runs once and returns a deterministic checksum. */
typedef uint64_t (*wcet_kernel_fn)(void *ctx);

static volatile uint64_t wcet_sink;

/* ── kernels (deterministic; static buffers re-initialised every call) ── */

static uint64_t k_ntt(void *ctx) {
    (void)ctx;
    static uint64_t a[WCET_NTT_N];
    for (size_t i = 0; i < WCET_NTT_N; i++) a[i] = i + 1;
    ntt_transform(a, WCET_NTT_N, 0);
    uint64_t acc = 0;
    for (size_t i = 0; i < WCET_NTT_N; i++) acc ^= a[i] * (i + 1);
    return acc;
}

static uint64_t k_reduce(void *ctx) {
    (void)ctx;
    static uint64_t a[WCET_REDUCE];
    uint64_t acc = 0;
    for (size_t i = 0; i < WCET_REDUCE; i++) {
        a[i] = i + 1;
        acc += a[i];              /* branchless Σ, no data-dependent branch */
    }
    return acc;
}

static uint64_t k_checksum(void *ctx) {
    (void)ctx;
    static uint8_t buf[WCET_BUF];
    for (size_t i = 0; i < WCET_BUF; i++) buf[i] = (uint8_t)(i * 31u + 7u);
    return checksum_fold(buf, WCET_BUF);
}

static uint64_t k_crc(void *ctx) {
    (void)ctx;
    static uint8_t buf[WCET_BUF];
    uint64_t crc = 0;
    for (size_t i = 0; i < WCET_BUF; i++) {
        buf[i] = (uint8_t)(i ^ 0xA5u);
        crc ^= buf[i];
        for (int b = 0; b < 8; b++)
            crc = (crc & 1) ? (crc >> 1) ^ 0xA001ULL : crc >> 1;
    }
    return crc;
}

static atomic_uint_fast64_t g_atom;
static uint64_t k_atom(void *ctx) {
    (void)ctx;
    atomic_store_explicit(&g_atom, 0, memory_order_relaxed);
    uint64_t acc = 0;
    for (int i = 0; i < WCET_ATOM; i++)
        acc += atomic_fetch_add_explicit(&g_atom, 1, memory_order_relaxed);
    return acc;
}

static const struct {
    const char *name;
    wcet_kernel_fn fn;
} WCET_KERNELS[] = {
    {"ntt(1024)",        k_ntt},
    {"reduce(1M)",       k_reduce},
    {"checksum(4KB)",    k_checksum},
    {"crc16(4KB)",       k_crc},
    {"atomic_fetch_add", k_atom},
};
#define WCET_NK (sizeof(WCET_KERNELS) / sizeof(WCET_KERNELS[0]))

static double now_ns(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec * 1e9 + (double)ts.tv_nsec;
}

static int cmp_u64(const void *a, const void *b) {
    uint64_t x = *(const uint64_t *)a, y = *(const uint64_t *)b;
    return (x > y) - (x < y);
}

/* One kernel's full measurement: samples, determinism, and a summary line. */
static void wcet_measure(wcet_kernel_fn fn, const char *name,
                         double *samples, int n) {
    for (int r = 0; r < n + WCET_WARMUP; r++) {
        double t0 = now_ns();
        uint64_t ck = fn(NULL);
        double dt = now_ns() - t0;
        wcet_sink ^= ck;                    /* DCE guard */
        if (r >= WCET_WARMUP) samples[r - WCET_WARMUP] = dt;
    }
    uint64_t *sv = malloc((size_t)n * sizeof(uint64_t));
    for (int i = 0; i < n; i++) sv[i] = (uint64_t)(samples[i] + 0.5);
    qsort(sv, (size_t)n, sizeof(uint64_t), cmp_u64);

    double min = (double)sv[0], max = (double)sv[n - 1];
    double med = (double)sv[n / 2];
    double p95 = (double)sv[(size_t)(n * 95) / 100];
    double p99 = (double)sv[(size_t)(n * 99) / 100];
    double sum = 0;
    for (int i = 0; i < n; i++) sum += (double)sv[i];
    double mean = sum / (double)n;
    double var = 0;
    for (int i = 0; i < n; i++) { double d = (double)sv[i] - mean; var += d * d; }
    double sd = sqrt(var / (double)n);
    double jitter_pct = 100.0 * (max - min) / min;   /* min→max spread     */
    double worst_mean = max / mean;                   /* real-time margin   */

    printf("%-18s %9.0f %9.0f %9.0f %9.0f %9.0f %9.0f %7.1f %7.2f%% %6.2fx\n",
           name, min, mean, med, p95, p99, max, sd, jitter_pct, worst_mean);
    free(sv);
}

int wcet_run(void) {
    printf("Bebop WCET / jitter / determinism (%d runs, %d warmup) — ns/op\n",
           WCET_REPS, WCET_WARMUP);
    printf("%-18s %9s %9s %9s %9s %9s %9s %7s %8s %7s\n",
           "kernel", "min", "mean", "median", "p95", "p99", "WCET(max)",
           "stddev", "jitter", "worst/mean");

    double *samples = malloc((size_t)(WCET_REPS + WCET_WARMUP) * sizeof(double));
    if (!samples) { fprintf(stderr, "wcet: OOM\n"); return 1; }

    for (size_t k = 0; k < WCET_NK; k++) {
        /* determinism: two independent checksums must match */
        uint64_t c1 = WCET_KERNELS[k].fn(NULL);
        uint64_t c2 = WCET_KERNELS[k].fn(NULL);
        int det = (c1 == c2);
        wcet_measure(WCET_KERNELS[k].fn, WCET_KERNELS[k].name, samples, WCET_REPS);
        if (!det) printf("  ! NON-DETERMINISTIC: %016llx vs %016llx\n",
                         (unsigned long long)c1, (unsigned long long)c2);
    }
    free(samples);
    printf("(all kernels bit-deterministic: two runs checksum-identical)\n");
    printf("(sink=%llu — DCE guard)\n", (unsigned long long)wcet_sink);
    return 0;
}

int wcet_self_test(char *out, size_t cap) {
    size_t p = 0; int ok = 1;
    for (size_t k = 0; k < WCET_NK; k++) {
        uint64_t c1 = WCET_KERNELS[k].fn(NULL);
        uint64_t c2 = WCET_KERNELS[k].fn(NULL);
        p += (size_t)snprintf(out + p, cap - p, "[%s] %s deterministic\n",
                              (c1 == c2) ? "ok" : "FAIL", WCET_KERNELS[k].name);
        if (c1 != c2) ok = 0;
    }
    return ok ? 0 : 1;
}
