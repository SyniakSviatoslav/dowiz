/* Bebop stats — mean/variance/stddev, min/max, percentiles, running/online
 * statistics, overflow-safe accumulation (port of dowiz stats.rs). */
#include "stats.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static int cmp_double(const void *a, const void *b) {
    double x = *(const double *)a;
    double y = *(const double *)b;
    if (x < y) return -1;
    if (x > y) return 1;
    return 0;
}

/* Kahan compensated summation: guards the accumulator against the precision
 * loss of nearly-cancelling inputs (and, unlike a naive Σx², against the
 * magnitude overflow of large values) — the overflow-safety primitive. */
double stats_mean(const double *x, size_t n) {
    if (n == 0) return 0.0;
    double sum = 0.0;
    double c = 0.0;
    for (size_t i = 0; i < n; i++) {
        double y = x[i] - c;
        double t = sum + y;
        c = (t - sum) - y;
        sum = t;
    }
    return sum / (double)n;
}

double stats_variance(const double *x, size_t n) {
    if (n < 2) return 0.0;
    double m = stats_mean(x, n);
    /* Two-pass Σ(x−m)² with compensated summation. */
    double ss = 0.0;
    double c = 0.0;
    for (size_t i = 0; i < n; i++) {
        double d = x[i] - m;
        double y = d * d - c;
        double t = ss + y;
        c = (t - ss) - y;
        ss = t;
    }
    return ss / (double)(n - 1);
}

double stats_stddev(const double *x, size_t n) {
    return sqrt(stats_variance(x, n));
}

double stats_mean_se(const double *x, size_t n) {
    if (n < 2) return 0.0;
    return stats_stddev(x, n) / sqrt((double)n);
}

double stats_min(const double *x, size_t n) {
    if (n == 0) return 0.0;
    double m = x[0];
    for (size_t i = 1; i < n; i++) {
        if (x[i] < m) m = x[i];
    }
    return m;
}

double stats_max(const double *x, size_t n) {
    if (n == 0) return 0.0;
    double m = x[0];
    for (size_t i = 1; i < n; i++) {
        if (x[i] > m) m = x[i];
    }
    return m;
}

int stats_minmax(const double *x, size_t n, double *minv, double *maxv) {
    if (n == 0) return -1;
    double lo = x[0];
    double hi = x[0];
    for (size_t i = 1; i < n; i++) {
        if (x[i] < lo) lo = x[i];
        if (x[i] > hi) hi = x[i];
    }
    *minv = lo;
    *maxv = hi;
    return 0;
}

double stats_percentile(const double *x, size_t n, double p) {
    if (n == 0) return 0.0;
    if (p < 0.0) p = 0.0;
    if (p > 100.0) p = 100.0;

    double *sorted = malloc(n * sizeof(double));
    if (sorted == NULL) return 0.0; /* allocation failure: fail-closed */
    memcpy(sorted, x, n * sizeof(double));
    qsort(sorted, n, sizeof(double), cmp_double);

    double h = (double)(n - 1) * (p / 100.0);
    size_t lo = (size_t)h;
    size_t hi = (lo + 1 < n) ? lo + 1 : lo;
    double frac = h - (double)lo;
    double v = sorted[lo] + frac * (sorted[hi] - sorted[lo]);
    free(sorted);
    return v;
}

/* ─── running / online statistics (Welford) ──────────────────────────────── */

void stats_running_init(RunningStats *rs) {
    rs->n = 0;
    rs->mean = 0.0;
    rs->m2 = 0.0;
}

void stats_running_push(RunningStats *rs, double x) {
    rs->n++;
    double delta = x - rs->mean;
    rs->mean += delta / (double)rs->n;
    double delta2 = x - rs->mean;
    rs->m2 += delta * delta2;
}

uint64_t stats_running_count(const RunningStats *rs) {
    return rs->n;
}

double stats_running_mean(const RunningStats *rs) {
    if (rs->n == 0) return 0.0;
    return rs->mean;
}

double stats_running_variance(const RunningStats *rs) {
    if (rs->n < 2) return 0.0;
    return rs->m2 / (double)(rs->n - 1);
}

double stats_running_stddev(const RunningStats *rs) {
    return sqrt(stats_running_variance(rs));
}

int stats_sum_i64(const int64_t *x, size_t n, int64_t *out) {
    int64_t acc = 0;
    for (size_t i = 0; i < n; i++) {
        if (__builtin_add_overflow(acc, x[i], &acc)) return -1;
    }
    *out = acc;
    return 0;
}

int stats_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define A(cond, name) do { \
    int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n", (cond) ? "ok" : "FAIL", name); \
    if (r_ > 0) pos += (size_t)r_; \
    if (!(cond)) all_ok = 0; \
} while (0)

    static const double d[] = {1.0, 2.0, 3.0, 4.0, 5.0};
    double lo, hi;
    RunningStats rs;
    int64_t sum;

    A(stats_mean(d, 5) == 3.0, "mean [1..5] == 3.0");
    A(stats_variance(d, 5) == 2.5, "variance [1..5] == 2.5 (Bessel n-1)");
    A(fabs(stats_stddev(d, 5) - 1.5811388300841898) < 1e-12,
      "stddev [1..5] == sqrt(2.5)");
    A(fabs(stats_mean_se(d, 5) - 0.7071067811865476) < 1e-12,
      "mean_se [1..5] == sqrt(0.5)");

    A(stats_minmax(d, 5, &lo, &hi) == 0 && lo == 1.0 && hi == 5.0,
      "minmax [1..5] == (1, 5)");

    A(stats_percentile(d, 5, 50.0) == 3.0, "median [1..5] == 3.0");
    A(stats_percentile(d, 5, 0.0) == 1.0 &&
      stats_percentile(d, 5, 100.0) == 5.0,
      "percentile 0/100 == min/max");

    stats_running_init(&rs);
    for (size_t i = 0; i < 5; i++) stats_running_push(&rs, d[i]);
    A(stats_running_count(&rs) == 5, "running count == 5");
    A(stats_running_mean(&rs) == 3.0, "running mean == 3.0");
    A(fabs(stats_running_variance(&rs) - 2.5) < 1e-12,
      "running variance == 2.5 (Welford matches batch)");

    A(stats_sum_i64((const int64_t[]){INT64_MAX, 1}, 2, &sum) == -1,
      "i64 sum overflow rejected");
    A(stats_sum_i64((const int64_t[]){1, 2, 3}, 3, &sum) == 0 && sum == 6,
      "i64 sum 1+2+3 == 6");

    return all_ok ? 0 : -1;
}
