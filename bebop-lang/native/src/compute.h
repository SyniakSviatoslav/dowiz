/* Bebop compute — GPU-like compute kernels lowered to NEON + multi-core.
 *
 * Implements the "no-GPU" compute model (per design):
 *   A. GPU-like programming: kernel = parallel fn over array + thread index
 *   B. NEON lowering: 4×f32 / 2×f64 / 16×u8 per 128-bit register
 *   C. Shared memory → L1/L2 + PRFM + 64B cache-line alignment
 *   D. Software workgroup dispatcher: pool.c across cores, no OS overhead
 */
#ifndef BEBOP_COMPUTE_H
#define BEBOP_COMPUTE_H

#include <stddef.h>
#include <stdint.h>

/* A compute kernel: pure elementwise fn in (const T*, size_t idx) -> T. */
typedef double (*ComputeFn)(const double *a, size_t i);

/* Elementwise map: out[i] = fn(a, i) for i in [0,n). Vectorized (NEON 2×f64)
 * inner loop + pool dispatch across cores. Returns 0. */
int compute_map(const double *a, double *out, size_t n, ComputeFn fn);

/* Fold: acc = fn(acc, a[i]). Returns the reduction. */
double compute_reduce(const double *a, size_t n, double (*fn)(double, double),
                      double init);

/* Software workgroup dispatcher: split [0,n) into W workgroups, run each on a
 * pool worker, then reduce. fn(acc, a[i]) combines. */
double compute_dispatch(const double *a, size_t n, size_t workgroups,
                        double (*fn)(double, double), double init);

int compute_self_test(char *out, size_t cap);

#endif