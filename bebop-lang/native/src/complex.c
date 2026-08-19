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

int complex_self_test(char *out, size_t cap) {
    size_t p=0; int ok=1;
    Complex a={3,4}, b={1,-2};
    Complex s=complex_add(a,b), m=complex_mul(a,b), d=complex_abs(a);
    int r=snprintf(out+p,cap-p,"[%s] add\n", s.re==4&&s.im==2?"ok":"FAIL"); if(r>0)p+=r; if(!(s.re==4&&s.im==2))ok=0;
    r=snprintf(out+p,cap-p,"[%s] mul\n", m.re==11&&m.im==-2?"ok":"FAIL"); if(r>0)p+=r; if(!(m.re==11&&m.im==-2))ok=0;
    r=snprintf(out+p,cap-p,"[%s] abs 3+4i=5\n", d==5.0?"ok":"FAIL"); if(r>0)p+=r; if(!(d==5.0))ok=0;
    return ok?0:-1;
}
