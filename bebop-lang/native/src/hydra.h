/* Bebop hydra — closed-loop inference engine (port of dowiz hydra_closed_loop.rs).
 * Combines oracle (predict) + autonomic (control) + feedback. */
#ifndef BEBOP_HYDRA_H
#define BEBOP_HYDRA_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#define HYDRA_MAX_METRICS 8

typedef struct {
    double metric[HYDRA_MAX_METRICS];
    int    n_metrics;
} HydraState;

typedef struct {
    double control[HYDRA_MAX_METRICS];  /* control signal (0-100 bounded rate) */
    double entropy;                      /* system entropy (Lyapunov) */
    double lyapunov;                     /* Lyapunov exponent estimate */
    int    converged;                    /* 1 if system is at steady state */
} HydraVerdict;

typedef struct {
    /* Configuration */
    double learning_rate;
    double control_gain;      /* Kp for PID-style adjustment */
    double stability_tol;     /* convergence threshold */

    /* State */
    HydraState state;
    HydraState target;        /* desired state (setpoint) */
    HydraState prev;
    double     entropy_ema;
    int        tick;
} Hydra;

/* Init with setpoint target. */
void hydra_init(Hydra *h, const double *target, int n, double lr, double gain);

/* Feed observation, produce control verdict. */
HydraVerdict hydra_step(Hydra *h, const double *obs, int n);

/* Estimate Lyapunov exponent from state trajectory. */
double hydra_lyapunov(const Hydra *h);

/* Is system at steady state? */
bool   hydra_converged(const Hydra *h, double tol);

/* ─── self-test ─────────────────────────────────────────────────────────── */
int    hydra_self_test(char *out, size_t cap);

#endif