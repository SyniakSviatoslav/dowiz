/* Bebop trig — implementation (port of dowiz trig.rs). Deterministic
 * hand-rolled sin/cos/atan2 (~1 ULP: Cody–Waite range reduction + Taylor
 * series, angle-halving for atan), angle normalization to (-π, π], and exact
 * phase identities. Only exact IEEE helpers (sqrt/round/fabs/fmod) come from
 * <math.h>; they match the dowiz hand-rolled equivalents bit-for-bit. */
#include "trig.h"

#include <math.h>
#include <stdio.h>

/* π/2 split into high + low parts (Cody–Waite) for accurate range reduction. */
static const double TRIG_FRAC_2_PI = 0.63661977236758134308; /* 2/π */
static const double TRIG_PIO2_HI = 1.5707963267948966;
static const double TRIG_PIO2_LO = 6.123233995736766e-17;

/* Saturating f64 → i32 (mirrors Rust `round(...) as i32`; no UB on overflow,
 * NaN → 0). Large inputs would lose precision under Cody–Waite reduction
 * anyway, so saturation is the fail-closed behaviour. */
static int32_t trig_sat_i32(double x) {
    if (x >= 2147483647.0) return INT32_MAX;
    if (x <= -2147483648.0) return INT32_MIN;
    if (x != x) return 0; /* NaN */
    return (int32_t)x;    /* truncate toward zero */
}

/* ─── sin / cos ──────────────────────────────────────────────────────────── */

/* Reduce x to (n, r) with x = n·(π/2) + r, r ∈ [-π/4, π/4]. */
static int32_t trig_rem_pio2(double x, double *r) {
    int32_t n = trig_sat_i32(round(x * TRIG_FRAC_2_PI));
    *r = (x - (double)n * TRIG_PIO2_HI) - (double)n * TRIG_PIO2_LO;
    return n;
}

/* sin(r) on |r| ≤ π/4 (Taylor; terms through r¹⁷/17!). */
static double trig_sin_poly(double r) {
    double r2 = r * r;
    double term = r;
    double sum = r;
    for (int k = 1; k <= 8; k++) {
        term *= -r2 / ((double)(2 * k) * (double)(2 * k + 1));
        sum += term;
    }
    return sum;
}

/* cos(r) on |r| ≤ π/4. */
static double trig_cos_poly(double r) {
    double r2 = r * r;
    double term = 1.0;
    double sum = 1.0;
    for (int k = 1; k <= 8; k++) {
        term *= -r2 / ((double)(2 * k - 1) * (double)(2 * k));
        sum += term;
    }
    return sum;
}

double trig_sin(double x) {
    if (!isfinite(x)) return NAN;
    double r;
    int32_t n = trig_rem_pio2(x, &r);
    switch (((n % 4) + 4) % 4) { /* euclidean mod 4, always non-negative */
        case 0: return trig_sin_poly(r);
        case 1: return trig_cos_poly(r);
        case 2: return -trig_sin_poly(r);
        default: return -trig_cos_poly(r);
    }
}

double trig_cos(double x) {
    if (!isfinite(x)) return NAN;
    double r;
    int32_t n = trig_rem_pio2(x, &r);
    switch (((n % 4) + 4) % 4) {
        case 0: return trig_cos_poly(r);
        case 1: return -trig_sin_poly(r);
        case 2: return -trig_cos_poly(r);
        default: return trig_sin_poly(r);
    }
}

/* ─── atan / atan2 ───────────────────────────────────────────────────────── */

/* atan(x) for x ≥ 0, via angle-halving reduction + Taylor. */
static double trig_atan_pos(double x) {
    if (x > 1.0) {
        return BEBOP_FRAC_PI_2 - trig_atan_pos(1.0 / x);
    }
    /* Halve the angle until |x| ≤ 0.01 (Taylor converges to full precision). */
    double mult = 1.0;
    while (fabs(x) > 0.01) {
        x = x / (1.0 + sqrt(1.0 + x * x));
        mult *= 2.0;
    }
    double x2 = x * x;
    double term = x;
    double sum = x;
    for (int k = 1; k <= 6; k++) {
        term *= -x2;
        sum += term / (double)(2 * k + 1);
    }
    return mult * sum;
}

/* atan2(y, x) — argument in (-π, π]. */
double trig_atan2(double y, double x) {
    if (x > 0.0) {
        return trig_atan_pos(y / x);
    }
    if (x < 0.0) {
        if (y >= 0.0) {
            return trig_atan_pos(y / x) + BEBOP_PI;
        }
        return trig_atan_pos(y / x) - BEBOP_PI;
    }
    if (y > 0.0) return BEBOP_FRAC_PI_2;
    if (y < 0.0) return -BEBOP_FRAC_PI_2;
    return 0.0;
}

double trig_atan(double x) {
    return trig_atan2(x, 1.0);
}

/* ─── angle normalization ────────────────────────────────────────────────── */

double trig_normalize_angle(double theta) {
    double t = fmod(theta, BEBOP_TAU);
    if (t > BEBOP_PI) t -= BEBOP_TAU;
    if (t <= -BEBOP_PI) t += BEBOP_TAU;
    return t;
}

/* ─── Phase ──────────────────────────────────────────────────────────────── */

Phase trig_phase_new(double theta) {
    double t = trig_normalize_angle(theta);
    Phase p;
    p.theta = t;
    p.cos = trig_cos(t);
    p.sin = trig_sin(t);
    return p;
}

Phase trig_phase_from_xy(double x, double y) {
    double theta = trig_atan2(y, x);
    double r = sqrt(x * x + y * y);
    if (r < 1e-15) return trig_phase_zero(); /* degenerate → fail-closed */
    Phase p;
    p.theta = theta;
    p.cos = x / r;
    p.sin = y / r;
    return p;
}

Phase trig_phase_one(void) {
    Phase p;
    p.theta = 0.0;
    p.cos = 1.0;
    p.sin = 0.0;
    return p;
}

Phase trig_phase_zero(void) {
    Phase p;
    p.theta = BEBOP_PI;
    p.cos = -1.0;
    p.sin = 0.0;
    return p;
}

Phase trig_phase_uncertain(void) {
    Phase p;
    p.theta = BEBOP_FRAC_PI_2;
    p.cos = 0.0;
    p.sin = 1.0;
    return p;
}

double trig_phase_delta(const Phase *a, const Phase *b) {
    return trig_normalize_angle(a->theta - b->theta);
}

double trig_phase_distance(const Phase *a, const Phase *b) {
    double d = trig_phase_delta(a, b);
    return d < 0.0 ? -d : d;
}

Phase trig_phase_lerp(const Phase *a, const Phase *b, double w) {
    double wc = w < 0.0 ? 0.0 : (w > 1.0 ? 1.0 : w); /* clamp to [0, 1] */
    double dtheta = trig_phase_delta(a, b);
    return trig_phase_new(a->theta + dtheta * wc);
}

double trig_phase_scalar(const Phase *p) {
    return p->cos;
}

double trig_phase_sign(const Phase *p) {
    if (p->cos > 0.0) return 1.0;
    if (p->cos < 0.0) return -1.0;
    return 0.0;
}

/* ─── self-test ──────────────────────────────────────────────────────────── */

static int trig_near(double a, double b, double tol) {
    double d = a - b;
    if (d < 0.0) d = -d;
    return d < tol;
}

int trig_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define A(cond, name) do { \
        int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n", (cond) ? "ok" : "FAIL", name); \
        if (r_ > 0) pos += (size_t)r_; \
        if (!(cond)) all_ok = 0; \
    } while (0)

    /* 1. Exact identities (no rounding). */
    Phase one = trig_phase_one();
    Phase zero = trig_phase_zero();
    Phase unc = trig_phase_uncertain();
    A(one.theta == 0.0 && one.cos == 1.0 && one.sin == 0.0 &&
          zero.cos == -1.0 && zero.sin == 0.0 && trig_near(zero.theta, BEBOP_PI, 1e-15) &&
          unc.cos == 0.0 && unc.sin == 1.0 && trig_near(unc.theta, BEBOP_FRAC_PI_2, 1e-15),
      "one/zero/uncertain are exact identities");

    /* 2. Angle normalization maps to (-π, π]. */
    A(trig_normalize_angle(0.0) == 0.0 &&
          trig_normalize_angle(BEBOP_TAU) == 0.0 &&
          trig_normalize_angle(BEBOP_PI) == BEBOP_PI &&
          trig_normalize_angle(-BEBOP_PI) == BEBOP_PI &&
          trig_near(trig_normalize_angle(3.0 * BEBOP_PI), BEBOP_PI, 1e-15),
      "normalize_angle maps to (-π, π]");

    /* 3. sin/cos exact values + unit-circle invariant. */
    double s = trig_sin(0.7), c = trig_cos(0.7);
    A(trig_near(trig_sin(0.0), 0.0, 1e-15) && trig_near(trig_cos(0.0), 1.0, 1e-15) &&
          trig_near(trig_sin(BEBOP_FRAC_PI_2), 1.0, 1e-15) &&
          trig_near(trig_cos(BEBOP_FRAC_PI_2), 0.0, 1e-15) &&
          trig_near(s * s + c * c, 1.0, 1e-12),
      "sin/cos exact values + sin²+cos² == 1");

    /* 4. atan2 / atan quadrants. */
    A(trig_near(trig_atan2(1.0, 1.0), BEBOP_PI / 4.0, 1e-15) &&
          trig_near(trig_atan2(1.0, 0.0), BEBOP_FRAC_PI_2, 1e-15) &&
          trig_near(trig_atan2(-1.0, -1.0), -3.0 * BEBOP_PI / 4.0, 1e-15) &&
          trig_near(trig_atan(1.0), BEBOP_PI / 4.0, 1e-15),
      "atan2/atan quadrants");

    /* 5. Phase delta / distance / lerp across opposite phases. */
    Phase mid = trig_phase_lerp(&one, &zero, 0.5);
    A(trig_near(trig_phase_delta(&one, &zero), BEBOP_PI, 1e-15) &&
          trig_near(trig_phase_distance(&one, &zero), BEBOP_PI, 1e-15) &&
          trig_near(mid.theta, BEBOP_FRAC_PI_2, 1e-15),
      "delta/distance(one,zero) == π, lerp midpoint == π/2");

    /* 6. from_xy normalizes to the unit circle; degenerate input → zero(). */
    Phase xy = trig_phase_from_xy(3.0, 4.0);
    Phase degen = trig_phase_from_xy(0.0, 0.0);
    A(trig_near(xy.cos, 0.6, 1e-15) && trig_near(xy.sin, 0.8, 1e-15) &&
          degen.cos == -1.0 && degen.sin == 0.0,
      "from_xy(3,4) == (0.6, 0.8), from_xy(0,0) -> zero()");

    return all_ok ? 0 : -1;
}
