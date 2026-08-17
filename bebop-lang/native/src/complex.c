/* Bebop Complex — implementation. */
#include "complex.h"

#include <math.h>

Complex c_new(double re, double im) {
    Complex c;
    c.re = re;
    c.im = im;
    return c;
}
Complex c_add(Complex a, Complex b) {
    Complex c;
    c.re = a.re + b.re;
    c.im = a.im + b.im;
    return c;
}
Complex c_sub(Complex a, Complex b) {
    Complex c;
    c.re = a.re - b.re;
    c.im = a.im - b.im;
    return c;
}
Complex c_mul(Complex a, Complex b) {
    Complex c;
    c.re = a.re * b.re - a.im * b.im;
    c.im = a.re * b.im + a.im * b.re;
    return c;
}
Complex c_div(Complex a, Complex b) {
    double d = b.re * b.re + b.im * b.im;
    Complex c;
    c.re = (a.re * b.re + a.im * b.im) / d;
    c.im = (a.im * b.re - a.re * b.im) / d;
    return c;
}
Complex c_conj(Complex a) {
    Complex c;
    c.re = a.re;
    c.im = -a.im;
    return c;
}
double c_abs(Complex a) {
    return sqrt(a.re * a.re + a.im * a.im);
}
