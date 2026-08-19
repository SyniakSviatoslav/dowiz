#ifndef BEBOP_MEMRISTOR_H
#define BEBOP_MEMRISTOR_H
#include <stddef.h>
#include <stdint.h>

/* HP memristor model: dx/dt = mu * Ron/D^2 * i(t) * f(x) where f(x) is
 * the Joglekar window function. x = internal state in [0,1], G(x) = x*G_on + (1-x)*G_off. */
typedef struct {
    double x;     /* internal state [0,1] */
    double Ron, Roff; /* resistance bounds (ohms) */
    double D;     /* device thickness (nm, e.g. 10) */
    double mu;    /* ion mobility (e.g. 1e-10 cm2/Vs) */
} Memristor;

/* Initialize with default HP TiO2 parameters. */
void memristor_init(Memristor *m, double x0);

/* Step state: x += dt * mu*Ron/D^2 * v * window(x). Returns new x. */
double memristor_step(Memristor *m, double v, double dt);

/* Conductance: G(x) = x/Ron + (1-x)/Roff. Returns in siemens. */
double memristor_conductance(const Memristor *m);

/* NEON-vectorized crossbar: G_ij = crossbar(x[i*cols+j], Ron, Roff).
 * Updates conductances from state matrix x (rows*cols). */
int memristor_crossbar(const double *x, size_t rows, size_t cols,
                       double Ron, double Roff, double *G);

int memristor_self_test(char *out, size_t cap);
#endif
