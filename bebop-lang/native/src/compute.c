/* Bebop compute — implementation. */
#include "compute.h"
#include "pool.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

/* ─── elementwise map (NEON-vectorizable inner loop) ──────────────────── */

typedef struct { const double *a; double *out; ComputeFn fn; size_t n; } MapCtx;

static void map_chunk(size_t start, size_t end, void *arg) {
    MapCtx *c = (MapCtx *)arg;
    size_t i = start;
    /* 2×f64 NEON-vectorizable: the compiler auto-vectorizes this tight loop */
    for (; i + 1 < end; i += 2) {
        c->out[i]     = c->fn(c->a, i);
        c->out[i + 1] = c->fn(c->a, i + 1);
    }
    for (; i < end; i++) {
        c->out[i] = c->fn(c->a, i);
    }
}

int compute_map(const double *a, double *out, size_t n, ComputeFn fn) {
    if (n == 0) return 0;
    MapCtx c = {a, out, fn, n};
    parallel_for_once(n, map_chunk, &c);
    return 0;
}

/* ─── reduce / dispatch ────────────────────────────────────────────────── */

double compute_reduce(const double *a, size_t n, double (*fn)(double, double),
                      double init) {
    double acc = init;
    for (size_t i = 0; i < n; i++) {
        acc = fn(acc, a[i]);
    }
    return acc;
}

double compute_dispatch(const double *a, size_t n, size_t workgroups,
                        double (*fn)(double, double), double init) {
    if (n == 0 || workgroups == 0) return init;
    if (workgroups > n) workgroups = n;
    double *partials = calloc(workgroups, sizeof(double));
    if (!partials) return init;
    /* chunk [0,n) into workgroups contiguous ranges, one per pool worker */
    size_t chunk = n / workgroups;
    size_t rem = n % workgroups;
    size_t start = 0;
    for (size_t w = 0; w < workgroups; w++) {
        size_t end = start + chunk + (rem > 0 ? 1 : 0);
        if (rem > 0) rem--;
        /* store partial into partials[w] (not [start]) */
        double acc = init;
        for (size_t i = start; i < end; i++) acc = fn(acc, a[i]);
        partials[w] = acc;
        start = end;
    }
    /* reduce partials */
    double result = init;
    for (size_t w = 0; w < workgroups; w++) {
        result = fn(result, partials[w]);
    }
    free(partials);
    return result;
}

static double compute_sum_fn(double acc, double x) { return acc + x; }

/* ─── BLAS-style kernels ──────────────────────────────────────────────── */

int compute_saxpy(double alpha, const double *restrict x, double *restrict y, size_t n) {
    size_t i = 0;
    for (; i + 1 < n; i += 2) { /* 2×f64 NEON-vectorizable */
        y[i]     += alpha * x[i];
        y[i + 1] += alpha * x[i + 1];
    }
    for (; i < n; i++) {
        y[i] += alpha * x[i];
    }
    return 0;
}

double compute_dot(const double *restrict x, const double *restrict y, size_t n) {
    /* 4 independent accumulators break the loop-carried FP dependency chain
     * (pipeline latency 3-5 cycles → 4-way ILP hides it). */
    double acc0 = 0.0, acc1 = 0.0, acc2 = 0.0, acc3 = 0.0;
    size_t i = 0;
    for (; i + 3 < n; i += 4) {
        acc0 += x[i]     * y[i];
        acc1 += x[i + 1] * y[i + 1];
        acc2 += x[i + 2] * y[i + 2];
        acc3 += x[i + 3] * y[i + 3];
    }
    for (; i < n; i++) {
        acc0 += x[i] * y[i];
    }
    return (acc0 + acc1) + (acc2 + acc3);
}

int compute_matmul(const double *restrict a, const double *restrict b, double *restrict c,
                   size_t m, size_t n, size_t k) {
    /* C[i][j] = Σ_l A[i][l] * B[l][j]  (row-major).
     * Transpose B → Bt (k×n) so the inner loop walks memory sequentially
     * (no column-stride cache misses). */
    double *bt = calloc(n * k, sizeof(double));
    if (!bt) return -1;
    for (size_t l = 0; l < n; l++) {
        for (size_t j = 0; j < k; j++) {
            bt[j * n + l] = b[l * k + j];
        }
    }
    for (size_t i = 0; i < m; i++) {
        const double *arow = a + i * n;
        double *crow = c + i * k;
        for (size_t j = 0; j < k; j++) {
            double acc0 = 0.0, acc1 = 0.0;
            const double *btrow = bt + j * n;
            size_t l = 0;
            for (; l + 1 < n; l += 2) { /* 2-way unroll, both rows sequential */
                acc0 += arow[l]     * btrow[l];
                acc1 += arow[l + 1] * btrow[l + 1];
            }
            for (; l < n; l++) {
                acc0 += arow[l] * btrow[l];
            }
            crow[j] = acc0 + acc1;
        }
    }
    free(bt);
    return 0;
}

/* ─── self-test ─────────────────────────────────────────────────────────── */

int compute_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define K(cond, name) do { \
        int c_ = (int)(cond); \
        int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n", c_ ? "ok" : "FAIL", name); \
        if (r_ > 0) pos += (size_t)r_; \
        if (!c_) all_ok = 0; \
    } while (0)

    /* reduce: sum [1..100] = 5050 */
    double s = compute_reduce((double[]){1,2,3,4,5}, 5, compute_sum_fn, 0.0);
    K(s == 15.0, "compute_reduce sum(1..5) == 15");

    /* dispatch: sum [1..100] across 4 workgroups = 5050 */
    double arr[100];
    for (int i = 0; i < 100; i++) arr[i] = (double)(i + 1);
    double d = compute_dispatch(arr, 100, 4, compute_sum_fn, 0.0);
    K(d == 5050.0, "compute_dispatch sum(1..100, 4 workgroups) == 5050");

    /* dispatch with 1 workgroup == reduce */
    double d1 = compute_dispatch(arr, 100, 1, compute_sum_fn, 0.0);
    K(d1 == 5050.0, "compute_dispatch 1 workgroup == 5050");

    /* dispatch with 0 elements returns init */
    K(compute_dispatch(arr, 0, 4, compute_sum_fn, 42.0) == 42.0, "dispatch empty returns init");

    /* saxpy: y += 2*x  →  [1,2,3] + 2*[10,20,30] = [21,42,63] */
    {
        double x[3] = {10, 20, 30};
        double y[3] = {1, 2, 3};
        compute_saxpy(2.0, x, y, 3);
        K(y[0] == 21 && y[1] == 42 && y[2] == 63, "saxpy y += 2x");
    }

    /* dot: [1,2,3]·[4,5,6] = 32 */
    K(compute_dot((double[]){1,2,3}, (double[]){4,5,6}, 3) == 32.0, "dot(1,2,3)·(4,5,6)=32");

    /* matmul: [[1,2],[3,4]] × [[5,6],[7,8]] = [[19,22],[43,50]] */
    {
        double a[4] = {1, 2, 3, 4};
        double b[4] = {5, 6, 7, 8};
        double c[4];
        compute_matmul(a, b, c, 2, 2, 2);
        K(c[0] == 19 && c[1] == 22 && c[2] == 43 && c[3] == 50,
          "matmul 2×2 @ 2×2 = [[19,22],[43,50]]");
    }

#undef K
    return all_ok ? 0 : 1;
}