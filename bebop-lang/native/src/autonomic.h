/* Bebop autonomic — bounded rate + gain-scheduling control law
 * (port of dowiz autonomic.rs).
 *
 * PURE C11, ZERO dependencies. BoundedRate is a newtype clamped to [0,100].
 */
#ifndef BEBOP_AUTONOMIC_H
#define BEBOP_AUTONOMIC_H

#include <stddef.h>
#include <stdbool.h>

/* BoundedRate ∈ [0, 100]. The raw value is private — only constructors
 * (clamp or reject) can produce an instance. */
typedef struct { double value; } BoundedRate;

BoundedRate bounded_rate_new(double v);   /* clamp */
bool bounded_rate_try(double v, BoundedRate *out); /* reject */

/* Single adjustment from the gain-scheduling table. */
typedef struct {
    int   direction; /* -1 (degrade), 0 (hold), +1 (boost) */
    BoundedRate rate;
} Adjustment;

/* Markov attractor verdict (port of markov::Verdict). */
typedef enum {
    MKV_HEALTHY = 0,
    MKV_DEGRADING,
    MKV_UNSTABLE
} MarkovVerdict;

/* Spectral drift class (port of spectral::DriftClass). */
typedef enum {
    DC_DAMPED = 0,
    DC_RESONANT,
    DC_UNSTABLE,
    DC_UNKNOWN
} DriftClass;

/* The pure control-law function: verdict + drift → adjustment. */
Adjustment autonomic_schedule(MarkovVerdict v, DriftClass d);

int autonomic_self_test(char *out, size_t cap);

#endif
