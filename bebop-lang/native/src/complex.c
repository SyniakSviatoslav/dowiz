/* Bebop Complex — implementation. */
#include "complex.h"

#include <math.h>
#include <stdio.h>
#include <string.h>

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

int complex_self_test(char *out, size_t cap) {
    size_t p = 0;
    int ok = 1;
    Complex a = c_new(3.0, 4.0);
    Complex b = c_new(1.0, -2.0);
    Complex s = c_add(a, b);
    int s_ok = (s.re == 4.0 && s.im == 2.0);
    snprintf(out + p, cap - p, "[%s] c_add (3+4i)+(1-2i)=4+2i\n", s_ok ? "ok" : "FAIL");
    p = strlen(out);
    if (!s_ok) ok = 0;
    Complex m = c_mul(a, b);
    int m_ok = (m.re == 11.0 && m.im == -2.0); /* (3+4i)(1-2i)=11-2i */
    snprintf(out + p, cap - p, "[%s] c_mul (3+4i)(1-2i)=11-2i\n", m_ok ? "ok" : "FAIL");
    p = strlen(out);
    if (!m_ok) ok = 0;
    double ab = c_abs(a);
    int ab_ok = (ab == 5.0);
    snprintf(out + p, cap - p, "[%s] c_abs |3+4i|=5\n", ab_ok ? "ok" : "FAIL");
    p = strlen(out);
    if (!ab_ok) ok = 0;
    Complex cj = c_conj(a);
    int cj_ok = (cj.re == 3.0 && cj.im == -4.0);
    snprintf(out + p, cap - p, "[%s] c_conj conj(3+4i)=3-4i\n", cj_ok ? "ok" : "FAIL");
    p = strlen(out);
    if (!cj_ok) ok = 0;
    Complex q = c_div(a, b);
    /* (3+4i)/(1-2i) = (3+4i)(1+2i)/5 = (-5+10i)/5 = -1+2i */
    int q_ok = (q.re == -1.0 && q.im == 2.0);
    snprintf(out + p, cap - p, "[%s] c_div (3+4i)/(1-2i)=-1+2i\n", q_ok ? "ok" : "FAIL");
    p = strlen(out);
    if (!q_ok) ok = 0;
    Complex d = c_sub(a, b);
    int d_ok = (d.re == 2.0 && d.im == 6.0);
    snprintf(out + p, cap - p, "[%s] c_sub (3+4i)-(1-2i)=2+6i\n", d_ok ? "ok" : "FAIL");
    p = strlen(out);
    if (!d_ok) ok = 0;
    return ok ? 0 : -1;
}
