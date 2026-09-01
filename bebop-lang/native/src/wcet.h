/* Bebop WCET / timing-jitter / determinism harness (checklist §6).
 *
 * For a fixed set of kernels, measures over R runs: min, mean, median, p95,
 * p99, max (= the observed worst-case execution time on this host), stddev
 * (timing jitter), the worst/mean ratio (the real-time safety margin), and
 * bit-determinism (a kernel's checksum must be identical across two
 * independent runs).
 *
 * Honest scope: this is an EMPIRICAL WCET (measured upper bound). It is NOT a
 * static WCET proof — that is a separate, much harder problem (aiT/OTAWA-style
 * path analysis + microarchitecture modelling) and is out of scope here.
 */
#ifndef BEBOP_WCET_H
#define BEBOP_WCET_H
#include <stddef.h>

int wcet_run(void);
int wcet_self_test(char *out, size_t cap);
#endif
