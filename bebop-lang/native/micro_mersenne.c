/* Bebop NTT — Mersenne prime variant (MOD = 2^31-1, root = 7).
 *
 * Reduction: (x & MOD) + (x >> 31) — ZERO multiplies, pure AND+shift+add.
 * This eliminates the Barrett 128-bit multiply and halves the inner-loop
 * latency vs the generic MOD=998244353 version.
 */
#include <stdint.h>
#include <stddef.h>
#include <stdbool.h>
#include <stdlib.h>
#include <string.h>

#define MERSENNE_MOD  2147483647U
#define MERSENNE_ROOT 7U

/* (a*b) mod M — pure AND+shift, no multiply beyond a*b itself. */
static inline uint32_t m_reduce(uint64_t x) {
    uint32_t r = (uint32_t)(x & MERSENNE_MOD) + (uint32_t)(x >> 31);
    if (r >= MERSENNE_MOD) r -= MERSENNE_MOD;
    return r;
}

static inline uint32_t m_mul(uint32_t a, uint32_t b) {
    return m_reduce((uint64_t)a * (uint64_t)b);
}

static uint32_t m_pow(uint32_t a, uint32_t e) {
    uint32_t r = 1;
    while (e) {
        if (e & 1) r = m_mul(r, a);
        a = m_mul(a, a);
        e >>= 1;
    }
    return r;
}

static uint32_t m_inv(uint32_t a) {
    return m_pow(a, MERSENNE_MOD - 2);
}

/* ─── NTT transform (Mersenne) ──────────────────────────────────── */
__attribute__((optimize("O2")))
void mersenne_ntt_transform(uint32_t *a, size_t n, bool invert) {
    for (size_t i = 0; i < n; i++) if (a[i] >= MERSENNE_MOD) a[i] %= MERSENNE_MOD;

    size_t j = 0;
    for (size_t i = 1; i < n; i++) {
        size_t bit = n >> 1;
        while (j & bit) { j ^= bit; bit >>= 1; }
        j ^= bit;
        if (i < j) { uint32_t t = a[i]; a[i] = a[j]; a[j] = t; }
    }

    static uint32_t roots_static[4096];
    uint32_t *roots = (n/2 <= 4096) ? roots_static : malloc((n/2)*sizeof(uint32_t));
    uint32_t wprim = m_pow(MERSENNE_ROOT, (MERSENNE_MOD-1)/(uint32_t)n);
    if (invert) wprim = m_inv(wprim);
    roots[0] = 1;
    for (size_t k = 1; k < n/2; k++)
        roots[k] = m_mul(roots[k-1], wprim);

    for (size_t len = 2; len <= n; len <<= 1) {
        size_t half = len/2, step = n/len;
        for (size_t i2 = 0; i2 < n; i2 += len) {
            for (size_t k = 0; k < half; k++) {
                uint32_t w = roots[k*step];
                uint32_t u = a[i2+k];
                uint32_t v = m_mul(a[i2+k+half], w);
                a[i2+k]       = u+v >= MERSENNE_MOD ? u+v-MERSENNE_MOD : u+v;
                a[i2+k+half]  = u >= v ? u-v : u+MERSENNE_MOD-v;
            }
        }
    }
    if (roots != roots_static) free(roots);

    if (invert) {
        uint32_t inv_n = m_inv((uint32_t)n);
        for (size_t i = 0; i < n; i++) a[i] = m_mul(a[i], inv_n);
    }
}

/* ─── convolution (Mersenne) ──────────────────────────────────── */
__attribute__((optimize("O2")))
void mersenne_ntt_convolve(const uint32_t *a, size_t alen,
                           const uint32_t *b, size_t blen, uint32_t *out) {
    size_t n = alen+blen-1, size=1;
    while (size < n) size <<= 1;
    uint32_t *fa = calloc(size, sizeof(uint32_t));
    uint32_t *fb = calloc(size, sizeof(uint32_t));
    memcpy(fa, a, alen*sizeof(uint32_t));
    memcpy(fb, b, blen*sizeof(uint32_t));
    mersenne_ntt_transform(fa, size, false);
    mersenne_ntt_transform(fb, size, false);
    for (size_t i = 0; i < size; i++) fa[i] = m_mul(fa[i], fb[i]);
    mersenne_ntt_transform(fa, size, true);
    memcpy(out, fa, n*sizeof(uint32_t));
    free(fa); free(fb);
}

/* ─── self-test ───────────────────────────────────────────────── */
#include <stdio.h>
int main() {
    printf("Mersenne NTT: MOD=%u, ROOT=%u\n", MERSENNE_MOD, MERSENNE_ROOT);
    printf("pow(7, MOD-1) = %u (should be 1)\n", m_pow(7, MERSENNE_MOD-1));

    uint32_t x[256];
    for (size_t i=0; i<256; i++) x[i]=(uint32_t)i;
    mersenne_ntt_transform(x,256,false);
    mersenne_ntt_transform(x,256,true);
    int ok=1;
    for (size_t i=0; i<256; i++)
        if (x[i]!=i % MERSENNE_MOD) { ok=0; break; }
    printf("NTT→INTT identity: %s\n", ok?"PASS":"FAIL");

    uint32_t ca[5]={1,2,3,4,5}, cb[3]={1,1,1}, co[7];
    mersenne_ntt_convolve(ca,5,cb,3,co);
    printf("conv: [%u %u %u %u %u %u %u]\n", co[0],co[1],co[2],co[3],co[4],co[5],co[6]);

    /* benchmark */
    #include <time.h>
    enum { inner=256 };
    struct timespec t0, t1;
    clock_gettime(CLOCK_MONOTONIC, &t0);
    for (int k=0; k<inner; k++)
        mersenne_ntt_convolve(ca,5,cb,3,co);
    clock_gettime(CLOCK_MONOTONIC, &t1);
    double ns = (t1.tv_sec-t0.tv_sec)*1e9 + (t1.tv_nsec-t0.tv_nsec);
    printf("small conv: %.0f ns/op\n", ns/inner);

    /* full-size benchmark */
    enum { n1024 = 1024, inner2 = 64 };
    uint32_t *a = calloc(n1024, sizeof(uint32_t));
    uint32_t *b = calloc(n1024, sizeof(uint32_t));
    uint32_t *out2 = calloc(2*n1024-1, sizeof(uint32_t));
    for (int i=0; i<n1024; i++) { a[i]=(uint32_t)i; b[i]=(uint32_t)(i*7); }
    // warmup
    for (int w=0; w<3; w++) mersenne_ntt_convolve(a,n1024,b,n1024,out2);
    clock_gettime(CLOCK_MONOTONIC, &t0);
    for (int k=0; k<inner2; k++)
        mersenne_ntt_convolve(a,n1024,b,n1024,out2);
    clock_gettime(CLOCK_MONOTONIC, &t1);
    ns = (t1.tv_sec-t0.tv_sec)*1e9 + (t1.tv_nsec-t0.tv_nsec);
    printf("n=1024 conv: %.0f ns/op\n", ns/inner2);
    printf("sink=%u\n", out2[512]);
    free(a); free(b); free(out2);
    return ok?0:1;
}
