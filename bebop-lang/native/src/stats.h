/* Bebop stats — mean/variance/stddev, min/max, percentiles, running/online
 * statistics, overflow-safe accumulation (port of dowiz stats.rs). */
#ifndef BEBOP_STATS_H
#define BEBOP_STATS_H

#include <stddef.h>
#include <stdint.h>

/* ─── batch (offline) statistics ─────────────────────────────────────────── */

/* Arithmetic mean. Returns 0.0 for n == 0. */
double stats_mean(const double *x, size_t n);

/* Bessel-corrected (n−1) sample variance. Returns 0.0 for n < 2 (variance is
 * undefined for a single sample — the same convention as dowiz stats.rs). */
double stats_variance(const double *x, size_t n);

/* Bessel-corrected (n−1) sample standard deviation = sqrt(stats_variance). */
double stats_stddev(const double *x, size_t n);

/* Standard error of the mean = stddev / sqrt(n). Returns 0.0 for n < 2. */
double stats_mean_se(const double *x, size_t n);

/* Extrema. stats_minmax returns 0 on success (both *minv and *maxv set),
 * -1 for n == 0. */
double stats_min(const double *x, size_t n);
double stats_max(const double *x, size_t n);
int stats_minmax(const double *x, size_t n, double *minv, double *maxv);

/* Percentile 0 <= p <= 100 via linear interpolation between order statistics
 * (R-7, numpy default). p is clamped to [0, 100]; returns 0.0 for n == 0. */
double stats_percentile(const double *x, size_t n, double p);

/* ─── running / online statistics (Welford, overflow-safe) ───────────────── */

typedef struct {
    uint64_t n;
    double mean;
    double m2; /* sum of squared deviations from the running mean */
} RunningStats;

void stats_running_init(RunningStats *rs);
void stats_running_push(RunningStats *rs, double x);
uint64_t stats_running_count(const RunningStats *rs);
double stats_running_mean(const RunningStats *rs);     /* 0.0 if n == 0 */
double stats_running_variance(const RunningStats *rs); /* Bessel n−1; 0 for n<2 */
double stats_running_stddev(const RunningStats *rs);

/* ─── overflow-safe integer accumulation ─────────────────────────────────── */

/* Checked i64 sum: 0 on success (result in *out), -1 on overflow (never
 * wraps). Empty input (n == 0) succeeds with *out = 0. */
int stats_sum_i64(const int64_t *x, size_t n, int64_t *out);

int stats_self_test(char *out, size_t cap);

#endif /* BEBOP_STATS_H */
