/* Bebop markov — implementation (port of dowiz markov.rs). */
#include "markov.h"

#include <math.h>
#include <stdio.h>

void markov_matrix_init(MarkovMatrix *a, size_t n) {
    if (!a) {
        return;
    }
    a->n = n;
    for (size_t i = 0; i < MARKOV_MAX_N; i++) {
        for (size_t j = 0; j < MARKOV_MAX_N; j++) {
            a->m[i][j] = 0.0;
        }
    }
}

int markov_row_normalize(MarkovMatrix *a) {
    if (!a || a->n == 0 || a->n > MARKOV_MAX_N) {
        return -1;
    }
    size_t n = a->n;
    for (size_t i = 0; i < n; i++) {
        double s = 0.0;
        for (size_t j = 0; j < n; j++) {
            s += a->m[i][j];
        }
        if (s > 0.0) {
            for (size_t j = 0; j < n; j++) {
                a->m[i][j] /= s;
            }
        } else {
            /* unseen row (dowiz "unseen row => uniform" rule). */
            for (size_t j = 0; j < n; j++) {
                a->m[i][j] = 1.0 / (double)n;
            }
        }
    }
    return 0;
}

int markov_step(const double *pi, const MarkovMatrix *a, double *out) {
    if (!pi || !a || !out || a->n == 0 || a->n > MARKOV_MAX_N) {
        return -1;
    }
    size_t n = a->n;
    for (size_t j = 0; j < n; j++) {
        double acc = 0.0;
        for (size_t i = 0; i < n; i++) {
            acc += pi[i] * a->m[i][j];
        }
        out[j] = acc;
    }
    return 0;
}

int markov_stationary(const MarkovMatrix *a, double damping, int iters,
                      double *pi_out) {
    if (!a || !pi_out || a->n == 0 || a->n > MARKOV_MAX_N ||
        damping < 0.0 || damping >= 1.0 || iters <= 0) {
        return -1;
    }
    size_t n = a->n;
    double pi[MARKOV_MAX_N];
    double nxt[MARKOV_MAX_N];
    for (size_t i = 0; i < n; i++) {
        pi[i] = 1.0 / (double)n;
    }
    for (int it = 0; it < iters; it++) {
        for (size_t j = 0; j < n; j++) {
            nxt[j] = 0.0;
        }
        for (size_t i = 0; i < n; i++) {
            double pii = pi[i];
            if (pii == 0.0) {
                continue;
            }
            for (size_t j = 0; j < n; j++) {
                nxt[j] += pii * ((1.0 - damping) * a->m[i][j] +
                                 damping / (double)n);
            }
        }
        double sum = 0.0;
        for (size_t j = 0; j < n; j++) {
            sum += nxt[j];
        }
        if (sum <= 0.0) {
            sum = 1.0; /* unreachable for a stochastic matrix; fail-safe */
        }
        for (size_t j = 0; j < n; j++) {
            nxt[j] /= sum;
        }
        for (size_t j = 0; j < n; j++) {
            pi[j] = nxt[j];
        }
    }
    for (size_t j = 0; j < n; j++) {
        pi_out[j] = pi[j];
    }
    return 0;
}

double markov_budget(double slem, double tol) {
    if (slem <= 0.0 || slem >= 1.0 || tol <= 0.0) {
        return INFINITY;
    }
    return log(1.0 / tol) / log(1.0 / slem);
}

int markov_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define A(cond, name) do { \
    int r_ = snprintf(out + pos, cap - pos, "[%s] %s\n", (cond) ? "ok" : "FAIL", name); \
    if (r_ > 0) pos += (size_t)r_; \
    if (!(cond)) all_ok = 0; \
} while (0)

    /* 1. row-normalise: every row sums to 1 */
    {
        MarkovMatrix a;
        markov_matrix_init(&a, 3);
        double raw[3][3] = {
            {2.0, 1.0, 1.0},
            {0.0, 3.0, 1.0},
            {5.0, 0.0, 0.0},
        };
        for (size_t i = 0; i < 3; i++) {
            for (size_t j = 0; j < 3; j++) {
                a.m[i][j] = raw[i][j];
            }
        }
        int ok = markov_row_normalize(&a) == 0;
        for (size_t i = 0; i < 3 && ok; i++) {
            double s = 0.0;
            for (size_t j = 0; j < 3; j++) {
                s += a.m[i][j];
            }
            if (fabs(s - 1.0) > 1e-12) {
                ok = 0;
            }
        }
        A(ok, "row-normalise: rows sum to 1");
    }

    /* 2. zero (unseen) row becomes uniform 1/n */
    {
        MarkovMatrix a;
        markov_matrix_init(&a, 3);
        a.m[0][0] = 1.0; a.m[0][1] = 0.0; a.m[0][2] = 0.0;
        a.m[1][0] = 0.5; a.m[1][1] = 0.5; a.m[1][2] = 0.0;
        /* row 2 left all-zero => unseen */
        markov_row_normalize(&a);
        int ok = 1;
        for (size_t j = 0; j < 3; j++) {
            if (fabs(a.m[2][j] - 1.0 / 3.0) > 1e-12) {
                ok = 0;
            }
        }
        A(ok, "unseen row -> uniform 1/n");
    }

    /* 3. one step preserves total probability mass */
    {
        MarkovMatrix a;
        markov_matrix_init(&a, 3);
        double raw[3][3] = {
            {0.5, 0.3, 0.2},
            {0.1, 0.6, 0.3},
            {0.4, 0.1, 0.5},
        };
        for (size_t i = 0; i < 3; i++) {
            for (size_t j = 0; j < 3; j++) {
                a.m[i][j] = raw[i][j];
            }
        }
        markov_row_normalize(&a);
        double pi[3] = {0.2, 0.5, 0.3};
        double outv[3];
        int r = markov_step(pi, &a, outv);
        double s = outv[0] + outv[1] + outv[2];
        A(r == 0 && fabs(s - 1.0) < 1e-12, "step preserves mass");
    }

    /* 4. absorbing state stays put */
    {
        MarkovMatrix a;
        markov_matrix_init(&a, 2);
        a.m[0][0] = 1.0; a.m[0][1] = 0.0;
        a.m[1][0] = 0.5; a.m[1][1] = 0.5;
        double pi[2] = {1.0, 0.0};
        double outv[2];
        markov_step(pi, &a, outv);
        A(fabs(outv[0] - 1.0) < 1e-12 && fabs(outv[1]) < 1e-12,
          "absorbing state stays");
    }

    /* 5. stationary is a left eigenvector of the damped chain */
    {
        MarkovMatrix a;
        markov_matrix_init(&a, 3);
        double raw[3][3] = {
            {0.5, 0.3, 0.2},
            {0.1, 0.6, 0.3},
            {0.4, 0.1, 0.5},
        };
        for (size_t i = 0; i < 3; i++) {
            for (size_t j = 0; j < 3; j++) {
                a.m[i][j] = raw[i][j];
            }
        }
        markov_row_normalize(&a);
        double damping = 0.02;
        double pi[3];
        markov_stationary(&a, damping, 1000, pi);
        /* damped matrix Â' = (1 - d) * a + d/n */
        MarkovMatrix ap;
        markov_matrix_init(&ap, 3);
        for (size_t i = 0; i < 3; i++) {
            for (size_t j = 0; j < 3; j++) {
                ap.m[i][j] = (1.0 - damping) * a.m[i][j] + damping / 3.0;
            }
        }
        double outv[3];
        markov_step(pi, &ap, outv);
        int ok = 1;
        for (size_t j = 0; j < 3; j++) {
            if (fabs(outv[j] - pi[j]) > 1e-9) {
                ok = 0;
            }
        }
        A(ok, "stationary is a left eigenvector");
    }

    /* 6. symmetric 2x2 chain -> uniform stationary */
    {
        MarkovMatrix a;
        markov_matrix_init(&a, 2);
        a.m[0][0] = 0.5; a.m[0][1] = 0.5;
        a.m[1][0] = 0.5; a.m[1][1] = 0.5;
        double pi[2];
        markov_stationary(&a, 0.0, 100, pi);
        A(fabs(pi[0] - 0.5) < 1e-9 && fabs(pi[1] - 0.5) < 1e-9,
          "symmetric chain -> uniform stationary");
    }

#undef A
    return all_ok ? 0 : -1;
}
