/* Bebop Modular — implementation (port of dowiz modular.rs). */
#include "modular.h"

#include <math.h>
#include <stdio.h>

Mobius mobius_identity(void) {
    Mobius m = {1, 0, 0, 1};
    return m;
}
Mobius mobius_s(void) {
    Mobius m = {0, -1, 1, 0};
    return m;
}
Mobius mobius_t(void) {
    Mobius m = {1, 1, 0, 1};
    return m;
}
int64_t mobius_det(Mobius m) {
    return m.a * m.d - m.b * m.c;
}

Complex mobius_apply(Mobius m, Complex z) {
    double nr = (double)m.a * z.re + (double)m.b;
    double ni = (double)m.a * z.im;
    double dr = (double)m.c * z.re + (double)m.d;
    double di = (double)m.c * z.im;
    double dn = dr * dr + di * di;
    if (dn < 1e-15) {
        return z; /* pole: fail-closed, no NaN propagation */
    }
    return c_div(c_new(nr, ni), c_new(dr, di));
}

Mobius mobius_compose(Mobius m, Mobius o) {
    Mobius r;
    r.a = m.a * o.a + m.b * o.c;
    r.b = m.a * o.b + m.b * o.d;
    r.c = m.c * o.a + m.d * o.c;
    r.d = m.c * o.b + m.d * o.d;
    return r;
}

int mobius_in_fundamental_domain(Complex z) {
    return z.im > 0.0 && c_abs(z) >= 1.0 - 1e-12 &&
           fabs(z.re) <= 0.5 + 1e-12;
}

Complex mobius_reduce(Complex z, int max_iter) {
    Mobius s = mobius_s();
    for (int i = 0; i < max_iter; i++) {
        double shift = round(z.re);
        if (shift != 0.0) {
            Mobius t = {1, -(int64_t)shift, 0, 1};
            z = mobius_apply(t, z);
        }
        if (c_abs(z) < 1.0 - 1e-12) {
            z = mobius_apply(s, z);
        } else if (mobius_in_fundamental_domain(z)) {
            break;
        } else {
            break;
        }
    }
    return z;
}

int modular_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define D(cond, name)                                                \
    do {                                                             \
        int r = snprintf(out + pos, cap - pos, "[%s] %s\n",          \
                         (cond) ? "ok" : "FAIL", name);              \
        if (r > 0) pos += (size_t)r;                                 \
        if (!(cond)) all_ok = 0;                                     \
    } while (0)

    Complex z = c_new(0.3, 1.5);

    Complex once = mobius_apply(mobius_s(), z);
    Complex twice = mobius_apply(mobius_s(), once);
    D(fabs(twice.re - z.re) < 1e-9 && fabs(twice.im - z.im) < 1e-9,
      "S^2 == id");

    Complex tz = mobius_apply(mobius_t(), z);
    D(fabs(tz.re - (z.re + 1.0)) < 1e-9 && fabs(tz.im - z.im) < 1e-9,
      "T adds 1");

    Mobius st = mobius_compose(mobius_s(), mobius_t());
    Mobius st3 = mobius_compose(mobius_compose(st, st), st);
    D(mobius_det(st3) == 1, "(ST)^3 det == 1");
    Complex r3 = mobius_apply(st3, z);
    D(fabs(r3.re - z.re) < 1e-9 && fabs(r3.im - z.im) < 1e-9,
      "(ST)^3 == id");

    D(mobius_det(mobius_s()) == 1 && mobius_det(mobius_t()) == 1 &&
          mobius_det(mobius_identity()) == 1,
      "generators have det 1");

    Complex far = c_new(3.7, 0.2);
    Complex red = mobius_reduce(far, 200);
    D(mobius_in_fundamental_domain(red), "reduce lands in fundamental domain");

    D(mobius_in_fundamental_domain(c_new(0.2, 1.0)) &&
          !mobius_in_fundamental_domain(c_new(0.9, 1.0)) &&
          !mobius_in_fundamental_domain(c_new(0.0, 0.5)),
      "fundamental-domain classifier");

    return all_ok ? 0 : -1;
}
