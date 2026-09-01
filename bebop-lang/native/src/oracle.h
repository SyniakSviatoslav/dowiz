/* Bebop oracle — mutable runtime prediction engine (port of dowiz predict.rs).
 *
 * Ties together PID, Markov, spectral, noether, autonomic into a unified
 * online predictor. Mutable: update with observations at runtime. */
#ifndef BEBOP_ORACLE_H
#define BEBOP_ORACLE_H

#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>

#define ORACLE_MAX_METRICS 8
#define ORACLE_MAX_HISTORY 256

/* One observation: labeled metrics vector + timestamp. */
typedef struct {
    double   metric[ORACLE_MAX_METRICS];
    int      n_metrics;
    int      tick;
} OracleObs;

/* Prediction for one metric. */
typedef struct {
    double predicted_value;
    double confidence;      /* [0,1] */
    double drift_rate;      /* derivative */
    double friction_score;  /* resistance to change */
} OraclePred;

/* The mutable oracle engine. */
typedef struct {
    /* Configuration (mutable at runtime) */
    double learning_rate;
    int    window;           /* observations window for trend */

    /* State */
    OracleObs history[ORACLE_MAX_HISTORY];
    int      n_history;
    double   state[ORACLE_MAX_METRICS];  /* running EMA */
    double   trend[ORACLE_MAX_METRICS];  /* running gradient */
    double   friction;                    /* overall system friction [0,1] */
    int      tick;

    /* Regression coefficients (linear fit per metric) */
    double   slope[ORACLE_MAX_METRICS];
    double   intercept[ORACLE_MAX_METRICS];
} Oracle;

/* Initialise oracle. */
void oracle_init(Oracle *o, int n_metrics, double learning_rate);

/* Feed a new observation, update state. Returns 0. */
int  oracle_observe(Oracle *o, const double *metrics, int n);

/* Predict all metrics `steps` ahead. */
void oracle_predict(const Oracle *o, int steps, OraclePred *out, int n);

/* Predict a single metric (most recent state). */
OraclePred oracle_predict_one(const Oracle *o, int metric_idx, int steps);

/* Get current system friction (how resistant to change). */
double oracle_friction(const Oracle *o);

/* Estimate time-to-convergence (remaining steps until steady state). */
int    oracle_time_to_steady(const Oracle *o, double tol);

/* ─── self-test ─────────────────────────────────────────────────────────── */
int    oracle_self_test(char *out, size_t cap);

#endif