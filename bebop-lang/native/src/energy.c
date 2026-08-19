/* Bebop energy-per-operation — implementation (software cycle-energy model). */
#define _POSIX_C_SOURCE 200809L
#include "energy.h"
#include "checksum.h"
#include "ntt.h"

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

/* Nominal energy per active cycle (nJ). Model constant, clearly documented:
 * for a mid-range AArch64 core P ≈ 2 W @ 2.5 GHz → ~0.8 nJ/cycle. */
#define ENERGY_NJ_PER_CYCLE 1.0

#define ENERGY_REPS   20
#define ENERGY_WARMUP 3
#define ENERGY_NTT_N  1024
#define ENERGY_MEM    1000000

/* Read the current CPU frequency in Hz (best-effort; nominal fallback). */
static double read_cpu_freq_hz(void) {
    long khz = 0;
    FILE *f = fopen("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_cur_freq", "r");
    if (f) {
        if (fscanf(f, "%ld", &khz) != 1) khz = 0;
        fclose(f);
    }
    if (khz <= 0) {
        /* fallback: cpuinfo_max_freq is ALSO in kHz (all cpufreq sysfs *_freq
         * files are kHz); do NOT multiply. */
        f = fopen("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq", "r");
        if (f) {
            if (fscanf(f, "%ld", &khz) != 1) khz = 0;
            fclose(f);
        }
    }
    if (khz <= 0) khz = 2000000L;   /* nominal 2 GHz fallback */
    return (double)khz * 1000.0;
}

/* A kernel: run once, return a deterministic checksum. */
typedef uint64_t (*energy_kernel_fn)(void);

static uint64_t e_ntt(void) {
    static uint64_t a[ENERGY_NTT_N];
    for (size_t i = 0; i < ENERGY_NTT_N; i++) a[i] = i + 1;
    ntt_transform(a, ENERGY_NTT_N, 0);
    uint64_t acc = 0;
    for (size_t i = 0; i < ENERGY_NTT_N; i++) acc ^= a[i];
    return acc;
}

static uint64_t e_reduce(void) {
    static uint64_t a[ENERGY_MEM];
    uint64_t acc = 0;
    for (size_t i = 0; i < ENERGY_MEM; i++) { a[i] = i + 1; acc += a[i]; }
    return acc;
}

static uint64_t e_checksum(void) {
    static uint8_t b[ENERGY_MEM];
    for (size_t i = 0; i < ENERGY_MEM; i++) b[i] = (uint8_t)(i * 31u + 7u);
    return checksum_fold(b, ENERGY_MEM);
}

static uint64_t e_memcpy(void) {
    static uint8_t src[ENERGY_MEM], dst[ENERGY_MEM];
    for (size_t i = 0; i < ENERGY_MEM; i++) src[i] = (uint8_t)(i & 0xff);
    memcpy(dst, src, ENERGY_MEM);
    uint64_t acc = 0;
    for (size_t i = 0; i < ENERGY_MEM; i += 997) acc += dst[i];
    return acc;
}

static const struct {
    const char *name;
    energy_kernel_fn fn;
    double ops_per_call;   /* elementary ops / call (elements or butterfly ops) */
} ENERGY_KERNELS[] = {
    {"ntt(1024)",       e_ntt,      1024.0 * 10.0},  /* ~n·log2(n) butterflies */
    {"reduce(1M)",      e_reduce,   1000000.0},
    {"checksum(1MB)",   e_checksum, 1000000.0},
    {"memcpy(1MB)",     e_memcpy,   1000000.0},
};
#define ENERGY_NK (sizeof(ENERGY_KERNELS) / sizeof(ENERGY_KERNELS[0]))

static volatile uint64_t energy_sink;

static double now_ns(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec * 1e9 + (double)ts.tv_nsec;
}

int energy_run(void) {
    double freq = read_cpu_freq_hz();
    double nj_per_cycle = ENERGY_NJ_PER_CYCLE;
    printf("Bebop energy-per-op (software model) — freq=%.3f GHz, %g nJ/cycle\n",
           freq / 1e9, nj_per_cycle);
    printf("%-16s %12s %12s %12s %14s\n",
           "kernel", "ns/call", "nJ/call", "J/Mop", "Mop/s");

    for (size_t k = 0; k < ENERGY_NK; k++) {
        double best = 0.0;
        for (int r = 0; r < ENERGY_REPS + ENERGY_WARMUP; r++) {
            double t0 = now_ns();
            uint64_t ck = ENERGY_KERNELS[k].fn();
            double dt = now_ns() - t0;
            energy_sink ^= ck;
            if (r >= ENERGY_WARMUP && (best == 0.0 || dt < best)) best = dt;
        }
        double cycles = best * (freq / 1e9);      /* ns × GHz = cycles      */
        double nj = cycles * nj_per_cycle;         /* energy per call (nJ)   */
        double ops = ENERGY_KERNELS[k].ops_per_call;
        double j_per_op = nj * 1e-9 / ops;         /* joules per op          */
        double j_per_mop = j_per_op * 1e6;         /* joules per 1e6 ops     */
        double mops = ops / (best * 1e-3);         /* Mega-ops/sec           */
        printf("%-16s %12.1f %12.2f %12.4f %14.1f\n",
               ENERGY_KERNELS[k].name, best, nj, j_per_mop, mops);
    }
    printf("(sink=%llu; software model — swap in INA219/pt_sample_adc on bare-metal)\n",
           (unsigned long long)energy_sink);
    return 0;
}

int energy_self_test(char *out, size_t cap) {
    size_t p = 0; int ok = 1;
    double f = read_cpu_freq_hz();
    p += (size_t)snprintf(out + p, cap - p, "[%s] cpu freq = %.3f GHz (>0)\n",
                          (f > 0) ? "ok" : "FAIL", f / 1e9);
    if (f <= 0) ok = 0;
    for (size_t k = 0; k < ENERGY_NK; k++) {
        uint64_t c1 = ENERGY_KERNELS[k].fn();
        uint64_t c2 = ENERGY_KERNELS[k].fn();
        p += (size_t)snprintf(out + p, cap - p, "[%s] %s deterministic\n",
                              (c1 == c2) ? "ok" : "FAIL", ENERGY_KERNELS[k].name);
        if (c1 != c2) ok = 0;
    }
    return ok ? 0 : 1;
}
