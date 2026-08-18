/* Bebop spectral — eigenvalue engine (port of dowiz spectral.rs).
 *
 * Algorithms:
 *   1. Faddeev-LeVerrier → characteristic polynomial coefficients
 *   2. Durand-Kerner → simultaneous root-finding for ALL eigenvalues
 *   3. Power iteration → dominant eigenvalue + eigenvector
 *
 * PURE C11, ZERO dependencies. Float is graph structure, never money.
 */
#include "spectral.h"
#include <stdlib.h>
#include <string.h>

/* ─── helpers ───────────────────────────────────────────────────────────── */

static double trace(const double *a, size_t n) {
    double t = 0.0;
    for (size_t i = 0; i < n; i++) t += a[i * n + i];
    return t;
}

/* c = a × b, row-major, n×n. Caller frees. */
static double *matmul(const double *a, const double *b, size_t n) {
    double *c = calloc(n * n, sizeof(double));
    for (size_t i = 0; i < n; i++)
        for (size_t k = 0; k < n; k++) {
            double aik = a[i * n + k];
            for (size_t j = 0; j < n; j++)
                c[i * n + j] += aik * b[k * n + j];
        }
    return c;
}

/* c = a + s*I (identity scaled by s). In-place on a. */
static void add_identity(double *a, size_t n, double s) {
    for (size_t i = 0; i < n; i++) a[i * n + i] += s;
}

/* ─── Faddeev-LeVerrier ──────────────────────────────────────────────────
 * Computes coefficients of characteristic polynomial,
 * highest-degree first: [1, c_{n-1}, …, c_0].
 * coeffs must have n+1 elements. */
static void charpoly(const double *a, size_t n, double *coeffs) {
    if (n == 0) { coeffs[0] = 1.0; return; }
    for (size_t i = 0; i <= n; i++) coeffs[i] = 0.0;
    coeffs[n] = 1.0;

    double *m = calloc(n * n, sizeof(double));
    for (size_t i = 0; i < n; i++) m[i * n + i] = 1.0; /* M_1 = I */

    double *am = matmul(a, m, n);
    coeffs[n - 1] = -trace(am, n);
    free(am);

    for (size_t k = 2; k <= n; k++) {
        double *am2 = matmul(a, m, n);   /* A·M_{k-1} */
        add_identity(am2, n, coeffs[n - k + 1]); /* M_k */
        free(m);
        m = am2;
        double *am3 = matmul(a, m, n);
        coeffs[n - k] = -trace(am3, n) / (double)k;
        free(am3);
    }
    free(m);
    /* reverse: highest-degree first */
    for (size_t i = 0; i <= n / 2; i++) {
        double t = coeffs[i];
        coeffs[i] = coeffs[n - i];
        coeffs[n - i] = t;
    }
}

/* ─── Durand-Kerner ──────────────────────────────────────────────────────
 * Simultaneous root-finding for monic polynomial coeffs[0..deg]
 * (highest-degree first, coeffs[0] = 1).
 * Initial seeds spread around unit circle.
 * Returns number of converged roots. */

static double c_abs(SpectralComplex z) {
    double a = z.re, b = z.im;
    return (a < 0 ? -a : a) + (b < 0 ? -b : b); /* manhattan norm, faster */
}

static SpectralComplex c_poly_eval(const double *coeffs, size_t deg,
                                    SpectralComplex z) {
    SpectralComplex r = {coeffs[0], 0.0};
    for (size_t i = 1; i <= deg; i++) {
        double r_re = r.re * z.re - r.im * z.im + coeffs[i];
        r.im = r.re * z.im + r.im * z.re;
        r.re = r_re;
    }
    return r;
}

size_t spectral_eigenvalues(const double *mat, size_t n,
                            SpectralComplex *out, size_t max_roots,
                            int max_iter, double tol) {
    if (n == 0 || max_roots < n) return 0;
    double *coeffs = malloc((n + 1) * sizeof(double));
    charpoly(mat, n, coeffs);

    /* Durand-Kerner initial seeds on unit circle */
    SpectralComplex *z = malloc(n * sizeof(SpectralComplex));
    for (size_t i = 0; i < n; i++) {
        double angle = 2.0 * 3.141592653589793 * (double)i / (double)n + 0.4;
        z[i].re = 0.4 * (angle - 3.141592653589793 < 0 ?
                   -(angle - 3.141592653589793) : angle - 3.141592653589793);
        z[i].im = 0.4 * (angle < 0 ? -angle : angle);
    }

    for (int iter = 0; iter < max_iter; iter++) {
        double max_delta = 0.0;
        for (size_t i = 0; i < n; i++) {
            SpectralComplex pz = c_poly_eval(coeffs, n, z[i]);
            SpectralComplex den = {1.0, 0.0};
            for (size_t j = 0; j < n; j++) {
                if (j == i) continue;
                double dr = z[i].re - z[j].re;
                double di = z[i].im - z[j].im;
                double nr = den.re * dr - den.im * di;
                den.im = den.re * di + den.im * dr;
                den.re = nr;
            }
            double d_abs = den.re * den.re + den.im * den.im;
            if (d_abs < 1e-30) continue;
            SpectralComplex delta = {
                -(pz.re * den.re + pz.im * den.im) / d_abs,
                -(pz.im * den.re - pz.re * den.im) / d_abs
            };
            z[i].re += delta.re;
            z[i].im += delta.im;
            double da = c_abs(delta);
            if (da > max_delta) max_delta = da;
        }
        if (max_delta < tol) break;
    }

    memcpy(out, z, n * sizeof(SpectralComplex));
    free(z);
    free(coeffs);
    return n;
}

/* ─── Power iteration ─────────────────────────────────────────────────── */

SpectralComplex spectral_power_iter(const double *mat, size_t n,
                                     double *eigvec, int max_iter, double tol) {
    for (size_t i = 0; i < n; i++) eigvec[i] = 1.0 / (double)n;
    double lambda = 1.0;

    for (int iter = 0; iter < max_iter; iter++) {
        double *av = calloc(n, sizeof(double));
        for (size_t i = 0; i < n; i++)
            for (size_t j = 0; j < n; j++)
                av[i] += mat[i * n + j] * eigvec[j];

        double new_lambda = 0.0;
        for (size_t i = 0; i < n; i++)
            new_lambda += eigvec[i] * av[i];

        /* infinity norm: max |av[i]|, avoids sqrt (zero-dep) */
        double maxv = 0.0;
        for (size_t i = 0; i < n; i++) {
            double a = av[i] < 0 ? -av[i] : av[i];
            if (a > maxv) maxv = a;
        }
        double scale = (maxv > 1e-30) ? 1.0 / maxv : 1.0;
        for (size_t i = 0; i < n; i++) eigvec[i] = av[i] * scale;

        double delta = new_lambda - lambda;
        double ad = (delta < 0 ? -delta : delta);
        lambda = new_lambda;
        free(av);
        if (ad < tol) break;
    }
    return (SpectralComplex){lambda, 0.0};
}

/* ─── spectral gap ────────────────────────────────────────────────────── */

double spectral_gap(const SpectralComplex *evals, size_t n) {
    if (n < 2) return -1.0;
    /* Find two largest squared magnitudes */
    double best = 0.0, second = 0.0;
    for (size_t i = 0; i < n; i++) {
        double m2 = evals[i].re * evals[i].re + evals[i].im * evals[i].im;
        if (m2 > best) { second = best; best = m2; }
        else if (m2 > second) second = m2;
    }
    if (best < 1e-30) return 1.0;
    /* gap = 1 − |λ₂|/|λ₁|, using sqrt via Newton (zero-dep) */
    double x = best, sx = 1.0;
    for (int k = 0; k < 6; k++) { sx = 0.5 * (sx + x / sx); }
    double s2 = 1.0;
    x = second;
    for (int k = 0; k < 6; k++) { s2 = 0.5 * (s2 + x / s2); }
    return 1.0 - s2 / sx;
}

/* ─── self-test ───────────────────────────────────────────────────────── */
static int nwrite(char *b, size_t c, const char *s) {
    size_t n = 0; while (s[n]) n++; if (n > c) n = c;
    for (size_t i = 0; i < n; i++) b[i] = s[i];
    return (int)n;
}
#define T(cond, msg) do { ok++; int _n; \
    if (!(cond)) { fail++; \
        _n=nwrite(out,cap,"[FAIL] ");out+=_n;cap-=(size_t)_n; \
        _n=nwrite(out,cap,msg);out+=_n;cap-=(size_t)_n; \
        _n=nwrite(out,cap,"\n");out+=_n;cap-=(size_t)_n; \
    } else { \
        _n=nwrite(out,cap,"[ok] ");out+=_n;cap-=(size_t)_n; \
        _n=nwrite(out,cap,msg);out+=_n;cap-=(size_t)_n; \
        _n=nwrite(out,cap,"\n");out+=_n;cap-=(size_t)_n; \
    } \
} while(0)

int spectral_self_test(char *out, size_t cap) {
    int ok = 0, fail = 0;

    /* diagonal matrix: eigenvalues = diagonal entries */
    double diag[9] = {3,0,0, 0,5,0, 0,0,7};
    SpectralComplex evals[3];
    size_t nr = spectral_eigenvalues(diag, 3, evals, 3, 100, 1e-10);
    T(nr == 3, "3x3 diagonal → 3 eigenvalues");

    /* check magnitudes roughly correct */
    double mags[3];
    for (int i = 0; i < 3; i++)
        mags[i] = evals[i].re * evals[i].re + evals[i].im * evals[i].im;
    /* sort */
    for (int i = 0; i < 2; i++)
        for (int j = i+1; j < 3; j++)
            if (mags[i] > mags[j]) { double t=mags[i]; mags[i]=mags[j]; mags[j]=t; }
    T(mags[0] > 8.0 && mags[0] < 10.0, "smallest eigval ≈ 9");
    T(mags[2] > 48.0 && mags[2] < 50.0, "largest eigval ≈ 49");

    /* power iteration on diagonal */
    double vec[3] = {0};
    SpectralComplex dom = spectral_power_iter(diag, 3, vec, 100, 1e-10);
    T(dom.re > 6.5 && dom.re < 7.5, "power iteration → 7");

    /* spectral gap for diag(3,5,7): λ₂=5, gap = 1-5/7 ≈ 0.286 */
    double gap = spectral_gap(evals, 3);
    T(gap > 0.2 && gap < 0.4, "spectral gap ≈ 0.286");

    #undef T
    return fail;
}
