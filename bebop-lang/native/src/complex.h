#include <stddef.h>
/* Bebop Complex — shared complex number primitive (used by fft, modular,
 * spectral, …). */
#ifndef BEBOP_COMPLEX_H
#define BEBOP_COMPLEX_H

typedef struct {
    double re, im;
} Complex;

Complex c_new(double re, double im);
Complex c_add(Complex a, Complex b);
Complex c_sub(Complex a, Complex b);
Complex c_mul(Complex a, Complex b);
Complex c_div(Complex a, Complex b);
Complex c_conj(Complex a);
double c_abs(Complex a);

int complex_self_test(char *out, size_t cap);
#endif /* BEBOP_COMPLEX_H */
