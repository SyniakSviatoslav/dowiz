#include "memristor.h"
#include <stdio.h>
#include <string.h>

/* Joglekar window: f(x)=1-(2x-1)^2p, prevents x from leaving [0,1]. p=3 gives soft boundary. */
static double window(double x) {
    double d = 2.0*x - 1.0;
    double d2 = d*d;
    double d4 = d2*d2;
    return 1.0 - d4*d2; /* p=3: d^(2*3)=d^6 = d4*d2 */
}

void memristor_init(Memristor *m, double x0) {
    memset(m,0,sizeof *m);
    m->x = (x0 < 0) ? 0 : ((x0 > 1) ? 1 : x0);
    m->Ron = 100.0;   /* 100 ohm ON resistance */
    m->Roff = 16000.0; /* 16 kohm OFF resistance (TiO2 typical) */
    m->D = 1e-6;       /* 10 nm = 1e-6 cm (consistent with mu in cm2/Vs) */
    m->mu = 1e-10;     /* ion mobility cm2/Vs */
}

double memristor_step(Memristor *m, double v, double dt) {
    /* i = v / R(x), R(x) = Ron*x + Roff*(1-x) */
    double R = m->Ron * m->x + m->Roff * (1.0 - m->x);
    if (R < 1e-12) R = 1e-12;
    double i = v / R;
    double k = m->mu * m->Ron / (m->D * m->D);
    double dx = k * i * window(m->x) * dt;
    m->x += dx;
    if (m->x > 1.0) m->x = 1.0;
    if (m->x < 0.0) m->x = 0.0;
    return m->x;
}

double memristor_conductance(const Memristor *m) {
    double G_on = 1.0 / m->Ron, G_off = 1.0 / m->Roff;
    return m->x * G_on + (1.0 - m->x) * G_off;
}

int memristor_crossbar(const double *x, size_t rows, size_t cols,
                       double Ron, double Roff, double *G) {
    double Gon = 1.0/Ron, Goff = 1.0/Roff;
    for (size_t i = 0; i < rows * cols; i++) {
        double s = x[i];
        if (s > 1.0) { s = 1.0; }
        if (s < 0.0) { s = 0.0; }
        G[i] = s * Gon + (1.0 - s) * Goff;
    }
    return 0;
}
int memristor_self_test(char *out, size_t cap) {
    size_t p=0; int ok=1;
#define K(c,n) do{int r_=snprintf(out+p,cap-p,"[%s] %s\n",(c)?"ok":"FAIL",n); if(r_>0)p+=r_; if(!(c))ok=0;}while(0)
    Memristor m; memristor_init(&m, 0.5);
    double x1 = memristor_step(&m, 1.0, 1e-6);
    K(x1 > 0.5, "positive v moves x toward 1");
    double G = memristor_conductance(&m);
    K(G > 0.0 && G < 1.0, "conductance in valid range");
    /* crossbar: 2x2 state matrix */
    double xb[4] = {0.2, 0.8, 0.5, 0.3}, Gb[4];
    memristor_crossbar(xb, 2, 2, 100.0, 16000.0, Gb);
    K(Gb[0] < Gb[1] && Gb[1] > Gb[3], "crossbar: high-x gives higher G");
    return ok?0:-1;
}
