/* Bebop spectral — eigenvalue/eigenvector engine (port of dowiz spectral.rs).
 *
 * PURE C11, ZERO dependencies. Computes eigenvalues via Faddeev-LeVerrier
 * characteristic polynomial + Durand-Kerner root-finding.
 */
#ifndef BEBOP_SPECTRAL_H
#define BEBOP_SPECTRAL_H

#include <stddef.h>
#include <stdbool.h>

/* Complex number for eigenvalue representation. */
typedef struct { double re, im; } SpectralComplex;

/* Compute ALL eigenvalues of an n×n real matrix (row-major: mat[i*n + j]).
 * Stores up to max_roots eigenvalues in out[]. Returns number found (should be n). */
size_t spectral_eigenvalues(const double *mat, size_t n,
                            SpectralComplex *out, size_t max_roots,
                            int max_iter, double tol);

/* Power iteration: dominant eigenvalue + eigenvector. */
SpectralComplex spectral_power_iter(const double *mat, size_t n,
                                    double *eigvec /* [n] output */,
                                    int max_iter, double tol);

/* Spectral gap γ = 1 − |λ₂|. Returns -1 if n < 2. */
double spectral_gap(const SpectralComplex *evals, size_t n);

int spectral_self_test(char *out, size_t cap);

#endif
