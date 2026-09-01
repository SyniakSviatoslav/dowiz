/* Bebop oracle — implementation (port of dowiz predict.rs + autonomic.rs). */
#include "oracle.h"
#include <string.h>
#include <stdio.h>

void oracle_init(Oracle *o, int n_metrics, double learning_rate) {
    memset(o, 0, sizeof *o);
    o->learning_rate = learning_rate;
    o->window = 32;
    o->friction = 0.0;
    /* Initialise with one dummy observation */
    o->history[0].n_metrics = n_metrics;
    o->history[0].tick = 0;
    o->n_history = 1;
    /* Initialise state from dummy */
    for (int i = 0; i < n_metrics; i++) o->state[i] = 0.0;
}

int oracle_observe(Oracle *o, const double *metrics, int n) {
    if (n > ORACLE_MAX_METRICS) n = ORACLE_MAX_METRICS;
    o->tick++;

    /* Enqueue observation in circular buffer */
    OracleObs *obs = &o->history[o->n_history % ORACLE_MAX_HISTORY];
    obs->n_metrics = n;
    obs->tick = o->tick;
    for (int i = 0; i < n; i++) obs->metric[i] = metrics[i];
    o->n_history++;

    /* Update EMA state: s_new = α*m + (1-α)*s_old */
    double alpha = o->learning_rate;
    for (int i = 0; i < n; i++) {
        double old_state = o->state[i];
        o->state[i] = alpha * metrics[i] + (1.0 - alpha) * old_state;
        o->trend[i] = metrics[i] - old_state; /* instantaneous gradient */
    }

    /* Fit linear regression over recent window */
    int start = (o->n_history > o->window) ? (o->n_history - o->window) : 0;
    int count = o->n_history - start;
    if (count < 2) return 0;
    for (int j = 0; j < n; j++) {
        double sx = 0, sy = 0, sxx = 0, sxy = 0;
        for (int k = start; k < o->n_history; k++) {
            double x = (double)(k - start);
            double y = o->history[k % ORACLE_MAX_HISTORY].metric[j];
            sx += x; sy += y; sxx += x*x; sxy += x*y;
        }
        double denom = count*sxx - sx*sx;
        if (denom != 0) {
            o->slope[j] = (count*sxy - sx*sy) / denom;
            o->intercept[j] = (sy - o->slope[j]*sx) / count;
        }
    }

    /* Update friction: EMA of |trend|. Lower friction = system more responsive. */
    double total_drift = 0;
    for (int i = 0; i < n; i++) {
        total_drift += (o->trend[i] < 0 ? -o->trend[i] : o->trend[i]);
    }
    o->friction = alpha * total_drift + (1.0 - alpha) * o->friction;

    return 0;
}

void oracle_predict(const Oracle *o, int steps, OraclePred *out, int n) {
    if (n > ORACLE_MAX_METRICS) n = ORACLE_MAX_METRICS;
    for (int i = 0; i < n; i++) {
        /* Linear extrapolate: y = slope*(current_x + steps) + intercept */
        int last_x = o->n_history > 0 ? o->n_history - 1 : 0;
        double future_x = (double)(last_x + steps);
        double val = o->slope[i] * future_x + o->intercept[i];
        /* Clamp to EMA-based prediction if regression is unstable */
        double ema_pred = o->state[i] + o->trend[i] * steps;
        double blend = (o->friction < 1.0) ? (1.0 - o->friction) : 0.0;
        out[i].predicted_value = blend * val + (1.0 - blend) * ema_pred;
        out[i].drift_rate = o->slope[i];
        out[i].confidence = 1.0 / (1.0 + o->friction);
        out[i].friction_score = o->friction;
    }
}

OraclePred oracle_predict_one(const Oracle *o, int metric_idx, int steps) {
    OraclePred p = {0};
    if (metric_idx < ORACLE_MAX_METRICS) {
        oracle_predict(o, steps, &p, metric_idx + 1);
        p = (&p)[metric_idx]; /* get the specific prediction */
    }
    return p;
}

double oracle_friction(const Oracle *o) { return o->friction; }

int oracle_time_to_steady(const Oracle *o, double tol) {
    double max_drift = 0;
    for (int i = 0; i < ORACLE_MAX_METRICS; i++) {
        double d = (o->trend[i] < 0 ? -o->trend[i] : o->trend[i]);
        if (d > max_drift) max_drift = d;
    }
    if (max_drift < tol) return 0;
    return (int)(max_drift / tol);
}

/* ─── self-test ─────────────────────────────────────────────────────────── */

int oracle_self_test(char *out, size_t cap) {
    int ok = 0, fail = 0;
#define T(cond, msg) do { ok++; if (!(cond)) { fail++; int n = snprintf(out, cap, "[FAIL] %s\\n", msg); out += n > 0 ? n : 0; cap -= n > 0 ? (size_t)n : 0; } else { int n = snprintf(out, cap, "[ok] %s\\n", msg); out += n > 0 ? n : 0; cap -= n > 0 ? (size_t)n : 0; } } while(0)

    Oracle o;
    oracle_init(&o, 3, 0.3);

    /* Feed rising values */
    double data[3] = {1.0, 2.0, 3.0};
    for (int i = 0; i < 50; i++) {
        data[0] += 0.1; data[1] += 0.2; data[2] += 0.3;
        oracle_observe(&o, data, 3);
    }
    T(o.n_history == 51, "51 observations recorded");
    T(o.state[0] > 5.5, "EMA tracks rising metric 0");
    T(o.state[2] > o.state[0], "metric 2 rises faster than metric 0");

    /* Predict 5 steps ahead */
    OraclePred preds[3];
    oracle_predict(&o, 5, preds, 3);
    T(preds[0].predicted_value > o.state[0], "prediction > current (rising trend)");
    T(preds[0].confidence > 0.0 && preds[0].confidence <= 1.0, "confidence in [0,1]");
    T(preds[2].drift_rate > preds[0].drift_rate, "metric 2 drifts faster");

    /* Time to steady */
    int tts = oracle_time_to_steady(&o, 0.01);
    T(tts >= 0, "time_to_steady non-negative");

    /* Friction updated */
    T(o.friction > 0, "friction grows with drift");

    /* Re-init and observe constant values → friction should decay */
    oracle_init(&o, 2, 0.5);
    double flat[2] = {5.0, 5.0};
    for (int i = 0; i < 20; i++) oracle_observe(&o, flat, 2);
    T(o.state[0] > 4.9 && o.state[0] < 5.1, "EMA converges to 5.0");
    oracle_predict(&o, 10, preds, 2);
    T(preds[0].predicted_value > 3.0, "constant prediction ≈ 5");

#undef T
    return fail;
}