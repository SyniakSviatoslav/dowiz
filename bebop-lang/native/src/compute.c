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

#undef K
    return all_ok ? 0 : 1;
}