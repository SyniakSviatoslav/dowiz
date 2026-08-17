/* Bebop markov — first-order Markov chain: row-normalised transition matrix,
 * one step, and damped stationary distribution via power iteration
 * (port of dowiz markov.rs). */
#ifndef BEBOP_MARKOV_H
#define BEBOP_MARKOV_H

#include <stddef.h>

/* Fixed-size state alphabet (dowiz markov.rs works on small-n chains). */
#define MARKOV_MAX_N 8

/* Row-normalised transition matrix (row-major: m[i][j] = P(i -> j)). */
typedef struct {
    size_t n;                              /* number of states, 1..MARKOV_MAX_N */
    double m[MARKOV_MAX_N][MARKOV_MAX_N];  /* transition probabilities, rows sum to 1 */
} MarkovMatrix;

/* Zero the matrix and record n. */
void markov_matrix_init(MarkovMatrix *a, size_t n);

/* Row-normalise in place; a zero row is set to uniform 1/n (dowiz "unseen
 * row" rule). Returns 0 on success, -1 if n is 0 or > MARKOV_MAX_N. */
int markov_row_normalize(MarkovMatrix *a);

/* One chain step: out = pi * a (pi is a row vector, left multiplication).
 * Returns 0 on success, -1 on a bad argument. */
int markov_step(const double *pi, const MarkovMatrix *a, double *out);

/* Damped stationary distribution by power iteration:
 * pi = pi * Â' with Â' = (1 - damping) * a + (damping / n) * J, started from
 * the uniform vector. Returns 0 on success, -1 on bad n / damping / iters. */
int markov_stationary(const MarkovMatrix *a, double damping, int iters,
                      double *pi_out);

/* Iteration budget k ~= ln(1/tol) / ln(1/slem) for a mixing rate slem
 * (requires 0 < slem < 1 and tol > 0; otherwise +infinity). */
double markov_budget(double slem, double tol);

int markov_self_test(char *out, size_t cap);

#endif /* BEBOP_MARKOV_H */
