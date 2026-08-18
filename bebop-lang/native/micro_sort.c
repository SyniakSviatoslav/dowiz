#define _POSIX_C_SOURCE 199309L
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include "sort.h"

static double now_ns(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (double)ts.tv_sec * 1e9 + (double)ts.tv_nsec;
}

static int cmp_desc_q(const void *a, const void *b) {
    double x = *(const double *)a, y = *(const double *)b;
    return (x > y) ? -1 : (x < y) ? 1 : 0;
}

int main(void) {
    const int n = 10000;
    const int reps = 200;
    double *x = malloc((size_t)n * sizeof *x);
    double *src = malloc((size_t)n * sizeof *x);
    for (int i = 0; i < n; i++) src[i] = (double)((i * 2654435761u) & 0xfffff);

    double best_fill = 1e18, best_qsort = 1e18, best_mine = 1e18;
    for (int r = 0; r < reps; r++) {
        /* fill only */
        double t0 = now_ns();
        for (int i = 0; i < n; i++) x[i] = (double)((i * 2654435761u + (unsigned)r) & 0xfffff);
        double t1 = now_ns();
        if (t1 - t0 < best_fill) best_fill = t1 - t0;

        /* qsort */
        memcpy(x, src, (size_t)n * sizeof *x);
        t0 = now_ns();
        qsort(x, n, sizeof(double), cmp_desc_q);
        t1 = now_ns();
        if (t1 - t0 < best_qsort) best_qsort = t1 - t0;

        /* mine */
        memcpy(x, src, (size_t)n * sizeof *x);
        t0 = now_ns();
        sort_f64_desc(x, n);
        t1 = now_ns();
        if (t1 - t0 < best_mine) best_mine = t1 - t0;
    }
    printf("n=%d reps=%d\n", n, reps);
    printf("fill   : %8.1f ns\n", best_fill);
    printf("qsort  : %8.1f ns  (baseline)\n", best_qsort);
    printf("mine   : %8.1f ns  (%.2fx vs qsort)\n", best_mine, best_qsort / best_mine);

    int ok = 1;
    for (int i = 1; i < n; i++) if (x[i - 1] < x[i]) ok = 0;
    printf("sorted desc: %s\n", ok ? "OK" : "FAIL");
    free(x); free(src);
    return 0;
}
