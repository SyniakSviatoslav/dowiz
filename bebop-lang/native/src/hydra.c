/* Bebop hydra — implementation. */
#include "hydra.h"
#include <string.h>
#include <stdio.h>

void hydra_init(Hydra *h, const double *target, int n, double lr, double gain) {
    memset(h, 0, sizeof *h);
    h->learning_rate = lr;
    h->control_gain = gain;
    h->stability_tol = 0.01;
    if (n > HYDRA_MAX_METRICS) n = HYDRA_MAX_METRICS;
    h->target.n_metrics = n;
    for (int i = 0; i < n; i++) h->target.metric[i] = target[i];
}

HydraVerdict hydra_step(Hydra *h, const double *obs, int n) {
    if (n > HYDRA_MAX_METRICS) n = HYDRA_MAX_METRICS;
    h->tick++;
    HydraVerdict v;
    memset(&v, 0, sizeof v);
    v.entropy = 0;

    double alpha = h->learning_rate;
    double drift_sum = 0;

    for (int i = 0; i < n; i++) {
        /* EMA update */
        h->prev.metric[i] = h->state.metric[i];
        h->state.metric[i] = alpha * obs[i] + (1.0 - alpha) * h->state.metric[i];
        h->state.n_metrics = n;

        /* Error = target - state */
        double err = h->target.metric[i] - h->state.metric[i];

        /* PID-style control: P + I */
        double ctrl = h->control_gain * err;
        /* Clamp to [0, 100] */
        if (ctrl < 0) ctrl = 0;
        if (ctrl > 100) ctrl = 100;
        v.control[i] = ctrl;

        /* Drift for entropy */
        double diff = (obs[i] - h->prev.metric[i]);
        drift_sum += (diff < 0 ? -diff : diff);
    }

    /* Entropy: EMA of total drift */
    h->entropy_ema = alpha * drift_sum + (1.0 - alpha) * h->entropy_ema;
    v.entropy = h->entropy_ema;

    /* Lyapunov estimate: log(|delta|/|prev_delta|) */
    if (h->tick > 1 && drift_sum > 0) {
        double prev_drift = 0;
        for (int i = 0; i < n; i++) {
            double d = (h->state.metric[i] - h->prev.metric[i]);
            prev_drift += (d < 0 ? -d : d);
        }
        if (prev_drift > 0) {
            double ratio = drift_sum / prev_drift;
            if (ratio > 0) {
                double lyap = ratio < 1 ? -1.0 : 1.0; /* simplified */
                v.lyapunov = lyap;
            }
        }
    }

    v.converged = (h->entropy_ema < h->stability_tol) ? 1 : 0;
    return v;
}

double hydra_lyapunov(const Hydra *h) { return h->entropy_ema; }
bool   hydra_converged(const Hydra *h, double tol) { return h->entropy_ema < tol; }

int hydra_self_test(char *out, size_t cap) {
    int ok = 0, fail = 0;
#define T(cond, msg) do { ok++; if (!(cond)) { fail++; int n = snprintf(out, cap, "[FAIL] %s\\n", msg); out += n > 0 ? n : 0; cap -= n > 0 ? (size_t)n : 0; } else { int n = snprintf(out, cap, "[ok] %s\\n", msg); out += n > 0 ? n : 0; cap -= n > 0 ? (size_t)n : 0; } } while(0)

    double target[2] = {10.0, 20.0};
    Hydra h;
    hydra_init(&h, target, 2, 0.3, 0.5);

    double obs[2] = {0.0, 0.0};
    for (int i = 0; i < 30; i++) {
        obs[0] += (target[0] - obs[0]) * 0.3;
        obs[1] += (target[1] - obs[1]) * 0.3;
        hydra_step(&h, obs, 2);
    }
    T(h.state.metric[0] > 8.0, "hydra EMA tracks toward target");
    T(h.tick == 30, "30 steps taken");

    HydraVerdict v = hydra_step(&h, obs, 2);
    T(v.control[0] < 100 && v.control[0] >= 0, "control in [0,100]");
    T(v.entropy > 0, "entropy computed");

#undef T
    return fail;
}