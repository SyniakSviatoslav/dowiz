/* Bebop FFT — implementation (port of dowiz fft.rs). */
#include "fft.h"

#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#define FFT_PI 3.14159265358979323846

static Complex c_add(Complex a, Complex b) {
    Complex r;
    r.re = a.re + b.re;
    r.im = a.im + b.im;
    return r;
}
static Complex c_sub(Complex a, Complex b) {
    Complex r;
    r.re = a.re - b.re;
    r.im = a.im - b.im;
    return r;
}
static Complex c_mul(Complex a, Complex b) {
    Complex r;
    r.re = a.re * b.re - a.im * b.im;
    r.im = a.re * b.im + a.im * b.re;
    return r;
}

static void bit_reverse(Complex *a, size_t n) {
    size_t j = 0;
    for (size_t i = 1; i < n; i++) {
        size_t bit = n >> 1;
        while (j & bit) {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if (i < j) {
            Complex t = a[i];
            a[i] = a[j];
            a[j] = t;
        }
    }
}

int fft_inplace(Complex *a, size_t n, int invert) {
    if (n == 0 || (n & (n - 1)) != 0) {
        return -1;
    }
    bit_reverse(a, n);
    Complex *twiddle = malloc((n / 2) * sizeof(Complex));
    if (!twiddle) {
        return -1;
    }
    double sign = invert ? 2.0 : -2.0;
    for (size_t k = 0; k < n / 2; k++) {
        double theta = sign * FFT_PI * (double)k / (double)n;
        twiddle[k].re = cos(theta);
        twiddle[k].im = sin(theta);
    }
    for (size_t len = 2; len <= n; len <<= 1) {
        size_t half = len / 2;
        size_t step = n / len;
        for (size_t start = 0; start < n; start += len) {
            for (size_t j = 0; j < half; j++) {
                Complex w = twiddle[j * step];
                Complex u = a[start + j];
                Complex v = c_mul(a[start + j + half], w);
                a[start + j] = c_add(u, v);
                a[start + j + half] = c_sub(u, v);
            }
        }
    }
    free(twiddle);
    if (invert) {
        for (size_t i = 0; i < n; i++) {
            a[i].re /= (double)n;
            a[i].im /= (double)n;
        }
    }
    return 0;
}

double fft_parseval_error(const Complex *x, size_t n) {
    Complex *y = malloc(n * sizeof(Complex));
    memcpy(y, x, n * sizeof(Complex));
    fft_inplace(y, n, 0);
    double time_energy = 0.0, freq_energy = 0.0;
    for (size_t i = 0; i < n; i++) {
        time_energy += x[i].re * x[i].re + x[i].im * x[i].im;
        freq_energy += y[i].re * y[i].re + y[i].im * y[i].im;
    }
    free(y);
    double e = freq_energy / (double)n - time_energy;
    return e < 0 ? -e : e;
}

int fft_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define F(cond, name)                                                \
    do {                                                             \
        int r = snprintf(out + pos, cap - pos, "[%s] %s\n",          \
                         (cond) ? "ok" : "FAIL", name);              \
        if (r > 0) pos += (size_t)r;                                 \
        if (!(cond)) all_ok = 0;                                     \
    } while (0)

    /* round-trip: ifft(fft(x)) == x */
    {
        size_t n = 16;
        Complex x[16];
        for (size_t i = 0; i < n; i++) {
            x[i].re = sin((double)i);
            x[i].im = cos((double)i * 0.5);
        }
        fft_inplace(x, n, 0);
        fft_inplace(x, n, 1);
        int ok = 1;
        for (size_t i = 0; i < n; i++) {
            if (fabs(x[i].re - sin((double)i)) > 1e-9 ||
                fabs(x[i].im - cos((double)i * 0.5)) > 1e-9) {
                ok = 0;
            }
        }
        F(ok, "ifft(fft(x)) == x (round-trip)");
    }

    /* DC impulse: constant -> single bin */
    {
        size_t n = 8;
        Complex x[8];
        for (size_t i = 0; i < n; i++) {
            x[i].re = 1.0;
            x[i].im = 0.0;
        }
        fft_inplace(x, n, 0);
        int ok = fabs(x[0].re - 8.0) < 1e-9;
        for (size_t k = 1; k < n; k++) {
            if (sqrt(x[k].re * x[k].re + x[k].im * x[k].im) > 1e-9) {
                ok = 0;
            }
        }
        F(ok, "DC impulse -> single bin");
    }

    /* Parseval */
    {
        size_t n = 32;
        Complex x[32];
        for (size_t i = 0; i < n; i++) {
            x[i].re = cos((double)i * 0.3);
            x[i].im = sin((double)i * 0.7);
        }
        F(fft_parseval_error(x, n) < 1e-8, "Parseval energy conservation");
    }

    /* non-power-of-two rejected */
    {
        Complex x[3] = {{1, 0}, {1, 0}, {1, 0}};
        F(fft_inplace(x, 3, 0) == -1, "non-power-of-two rejected");
    }

    return all_ok ? 0 : -1;
}
