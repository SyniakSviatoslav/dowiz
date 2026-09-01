/* Bebop NTT32 — quantized NTT using uint32_t elements (MOD < 2^31). */
#include "ntt32.h"
#include <stdlib.h>
#include <string.h>

#define NTT32_MU 18479187002ULL

static inline uint32_t ntt32_reduce(uint64_t x) {
    uint64_t q = (uint64_t)(((__uint128_t)x * (__uint128_t)NTT32_MU) >> 64);
    uint64_t r = x - q * BEBOP_NTT32_MOD;
    if (r >= BEBOP_NTT32_MOD) r -= BEBOP_NTT32_MOD;
    return (uint32_t)r;
}

static inline uint32_t ntt32_mulmod(uint32_t a, uint32_t b) {
    return ntt32_reduce((uint64_t)a * (uint64_t)b);
}

static uint32_t ntt32_pow(uint32_t a, uint32_t e) {
    uint32_t r = 1;
    while (e) {
        if (e & 1) r = ntt32_mulmod(r, a);
        a = ntt32_mulmod(a, a);
        e >>= 1;
    }
    return r;
}

static uint32_t ntt32_inv(uint32_t a) {
    return ntt32_pow(a, BEBOP_NTT32_MOD - 2);
}

__attribute__((optimize("O2")))
void ntt32_transform(uint32_t *a, size_t n, bool invert) {
    size_t i;
    for (i = 0; i < n; i++) a[i] %= BEBOP_NTT32_MOD;

    size_t j = 0;
    for (i = 1; i < n; i++) {
        size_t bit = n >> 1;
        while (j & bit) { j ^= bit; bit >>= 1; }
        j ^= bit;
        if (i < j) { uint32_t t = a[i]; a[i] = a[j]; a[j] = t; }
    }

    static uint32_t roots_static[4096];
    uint32_t *roots = (n / 2 <= 4096) ? roots_static : malloc((n/2)*sizeof(uint32_t));
    uint32_t wprim = ntt32_pow(BEBOP_NTT32_ROOT, (BEBOP_NTT32_MOD-1)/(uint32_t)n);
    if (invert) wprim = ntt32_inv(wprim);
    roots[0] = 1;
    for (size_t k = 1; k < n/2; k++)
        roots[k] = ntt32_mulmod(roots[k-1], wprim);

    for (size_t len = 2; len <= n; len <<= 1) {
        size_t half = len/2, step = n/len;
        for (size_t i2 = 0; i2 < n; i2 += len) {
            for (size_t k = 0; k < half; k++) {
                uint32_t w = roots[k * step];
                uint32_t u = a[i2 + k];
                uint32_t v = ntt32_mulmod(a[i2 + k + half], w);
                a[i2 + k] = u+v >= BEBOP_NTT32_MOD ? u+v-BEBOP_NTT32_MOD : u+v;
                a[i2 + k + half] = u >= v ? u-v : u+BEBOP_NTT32_MOD-v;
            }
        }
    }
    if (roots != roots_static) free(roots);

    if (invert) {
        uint32_t inv_n = ntt32_inv((uint32_t)n);
        for (i = 0; i < n; i++) a[i] = ntt32_mulmod(a[i], inv_n);
    }
}

__attribute__((optimize("O2")))
void ntt32_convolve(const uint32_t *a, size_t alen,
                    const uint32_t *b, size_t blen, uint32_t *out) {
    size_t n = alen+blen-1, size=1;
    while (size < n) size <<= 1;
    uint32_t *fa = calloc(size, sizeof(uint32_t));
    uint32_t *fb = calloc(size, sizeof(uint32_t));
    memcpy(fa, a, alen*sizeof(uint32_t));
    memcpy(fb, b, blen*sizeof(uint32_t));
    ntt32_transform(fa, size, false);
    ntt32_transform(fb, size, false);
    for (size_t i = 0; i < size; i++) fa[i] = ntt32_mulmod(fa[i], fb[i]);
    ntt32_transform(fa, size, true);
    memcpy(out, fa, n*sizeof(uint32_t));
    free(fa); free(fb);
}

int64_t ntt32_centered(uint32_t v) {
    v %= BEBOP_NTT32_MOD;
    return (v > BEBOP_NTT32_MOD/2) ? (int64_t)v-(int64_t)BEBOP_NTT32_MOD : (int64_t)v;
}

/* self-test */
static int nw32(char *b, size_t c, const char *s) {
    size_t n = 0; while (s[n]) n++; if (n > c) n = c;
    for (size_t i = 0; i < n; i++) b[i] = s[i];
    return (int)n;
}
#define T32(cond, msg) do { ok++; int _n; \
    if (!(cond)) { fail++; \
        _n=nw32(out,cap,"[FAIL] ");out+=_n;cap-=(size_t)_n; \
        _n=nw32(out,cap,msg);out+=_n;cap-=(size_t)_n; \
        _n=nw32(out,cap,"\\n");out+=_n;cap-=(size_t)_n; \
    } else { \
        _n=nw32(out,cap,"[ok] ");out+=_n;cap-=(size_t)_n; \
        _n=nw32(out,cap,msg);out+=_n;cap-=(size_t)_n; \
        _n=nw32(out,cap,"\\n");out+=_n;cap-=(size_t)_n; \
    } \
} while(0)

int ntt32_self_test(char *out, size_t cap) {
    int ok=0, fail=0;
    T32(ntt32_pow(2,10)==1024, "mod_pow(2,10)==1024");
    T32(ntt32_pow(2, BEBOP_NTT32_MOD-1)==1, "Fermat a^(p-1)==1");
    uint32_t x[256];
    for (size_t i=0; i<256; i++) x[i]=(uint32_t)i;
    ntt32_transform(x,256,false);
    ntt32_transform(x,256,true);
    bool ok_id=true;
    for (size_t i=0; i<256; i++)
        if (x[i]!=i % BEBOP_NTT32_MOD) ok_id=false;
    T32(ok_id, "NTT to INTT identity (256)");
    uint32_t ca[5]={1,2,3,4,5}, cb[3]={1,1,1}, co[7];
    ntt32_convolve(ca,5,cb,3,co);
    T32(co[0]==1 && co[1]==3 && co[2]==6
        && co[3]==9 && co[4]==12 && co[5]==9 && co[6]==5,
        "convolution == naive");
    #undef T32
    return fail;
}
