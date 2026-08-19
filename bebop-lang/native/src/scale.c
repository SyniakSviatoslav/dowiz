/* Bebop scalability benchmark — 1/2/4/8-core throughput on parallel NTT.
 *
 * One fixed workload: K independent radix-2 NTTs of size N, split across
 * 1, 2, 4 and 8 workers via the pool. Reports wall-clock (min-of-N), raw
 * throughput, speedup vs the single-thread baseline, and parallel efficiency.
 *
 * Honest methodology (matches bench_all.c):
 *   - CLOCK_MONOTONIC, warmup, min-of-REPS (best case, not mean)
 *   - each row folded into an atomic sink (no DCE under -flto)
 *   - the 1-core run uses the pool's single-thread fast path (no spawn cost)
 *   - correctness gate: parallel checksum == serial checksum before timing
 */
#define _POSIX_C_SOURCE 200809L

#include "scale.h"

#include "ntt.h"
#include "pool.h"

#include <stdatomic.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>

#define SCALE_NTT_N   1024   /* per-transform size (power of two) */
#define SCALE_K       1024   /* independent transforms             */
#define SCALE_REPS    8      /* timed reps (best kept)             */
#define SCALE_WARMUP  2

static atomic_uint_fast64_t scale_sink;

typedef struct {
    uint64_t *buf;   /* K*N elements, one transform per N-run */
    size_t    n;     /* NTT size                              */
} ScaleWork;

static double now_ms(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec * 1e3 + (double)ts.tv_nsec / 1e6;
}

/* Forward-NTT every transform in [start, end); fold a full-result checksum. */
static void scale_work(size_t start, size_t end, void *arg) {
    ScaleWork *w = (ScaleWork *)arg;
    size_t n = w->n;
    uint64_t acc = 0;
    for (size_t i = start; i < end; i++) {
        uint64_t *row = &w->buf[i * n];
        ntt_transform(row, n, 0);
        acc ^= row[0] ^ row[n / 2] ^ row[n - 1] ^ row[(n * 3) / 4];
    }
    atomic_fetch_xor_explicit(&scale_sink, acc, memory_order_relaxed);
}

static void scale_init(ScaleWork *w) {
    for (size_t i = 0; i < SCALE_K * w->n; i++) {
        w->buf[i] = (uint64_t)(i + 1);
    }
}

/* Serial checksum of one forward pass from a pristine buffer (correctness
 * reference; XOR is order-independent so it equals the parallel checksum). */
static uint64_t scale_checksum(ScaleWork *w) {
    uint64_t acc = 0;
    for (size_t i = 0; i < SCALE_K; i++) {
        uint64_t *row = &w->buf[i * w->n];
        ntt_transform(row, w->n, 0);
        acc ^= row[0] ^ row[w->n / 2] ^ row[w->n - 1] ^ row[(w->n * 3) / 4];
    }
    return acc;
}

/* Best-of (REPS) wall time for one full pass.  nthreads<=1 → serial path. */
static double scale_time(Pool *p, ScaleWork *w, int nthreads) {
    double best = 0.0;
    for (int r = 0; r < SCALE_WARMUP + SCALE_REPS; r++) {
        double t0 = now_ms();
        if (nthreads <= 1) {
            scale_work(0, SCALE_K, w);
        } else {
            pool_parallel_for(p, (size_t)SCALE_K, scale_work, w);
        }
        double dt = now_ms() - t0;
        if (r >= SCALE_WARMUP && (best == 0.0 || dt < best)) best = dt;
    }
    return best;
}

int scale_run(void) {
    static const int cores[] = {1, 2, 4, 8};
    double t[4];

    ScaleWork w;
    w.n = SCALE_NTT_N;
    w.buf = malloc(SCALE_K * w.n * sizeof(uint64_t));
    if (!w.buf) {
        fprintf(stderr, "scale: OOM (%zu elements)\n", SCALE_K * w.n);
        return 1;
    }

    /* Correctness gate: parallel split must reproduce the serial checksum. */
    scale_init(&w);
    uint64_t c_serial = scale_checksum(&w);

    scale_init(&w);
    atomic_store(&scale_sink, 0);
    Pool *pc = pool_new(4);
    pool_parallel_for(pc, (size_t)SCALE_K, scale_work, &w);
    uint64_t c_par = atomic_load(&scale_sink);
    pool_free(pc);

    printf("Bebop scalability — parallel NTT (n=%d, %d transforms, %zu elems)\n",
           SCALE_NTT_N, SCALE_K, SCALE_K * w.n);
    printf("checksum: serial=%016llx parallel=%016llx  %s\n",
           (unsigned long long)c_serial, (unsigned long long)c_par,
           c_serial == c_par ? "OK" : "MISMATCH");

    atomic_store(&scale_sink, 0);
    for (int c = 0; c < 4; c++) {
        int nc = cores[c];
        Pool *p = (nc > 1) ? pool_new(nc) : NULL;
        t[c] = scale_time(p, &w, nc);
        if (p) pool_free(p);
    }

    printf("\n%5s  %9s  %12s  %8s  %10s\n",
           "cores", "time(ms)", "Melem/s", "speedup", "efficiency");
    for (int c = 0; c < 4; c++) {
        int nc = cores[c];
        double tp = (double)(SCALE_K * w.n) / (1000.0 * t[c]); /* Melem/s */
        double sp = t[0] / t[c];
        double eff = 100.0 * sp / (double)nc;
        printf("%5d  %9.3f  %12.1f  %7.3fx  %9.1f%%\n",
               nc, t[c], tp, sp, eff);
    }

    free(w.buf);
    return c_serial == c_par ? 0 : 1;
}
