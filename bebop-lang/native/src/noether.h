/* Bebop noether — deterministic invariant + Lyapunov verifier.
 *
 * Callbacks operate on double[n] vectors. The update function receives the
 * mutable state vector; the invariant/potential function computes a scalar.
 *
 * ctx pointers are opaque — pass your own scratch space or parameters.
 */
#ifndef BEBOP_NOETHER_H
#define BEBOP_NOETHER_H

#include <stdbool.h>
#include <stddef.h>

/* Update function: x[] → f(x[]).  Modifies x in-place; n is the vector length. */
typedef void (*noether_update_fn)(double *x, size_t n, void *ctx);

/* Invariant / potential function: I(x[]) → scalar.  Must be pure. */
typedef double (*noether_invariant_fn)(const double *x, size_t n, void *ctx);

/* Two-sided: |I(f(x)) − I(x)| ≤ tol at every step. */
bool noether_step_preserves(
    const double *x0, size_t n,
    noether_update_fn update, void *update_ctx,
    noether_invariant_fn invariant, void *inv_ctx,
    size_t steps, double tol);

/* Total variation Σ|ΔI| along trajectory. */
double noether_invariant_drift(
    const double *x0, size_t n,
    noether_update_fn update, void *update_ctx,
    noether_invariant_fn invariant, void *inv_ctx,
    size_t steps);

/* One-sided Lyapunov: V(f(x)) - V(x) must never exceed tol.
 * Accepts decreases (dissipative). */
bool noether_lyapunov_nonincreasing(
    const double *x0, size_t n,
    noether_update_fn update, void *update_ctx,
    noether_invariant_fn potential, void *pot_ctx,
    size_t steps, double tol);

int noether_self_test(char *out, size_t cap);

#endif /* BEBOP_NOETHER_H */