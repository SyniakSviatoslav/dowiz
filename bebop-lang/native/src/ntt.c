/* Bebop NTT — implementation (port of dowiz ntt.rs). Zero dependencies. */
#include "ntt.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

uint64_t ntt_mod_pow(uint64_t base, uint64_t exp, uint64_t m) {
    base %= m;
    uint64_t result = 1;
    while (exp > 0) {
        if (exp & 1) {
            result = result * base % m;
        }
        base = base * base % m;
        exp >>= 1;
    }
    return result;
}

uint64_t ntt_mod_inv(uint64_t a, uint64_t m) {
    return ntt_mod_pow(a, m - 2, m);
}

void ntt_transform(uint64_t *a, size_t n, int invert) {
    for (size_t i = 0; i < n; i++) {
        a[i] %= BEBOP_NTT_MOD;
    }

    /* bit-reversal permutation */
    size_t j = 0;
    for (size_t i = 1; i < n; i++) {
        size_t bit = n >> 1;
        while (j & bit) {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if (i < j) {
            uint64_t t = a[i];
            a[i] = a[j];
            a[j] = t;
        }
    }

    for (size_t len = 2; len <= n; len <<= 1) {
        uint64_t root = ntt_mod_pow(BEBOP_NTT_ROOT, (BEBOP_NTT_MOD - 1) / len, BEBOP_NTT_MOD);
        uint64_t wlen = invert ? ntt_mod_inv(root, BEBOP_NTT_MOD) : root;
        for (size_t i = 0; i < n; i += len) {
            uint64_t w = 1;
            for (size_t k = 0; k < len / 2; k++) {
                uint64_t u = a[i + k];
                uint64_t v = a[i + k + len / 2] * w % BEBOP_NTT_MOD;
                a[i + k] = (u + v >= BEBOP_NTT_MOD) ? (u + v - BEBOP_NTT_MOD) : (u + v);
                a[i + k + len / 2] = (u >= v) ? (u - v) : (u + BEBOP_NTT_MOD - v);
                w = w * wlen % BEBOP_NTT_MOD;
            }
        }
    }

    if (invert) {
        uint64_t inv_n = ntt_mod_inv((uint64_t)n, BEBOP_NTT_MOD);
        for (size_t i = 0; i < n; i++) {
            a[i] = a[i] * inv_n % BEBOP_NTT_MOD;
        }
    }
}

void ntt_convolve(const uint64_t *a, size_t alen, const uint64_t *b, size_t blen, uint64_t *out) {
    size_t n = alen + blen - 1;
    size_t size = 1;
    while (size < n) {
        size <<= 1;
    }
    uint64_t *fa = calloc(size, sizeof(uint64_t));
    uint64_t *fb = calloc(size, sizeof(uint64_t));
    memcpy(fa, a, alen * sizeof(uint64_t));
    memcpy(fb, b, blen * sizeof(uint64_t));
    ntt_transform(fa, size, 0);
    ntt_transform(fb, size, 0);
    for (size_t i = 0; i < size; i++) {
        fa[i] = fa[i] * fb[i] % BEBOP_NTT_MOD;
    }
    ntt_transform(fa, size, 1);
    memcpy(out, fa, n * sizeof(uint64_t));
    free(fa);
    free(fb);
}

void ntt_circular(const uint64_t *a, const uint64_t *b, size_t n, uint64_t *out) {
    uint64_t *fa = malloc(n * sizeof(uint64_t));
    uint64_t *fb = malloc(n * sizeof(uint64_t));
    memcpy(fa, a, n * sizeof(uint64_t));
    memcpy(fb, b, n * sizeof(uint64_t));
    ntt_transform(fa, n, 0);
    ntt_transform(fb, n, 0);
    for (size_t i = 0; i < n; i++) {
        fa[i] = fa[i] * fb[i] % BEBOP_NTT_MOD;
    }
    ntt_transform(fa, n, 1);
    memcpy(out, fa, n * sizeof(uint64_t));
    free(fa);
    free(fb);
}

int64_t ntt_centered(uint64_t v) {
    v %= BEBOP_NTT_MOD;
    if (v > BEBOP_NTT_MOD / 2) {
        return (int64_t)v - (int64_t)BEBOP_NTT_MOD;
    }
    return (int64_t)v;
}

int ntt_self_test(char *out, size_t cap) {
    size_t pos = 0;
    int all_ok = 1;
#define N(cond, name)                                                    \
    do {                                                                 \
        int r = snprintf(out + pos, cap - pos, "[%s] %s\n",              \
                         (cond) ? "ok" : "FAIL", name);                  \
        if (r > 0) pos += (size_t)r;                                     \
        if (!(cond)) all_ok = 0;                                         \
    } while (0)

    N(ntt_mod_pow(2, 10, BEBOP_NTT_MOD) == 1024, "mod_pow(2,10) == 1024");
    N(ntt_mod_pow(12345, BEBOP_NTT_MOD - 1, BEBOP_NTT_MOD) == 1, "Fermat a^(p-1) == 1");
    N(7 * ntt_mod_inv(7, BEBOP_NTT_MOD) % BEBOP_NTT_MOD == 1, "mod_inv(7)");

    /* NTT → INTT identity (256) */
    uint64_t a[256], orig[256];
    for (int i = 0; i < 256; i++) {
        a[i] = ((uint64_t)i * i + 7) % BEBOP_NTT_MOD;
        orig[i] = a[i];
    }
    ntt_transform(a, 256, 0);
    ntt_transform(a, 256, 1);
    int ok = 1;
    for (int i = 0; i < 256; i++) {
        if (a[i] != orig[i]) ok = 0;
    }
    N(ok, "NTT→INTT identity (256)");

    /* convolution == naive */
    uint64_t ca[5] = {1, 2, 3, 4, 5};
    uint64_t cb[3] = {6, 7, 8};
    uint64_t cfast[7], cslow[7];
    ntt_convolve(ca, 5, cb, 3, cfast);
    for (int k = 0; k < 7; k++) {
        uint64_t acc = 0;
        for (int i = 0; i <= k && i < 5; i++) {
            if (k - i < 3) {
                acc += ca[i] * cb[k - i];
            }
        }
        cslow[k] = acc % BEBOP_NTT_MOD;
    }
    ok = 1;
    for (int i = 0; i < 7; i++) {
        if (cfast[i] != cslow[i]) ok = 0;
    }
    N(ok, "convolution == naive");

    /* circular shift delta */
    uint64_t delta[8] = {0};
    delta[3] = 1;
    uint64_t sig[8] = {5, 1, 4, 2, 6, 3, 7, 9};
    uint64_t ccirc[8];
    ntt_circular(delta, sig, 8, ccirc);
    ok = 1;
    for (int k = 0; k < 8; k++) {
        if (ccirc[k] != sig[(k + 8 - 3) % 8]) ok = 0;
    }
    N(ok, "circular shift delta recovers shift");

    /* centered correlation */
    uint64_t ones[8], negones[8], corr[8];
    for (int i = 0; i < 8; i++) {
        ones[i] = 1;
        negones[i] = BEBOP_NTT_MOD - 1;
    }
    ntt_circular(ones, ones, 8, corr);
    N(ntt_centered(corr[0]) == 8, "centered correlation == +n");
    ntt_circular(ones, negones, 8, corr);
    N(ntt_centered(corr[0]) == -8, "centered correlation == -n");

    return all_ok ? 0 : -1;
}
