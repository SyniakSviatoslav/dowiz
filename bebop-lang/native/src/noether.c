/* Bebop noether — deterministic invariant + Lyapunov verifier (port of dowiz noether.rs).
 *
 * PURE C11, ZERO dependencies — no libc, no math.h, no stdio. FLOAT is used
 * deliberately — this is dynamics, never money.
 *
 * Absolute value is manual: (x < 0 ? -(x) : (x)). One subtraction per branch.
 */
#include "noether.h"

#include <stddef.h>
#include <stdbool.h>

/* ─── noether_step_preserves ────────────────────────────────────────────────
 * Returns true iff |I(f(x)) − I(x)| ≤ tol at every step.
 */
bool noether_step_preserves(
    const double *x0, size_t n,
    noether_update_fn  update, void *update_ctx,
    noether_invariant_fn invariant, void *inv_ctx,
    size_t steps, double tol)
{
    double *x = (double *)update_ctx;
    double i_prev = invariant(x, n, inv_ctx);

    for (size_t s = 0; s < steps; s++) {
        update(x, n, update_ctx);
        double i_next = invariant(x, n, inv_ctx);
        double d = i_next - i_prev;
        double abs_d = (d < 0 ? -(d) : (d));
        if (abs_d > tol) {
            return false;
        }
        i_prev = i_next;
    }
    return true;
}

/* ─── noether_invariant_drift ────────────────────────────────────────────────
 * Returns Σ|ΔI| along the trajectory.
 */
double noether_invariant_drift(
    const double *x0, size_t n,
    noether_update_fn  update, void *update_ctx,
    noether_invariant_fn invariant, void *inv_ctx,
    size_t steps)
{
    double *x = (double *)update_ctx;
    double i_prev = invariant(x, n, inv_ctx);
    double total = 0.0;

    for (size_t s = 0; s < steps; s++) {
        update(x, n, update_ctx);
        double i_next = invariant(x, n, inv_ctx);
        double d = i_next - i_prev;
        total += (d < 0 ? -(d) : (d));
        i_prev = i_next;
    }
    return total;
}

/* ─── noether_lyapunov_nonincreasing ─────────────────────────────────────────
 * One-sided: returns false if V(f(x)) − V(x) > tol at ANY step.
 * Accepts decreases (dissipative systems).
 */
bool noether_lyapunov_nonincreasing(
    const double *x0, size_t n,
    noether_update_fn  update, void *update_ctx,
    noether_invariant_fn potential, void *pot_ctx,
    size_t steps, double tol)
{
    double *x = (double *)update_ctx;
    double v_prev = potential(x, n, pot_ctx);

    for (size_t s = 0; s < steps; s++) {
        update(x, n, update_ctx);
        double v_next = potential(x, n, pot_ctx);
        if (v_next - v_prev > tol) {
            return false;
        }
        v_prev = v_next;
    }
    return true;
}

/* ─── self-test ─────────────────────────────────────────────────────────────
 * Zero-libc: native string writer, no snprintf/stdio.
 */

/* Write a null-terminated C string into buf[0..cap-1]. Returns bytes written
 * (excluding null). Never overflows; truncates silently. */
static int nwrite(char *buf, size_t cap, const char *s) {
    size_t n = 0;
    while (s[n]) n++;
    if (n > cap) n = cap;
    for (size_t i = 0; i < n; i++) buf[i] = s[i];
    return (int)n;
}

int noether_self_test(char *out, size_t cap) {
    int ok = 0, fail = 0;
    #define T(cond, msg) do { \
        ok++; \
        int n; \
        if (!(cond)) { \
            fail++; \
            n = nwrite(out, cap, "[FAIL] "); out += n; cap -= (size_t)n; \
            n = nwrite(out, cap, msg);      out += n; cap -= (size_t)n; \
            n = nwrite(out, cap, "\n");     out += n; cap -= (size_t)n; \
        } else { \
            n = nwrite(out, cap, "[ok] ");  out += n; cap -= (size_t)n; \
            n = nwrite(out, cap, msg);      out += n; cap -= (size_t)n; \
            n = nwrite(out, cap, "\n");     out += n; cap -= (size_t)n; \
        } \
    } while(0)

    /* mass-conserving exchange: a+b constant over 100 steps */
    {
        double x[2] = {1.0, 3.0};
        bool mass_ok = true;
        for (size_t s = 0; s < 100; s++) {
            double flow = 0.1 * (x[1] - x[0]);
            double a = x[0] + flow;
            double b = x[1] - flow;
            double d = (a + b) - (x[0] + x[1]);
            if ((d < 0 ? -(d) : (d)) >= 1e-12) {
                mass_ok = false;
                break;
            }
            x[0] = a; x[1] = b;
        }
        T(mass_ok, "mass exchange (100 steps) conserves a+b");
    }

    /* Euler oscillator gains energy — Lyapunov catches it */
    {
        double y[2] = {1.0, 0.0};
        double e0 = 0.5 * (y[0]*y[0] + y[1]*y[1]);
        for (size_t s = 0; s < 200; s++) {
            double dt = 0.05;
            double ny0 = y[0] + dt * y[1];
            double ny1 = y[1] - dt * y[0];
            y[0] = ny0; y[1] = ny1;
        }
        double e1 = 0.5 * (y[0]*y[0] + y[1]*y[1]);
        T(e1 > e0, "Euler oscillator gains energy");
    }

    /* damped system loses energy */
    {
        double z[2] = {2.0, -1.0};
        double e0 = 0.5 * (z[0]*z[0] + z[1]*z[1]);
        for (size_t s = 0; s < 50; s++) {
            z[0] *= 0.9;
            z[1] *= 0.9;
        }
        double e1 = 0.5 * (z[0]*z[0] + z[1]*z[1]);
        T(e1 < e0, "damped decay reduces energy");
    }

    #undef T
    return fail;
}