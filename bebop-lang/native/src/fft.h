/* Bebop FFT — radix-2 Cooley–Tukey FFT / IFFT (port of dowiz fft.rs).
 * O(N log N), float-domain (signal math, never money). Deterministic:
 * fixed order, twiddle LUT, no fast-math. */
#ifndef BEBOP_FFT_H
#define BEBOP_FFT_H

#include <stddef.h>

#include "complex.h"

/* In-place radix-2 FFT (invert=0) or normalized IFFT (invert=1). Returns 0 on
 * success, -1 if n is not a power of two (or zero). */
int fft_inplace(Complex *a, size_t n, int invert);

/* Parseval energy error: |sum|X|²/n − sum|x|²| (falsifiable probe). */
double fft_parseval_error(const Complex *x, size_t n);

int fft_self_test(char *out, size_t cap);

#endif /* BEBOP_FFT_H */
