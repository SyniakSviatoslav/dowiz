/* r10 side-channel timing harness — Bebop crypto primitives.
 *
 * Measures per-call wall time (clock_gettime CLOCK_MONOTONIC) of x25519
 * scalar-mult, sha256, and aes_gcm under a FIXED secret vs RANDOM secrets,
 * emitting raw nanosecond samples for a Welch t-test downstream.
 *
 * Methodology (defense-only timing audit):
 *   - N=2000 samples per distribution (fixed + random), interleaved F/R/F/R...
 *     so slow frequency drift / thermal effects hit both groups equally.
 *   - All random secrets are pre-generated before the timed region, so PRNG
 *     cost is excluded from the samples.
 *   - Warmup loop before timing to populate L1 (AES S-box), I-cache, branch
 *     predictor, and let the Walt governor settle.
 *   - volatile sink prevents any dead-code elimination of the primitive calls.
 *   - pin with `taskset -c 0` at launch (Walt governor jitter mitigation).
 */
#define _POSIX_C_SOURCE 199309L
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <stdint.h>

#include "x25519.h"
#include "sha256.h"
#include "aes_gcm.h"

#define N 2000

static int MIN_K = 1; /* min-of-K repetitions per sample (1 = raw single-shot) */

static inline uint64_t now(void) {
    struct timespec ts;
    clock_gettime(CLOCK_MONOTONIC, &ts);
    return (uint64_t)ts.tv_sec * 1000000000ull + (uint64_t)ts.tv_nsec;
}

/* xorshift64* — deterministic, fast, no libc dependency. */
static uint64_t rng_state = 0x9E3779B97F4A7C15ull;
static uint64_t rng64(void) {
    uint64_t x = rng_state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    rng_state = x;
    return x * 0x2545F4914F6CDD1Dull;
}
static void fill_random(uint8_t *buf, size_t n) {
    for (size_t i = 0; i < n; i++) buf[i] = (uint8_t)(rng64() >> 32);
}

/* ---- x25519 scalar-mult ---- */
static void bench_x25519(void) {
    uint8_t fixed_sk[32], peer_pk[32], out[32];
    static uint8_t rand_sk[N][32];
    memset(fixed_sk, 0, 32); fixed_sk[0] = 0xAB; fixed_sk[1] = 0xCD; fixed_sk[31] = 0x42;
    /* basepoint u-coordinate = 9 (RFC 7748) */
    memset(peer_pk, 0, 32); peer_pk[0] = 9;
    for (int i = 0; i < N; i++) fill_random(rand_sk[i], 32);

    volatile uint8_t sink = 0;
    for (int i = 0; i < 500; i++) x25519_shared_secret(fixed_sk, peer_pk, out);
    for (int t = 0; t < 200; t++) x25519_shared_secret(fixed_sk, peer_pk, out); /* throwaway warm */
    for (int t = 0; t < 2 * N; t++) {
        int fixed = (t & 1) == 0;
        const uint8_t *sk = fixed ? fixed_sk : rand_sk[t >> 1];
        uint64_t best = ~0ull;
        for (int k = 0; k < MIN_K; k++) {
            uint64_t a = now();
            x25519_shared_secret(sk, peer_pk, out);
            uint64_t b = now();
            uint64_t d = b - a;
            if (d < best) best = d;
        }
        sink ^= out[0];
        printf("x25519 %c %lld\n", fixed ? 'F' : 'R', (long long)best);
    }
    fprintf(stderr, "x25519 sink=%u\n", (unsigned)sink);
}

/* ---- sha256 (fixed 64-byte message, vary content) ---- */
static void bench_sha256(void) {
    uint8_t fixed_msg[64], out[32];
    static uint8_t rand_msg[N][64];
    memset(fixed_msg, 0x5A, 64);
    for (int i = 0; i < N; i++) fill_random(rand_msg[i], 64);

    volatile uint8_t sink = 0;
    for (int i = 0; i < 500; i++) sha256(fixed_msg, 64, out);
    for (int t = 0; t < 200; t++) sha256(fixed_msg, 64, out);
    for (int t = 0; t < 2 * N; t++) {
        int fixed = (t & 1) == 0;
        const uint8_t *m = fixed ? fixed_msg : rand_msg[t >> 1];
        uint64_t best = ~0ull;
        for (int k = 0; k < MIN_K; k++) {
            uint64_t a = now();
            sha256(m, 64, out);
            uint64_t b = now();
            uint64_t d = b - a;
            if (d < best) best = d;
        }
        sink ^= out[0];
        printf("sha256 %c %lld\n", fixed ? 'F' : 'R', (long long)best);
    }
    fprintf(stderr, "sha256 sink=%u\n", (unsigned)sink);
}

/* ---- aes_gcm (vary key + plaintext together; IV fixed) ---- */
static void bench_aesgcm(void) {
    uint8_t fixed_key[16], fixed_pt[16], iv[12], ct[16], tag[16];
    static uint8_t rand_key[N][16], rand_pt[N][16];
    aes_gcm_ctx ctx;
    memset(fixed_key, 0x11, 16);
    memset(fixed_pt, 0x22, 16);
    memset(iv, 0x33, 12);
    for (int i = 0; i < N; i++) { fill_random(rand_key[i], 16); fill_random(rand_pt[i], 16); }

    volatile uint8_t sink = 0;
    for (int i = 0; i < 500; i++) { aes_gcm_init(&ctx, fixed_key, iv, 12); aes_gcm_encrypt(&ctx, NULL, 0, fixed_pt, 16, ct, tag); }
    for (int t = 0; t < 200; t++) { aes_gcm_init(&ctx, fixed_key, iv, 12); aes_gcm_encrypt(&ctx, NULL, 0, fixed_pt, 16, ct, tag); }
    for (int t = 0; t < 2 * N; t++) {
        int fixed = (t & 1) == 0;
        const uint8_t *k = fixed ? fixed_key : rand_key[t >> 1];
        const uint8_t *p = fixed ? fixed_pt : rand_pt[t >> 1];
        uint64_t best = ~0ull;
        for (int r = 0; r < MIN_K; r++) {
            uint64_t a = now();
            aes_gcm_init(&ctx, k, iv, 12);
            aes_gcm_encrypt(&ctx, NULL, 0, p, 16, ct, tag);
            uint64_t b = now();
            uint64_t d = b - a;
            if (d < best) best = d;
        }
        sink ^= tag[0];
        printf("aesgcm %c %lld\n", fixed ? 'F' : 'R', (long long)best);
    }
    fprintf(stderr, "aesgcm sink=%u\n", (unsigned)sink);
}

int main(int argc, char **argv) {
    if (argc < 2) { fprintf(stderr, "usage: %s <x25519|sha256|aesgcm>\n", argv[0]); return 2; }
    if (argc >= 3) MIN_K = atoi(argv[2]);
    if (MIN_K < 1) MIN_K = 1;

    /* KAT gate: verify each primitive against its RFC/NIST vectors first. */
    char diag[512];
    if (x25519_self_test(diag, sizeof diag) != 0) { fprintf(stderr, "x25519 self-test FAIL\n%s\n", diag); return 1; }
    if (sha256_self_test(diag, sizeof diag) != 0) { fprintf(stderr, "sha256 self-test FAIL\n%s\n", diag); return 1; }
    if (aes_gcm_self_test(diag, sizeof diag) != 0) { fprintf(stderr, "aes_gcm self-test FAIL\n%s\n", diag); return 1; }
    fprintf(stderr, "KAT self-tests: PASS\n");

    if (strcmp(argv[1], "x25519") == 0) bench_x25519();
    else if (strcmp(argv[1], "sha256") == 0) bench_sha256();
    else if (strcmp(argv[1], "aesgcm") == 0) bench_aesgcm();
    else { fprintf(stderr, "unknown bench\n"); return 2; }
    return 0;
}
