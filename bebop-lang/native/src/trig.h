/* Bebop trig — continuous phase encoding on the unit circle S¹ (port of dowiz
 * trig.rs). A scalar becomes a (cos θ, sin θ) pair; sin/cos/atan2 are
 * hand-rolled (Cody–Waite range reduction + Taylor, ~1 ULP, deterministic),
 * angle normalization maps θ to (−π, π], and one/zero/uncertain are exact
 * identities (no rounding). */
#ifndef BEBOP_TRIG_H
#define BEBOP_TRIG_H

#include <stddef.h>
#include <stdint.h>

#define BEBOP_PI 3.14159265358979323846
#define BEBOP_TAU 6.28318530717958647692
#define BEBOP_FRAC_PI_2 1.57079632679489661923

/* A phase angle + its (cos, sin) encoding — a point on S¹. */
typedef struct {
    double theta; /* angle in radians, normalized to (-π, π] */
    double cos;   /* x-coordinate on the unit circle */
    double sin;   /* y-coordinate on the unit circle */
} Phase;

/* ─── trig primitives (deterministic, ~1 ULP) ───────────────────────────── */

double trig_sin(double x);
double trig_cos(double x);
double trig_atan2(double y, double x);
double trig_atan(double x);

/* ─── angle normalization ────────────────────────────────────────────────── */

/* Map θ to (-π, π]: θ % TAU, then fold into the principal range. */
double trig_normalize_angle(double theta);

/* ─── Phase constructors ─────────────────────────────────────────────────── */

/* From angle θ (normalized to (-π, π]). */
Phase trig_phase_new(double theta);
/* From (x, y) coordinates; normalizes to the unit circle. Degenerate
 * (r < 1e-15) inputs fail-closed to zero(). */
Phase trig_phase_from_xy(double x, double y);

/* Exact identities (no rounding). */
Phase trig_phase_one(void);       /* θ = 0     → ( 1, 0) — "True"  */
Phase trig_phase_zero(void);      /* θ = π     → (-1, 0) — "False" */
Phase trig_phase_uncertain(void); /* θ = π/2   → ( 0, 1) — orthogonal */

/* ─── Phase ops ──────────────────────────────────────────────────────────── */

double trig_phase_delta(const Phase *a, const Phase *b);    /* signed, (-π, π] */
double trig_phase_distance(const Phase *a, const Phase *b); /* unsigned, [0, π] */
Phase trig_phase_lerp(const Phase *a, const Phase *b, double w); /* slerp, w∈[0,1] */
double trig_phase_scalar(const Phase *p); /* cos projection onto [-1, 1] */
double trig_phase_sign(const Phase *p);   /* -1 / 0 / +1 along the cos axis */

int trig_self_test(char *out, size_t cap);

#endif /* BEBOP_TRIG_H */
